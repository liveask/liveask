use async_trait::async_trait;
use axum::extract::ws::{CloseFrame, Message, WebSocket, close_code::RESTART};
use shared::{
    AddEvent, Color, ContextValidation, EventInfo, EventResponseFlags, EventState, EventTags,
    EventTokens, EventUpgradeResponse, GetEventResponse, ModEvent, ModInfo, ModQuestion,
    PasswordValidation, PaymentCapture, QuestionItem, States, TagValidation,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, mpsc},
    task,
    time::sleep,
};
use tracing::instrument;
use ulid::Ulid;

use crate::{
    bail, env,
    error::{InternalError, Result},
    eventsdb::{ApiEventInfo, EventEntry, EventsDB, PremiumOrder},
    mail::MailConfig,
    payment::Payment,
    pubsub::{PubSubPublish, PubSubReceiver},
    tracking::{EditEvent, Tracking},
    utils::timestamp_now,
    viewers::Viewers,
    weeme,
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
    ezlime_key: Option<String>,
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
        let weeme_key = Self::weeme_key();

        let mail_config = MailConfig::new();

        Self {
            eventsdb,
            pubsub_publish,
            channels: Arc::default(),
            base_url,
            ezlime_key: weeme_key,
            mail_config,
            payment,
            viewers,
            tracking,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    fn weeme_key() -> Option<String> {
        let key = std::env::var(env::ENV_WEEME_KEY).ok();

        if key.clone().unwrap_or_default().trim().is_empty() {
            tracing::warn!("no url shorten token set, use `WEEME_KEY` to do so");
        } else {
            tracing::info!(
                "weeme-key set (len: {})",
                key.clone().unwrap_or_default().trim().len()
            );
        }

        key
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
        if let Some(ezlime_key) = &self.ezlime_key
            && !ezlime_key.trim().is_empty()
        {
            let now = Instant::now();

            let ezlime = ezlime_rs::EzlimeApi::new(ezlime_key.clone());

            match ezlime.create_short_url(url).await {
                Ok(shortened_url) => {
                    tracing::info!(
                        "short url: '{}' (in {}ms)",
                        shortened_url,
                        now.elapsed().as_millis()
                    );
                    return shortened_url;
                }
                Err(e) => {
                    tracing::error!("failed to create short url: {}", e);
                }
            }
        } else {
            tracing::info!("no weeme key");
        }

        url.to_owned()
    }

    #[instrument(skip(self, request))]
    pub async fn create_event(&self, request: AddEvent) -> Result<EventInfo> {
        let validation = shared::CreateEventValidation::default().check(
            &request.data.name,
            &request.data.description,
            request.moderator_email.clone().unwrap_or_default().as_str(),
        );

        if validation.has_any() {
            return Err(InternalError::MetaValidation(shared::EditMetaData {
                title: request.data.name.clone(),
                description: request.data.description.clone(),
            }));
        }

        let now = timestamp_now();

        let request_mod_mail = request.moderator_email.clone();

        let public_token = Ulid::new().to_string();
        let mod_token = Ulid::new().to_string();

        let mut e = ApiEventInfo {
            create_time_unix: now,
            delete_time_unix: 0,
            last_edit_unix: now,
            deleted: false,
            premium_id: None,
            password: shared::EventPassword::Disabled,
            questions: Vec::new(),
            do_screening: false,
            state: EventState {
                state: States::Open,
            },
            data: request.data,
            tokens: EventTokens {
                public_token: public_token.clone(),
                moderator_token: Some(mod_token.clone()),
            },
            context: Vec::new(),
            tags: EventTags::default(),
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
                public_token,
                mail.clone(),
                result.data.name.clone(),
                result.data.short_url.clone(),
                self.mod_link(&result.tokens),
            );
        }

        if !request.test {
            self.tracking
                .track_event_create(
                    result.tokens.public_token.clone(),
                    url,
                    result.data.name.clone(),
                )
                .await?;
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
    pub async fn check_event_password(&self, id: String, password: &str) -> Result<bool> {
        tracing::info!("check_event_password");

        let mut validation = PasswordValidation::default();
        validation.check(password);
        if validation.has_any() {
            return Err(InternalError::PasswordValidation(validation));
        }

        let e = self.eventsdb.get(&id).await?.event;

        if e.deleted {
            return Err(InternalError::AccessingDeletedEvent(id));
        }

        Ok(e.password.matches(&Some(password.to_string())))
    }

    #[instrument(skip(self))]
    pub async fn get_event(
        &self,
        id: String,
        secret: Option<String>,
        admin: bool,
        password: Option<String>,
    ) -> Result<GetEventResponse> {
        tracing::info!("get_event");

        let mut e = self.eventsdb.get(&id).await?.event;

        if let Some(secret) = &secret {
            if e.tokens
                .moderator_token
                .as_ref()
                .is_some_and(|mod_token| mod_token != secret)
            {
                return Err(InternalError::WrongModeratorToken(id));
            }
        }

        let is_mod = secret.is_some();

        if e.deleted && !admin {
            return Ok(GetEventResponse::deleted(id));
        }

        let mod_info = is_mod.then(|| ModInfo {
            pwd: e.password.clone(),
            private_token: e.tokens.moderator_token.clone().unwrap_or_default(),
        });

        if !is_mod && !admin {
            //TODO: can be NONE?
            e.tokens.moderator_token = Some(String::new());

            e.questions = e
                .questions
                .into_iter()
                .filter(|q| !q.hidden && !q.screening)
                .collect::<Vec<_>>();
        }

        let time_out_masked = if admin { false } else { e.adapt_if_timedout() };

        let password_matches = e.password.matches(&password);

        let pwd_masked = if (e.password.is_enabled() && !password_matches) && !admin && !is_mod {
            e.mask_data();
            true
        } else {
            false
        };

        let timed_out = e.is_timed_out_and_free();
        let viewers = if admin || e.premium() {
            self.viewers.count(&id).await
        } else {
            0
        };

        let masked = time_out_masked || pwd_masked;

        let mut flags = EventResponseFlags::empty();

        flags.set(EventResponseFlags::TIMED_OUT, timed_out);
        flags.set(EventResponseFlags::WRONG_PASSWORD, pwd_masked);

        Ok(GetEventResponse {
            info: e.into(),
            admin,
            viewers,
            flags,
            masked,
            mod_info,
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
            .is_some_and(|tokens| tokens.0 != tokens.1);

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
                .is_some_and(|mod_token| mod_token != &secret)
            {
                return Err(InternalError::WrongModeratorToken(id));
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

    pub async fn mod_edit_event(
        &self,
        id: String,
        secret: String,
        changes: ModEvent,
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
            .is_some_and(|mod_token| mod_token != &secret)
        {
            return Err(InternalError::WrongModeratorToken(id));
        }

        if let Some(state) = changes.state {
            e.state = state;
        }
        if let Some(screening) = changes.screening {
            e.do_screening = screening;
        }
        if let Some(password) = changes.password {
            self.mod_edit_password(e, password).await?;
        }
        if let Some(current_tag) = &changes.current_tag {
            self.mod_edit_tag(e, current_tag).await?;
        }
        if let Some(context_link) = &changes.context {
            self.mod_context(e, context_link).await?;
        }
        if let Some(meta) = &changes.meta {
            self.mod_meta(e, meta).await?;
        }
        if let Some(color) = &changes.color {
            self.mod_color(e, color).await?;
        }

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
            .is_some_and(|mod_token| mod_token != &secret)
        {
            return Err(InternalError::WrongModeratorToken(id));
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
        admin: bool,
        payload: shared::ModRequestPremium,
    ) -> Result<EventUpgradeResponse> {
        let mut entry = self.eventsdb.get(&id).await?;

        let e = &mut entry.event;

        if e.deleted {
            return Err(InternalError::AccessingDeletedEvent(id));
        }

        let response = if admin {
            let upgraded = self
                .upgrade_event(id, PremiumOrder::Admin("unknown".into()))
                .await?;
            if !upgraded {
                tracing::error!("admin upgrade failed");
            }
            EventUpgradeResponse::AdminUpgrade
        } else {
            if e.tokens
                .moderator_token
                .as_ref()
                .is_some_and(|mod_token| mod_token != &secret)
            {
                return Err(InternalError::WrongModeratorToken(id));
            }

            let mod_url = self.mod_link(&e.tokens);
            let approve_url = self
                .payment
                .create_order(
                    &e.tokens.public_token,
                    &mod_url,
                    &format!("{mod_url}?payment=true&token={{CHECKOUT_SESSION_ID}}"),
                )
                .await?;

            EventUpgradeResponse::Redirect { url: approve_url }
        };

        task::spawn({
            let tracking = self.tracking.clone();
            let event = e.tokens.public_token.clone();
            let context = format!("{:?}", payload.context);
            async move {
                let _ = tracking.track_event_request_upgrade(event, context).await;
            }
        });

        Ok(response)
    }

    #[instrument(skip(self))]
    pub async fn premium_capture(
        &self,
        id: String,
        stripe_order_id: String,
    ) -> Result<PaymentCapture> {
        tracing::info!("premium_capture");

        let (event_id, complete) = self
            .payment
            .retrieve_event_state(stripe_order_id.clone())
            .await?;

        if event_id != id {
            return Err(InternalError::General("invalid parameter".into()));
        }

        let order_captured = complete;

        if order_captured {
            self.upgrade_event(id, PremiumOrder::StripeSessionId(stripe_order_id))
                .await?;
        }

        Ok(PaymentCapture { order_captured })
    }

    #[instrument(skip(self))]
    pub async fn payment_webhook(&self, stripe_session_id: String, event_id: String) -> Result<()> {
        tracing::info!("order processing");

        if !self
            .upgrade_event(event_id, PremiumOrder::StripeSessionId(stripe_session_id))
            .await?
        {
            tracing::warn!("webhook failed");
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn upgrade_event(&self, event: String, order_id: PremiumOrder) -> Result<bool> {
        tracing::info!("upgrade_event");

        let mut entry = self.eventsdb.get(&event).await?;

        if entry.event.premium() {
            tracing::info!("event already premium");
            return Ok(true);
        }

        entry.event.premium_id = Some(order_id.clone());

        entry.bump();

        let (name, long_url, age) = (
            entry.event.data.name.clone(),
            entry.event.data.long_url.clone().unwrap_or_default(),
            entry.event.age_in_seconds(),
        );

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(&event, Notification::Event).await;

        self.tracking
            .track_event_upgrade(event.clone(), name, long_url, age, order_id.into())
            .await?;

        Ok(true)
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
            tag: question.tag.or(e.tags.current_tag),
        };

        e.questions.push(question.clone());

        entry.bump();

        self.eventsdb.put(entry).await?;

        self.notify_subscribers(&id, Notification::Question(question_id))
            .await;

        self.tracking
            .track_event_question_added(id.clone(), question_id.saturating_add(1))
            .await?;

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

    // TODO: cleanup
    #[allow(clippy::cognitive_complexity)]
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
                let well_known = error_string_lowcase.starts_with("connection closed normally")
                    || error_string_lowcase.starts_with("trying to work with closed connection");
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
        event_id: String,
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
                .send_mail(
                    event_id,
                    receiver.clone(),
                    event_name,
                    public_link,
                    mod_link,
                )
                .await
            {
                tracing::error!("mail send error: {e}");
            }
        });
    }

    async fn mod_edit_password(
        &self,
        e: &mut ApiEventInfo,
        password: shared::EventPassword,
    ) -> Result<()> {
        let edit_type = match (e.password.is_enabled(), password.is_enabled()) {
            (false, true) => Some(EditEvent::Enabled),
            (true, false) => Some(EditEvent::Disabled),
            (true, true) => Some(EditEvent::Changed),
            _ => None,
        };

        if let Some(edit_type) = edit_type {
            self.tracking
                .track_event_password_set(e.tokens.public_token.clone(), edit_type)
                .await?;
        }

        e.password = password;

        Ok(())
    }

    async fn mod_edit_tag(
        &self,
        e: &mut ApiEventInfo,
        current_tag: &shared::CurrentTag,
    ) -> Result<()> {
        if !e.premium() {
            return Err(InternalError::PremiumOnlyFeature(
                e.tokens.public_token.clone(),
            ));
        }

        let edit_type = match (e.tags.current_tag.is_some(), current_tag.is_enabled()) {
            (false, true) => Some(EditEvent::Enabled),
            (true, false) => Some(EditEvent::Disabled),
            (true, true) => Some(EditEvent::Changed),
            _ => None,
        };

        if let Some(edit_type) = edit_type {
            self.tracking
                .track_event_tag_set(e.tokens.public_token.clone(), edit_type, e.age_in_seconds())
                .await?;
        }

        if let shared::CurrentTag::Enabled(tag) = &current_tag {
            let mut validation = TagValidation::default();
            validation.check(tag);

            if validation.has_any() {
                return Err(InternalError::TagValidation(validation));
            }

            if !e.tags.set_or_add_tag(tag) {
                bail!("max tags reached");
            }
        } else {
            e.tags.current_tag = None;
        }

        Ok(())
    }

    async fn mod_context(
        &self,
        e: &mut ApiEventInfo,
        context_link: &shared::EditContextLink,
    ) -> Result<()> {
        if !e.premium() {
            return Err(InternalError::PremiumOnlyFeature(
                e.tokens.public_token.clone(),
            ));
        }

        match context_link {
            shared::EditContextLink::Disabled => e.context = vec![],
            shared::EditContextLink::Enabled(item) => {
                let mut validation = ContextValidation::default();

                validation.check(&item.label, &item.url);
                if validation.has_any() {
                    return Err(InternalError::ContextValidation(validation));
                }

                e.context = vec![item.clone()];

                self.tracking
                    .track_event_context_set(e.tokens.public_token.clone(), &item.label, &item.url)
                    .await?;
            }
        }

        Ok(())
    }

    async fn mod_meta(&self, e: &mut ApiEventInfo, edit: &shared::EditMetaData) -> Result<()> {
        if !shared::EventInfo::during_first_day(e.create_time_unix) {
            bail!("event meta can only be changed during first 24h")
        }

        let validation =
            shared::CreateEventValidation::default().check(&edit.title, &edit.description, "");
        if validation.has_any() {
            return Err(InternalError::MetaValidation(edit.clone()));
        }

        e.data.name.clone_from(&edit.title);
        e.data.description.clone_from(&edit.description);

        self.tracking
            .track_event_meta_change(e.tokens.public_token.clone(), edit)
            .await?;

        Ok(())
    }

    async fn mod_color(&self, e: &mut ApiEventInfo, color: &shared::EditColor) -> Result<()> {
        e.data.color = Some(Color(color.0.clone()));

        self.tracking
            .track_event_color_change(e.tokens.public_token.clone(), color, e.premium())
            .await?;

        Ok(())
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
        eventsdb::{InMemoryEventsDB, PremiumOrder, event_key},
        pubsub::{PubSubInMemory, PubSubReceiverInMemory},
        viewers::MockViewers,
    };
    use pretty_assertions::{assert_eq, assert_ne};
    use shared::{
        AddQuestion, CurrentTag, EventData, TEST_EVENT_DESC, TEST_EVENT_NAME, TEST_VALID_QUESTION,
        TagId,
    };
    use std::sync::Arc;

    #[tokio::test]
    #[tracing_test::traced_test]
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
                    ..EventData::default()
                },
                moderator_email: None,
                test: false,
            })
            .await;

        assert!(res.is_err());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_event_create_email_fail_validation() {
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
                    name: TEST_EVENT_NAME.to_string(),
                    description: TEST_EVENT_DESC.to_string(),
                    ..EventData::default()
                },
                moderator_email: Option::Some("a@a".to_string()),
                test: false,
            })
            .await;

        assert!(res.is_err());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_event_create_email_pass_validation() {
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
                    name: TEST_EVENT_NAME.to_string(),
                    description: TEST_EVENT_DESC.to_string(),
                    ..EventData::default()
                },
                moderator_email: Option::Some("testuser@live-ask.com".to_string()),
                test: false,
            })
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
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
                ..EventData::default()
            },
            moderator_email: None,
            test: false,
        })
        .await
        .unwrap();

        assert_eq!(eventdb.db.lock().await.len(), 1);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
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
                    ..EventData::default()
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
                    tag: None,
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
    #[tracing_test::traced_test]
    async fn test_screening_question() {
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
                    ..EventData::default()
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
                    tag: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(q.screening, true);

        let e = app
            .get_event(res.tokens.public_token.clone(), None, false, None)
            .await
            .unwrap();

        assert_eq!(e.info.questions.len(), 0);

        let e = app
            .get_event(
                res.tokens.public_token.clone(),
                Some(res.tokens.moderator_token.clone().unwrap()),
                false,
                None,
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
            .get_event(res.tokens.public_token.clone(), None, false, None)
            .await
            .unwrap();

        assert_eq!(e.info.questions.len(), 1);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_screening_question_disapprove() {
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
                    ..EventData::default()
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
                    tag: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(q.screening, true);

        let e = app
            .get_event(res.tokens.public_token.clone(), None, false, None)
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
    #[tracing_test::traced_test]
    async fn test_screening_enable() {
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
                    ..EventData::default()
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
            .premium_id = Some(PremiumOrder::PaypalOrderId(String::from("foo")));

        let e = app
            .mod_edit_event(
                res.tokens.public_token.clone(),
                res.tokens.moderator_token.clone().unwrap(),
                ModEvent {
                    screening: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(e.is_screening());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_duplicate_question_check() {
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
                    ..EventData::default()
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
                tag: None,
            },
        )
        .await
        .unwrap();

        let request = app
            .add_question(
                res.tokens.public_token.clone(),
                AddQuestion {
                    text: String::from(TEST_VALID_QUESTION),
                    tag: None,
                },
            )
            .await;

        assert!(matches!(
            request.unwrap_err(),
            InternalError::DuplicateQuestion
        ))
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_password_protection() {
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
                    ..EventData::default()
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        let event_id = res.tokens.public_token.clone();
        let mod_token = res.tokens.moderator_token.clone().unwrap();

        let question_text = "very long, sophisticated question you can ask!";
        app.add_question(
            event_id.clone(),
            AddQuestion {
                text: String::from(question_text),
                tag: None,
            },
        )
        .await
        .unwrap();

        app.mod_edit_event(
            event_id.clone(),
            mod_token.clone(),
            ModEvent {
                password: Some(shared::EventPassword::Enabled(String::from("pwd"))),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let e = app
            .get_event(event_id.clone(), None, false, None)
            .await
            .unwrap();

        assert_ne!(&e.info.questions[0].text, question_text);
        assert!(e.flags.contains(EventResponseFlags::WRONG_PASSWORD));

        let e = app
            .get_event(event_id.clone(), None, false, Some(String::from("pwd")))
            .await
            .unwrap();

        assert_eq!(&e.info.questions[0].text, question_text);
        assert!(!e.flags.contains(EventResponseFlags::WRONG_PASSWORD));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_event_question_tags() {
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
                    ..EventData::default()
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
            .premium_id = Some(PremiumOrder::PaypalOrderId(String::from("foo")));

        assert_eq!(
            app.mod_edit_event(
                res.tokens.public_token.clone(),
                res.tokens.moderator_token.clone().unwrap(),
                ModEvent {
                    current_tag: Some(CurrentTag::Enabled(String::from("tag1"))),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .tags
            .current_tag
            .unwrap(),
            TagId(0)
        );

        let request = app
            .add_question(
                res.tokens.public_token.clone(),
                AddQuestion {
                    text: String::from(TEST_VALID_QUESTION),
                    tag: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(request.tag.unwrap(), TagId(0))
    }
}
