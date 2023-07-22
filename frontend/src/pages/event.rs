use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use const_format::formatcp;
use serde::Deserialize;
use shared::{EventInfo, GetEventResponse, ModQuestion, QuestionItem, States};
use std::{rc::Rc, str::FromStr};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlAnchorElement;
use worker::{WordCloudAgent, WordCloudInput, WordCloudOutput};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yew_router::{prelude::Location, scope_ext::RouterScopeExt};
use yewdux::prelude::*;

use crate::{
    agents::{EventAgent, GlobalEvent, SocketInput, WebSocketAgent, WsResponse},
    components::{DeletePopup, Question, QuestionClickType, QuestionPopup, SharePopup, Upgrade},
    environment::{la_env, LiveAskEnv},
    fetch,
    local_cache::LocalCache,
    tracking, State,
};

enum Mode {
    Moderator,
    Viewer,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub id: String,
    pub secret: Option<String>,
}

pub enum LoadingState {
    Loading,
    Loaded,
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
    event_id: String,
    copied_to_clipboard: bool,
    wordcloud: Option<String>,
    query_params: QueryParams,
    mode: Mode,
    state: Rc<State>,
    unanswered: Vec<Rc<QuestionItem>>,
    answered: Vec<Rc<QuestionItem>>,
    hidden: Vec<Rc<QuestionItem>>,
    unscreened: Vec<Rc<QuestionItem>>,
    loading_state: LoadingState,
    dispatch: Dispatch<State>,
    socket_agent: Box<dyn Bridge<WebSocketAgent>>,
    events: Box<dyn Bridge<EventAgent>>,
    wordcloud_agent: Box<dyn Bridge<WordCloudAgent>>,
}
pub enum Msg {
    ShareEventClick,
    AskQuestionClick,
    Fetched(Option<GetEventResponse>),
    Captured,
    SocketMsg(WsResponse),
    QuestionClick((i64, QuestionClickType)),
    QuestionUpdated(i64),
    ModDelete,
    ModExport,
    ModStateChange(yew::Event),
    StateChanged,
    CopyLink,
    ModEditScreening,
    GlobalEvent(GlobalEvent),
    WordCloud(WordCloudOutput),
}
impl Component for Event {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let event_id = ctx.props().id.clone();
        request_fetch(event_id.clone(), ctx.props().secret.clone(), ctx.link());

        let mut ws = WebSocketAgent::bridge(ctx.link().callback(Msg::SocketMsg));
        ws.send(SocketInput::Connect(format!(
            "{BASE_SOCKET}/push/{event_id}",
        )));

        let query_params = ctx
            .link()
            .location()
            .and_then(|loc| loc.query::<QueryParams>().ok())
            .unwrap_or_default();

        if let Some(token) = &query_params.paypal_token {
            log::info!("paypal-token: {}", token);
        }

        Self {
            event_id,
            query_params,
            wordcloud: None,
            copied_to_clipboard: false,
            mode: if ctx.props().secret.is_some() {
                Mode::Moderator
            } else {
                Mode::Viewer
            },
            loading_state: LoadingState::Loading,
            state: Rc::default(),
            unanswered: Vec::new(),
            answered: Vec::new(),
            hidden: Vec::new(),
            unscreened: Vec::new(),
            dispatch: Dispatch::<State>::subscribe(Callback::noop()),
            socket_agent: ws,
            events: EventAgent::bridge(ctx.link().callback(Msg::GlobalEvent)),
            wordcloud_agent: WordCloudAgent::bridge(ctx.link().callback(Msg::WordCloud)),
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
                gloo_utils::window()
                    .navigator()
                    .clipboard()
                    .map(|c| c.write_text(&self.moderator_url()));
                true
            }
            Msg::SocketMsg(msg) => self.push_received(msg, ctx),
            Msg::StateChanged => false, // nothing needs to happen here
            Msg::ModStateChange(ev) => {
                let e: web_sys::HtmlSelectElement =
                    ev.target().unwrap_throw().dyn_into().unwrap_throw();
                let new_state = States::from_str(e.value().as_str()).unwrap_throw();

                request_state_change(
                    self.event_id.clone(),
                    ctx.props().secret.clone(),
                    new_state,
                    ctx.link(),
                );

                false
            }
            Msg::ModEditScreening => {
                request_screening_change(
                    self.event_id.clone(),
                    ctx.props().secret.clone(),
                    self.state
                        .event
                        .as_ref()
                        .map(|e| !e.info.screening)
                        .unwrap_or_default(),
                    ctx.link(),
                );

                false
            }

