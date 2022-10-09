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
    Event,
    Ask,
    Home,
}

#[derive(Properties, PartialEq, Eq)]
pub struct IconBarProps;

pub struct IconBar {
    state: Rc<State>,
    _dispatch: Dispatch<State>,
    #[allow(dead_code)]
    events: Box<dyn Bridge<EventAgent>>,
}
impl Component for IconBar {
    type Message = Msg;
    type Properties = IconBarProps;

    fn create(ctx: &Context<Self>) -> Self {
        let events = EventAgent::bridge(ctx.link().callback(|_msg| Msg::Event));

        Self {
            _dispatch: Dispatch::<State>::subscribe(ctx.link().callback(Msg::State)),
            events,
            state: Default::default(),
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
            Msg::Event => false,
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

        html! {
            <div class="topbar shrink" /*[class.offline]="isOffline()" [class.shrink]="isShrink()"*/>
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
        //TODO: check if event is closed
        html! {
            <a>
                <div class="createevent" onclick={ctx.link().callback(|_| Msg::Ask)}>
                    {"Ask a question"}
                </div>
            </a>
        }
    }
}
