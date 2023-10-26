use async_trait::async_trait;
use axum::extract::ws::{close_code::RESTART, CloseFrame, Message, WebSocket};
use shared::{
    AddEvent, EventInfo, EventState, EventTokens, EventUpgrade, GetEventResponse, ModQuestion,
    PaymentCapture, QuestionItem, States,
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
    mail::MailConfig,
    payment::Payment,
    pubsub::{PubSubPublish, PubSubReceiver},
    tracking::Tracking,
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
    tracking: Tracking,
    base_url: String,
    tiny_url_token: Option<String>,
    mail_config: MailConfig,
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
        tracking: Tracking,
        base_url: String,
    ) -> Self {
        let tiny_url_token = Self::tinyurl_token();

        let mail_config = MailConfig::new();

        Self {
            eventsdb,
            pubsub_publish,
            channels: Arc::default(),
            base_url,
            tiny_url_token,
            mail_config,
            payment,
            viewers,
            tracking,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    fn tinyurl_token() -> Option<String> {
        let tiny_url_token = std::env::var(env::ENV_TINY_TOKEN).ok();

        if tiny_url_token.clone().unwrap_or_default().trim().is_empty() {
            tracing::warn!("no url shorten token set, use `ENV_TINY_TOKEN` to do so");
        } else {
            tracing::info!(
                "tinyurl-token set (len: {})",
                tiny_url_token.clone().unwrap_or_default().trim().len()
            );
        }

        tiny_url_token
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
            if !tiny_url_token.trim().is_empty() {
                let tiny = TinyUrlAPI {
                    token: tiny_url_token.clone(),
                };

                let now = Instant::now();
                let res = tiny
                    .create(CreateRequest::new(url.to_string()))
                    .await
                    .map(|res| res.data.map(|url| url.tiny_url).unwrap_or_default());

                return match res {
                    Ok(res) => {
                        tracing::info!("tiny url: '{}' (in {}ms)", res, now.elapsed().as_millis());
                        res
                    }
                    Err(e) => {
                        tracing::error!("tiny url err: '{}' ", e);
                        url.to_owned()
                    }
                };
            }
        }

        tracing::info!("no tiny url token");
        url.to_owned()
    }

    #[instrument(skip(self, request))]
    pub async fn create_event(&self, request: AddEvent) -> Result<EventInfo> {
        let mut validation = shared::CreateEventValidation::default();

        validation.check(&request.data.name, &request.data.description);

        if validation.has_any() {
            bail!("request validation failed: {:?}", validation);
        }

        let now = timestamp_now();

        let request_mod_mail = request.moderator_email.clone();

        let mod_token = Ulid::new().to_string();

        let mut e = ApiEventInfo {
            create_time_unix: now,
            delete_time_unix: 0,
            last_edit_unix: now,
            deleted: false,
            premium_order: None,
            questions: Vec::new(),
            do_screening: false,
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

        //Note: only use shortener outside of e2e tests
        if !request.test {
            e.data.short_url = self.shorten_url(&url).await;
        }
        e.data.long_url = Some(url.clone());

        let result = e.clone();

        self.eventsdb
            .put(EventEntry::new(e, request.test.then_some(now + 60)))
            .await?;

        if let Some(mail) = request_mod_mail.as_ref() {
            self.send_mail(
                mail.clone(),
                result.data.name.clone(),
                result.data.short_url.clone(),
                self.mod_link(&result.tokens),
            );
        }

        if !request.test {
            self.tracking.track_event_create(
                result.tokens.public_token.clone(),
                url,
                result.data.name.clone(),
            );
        }

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
    pub async fn get_event(
        &self,
        id: String,
        secret: Option<String>,
        admin: bool,
    ) -> Result<GetEventResponse> {
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

        if e.deleted && !admin {
            return Ok(GetEventResponse::deleted(id));
        }

        if secret.is_none() && !admin {
            //TODO: can be NONE?
            e.tokens.moderator_token = Some(String::new());

            e.questions = e
                .questions
                .into_iter()
                .filter(|q| !q.hidden && !q.screening)
                .collect::<Vec<_>>();
        }

        if !admin {
            e.adapt_if_timedout();
        }

        let timed_out = e.is_timed_out_and_free();
        let viewers = if admin || e.premium_order.is_some() {
            self.viewers.count(&id).await
        } else {
            0
        };

        Ok(GetEventResponse {
            info: e.into(),
            timed_out,
            admin,
            viewers,
        })
    }

    pub async fn get_question(
        &self,
        id: String,
        secret: Option<String>,
        question_id: i64,
    ) -> Result<QuestionItem> {
        let e = self.eventsdb.get(&id).await?.event;

        if e.deleted {
            return Err(InternalError::AccessingDeletedEvent(id));
        }

        let is_mod = e
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

        if (q.screening || q.hidden) && !is_mod {
            bail!("q not found")
        }

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

            if e.deleted {
                return Err(InternalError::AccessingDeletedEvent(id));
            }

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

            if q.screening && state.screened {
                q.screening = false;
            }
            if q.screening && state.hide {
                //hiding an unscreened question equals a dis-approval
                q.screening = false;
            }
        }

        entry.bump();

        let e = entry.event.clone();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(&id, Notification::Question(question_id))
            .await;

        Ok(e.into())
    }

    pub async fn edit_event_state(
        &self,
        id: String,
        secret: String,
        state: EventState,
    ) -> Result<EventInfo> {
        let mut entry = self.eventsdb.get(&id).await?.clone();

        let e = &mut entry.event;

        if e.deleted {
            return Err(InternalError::AccessingDeletedEvent(id));
        }

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

        self.notify_subscribers(&id, Notification::Event).await;

        Ok(result.into())
    }

    pub async fn edit_event_screening(
        &self,
        id: String,
        secret: String,
        screening: bool,
    ) -> Result<EventInfo> {
        let mut entry = self.eventsdb.get(&id).await?.clone();

        let e = &mut entry.event;

        if e.deleted {
            bail!("event not found");
        }

        if e.premium_order.is_none() {
            bail!("event not premium");
        }

        if e.tokens
            .moderator_token
            .as_ref()
            .map(|mod_token| mod_token != &secret)
            .unwrap_or_default()
        {
            bail!("wrong mod token");
        }

        e.do_screening = screening;

        let result = e.clone();

        entry.bump();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(&id, Notification::Event).await;

        Ok(result.into())
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

        self.notify_subscribers(&id, Notification::Event).await;

        Ok(())
    }

    pub async fn request_premium_upgrade(
        &self,
        id: String,
        secret: String,
    ) -> Result<EventUpgrade> {
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

        let mod_url = self.mod_link(&e.tokens);
        let approve_url = self
            .payment
            .create_order(
                &e.tokens.public_token,
                &mod_url,
                &format!("{mod_url}?payment=true"),
            )
            .await?;

        Ok(EventUpgrade { url: approve_url })
    }

    #[instrument(skip(self))]
    pub async fn premium_capture(&self, id: String, order: String) -> Result<PaymentCapture> {
        tracing::info!("premium_capture");

        let event_id = self.payment.event_of_order(order.clone()).await?;
        if event_id != id {
            return Err(InternalError::General("invalid parameter".into()));
        }

        let order_captured = self.capture_payment_and_upgrade(event_id, order).await?;

        Ok(PaymentCapture { order_captured })
    }

    #[instrument(skip(self))]
    pub async fn payment_webhook(&self, id: String) -> Result<()> {
        tracing::info!("order processing");

        let event_id = self.payment.event_of_order(id.clone()).await?;

        if !self.capture_payment_and_upgrade(event_id, id).await? {
            tracing::warn!("webhook failed");
        }

        Ok(())
    }

    async fn capture_payment_and_upgrade(&self, event: String, order: String) -> Result<bool> {
        let mut entry = self.eventsdb.get(&event).await?;

        if entry.event.premium_order.is_some() {
            tracing::info!("event already premium");
            return Ok(false);
        }

        if self.payment.capture_payment(order.clone()).await? {
            tracing::info!("order captured");

            entry.event.premium_order = Some(order);

            entry.bump();

            let data = entry.event.data.clone();

            self.eventsdb.put(entry).await?;

            self.notify_subscribers(&event, Notification::Event).await;

            self.tracking.track_event_upgrade(&event, &data);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    //TODO: fix clippy-allow
    #[allow(clippy::cast_possible_wrap)]
    pub async fn add_question(
        &self,
        id: String,
        question: shared::AddQuestion,
    ) -> Result<QuestionItem> {
        let trimmed_question = question.text.trim().to_string();

        let mut validation = shared::AddQuestionValidation::default();

        validation.check(&trimmed_question);

        if validation.has_any() {
            return Err(InternalError::AddQuestionValidation(validation));
        }

        let mut entry = self.eventsdb.get(&id).await?;

        let e = &mut entry.event;

        if e.is_timed_out_and_free() {
            return Err(InternalError::TimedOutFreeEvent(id));
        }

        if e.questions.len() > 500 {
            bail!("max number of questions reached");
        }

        if !matches!(e.state.state, States::Open) {
            bail!("event not open");
        }

        if e.questions
            .iter()
            .any(|q| q.text.trim() == trimmed_question)
        {
            return Err(InternalError::DuplicateQuestion);
        }

        let question_id = e.questions.len() as i64;

        let question = shared::QuestionItem {
            text: trimmed_question,
            answered: false,
            create_time_unix: timestamp_now(),
            hidden: false,
            screening: e.do_screening,
            id: question_id,
            likes: 1,
        };

        e.questions.push(question.clone());

        entry.bump();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(&id, Notification::Question(question_id))
            .await;

        Ok(question)
    }

    pub async fn edit_like(&self, id: String, edit: shared::EditLike) -> Result<QuestionItem> {
        let mut entry = self.eventsdb.get(&id).await?.clone();

        let e = &mut entry.event;

        if e.is_timed_out_and_free() {
            return Err(InternalError::TimedOutFreeEvent(id));
        }

        if matches!(e.state.state, States::Closed) {
            bail!("event closed");
        }

        if let Some(f) = e.questions.iter_mut().find(|e| e.id == edit.question_id) {
            f.likes = if edit.like {
                f.likes.saturating_add(1)
            } else {
                f.likes.saturating_sub(1)
            };

            let res = f.clone();

            entry.bump();

            self.eventsdb.put(entry).await?;

            self.notify_subscribers(&id, Notification::Question(edit.question_id))
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

        self.viewers.remove(&id).await;

        //Note: lets not spam everyone if its a shutdown
        if !self.is_shutting_down() {
            self.notify_viewer_count_change(&id);
        }

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

            app.notify_subscribers(&event, Notification::Viewers(count))
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
                let error_string_lowcase = e.to_string().to_lowercase();
                let well_known = error_string_lowcase == "connection closed normally"
                    || error_string_lowcase == "trying to work with closed connection";
                if !well_known {
                    tracing::warn!("websocket send error: {}", e);
                }
            }
        }));

        sender
    }

    async fn notify_subscribers(&self, event_id: &str, n: Notification) {
        let msg = match n {
            Notification::Event => "e".to_string(),
            Notification::Question(id) => format!("q:{id}"),
            Notification::Viewers(count) => format!("v:{count}"),
        };

        self.pubsub_publish.publish(event_id, &msg).await;
    }

    fn send_mail(
        &self,
        receiver: String,
        event_name: String,
        public_link: String,
        mod_link: String,
    ) {
        if receiver.trim().is_empty() {
            tracing::debug!("mail not sent, no receiver specified");
            return;
        }

        let mail = self.mail_config.clone();

        tokio::spawn(async move {
            if let Err(e) = mail
                .send_mail(receiver.clone(), event_name, public_link, mod_link)
                .await
            {
                tracing::error!("mail send error: {e}");
            }
        });
    }
}

