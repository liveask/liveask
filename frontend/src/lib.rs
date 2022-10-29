#[deny(clippy::redundant_closure)]
#[deny(clippy::pedantic)]
mod agents;
mod components;
mod fetch;
mod local_cache;
mod pages;
mod routes;

use std::rc::Rc;

use routes::Route;
use shared::EventInfo;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::{prelude::Dispatch, store::Store};

use crate::{
    components::IconBar,
    pages::{Event, Home, NewEvent, Privacy},
};

pub const VERSION_STR: &str = "2.0.0";

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Default, Clone, Eq, PartialEq, Store)]
pub struct State {
    pub event: Option<EventInfo>,
}

pub enum Msg {
    State(Rc<State>),
}

struct AppRoot {
    state: Rc<State>,
    _dispatch: Dispatch<State>,
}
impl Component for AppRoot {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            _dispatch: Dispatch::<State>::subscribe(ctx.link().callback(Msg::State)),
            state: Default::default(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::State(state) => {
                self.state = state;
                false
            }
        }
    }

    fn view(&self, _: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                <div class="app-host">
                    <div class="main">
                        <IconBar />

                        <div class="router">
                            <Switch<Route> render={Switch::render(switch)} />
                        </div>
                    </div>
                </div>
            </BrowserRouter>
        }
    }
}

fn switch(switch: &Route) -> Html {
    match switch {
        Route::Event { id } => {
            html! { <Event id={id.clone()} /> }
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
        } // AppRoute::PageNotFound(Permissive(route)) => {
          //     html! { <PageNotFound route=route /> }
          // }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    yew::start_app::<AppRoot>();
}
