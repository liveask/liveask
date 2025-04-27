use crate::{components::Popup, fetch, pages::BASE_API};
use shared::{
    ContextItem, ContextLabelError, ContextUrlError, ContextValidation, ModEvent, ValidationState,
};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Debug)]
pub enum Input {
    Label,
    Url,
}

pub enum Msg {
    ConfirmedDelete,
    ConfirmEdit,
    ServerResponed,
    Close,
    InputChange(Input, InputEvent),
}

enum State {
    Create,
    Edit,
}

pub struct ContextPopup {
    label: String,
    url: String,
    send_pending: bool,
    errors: ContextValidation,
    state: State,
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct ContextPopupProps {
    pub tokens: shared::EventTokens,
    #[prop_or_default]
    pub on_close: Callback<()>,
    pub show: bool,
    pub context: Vec<ContextItem>,
}

impl Component for ContextPopup {
    type Message = Msg;
    type Properties = ContextPopupProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            label: String::new(),
            url: String::new(),
            send_pending: false,
            errors: ContextValidation::default(),
            state: State::Create,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        if ctx.props().show {
            self.state = State::Create;
            if let Some(item) = ctx.props().context.first() {
                self.label.clone_from(&item.label);
                self.url.clone_from(&item.url);
                self.errors.check(&self.label, &self.url);
                self.state = State::Edit;
            }
        } else {
            self.label = String::new();
            self.url = String::new();
            self.state = State::Create;
        }

        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Close => {
                if self.send_pending {
                    false
                } else {
                    ctx.props().on_close.emit(());
                    true
                }
            }
            Msg::ConfirmedDelete => {
                self.send_pending = true;

                let tokens = ctx.props().tokens.clone();
                ctx.link().send_future(async move {
                    fetch::mod_edit_event(
                        BASE_API,
                        tokens.public_token.clone(),
                        tokens.moderator_token.unwrap_throw(),
                        ModEvent {
                            context: Some(shared::EditContextLink::Disabled),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_or_else(
                        |e| {
                            log::error!("mod_edit_event error: {e}");
                            Msg::ServerResponed
                        },
                        |_| Msg::ServerResponed,
                    )
                });
                true
            }
            Msg::ConfirmEdit => {
                self.send_pending = true;

                let tokens = ctx.props().tokens.clone();
                let item = ContextItem {
                    label: self.label.clone(),
                    url: self.url.clone(),
                };

                ctx.link().send_future(async move {
                    fetch::mod_edit_event(
                        BASE_API,
                        tokens.public_token.clone(),
                        tokens.moderator_token.unwrap_throw(),
                        ModEvent {
                            context: Some(shared::EditContextLink::Enabled(item)),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_or_else(
                        |e| {
                            log::error!("mod_edit_event error: {e}");
                            Msg::ServerResponed
                        },
                        |_| Msg::ServerResponed,
                    )
                });
                true
            }
            Msg::InputChange(input, c) => {
                match input {
                    Input::Label => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.label = target.value();
                        self.errors.check(&self.label, &self.url);
                    }
                    Input::Url => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.url = target.value();
                        self.errors.check(&self.label, &self.url);
                    }
                }
                true
            }
            Msg::ServerResponed => {
                self.send_pending = false;
                ctx.props().on_close.emit(());
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let show = ctx.props().show;
        let is_create = matches!(self.state, State::Create);

        if show {
            let on_close = ctx.link().callback(|()| Msg::Close);
            let on_click_delete = ctx.link().callback(|_| Msg::ConfirmedDelete);
            let on_click_ok = ctx.link().callback(|_| Msg::ConfirmEdit);

            let has_errors = self.errors.has_any();

            html! {
                <Popup class="context-popup" {on_close}>
                    <div class="title">{ if is_create { "Add context link" } else { "Edit context link" } }</div>
                    <div class="input-box">
                        <input
                            type="text"
                            name="label"
                            placeholder="Link Title"
                            value={self.label.clone()}
                            maxlength="20"
                            required=true
                            oninput={ctx.link().callback(|input| Msg::InputChange(Input::Label,input))}
                        />
                    </div>
                    <div hidden={self.errors.label.is_valid()} class="invalid">
                        { self.label_err().unwrap_or_default() }
                    </div>
                    <div class="input-box">
                        <input
                            type="text"
                            name="url"
                            placeholder="URL - https://"
                            value={self.url.clone()}
                            maxlength="100"
                            required=true
                            oninput={ctx.link().callback(|input| Msg::InputChange(Input::Url,input))}
                        />
                    </div>
                    <div hidden={self.errors.url.is_valid()} class="invalid">
                        { self.url_error().unwrap_or_default() }
                    </div>
                    <div class="buttons">
                        <button class="button-white"
                            disabled={self.send_pending}
                            onclick={on_click_delete}>
                            { if is_create { "cancel" } else { "remove" } }
                        </button>
                        <button class="button-red"
                            disabled={self.send_pending || has_errors}
                            onclick={on_click_ok}>
                            { if is_create { "create" } else { "change" } }
                        </button>
                    </div>
                </Popup>
            }
        } else {
            html! {}
        }
    }
}

impl ContextPopup {
    fn label_err(&self) -> Option<String> {
        match self.errors.label {
            ValidationState::Invalid(ContextLabelError::MinLength(len, max)) => Some(format!(
                "Title must be at least {max} characters long. ({len}/{max})",
            )),
            ValidationState::Invalid(ContextLabelError::MaxLength(_, max)) => {
                Some(format!("Title cannot be longer than {max} characters.",))
            }
            _ => None,
        }
    }

    fn url_error(&self) -> Option<String> {
        match self.errors.url {
            ValidationState::Invalid(ContextUrlError::Invalid(
                url::ParseError::RelativeUrlWithoutBase,
            )) => Some("Base missing (like http://)".to_string()),
            ValidationState::Invalid(ContextUrlError::Invalid(_)) => {
                Some("Invalid URL".to_string())
            }
            _ => None,
        }
    }
}
