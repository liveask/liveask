#![allow(clippy::non_ascii_literal)]

mod components;
mod environment;
mod fetch;
mod global_events;
mod local_cache;
mod pages;
mod routes;
mod tracking;

use events::{EventBridge, Events};
use global_events::GlobalEvent;
use pages::AdminLogin;
use routes::Route;
use shared::GetEventResponse;
use std::rc::Rc;
use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::{prelude::Dispatch, store::Store};

use crate::{
    components::IconBar,
    pages::{Event, Home, NewEvent, Print, Privacy},
};

pub const VERSION_STR: &str = "2.4.2";
pub const GIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");

#[derive(Default, Clone, Eq, PartialEq, Store)]
pub struct State {
    pub event: Option<GetEventResponse>,
    pub new_question: Option<i64>,
    pub admin: bool,
}

impl State {
    #[must_use]
    pub const fn set_new_question(mut self, v: Option<i64>) -> Self {
        self.new_question = v;
        self
    }
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn set_event(mut self, v: Option<GetEventResponse>) -> Self {
        self.event = v;
        self
    }
    #[must_use]
    pub const fn set_admin(mut self, v: bool) -> Self {
        self.admin = v;
        self
    }
}

pub enum Msg {
    State(Rc<State>),
    GlobalEvent(GlobalEvent),
}

pub struct AppRoot {
    connected: bool,
    events: EventBridge<GlobalEvent>,
    state: Rc<State>,
    _dispatch: Dispatch<State>,
}
impl Component for AppRoot {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let mut context = Events::<GlobalEvent>::default();

        let events = context.subscribe(ctx.link().callback(Msg::GlobalEvent));

        Self {
            _dispatch: Dispatch::<State>::subscribe(ctx.link().callback(Msg::State)),
            state: Rc::default(),
            connected: true,
            events,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::State(state) => {
                self.state = state;
                false
            }
            Msg::GlobalEvent(e) => match e {
                GlobalEvent::SocketStatus { connected, .. } => {
                    self.connected = connected;
                    true
                }
                _ => false,
            },
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                <div class="app-host">
                    <ContextProvider<Events<GlobalEvent>> context={self.events.clone()}>
                    <div class={classes!("main",not(self.connected).then_some("offline"))}>
                        <IconBar/>

                        <div class="router">
                            <Switch<Route> render={switch} />
                        </div>
                    </div>
                    </ContextProvider<Events<GlobalEvent>>>
                </div>
            </BrowserRouter>
        }
    }
}

#[must_use]
pub const fn not(b: bool) -> bool {
    !b
}

fn switch(switch: Route) -> Html {
    match switch {
        Route::Event { id } => {
            html! { <Event {id} /> }
        }
        Route::Print { id } => {
            html! { <Print {id} /> }
        }
        Route::EventMod { id, secret } => {
            html! { <Event {id} {secret} /> }
        }
        Route::NewEvent => {
            html! { <NewEvent /> }
        }
        Route::Home => {
            html! { <Home /> }
        }
        Route::Privacy => {
            html! { <Privacy /> }
        }
        Route::Login => {
            html! { <AdminLogin /> }
        }
    }
}
