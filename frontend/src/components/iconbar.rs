use std::rc::Rc;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::{
    agents::{EventAgent, GlobalEvent},
    routes::Route,
    State,
};

pub enum Msg {
    State(Rc<State>),
    Share,
    Event(GlobalEvent),
    Ask,
    Home,
}

#[derive(Properties, PartialEq, Eq)]
pub struct IconBarProps;

pub struct IconBar {
    connected: bool,
    state: Rc<State>,
    _dispatch: Dispatch<State>,
    #[allow(dead_code)]
    events: Box<dyn Bridge<EventAgent>>,
}
impl Component for IconBar {
    type Message = Msg;
    type Properties = IconBarProps;

    fn create(ctx: &Context<Self>) -> Self {
        let events = EventAgent::bridge(ctx.link().callback(|msg| Msg::Event(msg)));

        Self {
            _dispatch: Dispatch::<State>::subscribe(ctx.link().callback(Msg::State)),
            events,
            state: Default::default(),
            connected: true,
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
                ctx.link().history().unwrap().push(Route::Home);
                false
            }
            //ignore global events
            Msg::Event(msg) => match msg {
                GlobalEvent::SocketStatus(connected) => {
                    self.connected = connected;
                    true
                }
                _ => false,
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let doc = gloo_utils::document();

        let logo_svg = {
            let logo_svg = include_str!("../../inline-assets/logo.svg");
            let div = doc.create_element("div").unwrap();
            div.set_inner_html(logo_svg);
            Html::VRef(div.first_element_child().unwrap().into())
        };

        let logo_text_svg = {
            let logo_svg = include_str!("../../inline-assets/logo_text.svg");
            let div = doc.create_element("div").unwrap();
            div.set_inner_html(logo_svg);
            let svg = div.first_element_child().unwrap();
            //TODO: conditional
            svg.class_list().add_1("shrink").unwrap();
            Html::VRef(svg.into())
        };

        let has_event = self.state.event.is_some();

        let mut topbar_clasess = classes!(vec!["topbar", "shrink"]);
        if !self.connected {
            topbar_clasess.push(classes!("offline"));
        }

        html! {
            //TODO: shrink?
            <div class={topbar_clasess} /*[class.shrink]="isShrink()"*/>
                {
                    self.view_offline_bar()
                }

                <div class="innerbox">
                    <a
                        class="logo shrink"
                        onclick={ctx.link().callback(|_| Msg::Home)}
                        /*[class.shrink]="isShrink()"*/
                        >
                        {logo_svg}
                        {logo_text_svg}
                    </a>

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
                            else{html!{
                            <Link<Route> to={Route::NewEvent}>
                                <div class="createevent">
                                    {"Create Event"}
                                </div>
                            </Link<Route>>
                            }}
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

    fn view_offline_bar(&self) -> Html {
        let is_online = self.connected;

        let mut c = classes!();
        if is_online {
            c.push(classes!("hidden"));
        }

        html! {
            <div id="ico-offline" class={c}>
                <img hidden={is_online} src="/assets/offline.svg" />
                //TODO: reconnect timer
                <div hidden={is_online} class="timeout">{format!("{}",0)}{"s"}</div>
            </div>
        }
    }
}
