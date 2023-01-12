use async_trait::async_trait;
use axum::extract::ws::{close_code::RESTART, CloseFrame, Message, WebSocket};
use shared::{
    AddEvent, EventInfo, EventState, EventTokens, EventUpgrade, GetEventResponse, ModQuestion,
    QuestionItem, States,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tinyurl_rs::{CreateRequest, TinyUrlAPI, TinyUrlOpenAPI};
use tokio::{
    sync::{mpsc, RwLock},
    time::sleep,
};
use tracing::instrument;
use ulid::Ulid;

use crate::{
    bail, env,
    error::{InternalError, Result},
    eventsdb::{ApiEventInfo, EventEntry, EventsDB},
    mail::MailjetConfig,
    payment::Payment,
    pubsub::{PubSubPublish, PubSubReceiver},
    utils::timestamp_now,
    viewers::Viewers,
};

pub type SharedApp = Arc<App>;

enum Notification {
    Event,
    Question(i64),
    Viewers(i64),
}

#[derive(Clone)]
pub struct App {
    eventsdb: Arc<dyn EventsDB>,
    //TODO: order subscriber based on topic name into Concurrent Hashmap
    channels: Arc<RwLock<HashMap<usize, (String, OutBoundChannel)>>>,
    shutdown: Arc<AtomicBool>,
    pubsub_publish: Arc<dyn PubSubPublish>,
    viewers: Arc<dyn Viewers>,
    payment: Arc<Payment>,
    base_url: String,
    tiny_url_token: Option<String>,
    mailjet_config: Option<MailjetConfig>,
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

type OutBoundChannel =
    mpsc::UnboundedSender<std::result::Result<axum::extract::ws::Message, axum::Error>>;

impl App {
    pub fn new(
        eventsdb: Arc<dyn EventsDB>,
        pubsub_publish: Arc<dyn PubSubPublish>,
        viewers: Arc<dyn Viewers>,
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
            channels: Arc::default(),
            base_url,
            tiny_url_token,
            mailjet_config,
            payment,
            viewers,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("shutting down..");

        self.shutdown.store(true, Ordering::Relaxed);

        loop {
            let count = self.channels.read().await.len();

            if count == 0 {
                break;
            }

            tracing::info!("shutting down: {count} ws connections remain");

            sleep(Duration::from_secs(1)).await;
        }

        tracing::info!("shutting down.. done");

        Ok(())
    }

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

        let mut e = ApiEventInfo {
            create_time_unix: now,
            delete_time_unix: 0,
            last_edit_unix: now,
            deleted: false,
            premium_order: None,
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

        //TODO: put in request alread at right place
        e.data.mail = if request.moderator_email.is_empty() {
            None
        } else {
            Some(request.moderator_email.clone())
        };

        let url = format!("{}/event/{}", self.base_url, e.tokens.public_token);

        e.data.short_url = self.shorten_url(&url).await;
        e.data.long_url = Some(url);

        let result = e.clone();

        self.eventsdb
            .put(EventEntry::new(e, request.test.then_some(now + 60)))
            .await?;

        self.send_mail(
            request.moderator_email,
            result.data.name.clone(),
            result.data.short_url.clone(),
            self.mod_link(&result.tokens),
        )
        .await;

        Ok(result.into())
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

    #[instrument(skip(self))]
    pub async fn get_event(&self, id: String, secret: Option<String>) -> Result<GetEventResponse> {
        tracing::info!("get_event");

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
            return Err(InternalError::AccessingDeletedEvent(id));
        }

        if secret.is_none() {
            //TODO: can be NONE?
            e.tokens.moderator_token = Some(String::new());

            e.questions = e
                .questions
                .into_iter()
                .filter(|q| !q.hidden)
                .collect::<Vec<_>>();
        }

        let timed_out = e.is_timed_out_and_free();
        let viewers = self.viewers.count(&id).await;
        Ok(GetEventResponse {
            info: e.into(),
            timed_out,
            viewers,
        })
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

            if e.is_timed_out_and_free() {
                return Err(InternalError::TimedOutFreeEvent(id));
            }

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

        self.notify_subscribers(id, Notification::Question(question_id))
            .await;

        Ok(e.into())
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

        if e.is_timed_out_and_free() {
            return Err(InternalError::TimedOutFreeEvent(id));
        }

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

        self.notify_subscribers(id, Notification::Event).await;

        Ok(result.into())
    }

    //TODO: keep allowing to delete timedout event?
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

        self.notify_subscribers(id, Notification::Event).await;

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
            return Err(InternalError::AccessingDeletedEvent(id));
        }

        let approve_url = self
            .payment
            .create_order(
                e.tokens.public_token.clone(),
                format!("{}?payment=true", self.mod_link(&e.tokens)),
            )
            .await?;

        Ok(EventUpgrade { url: approve_url })
    }

    #[instrument(skip(self))]
    pub async fn payment_webhook(&self, id: String) -> Result<()> {
        tracing::info!("order processing");

        let event_id = self.payment.event_of_order(id.clone()).await?;

        let mut entry = self.eventsdb.get(&event_id).await?;

        if entry.event.premium_order.is_some() {
            return Err(InternalError::General("event already premium".into()));
        }

        if self.payment.capture_payment(id.clone()).await? {
            tracing::info!("order captured");

            entry.event.premium_order = Some(id);

            entry.bump();

            self.eventsdb.put(entry).await?;

            self.notify_subscribers(event_id, Notification::Event).await;
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

        if e.is_timed_out_and_free() {
            return Err(InternalError::TimedOutFreeEvent(id));
        }

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

        self.notify_subscribers(id, Notification::Question(question_id))
            .await;

        Ok(question)
    }

    //TODO: validate event is still votable
    pub async fn edit_like(&self, id: String, edit: shared::EditLike) -> Result<QuestionItem> {
        let mut entry = self.eventsdb.get(&id).await?.clone();

        let e = &mut entry.event;

        if e.is_timed_out_and_free() {
            return Err(InternalError::TimedOutFreeEvent(id));
        }

        if let Some(f) = e.questions.iter_mut().find(|e| e.id == edit.question_id) {
            f.likes = if edit.like {
                f.likes + 1
            } else {
                f.likes.saturating_sub(1)
            };

            let res = f.clone();

            entry.bump();

            self.eventsdb.put(entry).await?;

            self.notify_subscribers(id, Notification::Question(edit.question_id))
                .await;

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
            .insert(user_id, (id.clone(), send_channel.clone()));

        self.viewers.add(&id).await;

        self.notify_viewer_count_change(&id);

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
            if !matches!(&msg, Message::Text(text) if text=="p") {
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

            if self.is_shutting_down() {
                tracing::info!("shutdown: close client connection [{user_id}]");

                if let Err(e) = send_channel.send(Ok(Message::Close(Some(CloseFrame {
                    code: RESTART,
                    reason: "server shutdown".into(),
                })))) {
                    tracing::error!("shutdown error: {e}");
                }

                break;
            }
        }

        tracing::info!(
            "user disconnected: {} ({} remain)",
            user_id,
            self.channels.read().await.len().saturating_sub(1)
        );

        //Note: lets not spam everyone if its a shutdown
        if !self.is_shutting_down() {
            self.notify_viewer_count_change(&id);
        }

        self.viewers.remove(&id).await;

        self.channels.write().await.remove(&user_id);
    }

    fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    fn notify_viewer_count_change(&self, event: &str) {
        let event = event.to_string();
        let app = self.clone();

        tokio::spawn(async move {
            let count = app.viewers.count(&event).await;

            tracing::info!("notify viewer count: {count}");

            app.notify_subscribers(event, Notification::Viewers(count))
                .await;
        });
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

    async fn notify_subscribers(&self, event_id: String, n: Notification) {
        let msg = match n {
            Notification::Event => "e".to_string(),
            Notification::Question(id) => format!("q:{id}"),
            Notification::Viewers(count) => format!("v:{count}"),
        };

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
    use crate::{
        eventsdb::InMemoryEventsDB,
        pubsub::{PubSubInMemory, PubSubReceiverInMemory},
    };
    use pretty_assertions::assert_eq;
    use shared::{AddQuestion, EventData};
    use std::sync::Arc;

    mockall::mock! {
        pub TestViewers {}
        #[async_trait]
        impl Viewers for TestViewers{
            async fn count(&self, key: &str) -> i64;
            async fn add(&self, key: &str);
            async fn remove(&self, key: &str);
        }
    }

    #[tokio::test]
    async fn test_event_create_fail_validation() {
        let app = App::new(
            Arc::new(InMemoryEventsDB::default()),
            Arc::new(PubSubInMemory::default()),
            Arc::new(MockTestViewers::new()),
            Arc::new(Payment::default()),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    mail: None,
                    name: String::from("too short"),
                    description: String::new(),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: String::new(),
                test: false,
            })
            .await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_event_create() {
        let eventdb = Arc::new(InMemoryEventsDB::default());
        let app = App::new(
            eventdb.clone(),
            Arc::new(PubSubInMemory::default()),
            Arc::new(MockTestViewers::new()),
            Arc::new(Payment::default()),
            String::new(),
        );

        app.create_event(AddEvent {
            data: EventData {
                mail: None,
                name: String::from("123456789"),
                description: String::from("123456789 123456789 123456789 !"),
                short_url: String::new(),
                long_url: None,
            },
            moderator_email: String::new(),
            test: false,
        })
        .await
        .unwrap();

        assert_eq!(eventdb.db.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn test_event_mod_question_notification() {
        let pubsubreceiver = Arc::new(PubSubReceiverInMemory::default());
        let pubsub = PubSubInMemory::default();
        pubsub.set_receiver(pubsubreceiver.clone()).await;
        let app = App::new(
            Arc::new(InMemoryEventsDB::default()),
            Arc::new(pubsub),
            Arc::new(MockTestViewers::new()),
            Arc::new(Payment::default()),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    mail: None,
                    name: String::from("123456789"),
                    description: String::from("123456789 123456789 123456789 !"),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: String::new(),
                test: false,
            })
            .await
            .unwrap();

        let q = app
            .add_question(
                res.tokens.public_token.clone(),
                AddQuestion {
                    text: String::from("question"),
                },
            )
            .await
            .unwrap();

        app.mod_edit_question(
            res.tokens.public_token.clone(),
            res.tokens.moderator_token.unwrap(),
            q.id,
            ModQuestion {
                hide: true,
                answered: false,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            pubsubreceiver.log.read().await[0].clone(),
            (res.tokens.public_token.clone(), format!("q:{}", q.id))
        );

        assert_eq!(
            pubsubreceiver.log.read().await[1].clone(),
            (res.tokens.public_token.clone(), format!("q:{}", q.id))
        );
    }
}
