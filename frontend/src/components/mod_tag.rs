use std::{collections::HashMap, rc::Rc};

use shared::{CurrentTag, EventTokens, ModEvent, TagId};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{fetch, pages::BASE_API};

pub type SharableTags = Rc<HashMap<TagId, String>>;

#[derive(Eq, PartialEq, Properties)]
pub struct TagProps {
    pub tokens: EventTokens,
    pub tags: SharableTags,
    pub tag: Option<String>,
}

pub enum Msg {
    EnableInput,
    Edit,
    Disable,
    InputChange(InputEvent),
    KeyDown(KeyboardEvent),
    InputExit,
    Edited(bool),
}

enum State {
    Disabled,
    Editing(String),
    Confirmed(String),
}

pub struct ModTag {
    state: State,
    input: NodeRef,
}
impl Component for ModTag {
    type Message = Msg;
    type Properties = TagProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            state: Self::derive_state(&ctx.props().tag),
            input: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::EnableInput => {
                self.state = State::Editing(String::new());
                true
            }
            Msg::Edit => {
                self.state = State::Editing(self.current_value().to_string());
                true
            }
            Msg::Disable => {
                self.disable(ctx);
                true
            }
            Msg::InputChange(e) => {
                let target: HtmlInputElement = e.target_dyn_into().unwrap_throw();

                self.state = State::Editing(target.value());
                //TODO:
                // self.errors.check(&target.value());
                true
            }
            Msg::KeyDown(e) => {
                if e.key() == "Enter" {
                    self.set_pwd(ctx);
                }
                true
            }
            Msg::InputExit => {
                self.set_pwd(ctx);
                true
            }
            Msg::Edited(_) => true,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        self.state = Self::derive_state(&ctx.props().tag);
        true
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if let State::Editing(_) = self.state {
            let _ = self
                .input
                .cast::<HtmlInputElement>()
                .map(|input| input.focus())
                .unwrap_throw();
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let content = match &self.state {
            State::Disabled => Self::view_disabled(ctx),
            State::Editing(value) => self.view_input_unconfirmed(value, ctx),
            State::Confirmed(value) => Self::view_confirmed(value, ctx),
        };

        html! {
            <div class="password">
                {content}
            </div>
        }
    }
}

impl ModTag {
    fn view_disabled(ctx: &Context<Self>) -> Html {
        html! {
            <button class="button-white" onclick={ctx.link().callback(|_|Msg::EnableInput)} >
                {"Tag Questions"}
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
                    placeholder="tag"
                    maxlength="30"
                    {value}
                    oninput={ctx.link().callback(Msg::InputChange)}
                    onkeydown={ctx.link().callback(Msg::KeyDown)}
                    onblur={ctx.link().callback(|_|Msg::InputExit)} />
                <img id="edit" src="/assets/pwd/pwd-edit.svg"/>
            </>
        }
    }

    fn view_confirmed(value: &str, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="confirmed" onclick={ctx.link().callback(|_|Msg::Edit)}>
                    {value.to_string()}
                </div>
                <img id="delete" src="/assets/pwd/pwd-remove.svg" onmousedown={ctx.link().callback(|_|Msg::Disable)} />
            </>
        }
    }

    fn current_value(&self) -> &str {
        match &self.state {
            State::Editing(value) | State::Confirmed(value) => value,
            State::Disabled => "",
        }
    }

    fn request_edit(id: String, secret: String, link: &html::Scope<Self>, tag: CurrentTag) {
        link.send_future(async move {
            match fetch::mod_edit_event(
                BASE_API,
                id,
                secret,
                ModEvent {
                    current_tag: Some(tag),
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

    fn disable(&mut self, ctx: &Context<Self>) {
        let props = ctx.props();
        Self::request_edit(
            props.tokens.public_token.clone(),
            props.tokens.moderator_token.clone().unwrap_throw(),
            ctx.link(),
            CurrentTag::Disabled,
        );
        self.state = State::Disabled;
    }

    fn derive_state(tag: &Option<String>) -> State {
        tag.as_ref()
            .map_or(State::Disabled, |tag| State::Confirmed(tag.clone()))
    }

    fn set_pwd(&mut self, ctx: &Context<Self>) {
        let current = self.current_value().to_string();

        //TODO:
        // if self.errors.has_any() {
        //     self.disable_password(ctx);
        // } else {
        let props = ctx.props();
        Self::request_edit(
            props.tokens.public_token.clone(),
            props.tokens.moderator_token.clone().unwrap_or_default(),
            ctx.link(),
            CurrentTag::Enabled(current.clone()),
        );
        self.state = State::Confirmed(current);
        // }
    }
}
