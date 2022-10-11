use std::rc::Rc;

use shared::{EventInfo, Item};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yewdux::prelude::*;

use crate::{
    agents::{EventAgent, GlobalEvent, SocketInput, WebSocketAgent},
    components::{Question, QuestionPopup, SharePopup},
    fetch,
    local_cache::LocalCache,
    State,
};

#[allow(dead_code)]
enum Mode {
    Print,
    Moderator,
    Viewer,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub id: String,
    pub secret: Option<String>,
}

enum LoadingState {
    Loading,
    Loaded,
    NotFound,
}

pub struct Event {
    event_id: String,
    mode: Mode,
    state: Rc<State>,
    unanswered: Vec<Rc<Item>>,
    answered: Vec<Rc<Item>>,
    hidden: Vec<Rc<Item>>,
    loading_state: LoadingState,
    #[allow(dead_code)]
    socket_agent: Box<dyn Bridge<WebSocketAgent>>,
    #[allow(dead_code)]
    events: Box<dyn Bridge<EventAgent>>,
    dispatch: Dispatch<State>,
}
pub enum Msg {
    ShareEventClick,
    AskQuestionClick,
    Fetched(Option<EventInfo>),
    SocketMsg,
    Like(i64),
    Liked,
}
impl Component for Event {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let event_id = ctx.props().id.clone();
        request_fetch(event_id.clone(), ctx.props().secret.clone(), ctx.link());

        //TODO: this leads to socket events like OnConnect also fetching event again
        let mut ws = WebSocketAgent::bridge(ctx.link().callback(|_msg| Msg::SocketMsg));
        ws.send(SocketInput::Connect(format!(
            "wss://api.www.live-ask.com/push/{}",
            // "ws://localhost:8090/push/{}",
            event_id
        )));

        Self {
            event_id,
            mode: if ctx.props().secret.is_some() {
                Mode::Moderator
            } else {
                Mode::Viewer
            },
            events: EventAgent::bridge(Callback::noop()),
            loading_state: LoadingState::Loading,
            state: Default::default(),
            unanswered: Vec::new(),
            answered: Vec::new(),
            hidden: Vec::new(),
            socket_agent: ws,
            dispatch: Dispatch::<State>::subscribe(Callback::noop()),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Like(id) => {
                let liked = LocalCache::is_liked(&self.event_id, id);
                LocalCache::set_like_state(&self.event_id, id, !liked);
                request_like(self.event_id.clone(), id, !liked, ctx.link());
                false
            }
            Msg::Liked => {
                request_fetch(
                    self.event_id.clone(),
                    ctx.props().secret.clone(),
                    ctx.link(),
                );
                false
            }
            Msg::SocketMsg => {
                request_fetch(
                    self.event_id.clone(),
                    ctx.props().secret.clone(),
                    ctx.link(),
                );
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
                self.loading_state = if res.is_none() {
                    LoadingState::NotFound
                } else {
                    LoadingState::Loaded
                };

                if matches!(self.loading_state, LoadingState::Loaded) {
                    self.dispatch.reduce(|old| State {
                        event: Some(res.clone().unwrap()),
                        modal_open: old.modal_open,
                    });
                    self.state = self.dispatch.get();
                }

                self.init_event();

                true
            }
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.dispatch.reduce(|_| State::default());
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="event">
                {self.view_internal(ctx)}
            </div>
        }
    }
}

fn request_like(event: String, id: i64, like: bool, link: &html::Scope<Event>) {
    link.send_future(async move {
        let _res = fetch::like_question(event, id, like).await.unwrap();
        Msg::Liked
    });
}

fn request_fetch(id: String, secret: Option<String>, link: &html::Scope<Event>) {
    link.send_future(async move {
        let res = fetch::fetch_event(id, secret).await;

        if let Ok(val) = res {
            Msg::Fetched(Some(val))
        } else {
            Msg::Fetched(None)
        }
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
                Mode::Print => "bg-print",
                Mode::Moderator => "bg-mod",
                Mode::Viewer => "bg-event",
            });

            let mod_view = matches!(self.mode, Mode::Moderator);

            html! {
                <div>
                    <div class={background}>
                    </div>

                    <QuestionPopup event={e.tokens.public_token.clone()} />
                    <SharePopup url={share_url} />

                    <div class="event-block">
                        <div class="event-name-label">{"The Event"}</div>
                        <div class="event-name">{&e.data.name.clone()}</div>
                        //TODO: collapsable event desc
                        <div class="event-desc"
                            // [class.printable]="printable"
                            >
                            {{&e.data.description.clone()}}
                        </div>

                        {
                            self.mod_view()
                        }
                    </div>

                    {self.mod_urls(ctx)}

                    {self.view_questions(ctx,e)}

                    {
                        if mod_view {
                            html!{}
                        }else{
                            html!{
                                <div
                                    //TODO:
                                    // *ngIf="!canModerate && isEventOpen(event)"
                                    class="addquestion"
                                    >
                                    <button class="button-red" onclick={ctx.link().callback(|_| Msg::AskQuestionClick)}>
                                        {"Ask a Question"}
                                    </button>
                                </div>
                            }
                        }
                    }
                </div>
            }
        } else {
            html! {}
        }
    }

    fn view_questions(&self, ctx: &Context<Self>, e: &EventInfo) -> Html {
        if e.questions.is_empty() {
            let no_questions_classes = classes!(match self.mode {
                Mode::Print => "bg-print",
                Mode::Moderator => "noquestions modview",
                _ => "noquestions",
            });

            html! {
            <div class={no_questions_classes}>
                {"no questions yet"}
            </div>
            }
        } else {
            html! {
                <>
                    {self.view_items(ctx,&self.unanswered,"Hot Questions")}
                    {self.view_items(ctx,&self.answered,"Answered")}
                    {self.view_items(ctx,&self.hidden,"Hidden")}
                </>
            }
        }
    }

    fn view_items(&self, ctx: &Context<Self>, items: &[Rc<Item>], title: &str) -> Html {
        if !items.is_empty() {
            let title_classes = classes!(match self.mode {
                Mode::Moderator => "questions-seperator modview",
                _ => "questions-seperator",
            });

            return html! {
                <div>
                    <div class={title_classes}>{title}</div>
                    <div class="questions">
                        {
                            for items.iter().enumerate().map(|(e,i)|self.view_item(ctx,e,i))
                        }
                    </div>
                </div>
            };
        }

        html! {}
    }

    fn view_item(&self, ctx: &Context<Self>, index: usize, item: &Rc<Item>) -> Html {
        let local_like = LocalCache::is_liked(&self.event_id, item.id);
        let mod_view = matches!(self.mode, Mode::Moderator);

        html! {
            <Question {item} {index} key={item.id} {local_like} {mod_view} on_click={ctx.link().callback(Msg::Like)}/>
        }
    }

    fn mod_view(&self) -> Html {
        if matches!(self.mode, Mode::Moderator) {
            html! {
            <div class="deadline">
                {"Currently an event is valid for 30 days. Your event will close on "}
                <span>{self.get_event_timeout()}</span>
                {". Please "}
                <a href="mailto:mail@live-ask.com">
                {"contact us"}
                </a>
                {" if you need your event to persist."}
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
                        <div class="linkbox-copy" >{
                            //TODO:
                            // {copiedLinkToClipboard?'Copied':'Copy'}
                            {"Copy"}
                        }</div>
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

    fn get_event_timeout(&self) -> Html {
        //TODO: get_event_timeout
        html! {"1.2.1970"}
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
