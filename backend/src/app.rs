use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use shared::{
    AddEvent, EventInfo, EventState, EventTokens, EventUpgrade, ModQuestion, QuestionItem, States,
};
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
    bail, env,
    error::{InternalError, Result},
    eventsdb::{EventEntry, EventsDB},
    mail::MailjetConfig,
    payment::Payment,
    pubsub::{PubSubPublish, PubSubReceiver},
    utils::{format_timestamp, timestamp_now},
};

pub type SharedApp = Arc<App>;

#[derive(Clone)]
pub struct App {
    eventsdb: Arc<dyn EventsDB>,
    //TODO: order subscriber based on topic name into Concurrent Hashmap
    channels: Arc<RwLock<HashMap<usize, (String, OutBoundChannel)>>>,
    pubsub_publish: Arc<dyn PubSubPublish>,
    payment: Arc<Payment>,
    base_url: String,
    tiny_url_token: Option<String>,
    mailjet_config: Option<MailjetConfig>,
}

impl App {
    pub fn new(
        eventsdb: Arc<dyn EventsDB>,
        pubsub_publish: Arc<dyn PubSubPublish>,
        payment: Arc<Payment>,
        base_url: String,
    ) -> Self {
        let tiny_url_token = std::env::var(env::ENV_TINY_TOKEN).ok();

        if tiny_url_token.is_none() {
            tracing::warn!("no url shorten token set, use `ENV_TINY_TOKEN` to do so");
        }

        let mailjet_config = MailjetConfig::new();

        if mailjet_config.is_some() {
            tracing::info!("mail configured");
        } else {
            tracing::warn!("mail not configured");
        }

        Self {
            eventsdb,
            pubsub_publish,
            channels: std::sync::Arc::default(),
            base_url,
            tiny_url_token,
            mailjet_config,
            payment,
        }
    }
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

type OutBoundChannel =
    mpsc::UnboundedSender<std::result::Result<axum::extract::ws::Message, axum::Error>>;

impl App {
    #[instrument(skip(self))]
    async fn shorten_url(&self, url: &str) -> String {
        if let Some(tiny_url_token) = &self.tiny_url_token {
            let tiny = TinyUrlAPI {
                token: tiny_url_token.clone(),
            };

            let now = Instant::now();
            let res = tiny
                .create(CreateRequest::new(url.to_string()))
                .await
                .map(|res| res.data.map(|url| url.tiny_url).unwrap_or_default())
                .unwrap_or_default();

            tracing::info!("tiny url: '{}' (in {}ms)", res, now.elapsed().as_millis());

            res
        } else {
            tracing::warn!("no url shorten token set");

            url.to_owned()
        }
    }

    #[instrument(skip(self, request))]
    pub async fn create_event(&self, request: AddEvent) -> Result<EventInfo> {
        let mut validation = shared::CreateEventErrors::default();

        validation.check(&request.data.name, &request.data.description);

        if validation.has_any() {
            bail!("request validation failed: {:?}", validation);
        }

        let now = timestamp_now();

        let mod_token = Ulid::new().to_string();

        let mut e = EventInfo {
            create_time_unix: now,
            delete_time_unix: 0,
            last_edit_unix: now,
            create_time_utc: format_timestamp(now),
            deleted: false,
            questions: Vec::new(),
            state: EventState {
                state: States::Open,
            },
            data: request.data,
            tokens: EventTokens {
                public_token: Ulid::new().to_string(),
                moderator_token: Some(mod_token.clone()),
            },
        };

        let url = format!("{}/event/{}", self.base_url, e.tokens.public_token);

        e.data.short_url = self.shorten_url(&url).await;
        e.data.long_url = Some(url);

        let result = e.clone();

        self.eventsdb.put(EventEntry::new(e)).await?;

        self.send_mail(
            request.moderator_email,
            result.data.name.clone(),
            result.data.short_url.clone(),
            self.mod_link(&result.tokens),
        )
        .await;

        Ok(result)
    }

    fn mod_link(&self, tokens: &EventTokens) -> String {
        let mod_token = tokens
            .moderator_token
            .as_ref()
            .map_or_else(String::new, std::clone::Clone::clone);

        format!(
            "{}/eventmod/{}/{mod_token}",
            self.base_url, tokens.public_token,
        )
    }

    pub async fn get_event(&self, id: String, secret: Option<String>) -> Result<EventInfo> {
        let mut e = self.eventsdb.get(&id).await?.event;

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
            return Err(InternalError::AccessingDeletedEvent);
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

    //TODO: validate event is not deleted
    pub async fn get_question(
        &self,
        id: String,
        secret: Option<String>,
        question_id: i64,
    ) -> Result<QuestionItem> {
        let e = self.eventsdb.get(&id).await?.event;

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
            .ok_or_else(|| InternalError::General("q not found".into()))?
            .clone();

        if q.hidden && !can_see_hidden {
            bail!("q not found")
        }

        self.notify_subscribers(id, None).await;

        Ok(q)
    }

    //TODO: validate event is not deleted
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
                .ok_or_else(|| InternalError::General("q not found".into()))?;

            q.hidden = state.hide;
            q.answered = state.answered;
        }

        entry.bump();

        let e = entry.event.clone();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(id, None).await;

        Ok(e)
    }

    //TODO: validate event is not deleted
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

        entry.bump();

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
        e.delete_time_unix = timestamp_now();

        entry.bump();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(id, None).await;

