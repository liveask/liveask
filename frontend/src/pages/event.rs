use chrono::{DateTime, Duration, Utc};
use const_format::formatcp;
use events::{EventBridge, event_context};
use serde::Deserialize;
use shared::{
    EventFlags, EventInfo, GetEventResponse, ModEvent, ModQuestion, QuestionItem, States,
};
use std::{collections::HashMap, rc::Rc, str::FromStr};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlAnchorElement;
use yew::prelude::*;
use yew_router::scope_ext::RouterScopeExt;
use yewdux::prelude::*;

use crate::{
    GlobalEvent, State,
    components::{
        DeletePopup, EventMeta, EventSocket, Footer, ModPassword, ModTags, PasswordPopup, Question,
        QuestionClickType, QuestionFlags, QuestionPopup, SharableTags, SharePopup, SocketResponse,
        Upgrade,
    },
    environment::{LiveAskEnv, la_env},
    fetch,
    local_cache::LocalCache,
    tracking,
};

enum Mode {
    Moderator,
    Viewer,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub id: AttrValue,
    #[prop_or_default]
    pub secret: Option<String>,
}

pub enum LoadingState {
    Loading,
    Loaded,
    Deleted,
    NotFound,
}

const fn la_endpoints() -> (&'static str, &'static str) {
    match la_env(Some(env!("LA_ENV"))) {
        LiveAskEnv::Prod | LiveAskEnv::Beta => (
            formatcp!("https://{}.www.live-ask.com", env!("LA_ENV")),
            formatcp!("wss://{}.www.live-ask.com", env!("LA_ENV")),
        ),
        LiveAskEnv::Local => (BASE_API_LOCAL, BASE_SOCKET_LOCAL),
    }
}

pub const BASE_API: &str = la_endpoints().0;
pub const BASE_SOCKET: &str = la_endpoints().1;

pub const BASE_API_LOCAL: &str = "http://localhost:8090";
pub const BASE_SOCKET_LOCAL: &str = "ws://localhost:8090";

const FREE_EVENT_DURATION_DAYS: i64 = 7;

#[derive(Debug, Default, Deserialize)]
struct QueryParams {
    #[serde(rename = "token")]
    pub paypal_token: Option<String>,
}

pub struct Event {
    current_event_id: String,
    copied_to_clipboard: bool,
    query_params: QueryParams,
    mode: Mode,
    tags: SharableTags,
    unanswered: Vec<Rc<QuestionItem>>,
    answered: Vec<Rc<QuestionItem>>,
    hidden: Vec<Rc<QuestionItem>>,
    unscreened: Vec<Rc<QuestionItem>>,
    loading_state: LoadingState,
    state: Rc<State>,
    dispatch: Dispatch<State>,
    events: EventBridge<GlobalEvent>,
    socket_url: String,
    manual_reconnect: bool,
}
pub enum Msg {
    FeedbackClick,
    ShareEventClick,
    AskQuestionClick,
    Fetched(Option<GetEventResponse>),
    Captured,
    Socket(SocketResponse),
    QuestionClick((i64, QuestionClickType)),
    QuestionUpdated(i64),
    ModDelete,
    ModExport,
    ModStateChange(yew::Event),
    StateChanged,
    PasswordSet,
    CopyLink,
    ModEditScreening,
    GlobalEvent(GlobalEvent),
}
impl Component for Event {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let event_id = ctx.props().id.to_string();

        request_fetch(event_id.clone(), ctx.props().secret.clone(), ctx.link());

        let socket_url = format!("{BASE_SOCKET}/push/{event_id}",);

        let query_params = ctx
            .link()
            .location()
            .and_then(|loc| loc.query::<QueryParams>().ok())
            .unwrap_or_default();

        if let Some(token) = &query_params.paypal_token {
            log::info!("paypal-token: {}", token);
        }

        let events = event_context(ctx)
            .unwrap_throw()
            .subscribe(ctx.link().callback(Msg::GlobalEvent));

