use crate::{
    components::{Spinner, TextArea},
    fetch,
    routes::Route,
    tracking,
};
use serde::Deserialize;
use shared::{CreateEventError, CreateEventValidation, EventInfo};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;
use yew_router::scope_ext::RouterScopeExt;

use super::event::BASE_API;

#[derive(Debug, Default, Deserialize)]
struct QueryParams {
    pub customer: Option<String>,
}

pub struct NewEvent {
    name: String,
    desc: String,
    email: String,
    name_ref: NodeRef,
    errors: CreateEventValidation,
    loading: bool,
}

#[derive(Debug)]
pub enum Input {
    Name,
    Email,
    Desc,
}

#[allow(clippy::empty_structs_with_brackets)]
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
            errors: CreateEventValidation::default(),
            loading: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Create => {
                let name = self.name.clone();
                let desc = self.desc.clone();
                let email: Option<String> = if self.email.trim().is_empty() {
                    None
                } else {
                    Some(self.email.trim().to_owned())
                };

                tracking::track_event(tracking::EVNT_NEWEVENT_FINISH);

                let query_params = ctx
                    .link()
                    .location()
                    .and_then(|loc| loc.query::<QueryParams>().ok())
                    .unwrap_or_default();

                let customer = query_params.customer.clone();

                self.loading = true;

                ctx.link().send_future(async move {
                    let res = fetch::create_event(BASE_API, name, desc, email, customer).await;

                    match res {
                        Ok(e) => Msg::CreatedResult(Some(e)),
                        Err(e) => {
                            log::error!("create error: {}", e);
                            Msg::CreatedResult(None)
                        }
                    }
                });
                true
            }

            Msg::CreatedResult(event) => {
                self.loading = false;
                if let Some(event) = event {
                    ctx.link()
                        .navigator()
                        .unwrap_throw()
                        .push(&Route::EventMod {
                            id: event.tokens.public_token,
                            secret: event.tokens.moderator_token.unwrap_throw(),
                        });
                    false
                } else {
                    log::error!("no event created");
                    true
                }
            }

            Msg::InputChange(input, c) => {
                match input {
                    Input::Name => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.name = target.value();

                        self.errors = self.errors.check(&self.name, &self.desc, &self.email);
                    }
                    Input::Email => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.email = target.value();

                        self.errors = self.errors.check(&self.name, &self.desc, &self.email);
                    }
                    Input::Desc => {
                        let target: HtmlTextAreaElement = c.target_dyn_into().unwrap_throw();
                        self.desc = target.value();

                        self.errors = self.errors.check(&self.name, &self.desc, &self.email);
                    }
                }

                true
            }
        }
    }

    #[allow(clippy::if_not_else)]
    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="newevent-bg">
                <div class="title">{ "Create Event" }</div>
                <div class="form">
                    <div class="newevent">
                        <div class="input-box">
                            <input
                                ref={self.name_ref.clone()}
                                type="text"
                                name="eventname"
                                placeholder="event name"
                                value={self.name.clone()}
                                maxlength="30"
                                required=true
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Name,input))}
                            />
                        </div>
                        <div hidden={self.errors.name.is_none()} class="invalid">
                            { Self::name_error(self.errors.name.as_ref()).unwrap_or_default() }
                        </div>
                        <div class="input-box">
                            <input
                                type="email"
                                name="mail"
                                placeholder="email (optional)"
                                value={self.email.clone()}
                                maxlength="100"
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Email,input))}
                            />
                        </div>
                        <div hidden={self.errors.email.is_none()} class="invalid">
                            { Self::email_error(self.errors.email.as_ref()).unwrap_or_default() }
                        </div>
                        <div class="input-box">
                            <TextArea
                                id="input-desc"
                                name="desc"
                                placeholder="event description"
                                value={self.desc.clone()}
                                maxlength="1000"
                                required=true
                                autosize=true
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Desc,input))}
                            />
                        </div>
                        <div hidden={self.errors.desc.is_none()} class="invalid">
                            { Self::desc_error(self.errors.desc.as_ref()).unwrap_or_default() }
                        </div>
                    </div>
                    if !self.loading {
                        <button
                            class="button-finish"
                            disabled={!self.can_create()}
                            onclick={ctx.link().callback(|_| Msg::Create)}
                        >
                            { "finish" }
                        </button>
                    } else {
                        <div id="spinner">
                            <Spinner />
                        </div>
                    }
                </div>

            </div>
        }
    }
}

impl NewEvent {
    const fn can_create(&self) -> bool {
        !self.errors.has_any() && !self.name.is_empty() && !self.desc.is_empty()
    }

    pub fn desc_error(state: Option<&CreateEventError>) -> Option<String> {
        match state {
            Some(CreateEventError::Empty) => Some("Description cannot be empty".to_string()),
            Some(CreateEventError::MinLength(len, max)) => Some(format!(
                "Description must be at least {max} characters long. ({len}/{max})",
            )),
            Some(CreateEventError::MaxLength(_, max)) => Some(format!(
                "Description cannot be longer than {max} characters.",
            )),
            Some(_) => Some("unknown error".to_string()),
            None => None,
        }
    }

    pub fn name_error(state: Option<&CreateEventError>) -> Option<String> {
        match state {
            Some(CreateEventError::Empty) => Some("Name is required.".to_string()),
            Some(CreateEventError::MinLength(len, max)) => Some(format!(
                "Name must be at least {max} characters long. ({len}/{max})"
            )),
            Some(CreateEventError::MaxLength(_, max)) => {
                Some(format!("Name cannot be longer than {max} characters."))
            }
            Some(CreateEventError::MaxWords(_, max)) => {
                Some(format!("Name must not contain more than {max} words."))
            }
            Some(CreateEventError::InvalidEmail) | None => None,
        }
    }

    pub fn email_error(state: Option<&CreateEventError>) -> Option<String> {
        match state {
            Some(CreateEventError::InvalidEmail) => Some("Invalid Email Provided".to_string()),
            _ => None,
        }
    }
}
