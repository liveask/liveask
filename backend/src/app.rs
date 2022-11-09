use anyhow::{bail, Result};
use axum::extract::ws::{Message, WebSocket};
use shared::{AddEvent, EventInfo, EventState, EventTokens, Item, ModQuestion, States};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
    time::Instant,
};
use tinyurl_rs::{CreateRequest, TinyUrlAPI, TinyUrlOpenAPI};
use tokio::sync::{mpsc, RwLock};
use tracing::instrument;
use ulid::Ulid;

use crate::{
    env,
    eventsdb::{EventEntry, EventsDB},
};

#[derive(Clone)]
pub struct App {
    eventsdb: Arc<dyn EventsDB>,
    channels: Arc<RwLock<HashMap<usize, (String, OutBoundChannel)>>>,
    base_url: String,
    tiny_url_token: String,
}

impl App {
    pub fn new(eventsdb: Arc<dyn EventsDB>) -> Self {
        Self {
            eventsdb,
            channels: Default::default(),
            base_url: std::env::var(env::ENV_BASE_URL)
                .unwrap_or_else(|_| "https://www.live-ask.com".into()),
            tiny_url_token: std::env::var(env::ENV_TINY_TOKEN).unwrap_or_default(),
        }
    }
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

type OutBoundChannel =
    mpsc::UnboundedSender<std::result::Result<axum::extract::ws::Message, axum::Error>>;

impl App {
    #[instrument(skip(self))]
    async fn shorten_url(&self, url: &str) -> String {
        let tiny = TinyUrlAPI {
            token: self.tiny_url_token.clone(),
        };

        let now = Instant::now();
        let res = tiny
            .create(CreateRequest::new(url.to_string()))
            .await
            .map(|res| res.data.unwrap().tiny_url)
            .unwrap_or_default();

        tracing::info!("tiny url: '{}' (in {}ms)", res, now.elapsed().as_millis());

        res
    }

    pub async fn create_event(&self, request: AddEvent) -> Result<EventInfo> {
        let mut validation = shared::CreateEventErrors::default();

        validation.check(&request.data.name, &request.data.description);

        if validation.has_any() {
            bail!("request validation failed");
        }

        let mut e = EventInfo {
            //TODO:
            create_time_unix: 0,
            delete_time_unix: 0,
            last_edit_unix: 0,
            create_time_utc: String::new(),
            deleted: false,
            questions: Vec::new(),
            state: EventState {
                state: States::Open,
            },
            data: request.data,
            tokens: EventTokens {
                public_token: Ulid::new().to_string(),
                moderator_token: Some(Ulid::new().to_string()),
            },
        };

        let url = format!("{}/event/{}", self.base_url, e.tokens.public_token);

        e.data.short_url = self.shorten_url(&url).await;
        e.data.long_url = Some(url);

        let result = e.clone();

        self.eventsdb.put(EventEntry::new(e)).await?;

        Ok(result)
    }

    pub async fn get_event(&self, id: String, secret: Option<String>) -> Result<EventInfo> {
        let mut e = self.eventsdb.get(&id).await?.clone().event;

        if let Some(secret) = &secret {
            if e.tokens
                .moderator_token
                .as_ref()
                .map(|mod_token| mod_token != secret)
                .unwrap_or_default()
            {
                bail!("wrong mod token");
            }
        }

        if e.deleted {
            bail!("ev not found");
        }

        if secret.is_none() {
            e.tokens.moderator_token = Some(String::new());

            e.questions = e
                .questions
                .into_iter()
                .filter(|q| !q.hidden)
                .collect::<Vec<_>>();
        }

        Ok(e)
    }

    pub async fn get_question(
        &self,
        id: String,
        secret: Option<String>,
        question_id: i64,
    ) -> Result<Item> {
        let e = self.eventsdb.get(&id).await?.clone().event;

        let can_see_hidden = e
            .tokens
            .moderator_token
            .clone()
            .zip(secret.clone())
            .map(|tokens| tokens.0 != tokens.1)
            .unwrap_or_default();

        let q = e
            .questions
            .iter()
            .find(|q| q.id == question_id)
            .ok_or_else(|| anyhow::anyhow!("q not found"))?
            .clone();

        if q.hidden && !can_see_hidden {
            bail!("q not found")
        }

        self.notify_subscribers(id, None).await;

        Ok(q)
    }

    pub async fn mod_edit_question(
        &self,
        id: String,
        secret: String,
        question_id: i64,
        state: ModQuestion,
    ) -> Result<EventInfo> {
        tracing::info!("mod_edit_question: {:?}", state);

        let mut entry = self.eventsdb.get(&id).await?;
        {
            let e = &mut entry.event;

            if e.tokens
                .moderator_token
                .as_ref()
                .map(|mod_token| mod_token != &secret)
                .unwrap_or_default()
            {
                bail!("wrong mod token");
            }

            let q = e
                .questions
                .iter_mut()
                .find(|q| q.id == question_id)
                .ok_or_else(|| anyhow::anyhow!("q not found"))?;

            q.hidden = state.hide;
            q.answered = state.answered;
        }

        entry.bump_version();

        let e = entry.event.clone();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(id, None).await;

        Ok(e)
    }

