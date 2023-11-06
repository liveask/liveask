use shared::{EventPassword, EventTokens, ModEvent, PasswordValidation};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{fetch, pages::BASE_API};

#[derive(Eq, PartialEq, Properties)]
pub struct PasswordProps {
    pub tokens: EventTokens,
    pub pwd: EventPassword,
}

pub enum Msg {
    EnablePasswordInput,
    EditPassword,
    DisablePassword,
    InputChange(InputEvent),
    InputExit,
    Edited(bool),
}

enum State {
    Disabled,
    PasswordEditing(String),
    Confirmed(String),
}

pub struct ModPassword {
    state: State,
    input: NodeRef,
    errors: PasswordValidation,
}
impl Component for ModPassword {
    type Message = Msg;
    type Properties = PasswordProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            state: Self::derive_state(&ctx.props().pwd),
            input: NodeRef::default(),
            errors: PasswordValidation::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::EnablePasswordInput => {
                self.state = State::PasswordEditing(String::new());
                true
            }
            Msg::EditPassword => {
                self.state = State::PasswordEditing(self.current_value().to_string());
                true
            }
            Msg::DisablePassword => {
                self.disable_password(ctx);
                true
            }
            Msg::InputChange(e) => {
                let target: HtmlInputElement = e.target_dyn_into().unwrap_throw();
                self.state = State::PasswordEditing(target.value());
                self.errors.check(&target.value());
                true
            }
            Msg::InputExit => {
                let current = self.current_value().to_string();

                if self.errors.has_any() {
                    self.disable_password(ctx);
                } else {
                    let props = ctx.props();
                    Self::request_edit(
                        props.tokens.public_token.clone(),
                        props.tokens.moderator_token.clone().unwrap_or_default(),
                        ctx.link(),
                        EventPassword::Enabled(current.clone()),
                    );
                    self.state = State::Confirmed(current);
                }

                true
            }
            Msg::Edited(_) => true,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        self.state = Self::derive_state(&ctx.props().pwd);
        true
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if let State::PasswordEditing(_) = self.state {
            self.input
                .cast::<HtmlInputElement>()
                .unwrap_throw()
                .focus()
                .unwrap_throw();
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let content = match &self.state {
            State::Disabled => Self::view_disabled(ctx),
            State::PasswordEditing(value) => self.view_input_unconfirmed(value, ctx),
            State::Confirmed(_) => Self::view_confirmed(ctx),
        };

        html! {
            <div class="password">
                {content}
            </div>
        }
    }
}

impl ModPassword {
    fn view_disabled(ctx: &Context<Self>) -> Html {
        html! {
            <button class="button-white" onclick={ctx.link().callback(|_|Msg::EnablePasswordInput)} >
                {"Password"}
            </button>
        }
    }

    fn view_input_unconfirmed(&self, current: &str, ctx: &Context<Self>) -> Html {
        let value = current.to_string();
        html! {
            <>
                <input
                    ref={self.input.clone()}
                    type="text"
                    placeholder="password"
                    maxlength="30"
                    {value}
                    oninput={ctx.link().callback(Msg::InputChange)}
                    onblur={ctx.link().callback(|_|Msg::InputExit)} />
                <img id="edit" src="/assets/pwd/pwd-edit.svg"/>
            </>
        }
    }

    fn view_confirmed(ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="confirmed" onclick={ctx.link().callback(|_|Msg::EditPassword)}>
                    {"*****"}
                </div>
                <img id="delete" src="/assets/pwd/pwd-remove.svg" onmousedown={ctx.link().callback(|_|Msg::DisablePassword)} />
            </>
        }
    }

    fn current_value(&self) -> &str {
        match &self.state {
            State::PasswordEditing(value) | State::Confirmed(value) => value,
            State::Disabled => "",
        }
    }

    fn request_edit(id: String, secret: String, link: &html::Scope<Self>, pwd: EventPassword) {
        link.send_future(async move {
            match fetch::mod_edit_event(
                BASE_API,
                id,
                secret,
                ModEvent {
                    password: Some(pwd),
                    ..Default::default()
                },
            )
            .await
            {
                Err(e) => {
                    log::error!("mod_edit_event error: {e}");
                    Msg::Edited(false)
                }
                Ok(_) => Msg::Edited(true),
            }
        });
    }

    fn disable_password(&mut self, ctx: &Context<Self>) {
        let props = ctx.props();
        Self::request_edit(
            props.tokens.public_token.clone(),
            props.tokens.moderator_token.clone().unwrap_throw(),
            ctx.link(),
            EventPassword::Disabled,
        );
        self.state = State::Disabled;
    }

    fn derive_state(pwd: &EventPassword) -> State {
        match pwd {
            EventPassword::Disabled => State::Disabled,
            EventPassword::Enabled(pwd) => State::Confirmed(pwd.clone()),
        }
    }
}
