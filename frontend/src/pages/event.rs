use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use shared::{EventInfo, Item, ModQuestion, States};
use std::{rc::Rc, str::FromStr};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yewdux::prelude::*;

use crate::{
    agents::{EventAgent, GlobalEvent, SocketInput, WebSocketAgent, WsResponse},
    components::{DeletePopup, Question, QuestionClickType, QuestionPopup, SharePopup},
    fetch,
    local_cache::LocalCache,
    State,
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

// pub const BASE_API: &str = "http://localhost:8090";
// pub const BASE_SOCKET: &str = "ws://localhost:8090";
// pub const BASE_API: &str = "https://beta.www.live-ask.com";
// pub const BASE_SOCKET: &str = "wss://beta.www.live-ask.com";
pub const BASE_API: &str = "https://api.www.live-ask.com";
pub const BASE_SOCKET: &str = "wss://api.www.live-ask.com";

pub struct Event {
    event_id: String,
    copied_to_clipboard: bool,
    mode: Mode,
    state: Rc<State>,
    unanswered: Vec<Rc<Item>>,
    answered: Vec<Rc<Item>>,
    hidden: Vec<Rc<Item>>,
    loading_state: LoadingState,
    dispatch: Dispatch<State>,
    _socket_agent: Box<dyn Bridge<WebSocketAgent>>,
    _events: Box<dyn Bridge<EventAgent>>,
}
pub enum Msg {
    ShareEventClick,
    AskQuestionClick,
    Fetched(Option<EventInfo>),
    SocketMsg(WsResponse),
    QuestionClick((i64, QuestionClickType)),
    QuestionUpdated(i64),
    ModDelete,
    ModStateChange(yew::Event),
    StateChanged,
    CopyLink,
    GlobalEvent(GlobalEvent),
}
impl Component for Event {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let event_id = ctx.props().id.clone();
        request_fetch(event_id.clone(), ctx.props().secret.clone(), ctx.link());

        let mut ws = WebSocketAgent::bridge(ctx.link().callback(Msg::SocketMsg));
        ws.send(SocketInput::Connect(format!(
            "{}/push/{}",
            BASE_SOCKET, event_id
        )));

        Self {
            event_id,
            copied_to_clipboard: false,
            mode: if ctx.props().secret.is_some() {
                Mode::Moderator
            } else {
                Mode::Viewer
            },

            loading_state: LoadingState::Loading,
            state: Default::default(),
            unanswered: Vec::new(),
            answered: Vec::new(),
            hidden: Vec::new(),
            dispatch: Dispatch::<State>::subscribe(Callback::noop()),
            _socket_agent: ws,
            _events: EventAgent::bridge(ctx.link().callback(Msg::GlobalEvent)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::QuestionClick((id, kind)) => {
                match kind {
                    QuestionClickType::Like => {
                        let liked = LocalCache::is_liked(&self.event_id, id);
                        LocalCache::set_like_state(&self.event_id, id, !liked);
                        request_like(self.event_id.clone(), id, !liked, ctx.link());
                    }
                    QuestionClickType::Hide => {
                        if let Some(q) = self.state.event.as_ref().unwrap().get_question(id) {
                            request_toggle_hide(
                                self.event_id.clone(),
                                ctx.props().secret.clone().unwrap(),
                                q,
                                ctx.link(),
                            )
                        }
                    }
                    QuestionClickType::Answer => {
                        if let Some(q) = self.state.event.as_ref().unwrap().get_question(id) {
                            request_toggle_answered(
                                self.event_id.clone(),
                                ctx.props().secret.clone().unwrap(),
                                q,
                                ctx.link(),
                            )
                        }
                    }
                }

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
            Msg::SocketMsg(msg) => {
                match msg {
                    WsResponse::Ready | WsResponse::Disconnected => {}
                    WsResponse::Message(msg) => {
                        //TODO: do we want to act differently here? only fetch q on "q"?
                        if msg == "e" {
                            log::info!("received event update");
                        } else if msg.starts_with('q') {
                            log::info!("received question update: {}", msg);
                        }

                        request_fetch(
                            self.event_id.clone(),
                            ctx.props().secret.clone(),
                            ctx.link(),
                        );
                    }
                }

                false
            }
            Msg::StateChanged => false, // nothing needs to happen here
            Msg::ModStateChange(ev) => {
                use wasm_bindgen::JsCast;

                let e: web_sys::HtmlSelectElement = ev.target().unwrap().dyn_into().unwrap();
                let new_state = States::from_str(e.value().as_str()).unwrap();

                request_state_change(
                    self.event_id.clone(),
                    ctx.props().secret.clone(),
                    new_state,
                    ctx.link(),
                );

                false
            }
            Msg::ModDelete => {
                self._events.send(GlobalEvent::DeletePopup);
                false
            }
            Msg::ShareEventClick => {
                self._events.send(GlobalEvent::OpenSharePopup);
                false
            }
            Msg::AskQuestionClick => {
                self._events.send(GlobalEvent::OpenQuestionPopup);
                false
            }
            Msg::Fetched(res) => {
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
                    self.dispatch.reduce(|old| State {
                        event: Some(res.clone().unwrap()),
                        new_question: old.new_question,
                    });
                    self.state = self.dispatch.get();

                    self.init_event();
                }

                true
            }
            Msg::GlobalEvent(ev) => match ev {
                GlobalEvent::QuestionCreated(id) => {
                    self.dispatch.reduce(|old| State {
                        event: old.event.clone(),
                        new_question: Some(id),
                    });
                    self.state = self.dispatch.get();
                    true
                }
                _ => false,
            },
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.dispatch.reduce(|_| State::default());
        self._socket_agent.send(SocketInput::Disconnect);
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="event">
                {self.view_internal(ctx)}
            </div>
        }
    }
}

fn request_toggle_hide(event: String, secret: String, item: Item, link: &html::Scope<Event>) {
    link.send_future(async move {
        let modify = ModQuestion {
            hide: !item.hidden,
            answered: item.answered,
        };
        if let Err(res) = fetch::mod_question(BASE_API, event, secret, item.id, modify).await {
            log::error!("hide error: {}", res);
        }

        Msg::QuestionUpdated(item.id)
    });
}

fn request_toggle_answered(event: String, secret: String, item: Item, link: &html::Scope<Event>) {
    link.send_future(async move {
        let modify = ModQuestion {
            hide: item.hidden,
            answered: !item.answered,
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

        if let Ok(val) = res {
            Msg::Fetched(Some(val))
        } else {
            Msg::Fetched(None)
        }
    });
}

fn request_state_change(
    id: String,
    secret: Option<String>,
    state: States,
    link: &html::Scope<Event>,
) {
    link.send_future(async move {
        if let Err(e) = fetch::mod_state_change(BASE_API, id, secret.unwrap(), state).await {
            log::error!("mod_state_change error: {e}");
        }

        Msg::StateChanged
    });
}

impl Event {
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

    fn view_event(&self, ctx: &Context<Self>) -> Html {
        if let Some(e) = self.state.event.as_ref() {
            let share_url = if e.data.short_url.is_empty() {
                e.data.long_url.clone().unwrap_or_default()
            } else {
                e.data.short_url.clone()
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

                    <QuestionPopup event={e.tokens.public_token.clone()} />
                    <SharePopup url={share_url} event_id={e.tokens.public_token.clone()}/>

                    <div class="event-block">
                        <div class="event-name-label">{"The Event"}</div>
                        <div class="event-name">{&e.data.name.clone()}</div>
                        //TODO: collapsable event desc
                        <div class="event-desc">
                            {{&e.data.description.clone()}}
                        </div>

                        {self.mod_view(ctx,e)}

                        <div class="not-open" hidden={!e.state.is_closed()}>
                            {"This event was closed by the moderator. You cannot add or vote questions anymore."}
                            <br/>
                            {"Updates by the moderator are still seen in real-time."}
                        </div>
                        <div class="not-open" hidden={!e.state.is_vote_only()}>
                            {"This event is set to vote-only by the moderator. You cannot add new questions. You can still vote though."}
                        </div>
                    </div>

                    {self.mod_urls(ctx)}

                    {self.view_questions(ctx,e)}

                    {self.view_ask_question(mod_view,ctx,e)}
                </div>
            }
        } else {
            html! {}
        }
    }

    fn view_ask_question(&self, mod_view: bool, ctx: &Context<Self>, e: &EventInfo) -> Html {
        if mod_view {
            html! {}
        } else {
            html! {
                <div class="addquestion" hidden={!e.state.is_open()}>
                    <button class="button-red" onclick={ctx.link().callback(|_| Msg::AskQuestionClick)}>
                        {"Ask a Question"}
                    </button>
                </div>
            }
        }
    }

    fn view_questions(&self, ctx: &Context<Self>, e: &EventInfo) -> Html {
        if e.questions.is_empty() {
            let no_questions_classes = classes!(match self.mode {
                Mode::Moderator => "noquestions modview",
                _ => "noquestions",
            });

            html! {
                <div class={no_questions_classes}>
                    {"no questions yet"}
                </div>
            }
        } else {
            let can_vote = !e.state.is_closed();
            html! {
                <>
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
        items: &[Rc<Item>],
        title: &str,
        can_vote: bool,
    ) -> Html {
        if !items.is_empty() {
            let title_classes = classes!(match self.mode {
                Mode::Moderator => "questions-seperator modview",
                _ => "questions-seperator",
            });

            return html! {
                <div>
                    <div class={title_classes}>
                        {title}
                    </div>
                    <div class="questions">
                        {
                            for items.iter().enumerate().map(|(e,i)|self.view_item(ctx,can_vote,e,i))
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
        index: usize,
        item: &Rc<Item>,
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
                on_click={ctx.link().callback(Msg::QuestionClick)}
                />
        }
    }

    fn mod_view(&self, ctx: &Context<Self>, e: &EventInfo) -> Html {
        if matches!(self.mode, Mode::Moderator) {
            html! {
            <>
            <div class="mod-panel" >
                <DeletePopup tokens={e.tokens.clone()} />

                <div class="state">
                    <select onchange={ctx.link().callback(Msg::ModStateChange)} >
                        <option value="0" selected={e.state.is_open()}>{"Event open"}</option>
                        <option value="1" selected={e.state.is_vote_only()}>{"Event vote only"}</option>
                        <option value="2" selected={e.state.is_closed()}>{"Event closed"}</option>
                    </select>
                </div>
                <button class="button-white" onclick={ctx.link().callback(|_|Msg::ModDelete)} >
                    {"Delete Event"}
                </button>
            </div>

            <div class="deadline">
                {"Currently an event is valid for 30 days. Your event will close on "}
                <span>{Self::get_event_timeout(e)}</span>
                {". Please "}
                <a href="mailto:mail@live-ask.com">
                {"contact us"}
                </a>
                {" if you need your event to persist."}
            </div>
            </>
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
                    e.tokens.public_token,
                    e.tokens.moderator_token.clone().unwrap()
                )
            })
            .unwrap_or_default()
    }

    //TODO: put event duration into object from backend
    fn get_event_timeout(e: &EventInfo) -> Html {
        let event_duration = Duration::days(30);

        let create_time =
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(e.create_time_unix, 0), Utc);
        let end_time = create_time + event_duration;

        html! {end_time.format("%F")}
    }

    fn init_event(&mut self) {
        use split_iter::Splittable;

        self.unanswered.clear();

        if let Some(e) = &self.state.event {
            let mut questions = e.questions.clone();
            questions.sort_by(|a, b| b.likes.cmp(&a.likes));

            let (not_hidden, hidden) = questions.into_iter().map(Rc::new).split(|i| i.hidden);

            let (unanswered, answered) = not_hidden.into_iter().split(|i| i.answered);

            self.answered = answered.collect();
            self.unanswered = unanswered.collect();
            self.hidden = hidden.collect();
        }
    }
}
