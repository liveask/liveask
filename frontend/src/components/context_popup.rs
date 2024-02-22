use crate::{components::Popup, fetch, pages::BASE_API};
use shared::{ContextItem, ModEvent};
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
    ServerResponed(bool),
    Close,
    InputChange(Input, InputEvent),
}

pub struct ContextPopup {
    label: String,
    url: String,
    send_pending: bool,
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
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        if ctx.props().show {
            if let Some(item) = ctx.props().context.first() {
                self.label = item.label.clone();
                self.url = item.url.clone();
            }
        } else {
            self.label = String::new();
            self.url = String::new();
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
                        |_| Msg::ServerResponed(false),
                        |_| Msg::ServerResponed(true),
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
                        |_| Msg::ServerResponed(false),
                        |_| Msg::ServerResponed(true),
                    )
                });
                true
            }
            Msg::InputChange(input, c) => {
                match input {
                    Input::Label => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.label = target.value();
                    }
                    Input::Url => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.url = target.value();
                    }
                };
                true
            }
            Msg::ServerResponed(_) => {
                self.send_pending = false;
                ctx.props().on_close.emit(());
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let show = ctx.props().show;

        if show {
            let on_close = ctx.link().callback(|()| Msg::Close);
            let on_click_delete = ctx.link().callback(|_| Msg::ConfirmedDelete);
            let on_click_ok = ctx.link().callback(|_| Msg::ConfirmEdit);

            html! {
                <Popup class="context-popup" {on_close}>
                    <div class="title">{ "Add or Edit context link" }</div>
                    <input
                        type="text"
                        name="label"
                        placeholder="label"
                        value={self.label.clone()}
                        maxlength="20"
                        required=true
                        oninput={ctx.link().callback(|input| Msg::InputChange(Input::Label,input))}
                    />
                    <input
                        type="text"
                        name="url"
                        placeholder="url"
                        value={self.url.clone()}
                        maxlength="100"
                        required=true
                        oninput={ctx.link().callback(|input| Msg::InputChange(Input::Url,input))}
                    />
                    <div class="buttons">
                        <button disabled={self.send_pending} class="btn-yes" onclick={on_click_ok}>{ "confirm" }</button>
                        <button disabled={self.send_pending} class="btn-yes" onclick={on_click_delete}>{ "delete" }</button>
                    </div>
                </Popup>
            }
        } else {
            html! {}
        }
    }
}