            Msg::WordCloud(w) => {
                self.wordcloud = Some(w.0);
                true
            }
            Msg::ModDelete => {
                self.events.send(GlobalEvent::DeletePopup);
                false
            }
            Msg::ShareEventClick => {
                self.events.send(GlobalEvent::OpenSharePopup);
                false
            }
            Msg::AskQuestionClick => {
                self.events.send(GlobalEvent::OpenQuestionPopup);
                false
            }
            Msg::Fetched(res) => {
                self.on_fetched(&res);

                if let Some(e) = res {
                    if !e.info.premium && self.query_params.paypal_token.is_some() {
                        request_capture(
                            e.info.tokens.public_token,
                            self.query_params
                                .paypal_token
                                .as_ref()
                                .cloned()
                                .unwrap_throw(),
                            ctx.link(),
                        );
                    }
                }

                true
            }
            Msg::ModExport => {
                self.export_event();
                false
            }
            Msg::GlobalEvent(ev) => match ev {
                GlobalEvent::QuestionCreated(id) => {
                    self.dispatch
                        .reduce(|old| (*old).clone().set_new_question(Some(id)));
                    self.state = self.dispatch.get();
                    true
                }
                _ => false,
            },
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.dispatch.reduce(|_| State::default());
        self.socket_agent.send(SocketInput::Disconnect);
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="event">
                {self.view_internal(ctx)}
            </div>
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
            screened: item.screened,
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
            screened: item.screened,
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

fn request_state_change(
    id: String,
    secret: Option<String>,
    state: States,
    link: &html::Scope<Event>,
) {
    link.send_future(async move {
        if let Err(e) = fetch::mod_state_change(BASE_API, id, secret.unwrap_throw(), state).await {
            log::error!("mod_state_change error: {e}");
        }

        Msg::StateChanged
    });
}

fn request_screening_change(
    id: String,
    secret: Option<String>,
    screening: bool,
    link: &html::Scope<Event>,
) {
    link.send_future(async move {
        if let Err(e) =
            fetch::mod_edit_screening(BASE_API, id, secret.unwrap_throw(), screening).await
        {
            log::error!("mod_edit_screening error: {e}");
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
            .map(|e| e.info.premium)
            .unwrap_or_default()
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

        let anchor = gloo::utils::document()
            .create_element("a")
            .unwrap_throw()
            .dyn_into::<HtmlAnchorElement>()
            .unwrap_throw();

        anchor.set_href(&format!(
            "data:text/csv;charset=utf-8,{}",
            js_sys::encode_uri_component(&csv)
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
            let create_time = DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(q.create_time_unix, 0).unwrap_throw(),
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
                        <h2>{"loading event..."}</h2>
                    </div>
                }
            }
            LoadingState::NotFound => {
                html! {
                    <div class="noevent">
                        <h2>{"event not found"}</h2>
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

            html! {
                <div>
                    <div class={background}>
                    </div>

                    <QuestionPopup event={e.info.tokens.public_token.clone()} />
                    <SharePopup url={share_url} event_id={e.info.tokens.public_token.clone()}/>

                    <div class="event-block">
                        <div class="event-name-label">{"The Event"}</div>
                        <div class="event-name">{&e.info.data.name.clone()}</div>
                        //TODO: collapsable event desc
                        <div class="event-desc">
                            {{&e.info.data.description.clone()}}
                        </div>

                        {self.mod_view(ctx,e)}

                        <div class="not-open" hidden={!e.info.state.is_closed()}>
                            {"This event was closed by the moderator. You cannot add or vote questions anymore."}
                            <br/>
                            {"Updates by the moderator are still seen in real-time."}
                        </div>
                        <div class="not-open" hidden={!e.info.state.is_vote_only()}>
                            {"This event is set to vote-only by the moderator. You cannot add new questions. You can still vote though."}
                        </div>
                        <div class="not-open" hidden={!e.timed_out}>
                            {"This free event timed out. Only the moderator can upgrade it to be accessible again."}
                        </div>
                    </div>

                    {self.mod_urls(ctx)}

                    {self.view_viewers()}

                    {self.view_questions(ctx,e)}

                    {self.view_cloud()}

                    {Self::view_ask_question(mod_view,ctx,e)}
                </div>
            }
        })
    }

    fn view_cloud(&self) -> Html {
        let title_classes = self.question_separator_classes();

        self.wordcloud.as_ref().map_or_else(
            || html!(),
            |cloud| {
                html! {
                    <div>
                    <div class={title_classes}>
                        {"Word Cloud"}
                    </div>
                    {cloud_as_yew_img(cloud)}
                    </div>
                }
            },
        )
    }

    #[allow(clippy::if_not_else)]
    fn view_ask_question(mod_view: bool, ctx: &Context<Self>, e: &GetEventResponse) -> Html {
        if mod_view {
            html! {}
        } else {
            html! {
                <div class="addquestion" hidden={!e.info.state.is_open()}>
                    <button class="button-red" onclick={ctx.link().callback(|_| Msg::AskQuestionClick)}>
                        {"Ask a Question"}
                    </button>
                </div>
            }
        }
    }

    fn question_separator_classes(&self) -> Classes {
        classes!(match self.mode {
            Mode::Moderator => "questions-seperator modview",
            Mode::Viewer => "questions-seperator",
        })
    }

    const fn is_mod(&self) -> bool {
        matches!(self.mode, Mode::Moderator)
    }

    fn view_questions(&self, ctx: &Context<Self>, e: &GetEventResponse) -> Html {
        if e.info.questions.is_empty() {
            let no_questions_classes = classes!(match self.mode {
                Mode::Moderator => "noquestions modview",
                Mode::Viewer => "noquestions",
            });

            html! {
                <div class={no_questions_classes}>
                    {"no questions yet"}
                </div>
            }
        } else {
            let can_vote = !e.is_closed();
            let is_mod = self.is_mod();
            html! {
                <>
                    {self.view_items(ctx,&self.unscreened,if is_mod {"For review"} else {"Questions currently in review by host"},can_vote)}
                    {self.view_items(ctx,&self.unanswered,"Hot Questions",can_vote)}
                    {self.view_items(ctx,&self.answered,"Answered",can_vote)}
                    {self.view_items(ctx,&self.hidden,"Hidden",can_vote)}
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
            let title_classes = self.question_separator_classes();

            let blurr = self
                .state
                .event
                .as_ref()
                .map(|e| e.timed_out && !e.admin)
                .unwrap_or_default();

            return html! {
                <div>
                    <div class={title_classes}>
                        {title}
                    </div>
                    <div class="questions">
                        {
                            for items.iter().enumerate().map(|(e,i)|self.view_item(ctx,can_vote,blurr,e,i))
                        }
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
        let local_like = LocalCache::is_liked(&self.event_id, item.id);
        let mod_view = matches!(self.mode, Mode::Moderator);
        let is_new = self
            .state
            .new_question
            .map(|id| id == item.id)
            .unwrap_or_default();

        html! {
            <Question
                {item}
                {index}
                {is_new}
                {can_vote}
                key={item.id}
                {local_like}
                {mod_view}
                {blurr}
                on_click={ctx.link().callback(Msg::QuestionClick)}
                />
        }
    }

    fn mod_view(&self, ctx: &Context<Self>, e: &GetEventResponse) -> Html {
        let payment_allowed = !e.info.premium;
        let pending_payment = self.query_params.paypal_token.is_some() && !e.info.premium;

        if matches!(self.mode, Mode::Moderator) {
            let timed_out = e.timed_out;

            html! {
            <>
            <div class="mod-panel" >
                <DeletePopup tokens={e.info.tokens.clone()} />

                {
                    if timed_out {html!{}}else {html!{
                    <div class="state">
                        <select onchange={ctx.link().callback(Msg::ModStateChange)} >
                            <option value="0" selected={e.info.state.is_open()}>{"Event open"}</option>
                            <option value="1" selected={e.info.state.is_vote_only()}>{"Event vote only"}</option>
                            <option value="2" selected={e.info.state.is_closed()}>{"Event closed"}</option>
                        </select>
                    </div>
                    }}
                }

                <button class="button-white" onclick={ctx.link().callback(|_|Msg::ModDelete)} >
                    {"Delete Event"}
                </button>

                {
                    self.mod_view_export(ctx)
                }

            </div>

            {
                if payment_allowed {html!{
                    <Upgrade pending={pending_payment} tokens={e.info.tokens.clone()} />
                }}else{html!{}}
            }

            {Self::mod_view_deadline(e)}
            {Self::mod_view_screening(ctx,e)}

            </>
            }
        } else {
            html! {}
        }
    }

    fn mod_view_export(&self, ctx: &Context<Self>) -> Html {
        if self.is_premium() {
            html! {
                <button class="button-white" onclick={ctx.link().callback(|_|Msg::ModExport)} >
                    {"Export"}
                </button>
            }
        } else {
            html! {}
        }
    }

    fn view_viewers(&self) -> Html {
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
            <div class="statistics" >
                <abbr title="current viewers" tabindex="0"><img src="/assets/symbols/viewers.svg"/></abbr>
                <div class="count">{{viewers}}</div>
                <abbr title="all questions" tabindex="0"><img src="/assets/symbols/questions.svg"/></abbr>
                <div class="count">{{questions}}</div>
                <abbr title="all likes" tabindex="0"><img src="/assets/symbols/likes.svg"/></abbr>
                <div class="count">{{likes}}</div>
            </div>
        }
    }

    fn mod_view_deadline(e: &GetEventResponse) -> Html {
        if e.info.premium {
            html! {
                <div class="deadline">
                {"This is a premium event and will not time out!"}
                </div>
            }
        } else {
            html! {
                <div class="deadline">
                {format!("Currently a free event is valid for {FREE_EVENT_DURATION_DAYS} days. Your event will be inaccessible on ")}
                <span>{Self::get_event_timeout(&e.info)}</span>
                {". Please upgrade to premium to make it permanent."}
                </div>
            }
        }
    }

    fn mod_view_screening(ctx: &Context<Self>, e: &GetEventResponse) -> Html {
        if e.info.premium {
            html! {
                <div class="deadline" onclick={ctx.link().callback(|_| Msg::ModEditScreening)}>
                    <input type="checkbox" id="vehicle1" name="vehicle1" checked={e.info.screening} />
                    {"Screening"}
                </div>
            }
        } else {
            html! {}
        }
    }

    fn mod_urls(&self, ctx: &Context<Self>) -> Html {
        if matches!(self.mode, Mode::Moderator) {
            html! {
                <div id="moderator-urls">
                    <div class="linkbox-title">{"This is your moderation link"}</div>
                    <div class="linkbox-box">
                        <div class="linkbox-url">
                            <div>{self.moderator_url()}</div>
                        </div>
                        <div class="linkbox-copy" onclick={ctx.link().callback(|_| Msg::CopyLink)}>
                            {if self.copied_to_clipboard {"Copied"}else{"Copy"}}
                        </div>
                    </div>

                    <div class="floating-share">
                        <button class="button-white" onclick={ctx.link().callback(|_| Msg::ShareEventClick)}>
                            {"Share my event"}
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

        let create_time = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp_opt(e.create_time_unix, 0).unwrap_throw(),
            Utc,
        );
        let end_time = create_time + event_duration;

        html! {end_time.format("%F")}
    }

    fn init_event(&mut self) {
        use split_iter::Splittable;

        self.unanswered.clear();

        if let Some(e) = &self.state.event {
            let mut questions = e.info.questions.clone();
            questions.sort_by(|a, b| b.likes.cmp(&a.likes));

            let local_unscreened =
                LocalCache::unscreened_questions(&e.info.tokens.public_token, &questions);

            questions.extend(local_unscreened.into_iter());

            let (unscreened, screened) = questions.into_iter().map(Rc::new).split(|i| i.screened);
            let (not_hidden, hidden) = screened.into_iter().split(|i| i.hidden);
            let (unanswered, answered) = not_hidden.into_iter().split(|i| i.answered);

            self.unscreened = unscreened.collect();
            self.answered = answered.collect();
            self.unanswered = unanswered.collect();
            self.hidden = hidden.collect();

            if e.info.premium {
                self.wordcloud_agent.send(WordCloudInput(
                    e.info
                        .questions
                        .iter()
                        .map(|q| q.text.clone())
                        .collect::<Vec<_>>(),
                ));
            }
        }
    }

    fn on_fetched(&mut self, res: &Option<GetEventResponse>) {
        //TODO: in subsequent fetches only update data if succesfully fetched

        if matches!(
            self.loading_state,
            LoadingState::Loading | LoadingState::NotFound
        ) {
            self.loading_state = if res.is_none() {
                LoadingState::NotFound
            } else {
                LoadingState::Loaded
            };
        }
        if res.is_some() {
            self.dispatch.reduce(|old| {
                (*old)
                    .clone()
                    .set_event(Some(res.clone().unwrap_throw()))
                    .set_admin(res.as_ref().map(|res| res.admin).unwrap_or_default())
            });
            self.state = self.dispatch.get();

            self.init_event();
        }
    }

    fn on_question_click(&mut self, kind: &QuestionClickType, id: i64, ctx: &Context<Event>) {
        match kind {
            QuestionClickType::Like => {
                let liked = LocalCache::is_liked(&self.event_id, id);
                if liked {
                    tracking::track_event(tracking::EVNT_QUESTION_UNLIKE);
                } else {
                    tracking::track_event(tracking::EVNT_QUESTION_LIKE);
                }
                LocalCache::set_like_state(&self.event_id, id, !liked);
                request_like(self.event_id.clone(), id, !liked, ctx.link());
            }
            QuestionClickType::Hide => {
                if let Some(q) = self.state.event.as_ref().unwrap_throw().get_question(id) {
                    request_toggle_hide(
                        self.event_id.clone(),
                        ctx.props().secret.clone().unwrap_throw(),
                        q,
                        ctx.link(),
                    );
                }
            }
            QuestionClickType::Answer => {
                if let Some(q) = self.state.event.as_ref().unwrap_throw().get_question(id) {
                    request_toggle_answered(
                        self.event_id.clone(),
                        ctx.props().secret.clone().unwrap_throw(),
                        q,
                        ctx.link(),
                    );
                }
            }
            QuestionClickType::Approve => {
                if let Some(q) = self.state.event.as_ref().unwrap_throw().get_question(id) {
                    request_approve_question(
                        self.event_id.clone(),
                        ctx.props().secret.clone().unwrap_throw(),
                        q,
                        ctx.link(),
                    );
                }
            }
        }
    }

    fn push_received(&mut self, msg: WsResponse, ctx: &Context<Event>) -> bool {
        match msg {
            WsResponse::Ready | WsResponse::Disconnected => false,
            WsResponse::Message(msg) => {
                let fetch_event = if msg == "e" {
                    log::info!("received event update");
                    true
                } else if msg.starts_with("q:") {
                    //TODO: do we want to act differently here? only fetch q on "q"?
                    log::info!("received question update: {}", msg);
                    true
                } else if msg.starts_with("v:") {
                    log::info!("received viewer update: {}", msg);

                    let viewers = msg
                        .split(':')
                        .nth(1)
                        .and_then(|text| text.parse::<i64>().ok())
                        .unwrap_or_default();

                    self.dispatch.reduce(|old| {
                        (*old).clone().set_event(Some(GetEventResponse {
                            info: old
                                .event
                                .as_ref()
                                .map(|e| e.info.clone())
                                .unwrap_or_default(),
                            timed_out: old.event.as_ref().map(|e| e.timed_out).unwrap_or_default(),
                            admin: old.event.as_ref().map(|e| e.admin).unwrap_or_default(),
                            viewers,
                        }))
                    });
                    self.state = self.dispatch.get();

                    false
                } else {
                    log::error!("unknown push msg: {msg}",);
                    true
                };

                if fetch_event {
                    request_fetch(
                        self.event_id.clone(),
                        ctx.props().secret.clone(),
                        ctx.link(),
                    );
                }

                !fetch_event
            }
        }
    }
}

pub fn cloud_as_yew_img(b64: &str) -> yew::Html {
    html! {
        <div class="cloud">
         <img src={format!("data:image/png;base64,{b64}")} />
        </div>
    }
}