#[async_trait]
impl PubSubReceiver for App {
    async fn notify(&self, topic: &str, payload: &str) {
        let topic = topic.to_string();
        let msg = Message::Text(payload.to_string());

        let channels = Arc::clone(&self.channels);

        if let Err(e) = tokio::spawn(async move {
            //TODO: lookup subscriber based on topic name
            let receivers = channels.read().await.clone();
            for (_user_id, (_id, c)) in receivers.iter().filter(|(_, (id, _))| id == &topic) {
                if let Err(e) = c.send(Ok(msg.clone())) {
                    if let Err(inner_err) = &e.0 {
                        tracing::error!("pubsub send err: {} ({})", e, inner_err);
                    } else {
                        tracing::info!("pubsub not sent: {}", e);
                    }
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
        eventsdb::{event_key, InMemoryEventsDB},
        pubsub::{PubSubInMemory, PubSubReceiverInMemory},
        viewers::MockViewers,
    };
    use pretty_assertions::assert_eq;
    use shared::{AddQuestion, EventData, TEST_VALID_QUESTION};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_event_create_fail_validation() {
        let app = App::new(
            Arc::new(InMemoryEventsDB::default()),
            Arc::new(PubSubInMemory::default()),
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    name: String::from("too short"),
                    description: String::new(),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: None,
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
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        );

        app.create_event(AddEvent {
            data: EventData {
                name: String::from("123456789"),
                description: String::from("123456789 123456789 123456789 !"),
                short_url: String::new(),
                long_url: None,
            },
            moderator_email: None,
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
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    name: String::from("123456789"),
                    description: String::from("123456789 123456789 123456789 !"),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        let q = app
            .add_question(
                res.tokens.public_token.clone(),
                AddQuestion {
                    text: String::from(TEST_VALID_QUESTION),
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
                screened: true,
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

    #[tokio::test]
    async fn test_screening_question() {
        // env_logger::init();

        let pubsubreceiver = Arc::new(PubSubReceiverInMemory::default());
        let pubsub = PubSubInMemory::default();
        pubsub.set_receiver(pubsubreceiver.clone()).await;
        let events = Arc::new(InMemoryEventsDB::default());
        let app = App::new(
            events.clone(),
            Arc::new(pubsub),
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    name: String::from("123456789"),
                    description: String::from("123456789 123456789 123456789 !"),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        events
            .db
            .lock()
            .await
            .get_mut(&event_key(&res.tokens.public_token))
            .unwrap()
            .event
            .do_screening = true;

        let q = app
            .add_question(
                res.tokens.public_token.clone(),
                AddQuestion {
                    text: String::from(TEST_VALID_QUESTION),
                },
            )
            .await
            .unwrap();

        assert_eq!(q.screening, true);

        let e = app
            .get_event(res.tokens.public_token.clone(), None, false)
            .await
            .unwrap();

        assert_eq!(e.info.questions.len(), 0);

        let e = app
            .get_event(
                res.tokens.public_token.clone(),
                Some(res.tokens.moderator_token.clone().unwrap()),
                false,
            )
            .await
            .unwrap();

        assert_eq!(e.info.questions.len(), 1);

        app.mod_edit_question(
            res.tokens.public_token.clone(),
            res.tokens.moderator_token.clone().unwrap(),
            q.id,
            ModQuestion {
                hide: false,
                answered: false,
                screened: true,
            },
        )
        .await
        .unwrap();

        let e = app
            .get_event(res.tokens.public_token.clone(), None, false)
            .await
            .unwrap();

        assert_eq!(e.info.questions.len(), 1);
    }

    #[tokio::test]
    async fn test_screening_question_disapprove() {
        // env_logger::init();

        let pubsubreceiver = Arc::new(PubSubReceiverInMemory::default());
        let pubsub = PubSubInMemory::default();
        pubsub.set_receiver(pubsubreceiver.clone()).await;
        let events = Arc::new(InMemoryEventsDB::default());
        let app = App::new(
            events.clone(),
            Arc::new(pubsub),
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    name: String::from("123456789"),
                    description: String::from("123456789 123456789 123456789 !"),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        events
            .db
            .lock()
            .await
            .get_mut(&event_key(&res.tokens.public_token))
            .unwrap()
            .event
            .do_screening = true;

        let q = app
            .add_question(
                res.tokens.public_token.clone(),
                AddQuestion {
                    text: String::from(TEST_VALID_QUESTION),
                },
            )
            .await
            .unwrap();

        assert_eq!(q.screening, true);

        let e = app
            .get_event(res.tokens.public_token.clone(), None, false)
            .await
            .unwrap();

        assert_eq!(e.info.questions.len(), 0);

        let e = app
            .mod_edit_question(
                res.tokens.public_token.clone(),
                res.tokens.moderator_token.clone().unwrap(),
                q.id,
                ModQuestion {
                    hide: true,
                    answered: false,
                    screened: false,
                },
            )
            .await
            .unwrap();

        assert_eq!(e.questions[0].screening, false);
    }

    #[tokio::test]
    async fn test_screening_enable() {
        // env_logger::init();

        let pubsubreceiver = Arc::new(PubSubReceiverInMemory::default());
        let pubsub = PubSubInMemory::default();
        pubsub.set_receiver(pubsubreceiver.clone()).await;
        let events = Arc::new(InMemoryEventsDB::default());
        let app = App::new(
            events.clone(),
            Arc::new(pubsub),
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    name: String::from("123456789"),
                    description: String::from("123456789 123456789 123456789 !"),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        events
            .db
            .lock()
            .await
            .get_mut(&event_key(&res.tokens.public_token))
            .unwrap()
            .event
            .premium_order = Some(String::from("foo"));

        let e = app
            .edit_event_screening(
                res.tokens.public_token.clone(),
                res.tokens.moderator_token.clone().unwrap(),
                true,
            )
            .await
            .unwrap();

        assert!(e.screening);
    }

    #[tokio::test]
    async fn test_duplicate_question_check() {
        // env_logger::init();

        let pubsubreceiver = Arc::new(PubSubReceiverInMemory::default());
        let pubsub = PubSubInMemory::default();
        pubsub.set_receiver(pubsubreceiver.clone()).await;
        let events = Arc::new(InMemoryEventsDB::default());
        let app = App::new(
            events.clone(),
            Arc::new(pubsub),
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        );

        let res = app
            .create_event(AddEvent {
                data: EventData {
                    name: String::from("123456789"),
                    description: String::from("123456789 123456789 123456789 !"),
                    short_url: String::new(),
                    long_url: None,
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        app.add_question(
            res.tokens.public_token.clone(),
            AddQuestion {
                text: String::from(TEST_VALID_QUESTION),
            },
        )
        .await
        .unwrap();

        let request = app
            .add_question(
                res.tokens.public_token.clone(),
                AddQuestion {
                    text: String::from(TEST_VALID_QUESTION),
                },
            )
            .await;

        assert!(matches!(
            request.unwrap_err(),
            InternalError::DuplicateQuestion
        ))
    }
}