        Ok(())
    }

    pub async fn premium_upgrade(&self, id: String, secret: String) -> Result<EventUpgrade> {
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

        if e.deleted {
            return Err(InternalError::AccessingDeletedEvent);
        }

        let approve_url = self
            .payment
            .create_order(e.tokens.public_token.clone(), self.mod_link(&e.tokens))
            .await?;

        Ok(EventUpgrade { url: approve_url })
    }

    #[instrument(skip(self))]
    pub async fn payment_webhook(&self, id: String) -> Result<()> {
        tracing::info!("order processing");

        let event_id = self.payment.capture_approved_payment(id.clone()).await?;

        tracing::info!("order processing done");

        if let Some(_event_id) = event_id {
            //TODO: mark event premium

            // self.notify_subscribers(id, Some(question_id)).await;
        }

        Ok(())
    }

    //TODO: validate event is still open
    pub async fn add_question(
        &self,
        id: String,
        question: shared::AddQuestion,
    ) -> Result<QuestionItem> {
        let mut entry = self.eventsdb.get(&id).await?;

        let e = &mut entry.event;

        let question_id = e.questions.len() as i64;

        let question = shared::QuestionItem {
            text: question.text,
            answered: false,
            create_time_unix: timestamp_now(),
            hidden: false,
            id: question_id,
            likes: 1,
        };

        e.questions.push(question.clone());

        entry.bump();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(id, Some(question_id)).await;

        Ok(question)
    }

    //TODO: validate event is still votable
    pub async fn edit_like(&self, id: String, edit: shared::EditLike) -> Result<QuestionItem> {
        let mut entry = self.eventsdb.get(&id).await?.clone();

        let e = &mut entry.event;

        if let Some(f) = e.questions.iter_mut().find(|e| e.id == edit.question_id) {
            f.likes = if edit.like {
                f.likes + 1
            } else {
                f.likes.saturating_sub(1)
            };

            let res = f.clone();

            entry.bump();

            self.eventsdb.put(entry).await?;

            self.notify_subscribers(id, Some(edit.question_id)).await;

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

        tracing::info!(
            "user connected: {} ({} total)",
            user_id,
            self.channels.read().await.len()
        );

        while let Some(result) = ws_receiver.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::info!("websocket receive err (id={}): '{}'", user_id, e);
                    break;
                }
            };

            //allow receiving `p` for app based pings
            if matches!(&msg, Message::Text(text) if text=="p") {
                continue;
            }

            match &msg {
                //TODO: do we need to respond manually?
                Message::Ping(_) => tracing::info!("received msg:ping"),
                Message::Pong(_) => tracing::warn!("received msg:pong"),
                Message::Text(txt) => tracing::warn!("received msg:text: '{txt}'"),
                Message::Binary(bin) => tracing::warn!("received msg:binary: {}b", bin.len()),
                Message::Close(frame) => tracing::info!("received msg:close: {frame:?}"),
            }

            let (disconnect, sent_data) = match msg {
                Message::Ping(_) | Message::Pong(_) => (false, false),
                Message::Close(_) => (true, false),
                _ => (true, true),
            };

            if disconnect {
                if sent_data {
                    tracing::warn!("user:{} sent data, disconnecting", user_id);
                }
                break;
            }
        }

        tracing::info!(
            "user disconnected: {} ({} remain)",
            user_id,
            self.channels.read().await.len().saturating_sub(1)
        );

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
                if e.to_string() != "Connection closed normally" {
                    tracing::error!("websocket send error: {}", e);
                }
            }
        }));

        sender
    }

    async fn notify_subscribers(&self, event_id: String, question_id: Option<i64>) {
        let msg = question_id.map_or_else(|| "e".to_string(), |q| format!("q:{q}"));

        self.pubsub_publish.publish(&event_id, &msg).await;
    }

    async fn send_mail(
        &self,
        receiver: String,
        event_name: String,
        public_link: String,
        mod_link: String,
    ) {
        if receiver.trim().is_empty() {
            return;
        }

        self.mailjet_config.clone().map_or_else(
            || {
                tracing::warn!("mail not send: not configured");
            },
            |mail| {
                tracing::debug!("mail sending to: {receiver}");

                tokio::spawn(async move {
                    if let Err(e) = mail
                        .send_mail(receiver.clone(), event_name, public_link, mod_link)
                        .await
                    {
                        tracing::error!("mail send error: {e}");
                    }
                });
            },
        );
    }
}

#[async_trait]
impl PubSubReceiver for App {
    async fn notify(&self, topic: &str, payload: &str) {
        let topic = topic.to_string();
        let msg = Message::Text(payload.to_string());

        let channels = self.channels.clone();

        if let Err(e) = tokio::spawn(async move {
            //TODO: lookup subscriber based on topic name
            for (_user_id, (_id, c)) in channels
                .read()
                .await
                .iter()
                .filter(|(_, (id, _))| id == &topic)
            {
                if let Err(e) = c.send(Ok(msg.clone())) {
                    tracing::error!("pubsub send error: {}", e);
                }
            }
        })
        .await
        {
            tracing::error!("pubsub notify error: {e}");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{eventsdb::InMemoryEventsDB, pubsub::PubSubInMemory};
    use shared::EventData;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_event_create_fail_validation() {
        let app = App::new(
            Arc::new(InMemoryEventsDB::default()),
            Arc::new(PubSubInMemory::default()),
            Arc::new(Payment::default()),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    max_likes: 0,
                    name: String::from("too short"),
                    description: String::new(),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: String::new(),
            })
            .await;

        assert!(res.is_err());
    }
}