        let dispatch = Dispatch::global().subscribe(Callback::noop());

        Self {
            current_event_id: event_id,
            query_params,
            copied_to_clipboard: false,
            mode: if ctx.props().secret.is_some() {
                Mode::Moderator
            } else {
                Mode::Viewer
            },
            loading_state: LoadingState::Loading,
            state: dispatch.get(),
            tags: Rc::new(HashMap::new()),
            unanswered: Vec::new(),
            answered: Vec::new(),
            hidden: Vec::new(),
            unscreened: Vec::new(),
            dispatch,
            events,
            socket_url,
            manual_reconnect: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Captured => {
                log::info!("payment captured");
                false
            }
            Msg::QuestionClick((id, kind)) => {
                self.on_question_click(&kind, id, ctx);
                false
            }
            Msg::QuestionUpdated(_id) => {
                //Note: we wait for the question socket event to poll
                false
            }
            Msg::CopyLink => {
                self.copied_to_clipboard = true;
                let _ = gloo_utils::window()
                    .navigator()
                    .clipboard()
                    .write_text(&self.moderator_url());
                true
            }
            Msg::Socket(msg) => self.handle_socket(msg, ctx),
            Msg::StateChanged => false, // nothing needs to happen here
            Msg::ModStateChange(ev) => {
                let e: web_sys::HtmlSelectElement =
                    ev.target().unwrap_throw().dyn_into().unwrap_throw();
                let new_state = States::from_str(e.value().as_str()).unwrap_throw();

                request_event_change(
                    self.current_event_id.clone(),
                    ctx.props().secret.clone(),
                    ModEvent {
                        state: Some(shared::EventState { state: new_state }),
                        ..Default::default()
                    },
                    ctx.link(),
                );

                false
            }

            Msg::ModEditScreening => {
                request_event_change(
                    self.current_event_id.clone(),
                    ctx.props().secret.clone(),
                    ModEvent {
                        screening: Some(
                            self.state
                                .event
                                .as_ref()
                                .is_some_and(|e| !e.info.is_screening()),
                        ),
                        ..Default::default()
                    },
                    ctx.link(),
                );

                false
            }
            Msg::ModDelete => {
                self.events.emit(GlobalEvent::DeletePopup);
                false
            }
            Msg::ShareEventClick => {
                self.events.emit(GlobalEvent::OpenSharePopup);
                false
            }
            Msg::AskQuestionClick => {
                self.events.emit(GlobalEvent::OpenQuestionPopup);
                false
            }
            Msg::FeedbackClick => {
                //
                log::info!("FeedbackClick");
                tracking::track_event(tracking::EVNT_SURVEY_OPENED);
                false
            }
            Msg::Fetched(res) => self.handle_fetched(res, ctx),
            Msg::ModExport => {
                self.export_event();
                false
            }
            Msg::PasswordSet => {
                request_fetch(
                    self.current_event_id.clone(),
                    ctx.props().secret.clone(),
                    ctx.link(),
                );
                false
            }
            Msg::GlobalEvent(ev) => self.handle_global_event(ev),
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.dispatch.reduce(|_| State::default().into());
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let msg = ctx.link().callback(Msg::Socket);
        html! {
            <>
                <div class="event">
                    <EventSocket
                        reconnect={self.manual_reconnect}
                        url={self.socket_url.clone()}
                        {msg}
                    />
                    { self.view_internal(ctx) }
                </div>
                <Footer />
            </>
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn request_toggle_hide(
    event: String,
    secret: String,
    item: QuestionItem,
    link: &html::Scope<Event>,
) {
    link.send_future(async move {
        let modify = ModQuestion {
            hide: !item.hidden,
            answered: item.answered,
            screened: !item.screening,
        };
        if let Err(res) = fetch::mod_question(BASE_API, event, secret, item.id, modify).await {
            log::error!("hide error: {}", res);
        }

        Msg::QuestionUpdated(item.id)
    });
}

#[allow(clippy::needless_pass_by_value)]
fn request_toggle_answered(
    event: String,
    secret: String,
    item: QuestionItem,
    link: &html::Scope<Event>,
) {
    link.send_future(async move {
        let modify = ModQuestion {
            hide: item.hidden,
            answered: !item.answered,
            screened: !item.screening,
        };

        if let Err(e) = fetch::mod_question(BASE_API, event, secret, item.id, modify).await {
            log::error!("mod_questio error: {e}");
        }

        Msg::QuestionUpdated(item.id)
    });
}

#[allow(clippy::needless_pass_by_value)]
fn request_approve_question(
    event: String,
    secret: String,
    item: QuestionItem,
    link: &html::Scope<Event>,
) {
    link.send_future(async move {
        let modify = ModQuestion {
            hide: false,
            answered: false,
            screened: true,
        };

        if let Err(e) = fetch::mod_question(BASE_API, event, secret, item.id, modify).await {
            log::error!("mod_questio error: {e}");
        }

        Msg::QuestionUpdated(item.id)
    });
}

fn request_like(event: String, id: i64, like: bool, link: &html::Scope<Event>) {
    link.send_future(async move {
        if let Err(e) = fetch::like_question(BASE_API, event, id, like).await {
            log::error!("like question error: {e}");
        }

        Msg::QuestionUpdated(id)
    });
}

//TODO: dedup
fn request_fetch(id: String, secret: Option<String>, link: &html::Scope<Event>) {
    link.send_future(async move {
        let res = fetch::fetch_event(BASE_API, id, secret).await;

        res.map_or(Msg::Fetched(None), |val| Msg::Fetched(Some(val)))
    });
}

fn request_capture(id: String, order_id: String, link: &html::Scope<Event>) {
    link.send_future(async move {
        if let Err(e) = fetch::mod_premium_capture(BASE_API, id, order_id).await {
            log::error!("mod_premium_capture error: {e}");
        }

        Msg::Captured
    });
}

fn request_event_change(
    id: String,
    secret: Option<String>,
    change: ModEvent,
    link: &html::Scope<Event>,
) {
    link.send_future(async move {
        if let Err(e) = fetch::mod_edit_event(BASE_API, id, secret.unwrap_throw(), change).await {
            log::error!("mod_edit_event error: {e}");
        }

        Msg::StateChanged
    });
}

const fn question_state(q: &QuestionItem) -> &str {
    if q.answered {
        "answered"
    } else if q.hidden {
        "hidden"
    } else {
        "asked"
    }
}

impl Event {
    fn is_premium(&self) -> bool {
        self.state
            .event
            .as_ref()
            .is_some_and(|e| e.info.is_premium())
    }

    fn export_event(&self) {
        if !self.is_premium() {
            return;
        }

        tracking::track_event(tracking::EVNT_EXPORT);

        let name = self
            .state
            .event
            .as_ref()
            .map(|e| e.info.data.name.clone())
            .unwrap_or_default();

        let csv = self.event_as_csv().unwrap_or_default();

        let anchor = gloo_utils::document()
            .create_element("a")
            .unwrap_throw()
            .dyn_into::<HtmlAnchorElement>()
            .unwrap_throw();

        anchor.set_href(&format!(
            "data:text/csv;charset=utf-8,{}",
            web_sys::js_sys::encode_uri_component(&csv)
        ));
        anchor.set_target("_blank");
        anchor.set_download(&format!("live-ask:{name}.csv"));
        anchor.click();
    }

    fn event_as_csv(&self) -> anyhow::Result<String> {
        use csv::WriterBuilder;

        let questions = self
            .state
            .event
            .as_ref()
            .map(|e| e.info.questions.clone())
            .unwrap_or_default();
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(["date (utc)", "text", "state", "likes"])
            .unwrap_throw();
        for q in questions {
            let create_time = DateTime::<Utc>::from_naive_utc_and_offset(
                DateTime::from_timestamp(q.create_time_unix, 0)
                    .unwrap_throw()
                    .naive_utc(),
                Utc,
            );
            let state = question_state(&q).to_string();

            wtr.write_record(&[
                create_time.format("%Y-%m-%d %H:%M").to_string(),
                q.text,
                state,
                q.likes.to_string(),
            ])?;
        }
        Ok(String::from_utf8(wtr.into_inner()?)?)
    }

    fn view_internal(&self, ctx: &Context<Self>) -> Html {
        match self.loading_state {
            LoadingState::Loaded => self.view_event(ctx),
            LoadingState::Loading => {
                html! {
                    <div class="noevent">
                        <h2>{ "loading event..." }</h2>
                    </div>
                }
            }
            LoadingState::NotFound => {
                html! {
                    <div class="noevent">
                        <h2>{ "event not found" }</h2>
                    </div>
                }
            }
            LoadingState::Deleted => {
                html! {
                    <div class="noevent">
                        <h2>{ "event deleted" }</h2>
                    </div>
                }
            }
        }
    }

    #[allow(clippy::if_not_else)]
    fn view_event(&self, ctx: &Context<Self>) -> Html {
        self.state.event.as_ref().map_or_else(|| html! {}, |e| {
            let share_url = if e.info.data.short_url.is_empty() {
                e.info.data.long_url.clone().unwrap_or_default()
            } else {
                e.info.data.short_url.clone()
            };

            let background = classes!(match self.mode {
                Mode::Moderator => "bg-mod",
                Mode::Viewer => "bg-event",
            });

            let mod_view = matches!(self.mode, Mode::Moderator);
            let admin = e.admin;
            let is_premium = e.info.is_premium();
            let is_masked = e.masked;
            let is_first_24h = EventInfo::during_first_day(e.info.create_time_unix);

            let tags = SharableTags::clone(&self.tags);
            let current_tag = e.info.tags.current_tag;
            let screening_enabled = e.info.flags.contains(EventFlags::SCREENING);

            let color = self
                .state
                .event
                .as_ref()
                .and_then(|e| e.info.data.color.clone())
                .map_or_else(|| String::from("#282828"),|c| c.0);

            html! {
                <div class="some-event">
                    <div class={background} />
                    <PasswordPopup
                        event={e.info.tokens.public_token.clone()}
                        show={e.is_wrong_pwd()}
                        onconfirmed={ctx.link().callback(|()|Msg::PasswordSet)}
                    />
                    <QuestionPopup event_id={e.info.tokens.public_token.clone()} {current_tag} tags={SharableTags::clone(&tags)} />
                    <SharePopup url={share_url} event_id={e.info.tokens.public_token.clone()} />
                    <div class="event-block">
                        <EventMeta
                            context={e.info.context.clone()}
                            tokens={e.info.tokens.clone()}
                            data={e.info.data.clone()}
                            {is_premium}
                            {is_masked}
                            {is_first_24h}
                             />
                        { self.mod_view(ctx,e,tags) }
                        <div class="not-open" hidden={!e.info.state.is_closed()}>
                            { "This event was closed by the moderator. You cannot add or vote questions anymore." }
                            <br />
                            { "Updates by the moderator are still seen in real-time." }
                        </div>
                        <div class="not-open" hidden={!e.info.state.is_vote_only()}>
                            { "This event is set to vote-only by the moderator. You cannot add new questions. You can still vote though." }
                        </div>
                        <div class="not-open" hidden={!e.is_timed_out()}>
                            { "This free event timed out. Only the moderator can upgrade it to be accessible again." }
                        </div>
                    </div>
                    { self.mod_urls(ctx,admin) }
                    <div class="event-area" style={format!("background-color: {color}")}>
                        { self.view_stats() }
                        <div class="review-note" hidden={!screening_enabled || mod_view}>
                        { "Moderator enabled question reviewing. New questions have to be approved first." }
                        </div>
                        { self.view_questions(ctx,e) }
                        { Self::view_ask_question(mod_view,ctx,e) }
                    </div>
                </div>
            }
        })
    }

    #[allow(clippy::if_not_else)]
    fn view_ask_question(mod_view: bool, ctx: &Context<Self>, e: &GetEventResponse) -> Html {
        if mod_view {
            html! {}
        } else {
            html! {
                <div class="addquestion" hidden={!e.info.state.is_open()}>
                    <button
                        class="button-red"
                        onclick={ctx.link().callback(|_| Msg::AskQuestionClick)}
                    >
                        { "Ask a Question" }
                    </button>
                </div>
            }
        }
    }

    const fn is_mod(&self) -> bool {
        matches!(self.mode, Mode::Moderator)
    }

    fn view_questions(&self, ctx: &Context<Self>, e: &GetEventResponse) -> Html {
        if e.info.questions.is_empty() && self.unscreened.is_empty() {
            html! { <div class="noquestions">{ "no questions yet" }</div> }
        } else {
            let can_vote = !e.is_closed();
            let is_mod = self.is_mod();
            html! {
                <>
                    { self.view_items(ctx,&self.unscreened,if is_mod {"For review"} else {"Your Questions in review by host"},can_vote) }
                    { self.view_items(ctx,&self.unanswered,"Hot Questions",can_vote) }
                    { self.view_items(ctx,&self.answered,"Answered",can_vote) }
                    { self.view_items(ctx,&self.hidden,"Hidden",can_vote) }
                </>
            }
        }
    }

    fn view_items(
        &self,
        ctx: &Context<Self>,
        items: &[Rc<QuestionItem>],
        title: &str,
        can_vote: bool,
    ) -> Html {
        if !items.is_empty() {
            let masked = self.state.event.as_ref().is_some_and(|e| e.masked);

            return html! {
                <div>
                    <div class="questions-seperator">{ title }</div>
                    <div class="questions">
                        { for items.iter().enumerate().map(|(e,i)|self.view_item(ctx,can_vote,masked,e,i)) }
                    </div>
                </div>
            };
        }

        html! {}
    }

    fn view_item(
        &self,
        ctx: &Context<Self>,
        can_vote: bool,
        blurr: bool,
        index: usize,
        item: &Rc<QuestionItem>,
    ) -> Html {
        let local_like = LocalCache::is_liked(&self.current_event_id, item.id);
        let mod_view = matches!(self.mode, Mode::Moderator);
        let is_new = self.state.new_question.is_some_and(|id| id == item.id);

        let mut flags = QuestionFlags::empty();

        flags.set(QuestionFlags::NEW_QUESTION, is_new);
        flags.set(QuestionFlags::MOD_VIEW, mod_view);
        flags.set(QuestionFlags::LOCAL_LIKE, local_like);
        flags.set(QuestionFlags::CAN_VOTE, can_vote);
        flags.set(QuestionFlags::BLURR, blurr);

        let tag = item
            .tag
            .as_ref()
            .and_then(|tag| self.tags.get(tag))
            .cloned();

        html! {
            <Question
                {item}
                {index}
                key={item.id}
                {flags}
                {tag}
                on_click={ctx.link().callback(Msg::QuestionClick)}
            />
        }
    }

    //TODO: make mod component
    fn mod_view(&self, ctx: &Context<Self>, e: &GetEventResponse, tags: SharableTags) -> Html {
        if !matches!(self.mode, Mode::Moderator) {
            return html! {};
        }

        let payment_allowed = !e.info.is_premium();
        let pending_payment = self.query_params.paypal_token.is_some() && payment_allowed;

        let timed_out = e.is_timed_out();
        let pwd = e
            .mod_info
            .as_ref()
            .map(|info| info.pwd.clone())
            .unwrap_or_default();

        html! {
            <>
                <div class="mod-panel">
                    <DeletePopup tokens={e.info.tokens.clone()} />
                    { if timed_out {html!{}}else {html!{
                        <div class="state">
                            <select onchange={ctx.link().callback(Msg::ModStateChange)} >
                                <option value="0" selected={e.info.state.is_open()}>{"Event open"}</option>
                                <option value="1" selected={e.info.state.is_vote_only()}>{"Event vote only"}</option>
                                <option value="2" selected={e.info.state.is_closed()}>{"Event closed"}</option>
                            </select>
                        </div>
                        }} }
                    <button class="button-white" onclick={ctx.link().callback(|_|Msg::ModDelete)}>
                        { "Delete Event" }
                    </button>
                    <ModPassword tokens={e.info.tokens.clone()} {pwd} />
                    { if e.info.is_premium() {
                            Self::mod_view_premium(ctx,e,tags)
                        } else { html!{} } }
                </div>
                { if payment_allowed {
                        html!{
                            <Upgrade pending={pending_payment} tokens={e.info.tokens.clone()} />
                        }
                    } else { html!{} } }
                { Self::mod_view_deadline(e) }
            </>
        }
    }

    fn mod_view_premium(ctx: &Context<Self>, e: &GetEventResponse, tags: SharableTags) -> Html {
        let tag = e.info.tags.get_current_tag_label();

        html! {
            <div class="premium">
                <div class="title">{ "This is a premium event" }</div>
                <div class="button-box">
                    <div
                        class="screening-option"
                        onclick={ctx.link().callback(|_| Msg::ModEditScreening)}
                    >
                        <input
                            type="checkbox"
                            id="vehicle1"
                            name="vehicle1"
                            checked={e.info.is_screening()}
                        />
                        { "Screening" }
                    </div>
                    <button class="button-white" onclick={ctx.link().callback(|_|Msg::ModExport)}>
                        { "Export" }
                    </button>
                </div>
                <ModTags tokens={e.info.tokens.clone()} {tag} {tags} />
            </div>
        }
    }

    fn view_stats(&self) -> Html {
        if !self.is_premium() {
            return html! {};
        }

        let viewers = self
            .state
            .event
            .as_ref()
            .map(|e| e.viewers)
            .unwrap_or_default();
        let likes = self
            .state
            .event
            .as_ref()
            .map(GetEventResponse::get_likes)
            .unwrap_or_default();
        let questions = self
            .state
            .event
            .as_ref()
            .map(|e| e.info.questions.len())
            .unwrap_or_default();

        html! {
            <div class="statistics">
                <abbr title="current viewers" tabindex="0">
                    <img alt="viewers" src="/assets/symbols/viewers.svg" />
                </abbr>
                <div class="count">{ {viewers} }</div>
                <abbr title="all questions" tabindex="0">
                    <img alt="questions" src="/assets/symbols/questions.svg" />
                </abbr>
                <div class="count">{ {questions} }</div>
                <abbr title="all likes" tabindex="0">
                    <img alt="likes" src="/assets/symbols/likes.svg" />
                </abbr>
                <div class="count">{ {likes} }</div>
            </div>
        }
    }

    fn mod_view_deadline(e: &GetEventResponse) -> Html {
        if e.info.is_premium() {
            html! {
                <div class="deadline">{ "This is a premium event and will not time out!" }</div>
            }
        } else {
            html! {
                <div class="deadline">
                    { "Currently a " }
                    <strong>{ "free" }</strong>
                    { format!(" event is valid for {FREE_EVENT_DURATION_DAYS} days. Your event will be accessible until ") }
                    <span>{ Self::get_event_timeout(&e.info) }</span>
                    { ". Please upgrade to a " }
                    <strong>{ "premium" }</strong>
                    { " event to make it " }
                    <strong>{ "permanent" }</strong>
                    { "." }
                </div>
            }
        }
    }

    fn mod_urls(&self, ctx: &Context<Self>, admin: bool) -> Html {
        if matches!(self.mode, Mode::Moderator) || (matches!(self.mode, Mode::Viewer) && admin) {
            html! {
                <div id="moderator-urls">
                    <div class="linkbox-title">{ "This is your moderation link" }</div>
                    <div class="linkbox-box">
                        <div class="linkbox-url">
                            <div>{ self.moderator_url() }</div>
                        </div>
                        <div class="linkbox-copy" onclick={ctx.link().callback(|_| Msg::CopyLink)}>
                            { if self.copied_to_clipboard {"Copied"}else{"Copy"} }
                        </div>
                    </div>
                    <div class="floating-share">
                        <button
                            class="button-dark"
                            onclick={ctx.link().callback(|_| Msg::ShareEventClick)}
                        >
                            { "Share my event" }
                        </button>
                        <button class="button-blue">
                            <a
                                class="feedback-anchor"
                                href="https://josephhillco.notion.site/037e452ab13c451b8e24161b115c8d00?pvs=105"
                                target="_blank"
                                onclick={ctx.link().callback(|_| Msg::FeedbackClick)}
                            >
                                <div class="feedback-text">{ "Give us feedback" }</div>
                            </a>
                        </button>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn moderator_url(&self) -> String {
        self.state
            .event
            .as_ref()
            .map(|e| {
                format!(
                    "https://www.live-ask.com/eventmod/{}/{}",
                    e.info.tokens.public_token,
                    e.info.tokens.moderator_token.clone().unwrap_throw()
                )
            })
            .unwrap_or_default()
    }

    //TODO: put event duration into object from backend
    fn get_event_timeout(e: &EventInfo) -> Html {
        let event_duration = Duration::days(FREE_EVENT_DURATION_DAYS);

        let create_time = DateTime::<Utc>::from_naive_utc_and_offset(
            DateTime::from_timestamp(e.create_time_unix, 0)
                .unwrap_throw()
                .naive_utc(),
            Utc,
        );
        let end_time = create_time + event_duration;

        html! { end_time.format("%F") }
    }

    fn init_event(&mut self) {
        use split_iter::Splittable;

        self.unanswered.clear();
        self.tags = Rc::default();

        if let Some(e) = &self.state.event {
            let mut questions = e.info.questions.clone();
            questions.sort_by(|a, b| b.likes.cmp(&a.likes));

            let local_unscreened =
                LocalCache::unscreened_questions(&e.info.tokens.public_token, &questions);

            questions.extend(local_unscreened);

            let (unscreened, screened) = questions.into_iter().map(Rc::new).split(|i| !i.screening);
            let (not_hidden, hidden) = screened.into_iter().split(|i| i.hidden);
            let (unanswered, answered) = not_hidden.into_iter().split(|i| i.answered);

            self.unscreened = unscreened.collect();
            self.answered = answered.collect();
            self.unanswered = unanswered.collect();
            self.hidden = hidden.collect();

            self.tags = e
                .info
                .tags
                .tags
                .iter()
                .map(|t| (t.id, t.name.clone()))
                .collect::<HashMap<_, _>>()
                .into();
        }
    }

    fn on_fetched(&mut self, res: Option<&GetEventResponse>) {
        //TODO: in subsequent fetches only update data if successfully fetched

        if matches!(
            self.loading_state,
            LoadingState::Loading | LoadingState::NotFound
        ) {
            if let Some(ev) = res {
                if ev.is_deleted() && !ev.admin {
                    self.loading_state = LoadingState::Deleted;
                    return;
                }
                self.loading_state = LoadingState::Loaded;
            } else {
                self.loading_state = LoadingState::NotFound;
                return;
            }
        }

        if let Some(ev) = res {
            self.dispatch.reduce(|old| {
                (*old)
                    .clone()
                    .set_event(Some(ev.clone()))
                    .set_admin(ev.admin)
                    .into()
            });
            self.state = self.dispatch.get();
            self.init_event();
        }
    }

    fn on_question_click(&self, kind: &QuestionClickType, id: i64, ctx: &Context<Self>) {
        match kind {
            QuestionClickType::Like => {
                let liked = LocalCache::is_liked(&self.current_event_id, id);
                if liked {
                    tracking::track_event(tracking::EVNT_QUESTION_UNLIKE);
                } else {
                    tracking::track_event(tracking::EVNT_QUESTION_LIKE);
                }
                LocalCache::set_like_state(&self.current_event_id, id, !liked);
                request_like(self.current_event_id.clone(), id, !liked, ctx.link());
            }
            QuestionClickType::Hide => {
                if let Some(q) = self.state.event.as_ref().unwrap_throw().get_question(id) {
                    request_toggle_hide(
                        self.current_event_id.clone(),
                        ctx.props().secret.clone().unwrap_throw(),
                        q,
                        ctx.link(),
                    );
                }
            }
            QuestionClickType::Answer => {
                if let Some(q) = self.state.event.as_ref().unwrap_throw().get_question(id) {
                    request_toggle_answered(
                        self.current_event_id.clone(),
                        ctx.props().secret.clone().unwrap_throw(),
                        q,
                        ctx.link(),
                    );
                }
            }
            QuestionClickType::Approve => {
                if let Some(q) = self.state.event.as_ref().unwrap_throw().get_question(id) {
                    request_approve_question(
                        self.current_event_id.clone(),
                        ctx.props().secret.clone().unwrap_throw(),
                        q,
                        ctx.link(),
                    );
                }
            }
        }
    }

    fn handle_socket(&mut self, msg: SocketResponse, ctx: &Context<Self>) -> bool {
        match msg {
            SocketResponse::Connecting | SocketResponse::Connected => {
                self.manual_reconnect = false;
                self.events.emit(GlobalEvent::SocketStatus {
                    connected: true,
                    timeout_secs: None,
                });

                false
            }
            SocketResponse::Disconnected { reconnect } => {
                self.events.emit(GlobalEvent::SocketStatus {
                    connected: false,
                    timeout_secs: reconnect.map(|duration| duration.num_seconds()),
                });

                false
            }
            SocketResponse::Message(msg) => {
                let fetch_event = if msg == "e" {
                    log::info!("received event update");
                    true
                } else if let Some(stripped_msg) = msg.strip_prefix("q:") {
                    //TODO: only fetch q on "q"?

                    let id = stripped_msg.parse::<i64>().unwrap_throw();

                    log::info!("received question update: {}", id);

                    let found = self
                        .state
                        .event
                        .as_ref()
                        .is_some_and(|e| e.info.questions.iter().any(|q| q.id == id));

                    if !found {
                        log::info!("new question: {}", id);
                        self.dispatch
                            .reduce(|old| (*old).clone().set_new_question(Some(id)).into());
                        self.state = self.dispatch.get();
                    }

                    true
                } else if msg.starts_with("v:") {
                    log::debug!("received viewer update: {}", msg);

                    let viewers = msg
                        .split(':')
                        .nth(1)
                        .and_then(|text| text.parse::<i64>().ok())
                        .unwrap_or_default();

                    self.dispatch
                        .reduce(|old| (*old).clone().set_event_viewers(viewers).into());
                    self.state = self.dispatch.get();

                    false
                } else {
                    log::error!("unknown push msg: {msg}",);
                    true
                };

                if fetch_event {
                    request_fetch(
                        self.current_event_id.clone(),
                        ctx.props().secret.clone(),
                        ctx.link(),
                    );
                }

                !fetch_event
            }
        }
    }

    fn handle_global_event(&mut self, ev: GlobalEvent) -> bool {
        match ev {
            GlobalEvent::QuestionCreated(id) => {
                self.dispatch
                    .reduce(|old| (*old).clone().set_new_question(Some(id)).into());
                self.state = self.dispatch.get();
                true
            }
            GlobalEvent::SocketManualReconnect => {
                self.manual_reconnect = true;
                true
            }
            _ => false,
        }
    }

    fn handle_fetched(&mut self, res: Option<GetEventResponse>, ctx: &Context<Self>) -> bool {
        self.on_fetched(res.as_ref());
        if let Some(e) = res {
            if !e.info.is_premium() && self.query_params.paypal_token.is_some() {
                request_capture(
                    e.info.tokens.public_token,
                    self.query_params.paypal_token.clone().unwrap_throw(),
                    ctx.link(),
                );
            }
        }
        true
    }
}