    pub async fn edit_event_state(
        &self,
        id: String,
        secret: String,
        state: EventState,
    ) -> Result<EventInfo> {
        let mut entry = self.eventsdb.get(&id).await?.clone();

        let e = &mut entry.event;
        if e.tokens
            .moderator_token
            .as_ref()
            .map(|mod_token| mod_token != &secret)
            .unwrap_or_default()
        {
            bail!("wrong mod token");
        }

        e.state = state;

        let result = e.clone();

        entry.bump_version();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(id, None).await;

        Ok(result)
    }

    pub async fn delete_event(&self, id: String, secret: String) -> Result<()> {
        let mut entry = self.eventsdb.get(&id).await?;

        let e = &mut entry.event;

        if e.tokens
            .moderator_token
            .as_ref()
            .map(|mod_token| mod_token != &secret)
            .unwrap_or_default()
        {
            bail!("wrong mod token");
        }

        e.deleted = true;

        entry.bump_version();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(id, None).await;

        Ok(())
    }

    pub async fn add_question(&self, id: String, question: shared::AddQuestion) -> Result<Item> {
        let mut entry = self.eventsdb.get(&id).await?;

        let e = &mut entry.event;

        let question_id = e.questions.len() as i64;

        let question = shared::Item {
            text: question.text,
            answered: false,
            create_time_unix: 0,
            hidden: false,
            id: question_id,
            likes: 1,
        };

        e.questions.push(question.clone());

        entry.bump_version();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(id, Some(question_id)).await;

        Ok(question)
    }

    pub async fn edit_like(&self, id: String, edit: shared::EditLike) -> Result<Item> {
        let mut entry = self.eventsdb.get(&id).await?.clone();

        let e = &mut entry.event;

        if let Some(f) = e.questions.iter_mut().find(|e| e.id == edit.question_id) {
            f.likes = if edit.like {
                f.likes + 1
            } else {
                f.likes.saturating_sub(1)
            };

            let res = f.clone();

            entry.bump_version();

            self.eventsdb.put(entry).await?;

            Ok(res)
        } else {
            bail!("question not found")
        }
    }

    pub async fn push_subscriber(&self, ws: WebSocket, id: String) {
        use futures_util::StreamExt;

        let (ws_sender, mut ws_receiver) = ws.split();

        let user_id = NEXT_USER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let send_channel = Self::create_send_channel(ws_sender);

        self.channels
            .write()
            .await
            .insert(user_id, (id.clone(), send_channel));

        while let Some(result) = ws_receiver.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::warn!("websocket receive err (id={}): '{}'", user_id, e);
                    break;
                }
            };

            tracing::warn!("user:{} sent data, disconnecting", user_id);

            match msg {
                Message::Ping(_) => tracing::warn!("received msg:ping"),
                Message::Pong(_) => tracing::warn!("received msg:pong"),
                Message::Text(txt) => tracing::warn!("received msg:text: '{txt}'"),
                Message::Binary(bin) => tracing::warn!("received msg:binary: {}b", bin.len()),
                Message::Close(frame) => tracing::warn!("received msg:close: {frame:?}"),
            }
        }

        tracing::debug!("user disconnected: {}", user_id);

        self.channels.write().await.remove(&user_id);
    }

    fn create_send_channel(
        ws_sender: futures_util::stream::SplitSink<WebSocket, axum::extract::ws::Message>,
    ) -> OutBoundChannel {
        use futures_util::FutureExt;
        use futures_util::StreamExt;
        use tokio_stream::wrappers::UnboundedReceiverStream;

        let (sender, receiver) = mpsc::unbounded_channel();
        let rx = UnboundedReceiverStream::new(receiver);

        tokio::task::spawn(rx.forward(ws_sender).map(|result| {
            if let Err(e) = result {
                tracing::error!("websocket send error: {}", e);
            }
        }));

        sender
    }

    async fn notify_subscribers(&self, event_id: String, question_id: Option<i64>) {
        let channels = self.channels.clone();

        let msg = if let Some(q) = question_id {
            Message::Text(format!("q:{}", q))
        } else {
            Message::Text("e".to_string())
        };

        tokio::spawn(async move {
            for (_user_id, (_id, c)) in channels
                .read()
                .await
                .iter()
                .filter(|(_, (id, _))| id == &event_id)
            {
                c.send(Ok(msg.clone())).unwrap();
            }
        })
        .await
        .unwrap();
    }
}
