use shared::EventInfo;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{fetch, routes::Route};

#[derive(Default)]
pub struct Errors {
    pub name: Option<String>,
    pub desc: Option<String>,
}

impl Errors {
    const fn has_errors(&self) -> bool {
        self.name.is_some() || self.desc.is_some()
    }
}

pub struct NewEvent {
    name: String,
    desc: String,
    email: String,
    name_ref: NodeRef,
    errors: Errors,
}

#[derive(Debug)]
pub enum Input {
    Name,
    Email,
    Desc,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct NewEventProps;

pub enum Msg {
    Create,
    CreatedResult(Option<EventInfo>),
    InputChange(Input, InputEvent),
}
impl Component for NewEvent {
    type Message = Msg;
    type Properties = NewEventProps;

    fn create(_: &Context<Self>) -> Self {
        Self {
            name: String::new(),
            desc: String::new(),
            email: String::new(),
            name_ref: NodeRef::default(),
            errors: Errors::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Create => {
                let name = self.name.clone();
                let desc = self.desc.clone();
                let email = self.email.clone();
                ctx.link().send_future(async move {
                    let res = fetch::create_event(name, desc, email).await;

                    match res {
                        Ok(e) => Msg::CreatedResult(Some(e)),
                        Err(e) => {
                            log::error!("create error: {}", e);
                            Msg::CreatedResult(None)
                        }
                    }
                });
                false
            }

            Msg::CreatedResult(event) => match event {
                Some(event) => {
                    ctx.link().history().unwrap().push(Route::EventMod {
                        id: event.tokens.public_token,
                        secret: event.tokens.moderator_token.unwrap(),
                    });
                    false
                }
                None => {
                    log::error!("no event created");
                    true
                }
            },

            Msg::InputChange(input, c) => {
                match input {
                    Input::Name => {
                        let e = self.name_ref.cast::<Element>().unwrap();
                        let e: HtmlInputElement = e.dyn_into().unwrap();

                        let valid = e.check_validity();

                        self.errors.name = None;

                        if !valid {
                            self.errors.name = e.validation_message().ok();
                        }

                        self.name = e.value();
                    }
                    Input::Email => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap();
                        self.email = target.value()
                    }
                    Input::Desc => {
                        let target: HtmlTextAreaElement = c.target_dyn_into().unwrap();
                        self.desc = target.value()
                    }
                }

                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="newevent-bg">
                <div class="title">
                    {"Create Event"}
                </div>
                <div class="form">
                    <div class="newevent">
                        <div class="input-box">
                            <input
                                ref={self.name_ref.clone()}
                                type="text"
                                name="eventname"
                                value={self.name.clone()} placeholder="event name"
                                minlength="8" maxlength="30" maxwordlength="13"
                                autocomplete="off" required=true
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Name,input))}/>
                        </div>
                        <div hidden={self.errors.name.is_none()} class="invalid">
                            {self.errors.name.clone().unwrap_or_default()}
                        </div>
                        <div class="input-box">
                            <input
                                type="email"
                                name="mail"
                                value={self.email.clone()} placeholder="email (optional)"
                                maxlength="100"
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Email,input))}/>
                        </div>
                        <div class="input-box">
                            <textarea
                                id="input-desc"
                                name="desc"
                                value={self.desc.clone()} placeholder="event description"
                                mintrimlength="10" maxlength="1000"
                                required=true
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Desc,input))}>
                            </textarea>
                        </div>
                        <div hidden={self.errors.desc.is_none()} class="invalid">
                            {self.errors.desc.clone().unwrap_or_default()}
                        </div>
                    </div>
                    <button
                        class="button-finish"
                        disabled={!self.can_create()}
                        onclick={ctx.link().callback(|_| Msg::Create)}>
                        {"finish"}
                    </button>
                </div>
            </div>
        }
    }
}

impl NewEvent {
    fn can_create(&self) -> bool {
        !self.errors.has_errors() && !self.name.is_empty() && !self.desc.is_empty()
    }
}
