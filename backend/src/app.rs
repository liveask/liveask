use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::{bail, Result};
use axum::extract::ws::{Message, WebSocket};
use shared::{AddEvent, EventInfo, EventState, EventTokens, Item, ModQuestion};
use tokio::sync::{mpsc, RwLock};
use ulid::Ulid;

#[derive(Clone, Default, Debug)]
pub struct App {
    events: Arc<RwLock<HashMap<String, EventInfo>>>,
    channels: Arc<RwLock<HashMap<usize, (String, OutBoundChannel)>>>,
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

type OutBoundChannel =
    mpsc::UnboundedSender<std::result::Result<axum::extract::ws::Message, axum::Error>>;

impl App {
    pub async fn create_event(&self, request: AddEvent) -> Result<EventInfo> {
        let e = EventInfo {
            //TODO:
            create_time_unix: 0,
            delete_time_unix: 0,
            last_edit_unix: 0,
            create_time_utc: String::new(),
            deleted: false,
            questions: Vec::new(),
            state: None,
            data: request.data,
            tokens: EventTokens {
                public_token: Ulid::new().to_string(),
                moderator_token: Some(Ulid::new().to_string()),
            },
        };

        self.events
            .write()
            .await
            .insert(e.tokens.public_token.clone(), e.clone());

        Ok(e)
    }

    pub async fn get_event(&self, id: String, secret: Option<String>) -> Result<EventInfo> {
        let mut e = self
            .events
            .read()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

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

        if secret.is_none() {
            e.tokens.moderator_token = Some(String::new());
        }

        Ok(e)
    }

    pub async fn get_question(
        &self,
        id: String,
        secret: Option<String>,
        question_id: i64,
    ) -> Result<Item> {
        let e = self
            .events
            .read()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

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
        let mut e = self
            .events
            .write()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

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

        self.notify_subscribers(id, None).await;

        Ok(e)
    }

    pub async fn edit_event_state(
        &self,
        id: String,
        secret: String,
        state: EventState,
    ) -> Result<EventInfo> {
        let mut e = self
            .events
            .write()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

        if e.tokens
            .moderator_token
            .as_ref()
            .map(|mod_token| mod_token != &secret)
            .unwrap_or_default()
        {
            bail!("wrong mod token");
        }

        e.state = Some(state);

        self.notify_subscribers(id, None).await;

        Ok(e)
    }

    pub async fn delete_event(&self, id: String, secret: String) -> Result<EventInfo> {
        let mut e = self
            .events
            .write()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

        if e.tokens
            .moderator_token
            .as_ref()
            .map(|mod_token| mod_token != &secret)
            .unwrap_or_default()
        {
            bail!("wrong mod token");
        }

        e.deleted = true;

        self.notify_subscribers(id, None).await;

        Ok(e)
    }

    pub async fn add_question(&self, id: String, question: shared::AddQuestion) -> Result<Item> {
        let mut events = self.events.write().await;

        let question_id = events
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .questions
            .len() as i64;

        let question = shared::Item {
            text: question.text,
            answered: false,
            create_time_unix: 0,
            hidden: false,
            id: question_id,
            likes: 1,
        };

        events
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .questions
            .push(question.clone());

        self.notify_subscribers(id, Some(question_id)).await;

        Ok(question)
    }

    pub async fn edit_like(&self, id: String, edit: shared::EditLike) -> Result<Item> {
        let mut e = self
            .events
            .read()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

        if let Some(f) = e.questions.iter_mut().find(|e| e.id == edit.question_id) {
            f.likes = if edit.like {
                f.likes + 1
            } else {
                f.likes.saturating_sub(1)
            };

            let res = f.clone();

            self.events.write().await.insert(id, e.clone());

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
            let _msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::warn!("websocket receive err (id={}): '{}'", user_id, e);
                    break;
                }
            };

            tracing::warn!("user:{} sent data, disconnecting", user_id);

            break;
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
