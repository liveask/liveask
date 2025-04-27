use crate::{
    components::{Popup, TextArea},
    fetch,
    pages::{BASE_API, NewEvent},
};
use shared::{CreateEventValidation, EditMetaData, ModEvent};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

#[derive(Debug)]
pub enum Input {
    Title,
    Desc,
}

pub enum Msg {
    ConfirmEdit,
    ServerResponed,
    Close,
    InputChange(Input, InputEvent),
}

pub struct MetaPopup {
    meta: EditMetaData,
    send_pending: bool,
    errors: CreateEventValidation,
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct MetaPopupProps {
    pub tokens: shared::EventTokens,
    #[prop_or_default]
    pub on_close: Callback<()>,
    pub show: bool,
    pub meta: EditMetaData,
}

impl Component for MetaPopup {
    type Message = Msg;
    type Properties = MetaPopupProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            meta: EditMetaData {
                title: String::new(),
                description: String::new(),
            },
            send_pending: false,
            errors: CreateEventValidation::default(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        if ctx.props().show {
            self.meta = ctx.props().meta.clone();
            self.errors = self
                .errors
                .check(&self.meta.title, &self.meta.description, "");
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
            Msg::ConfirmEdit => {
                self.send_pending = true;

                let tokens = ctx.props().tokens.clone();
                let meta: EditMetaData = self.meta.clone();

                ctx.link().send_future(async move {
                    fetch::mod_edit_event(
                        BASE_API,
                        tokens.public_token.clone(),
                        tokens.moderator_token.unwrap_throw(),
                        ModEvent {
                            meta: Some(meta),
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
                    Input::Title => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.meta.title = target.value();
                        self.errors =
                            self.errors
                                .check(&self.meta.title, &self.meta.description, "");
                    }
                    Input::Desc => {
                        let target: HtmlTextAreaElement = c.target_dyn_into().unwrap_throw();
                        self.meta.description = target.value();
                        self.errors =
                            self.errors
                                .check(&self.meta.title, &self.meta.description, "");
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

        if show {
            let on_close = ctx.link().callback(|()| Msg::Close);
            let on_click_ok = ctx.link().callback(|_| Msg::ConfirmEdit);
            let on_click_close = ctx.link().callback(|_| Msg::Close);

            let has_errors = self.errors.has_any();

            html! {
                <Popup class="meta-popup" {on_close}>
                    <div class="title">{ "Edit Event" }</div>
                    <div class="input-box">
                        <input
                            type="text"
                            name="title"
                            placeholder="Title"
                            value={self.meta.title.clone()}
                            maxlength="20"
                            required=true
                            oninput={ctx.link().callback(|input| Msg::InputChange(Input::Title,input))}
                        />
                    </div>
                    <div hidden={self.errors.name.is_none()} class="invalid">
                        { NewEvent::name_error(self.errors.name.as_ref()).unwrap_or_default() }
                    </div>
                    <div class="input-box">
                        <TextArea
                            id="input-desc"
                            name="desc"
                            placeholder="event description"
                            value={self.meta.description.clone()}
                            maxlength="1000"
                            required=true
                            autosize=true
                            oninput={ctx.link().callback(|input| Msg::InputChange(Input::Desc,input))}
                            />
                    </div>
                    <div hidden={self.errors.desc.is_none()} class="invalid">
                        { NewEvent::desc_error(self.errors.desc.as_ref()).unwrap_or_default() }
                    </div>
                    <div class="buttons">
                        <button class="button-white"
                            disabled={self.send_pending}
                            onclick={on_click_close}>
                            { "cancel" }
                        </button>
                        <button class="button-red"
                            disabled={self.send_pending || has_errors}
                            onclick={on_click_ok}>
                            { "change" }
                        </button>
                    </div>
                </Popup>
            }
        } else {
            html! {}
        }
    }
}
