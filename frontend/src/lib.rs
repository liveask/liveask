#![deny(
    warnings,
    unused_imports,
    unused_must_use,
    unused_variables,
    unused_mut,
    dead_code,
    clippy::expect_used
)]
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::dbg_macro,
    clippy::unwrap_used,
    clippy::panic,
    clippy::needless_update,
    clippy::match_like_matches_macro,
    clippy::from_over_into,
    clippy::useless_conversion,
    clippy::float_cmp_const,
    clippy::lossy_float_literal,
    clippy::string_to_string,
    clippy::unneeded_field_pattern,
    clippy::verbose_file_reads
)]
#![allow(
    clippy::use_self,
    clippy::module_name_repetitions,
    clippy::let_unit_value
)]
mod agents;
mod components;
mod environment;
mod fetch;
mod local_cache;
mod pages;
mod routes;
mod tracking;

use agents::{EventAgent, GlobalEvent};
use routes::Route;
use shared::GetEventResponse;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yew_router::prelude::*;
use yewdux::{prelude::Dispatch, store::Store};

use crate::{
    components::IconBar,
    pages::{Event, Home, NewEvent, Print, Privacy},
};

pub const VERSION_STR: &str = "2.1.2";

#[derive(Default, Clone, Eq, PartialEq, Store)]
pub struct State {
    pub event: Option<GetEventResponse>,
    pub new_question: Option<i64>,
}

pub enum Msg {
    State(Rc<State>),
    Event(GlobalEvent),
}

struct AppRoot {
    connected: bool,
    state: Rc<State>,
    _dispatch: Dispatch<State>,
    _events: Box<dyn Bridge<EventAgent>>,
}
impl Component for AppRoot {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let events = EventAgent::bridge(ctx.link().callback(Msg::Event));

        Self {
            _dispatch: Dispatch::<State>::subscribe(ctx.link().callback(Msg::State)),
            state: Rc::default(),
            _events: events,
            connected: true,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::State(state) => {
                self.state = state;
                false
            }
            Msg::Event(e) => match e {
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
                    <div class={classes!("main",not(self.connected).then_some("offline"))}>
                        <IconBar/>

                        <div class="router">
                            <Switch<Route> render={Switch::render(switch)} />
                        </div>
                    </div>
                </div>
            </BrowserRouter>
        }
    }
}

#[must_use]
pub const fn not(b: bool) -> bool {
    !b
}

fn switch(switch: &Route) -> Html {
    match switch {
        Route::Event { id } => {
            html! { <Event id={id.clone()} /> }
        }
        Route::Print { id } => {
            html! { <Print id={id.clone()} /> }
        }
        Route::EventMod { id, secret } => {
            html! { <Event id={id.clone()} secret={secret.clone()} /> }
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
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    console_error_panic_hook::set_once();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    yew::start_app::<AppRoot>();
}
