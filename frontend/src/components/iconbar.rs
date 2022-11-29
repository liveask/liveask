use chrono::{Duration, Utc};
use gloo::timers::callback::Interval;
use std::rc::Rc;
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yew_router::{prelude::*, scope_ext::HistoryHandle};
use yewdux::prelude::*;

use crate::{
    agents::{EventAgent, GlobalEvent, SocketInput, WebSocketAgent},
    not,
    routes::Route,
    State,
};

pub enum Msg {
    State(Rc<State>),
    Share,
    Event(GlobalEvent),
    Ask,
    Home,
    Reconnect,
    ReconnectTimer,
    RouteChange,
}

#[derive(Properties, PartialEq, Eq)]
pub struct IconBarProps;

pub struct IconBar {
    connected: bool,
    reconnect_timeout: Option<chrono::DateTime<Utc>>,
    state: Rc<State>,
    _dispatch: Dispatch<State>,
    events: Box<dyn Bridge<EventAgent>>,
    socket_agent: Box<dyn Bridge<WebSocketAgent>>,
    _interal: Interval,
    _route_listener: HistoryHandle,
}
impl Component for IconBar {
    type Message = Msg;
    type Properties = IconBarProps;

    fn create(ctx: &Context<Self>) -> Self {
        //TODO: only set once a timeout is initiated
        let timer_interval = {
            let link = ctx.link().clone();
            Interval::new(500, move || link.send_message(Msg::ReconnectTimer))
        };

        let events = EventAgent::bridge(ctx.link().callback(Msg::Event));

        Self {
            _dispatch: Dispatch::<State>::subscribe(ctx.link().callback(Msg::State)),
            events,
            state: Rc::default(),
            connected: true,
            socket_agent: WebSocketAgent::bridge(Callback::noop()),
            reconnect_timeout: None,
            _interal: timer_interval,
            _route_listener: ctx
                .link()
                .add_history_listener(ctx.link().callback(|_history| Msg::RouteChange))
                .unwrap_throw(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::State(state) => {
                self.state = state;
                true
            }
            Msg::Share => {
                self.events.send(GlobalEvent::OpenSharePopup);
                false
            }
            Msg::Ask => {
                self.events.send(GlobalEvent::OpenQuestionPopup);
                false
            }
            Msg::Home => {
                ctx.link().history().unwrap_throw().push(Route::Home);
                false
            }
            Msg::Reconnect => {
                self.socket_agent.send(SocketInput::Reconnect);
                false
            }
            //ignore global events
            Msg::Event(msg) => match msg {
                GlobalEvent::SocketStatus {
                    connected,
                    timeout_secs,
                } => {
                    self.connected = connected;
                    self.reconnect_timeout = None;
                    if let Some(timeout_secs) = timeout_secs {
                        self.reconnect_timeout = Some(Utc::now() + Duration::seconds(timeout_secs));
                    }
                    true
                }
                _ => false,
            },
            Msg::ReconnectTimer => {
                //TODO: refresh only during timeout being set
                self.reconnect_timeout.is_some()
            }
            Msg::RouteChange => true,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let doc = gloo_utils::document();

        let logo_svg = {
            let logo_svg = include_str!("../../inline-assets/logo.svg");
            let div = doc.create_element("div").unwrap_throw();
            div.set_inner_html(logo_svg);
            Html::VRef(div.first_element_child().unwrap_throw().into())
        };

        let logo_text_svg = {
            let logo_svg = include_str!("../../inline-assets/logo_text.svg");
            let div = doc.create_element("div").unwrap_throw();
            div.set_inner_html(logo_svg);
            let svg = div.first_element_child().unwrap_throw();
            svg.class_list().add_1("shrink").unwrap_throw();
            Html::VRef(svg.into())
        };

        let has_event = self.state.event.is_some();
        let is_newevent_page = ctx
            .link()
            .route::<Route>()
            .as_ref()
            .map(|route| route == &Route::NewEvent)
            .unwrap_or_default();

        html! {
            <div class={classes!(vec!["topbar", "shrink"],not(self.connected).then_some("offline"))}>
                {
                    self.view_offline_bar(ctx)
                }

                <div class="innerbox">
                    <div class="logo">
                        <a class="clickable-logo" onclick={ctx.link().callback(|_| Msg::Home)}>
                            {logo_svg}
                        </a>
                        {logo_text_svg}
                    </div>

                    {
                        if self.state.event.is_some() {
                            html! {
                                <a class="share"
                                    onclick={ctx.link().callback(|_| Msg::Share)}>
                                    {"Share"}
                                </a>
                            }
                        }else{html! {}}
                    }

                    <div class="iconbar">
                        {
                            if has_event {
                                self.view_ask_question(ctx)
                            }
                            else if !is_newevent_page {html!{
                            <Link<Route> to={Route::NewEvent}>
                                <div class="createevent">
                                    {"Create Event"}
                                </div>
                            </Link<Route>>
                            }}
                            else{html!()}
                        }
                    </div>
                </div>
            </div>
        }
    }
}

impl IconBar {
    fn view_ask_question(&self, ctx: &Context<Self>) -> Html {
        let is_open = self
            .state
            .event
            .as_ref()
            .map(|e| e.state.is_open())
            .unwrap_or_default();

        if is_open {
            return html! {
                <a>
                    <div class="createevent" onclick={ctx.link().callback(|_| Msg::Ask)}>
                        {"Ask a question"}
                    </div>
                </a>
            };
        }
        html! {}
    }

    fn view_offline_bar(&self, ctx: &Context<Self>) -> Html {
        let is_online = self.connected;

        let seconds_till_reconnect = self
            .reconnect_timeout
            .map(|timeout| (timeout - Utc::now()).num_seconds())
            .unwrap_or_default()
            .max(0);

        html! {
            <div id="ico-offline"
                class={classes!(is_online.then_some("hidden"))}
                onclick={ctx.link().callback(|_| Msg::Reconnect)}
                >
                <img hidden={is_online} src="/assets/offline.svg" />
                <div hidden={is_online} class="timeout">{format!("{}s",seconds_till_reconnect)}</div>
            </div>
        }
    }
}
