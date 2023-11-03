use crate::{
    components::{Popup, TextArea},
    fetch,
    pages::BASE_API,
};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlTextAreaElement;
use yew::{prelude::*, virtual_dom::AttrValue};

pub enum Msg {
    Send,
    PasswordSetResponse(bool),
    InputChanged(InputEvent),
}

pub struct PasswordPopup {
    show: bool,
    text: String,
    try_again: bool,
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct PasswordPopupProps {
    pub event: AttrValue,
    pub show: bool,
    #[prop_or_default]
    pub onconfirmed: Callback<()>,
}

impl Component for PasswordPopup {
    type Message = Msg;
    type Properties = PasswordPopupProps;

    fn create(ctx: &Context<Self>) -> Self {
        let show = ctx.props().show;

        Self {
            show,
            try_again: false,
            text: String::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Send => {
                let event_id: String = ctx.props().event.to_string();
                let text = self.text.clone();

                ctx.link().send_future(async move {
                    fetch::event_set_password(BASE_API, event_id.clone(), text)
                        .await
                        .map_or_else(
                            |_| Msg::PasswordSetResponse(false),
                            Msg::PasswordSetResponse,
                        )
                });

                self.text.clear();

                false
            }
            Msg::PasswordSetResponse(ok) => {
                if ok {
                    ctx.props().onconfirmed.emit(());
                    self.show = false;
                } else {
                    self.try_again = true;
                }
                true
            }
            Msg::InputChanged(ev) => {
                let target: HtmlTextAreaElement = ev.target_dyn_into().unwrap_throw();
                self.text = target.value();
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        let changed = self.show != ctx.props().show;
        self.show = ctx.props().show;
        if changed && self.show {
            self.try_again = false;
        }
        changed
    }

    #[allow(clippy::if_not_else)]
    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.show {
            let on_click_ask = ctx.link().callback(|_| Msg::Send);

            html! {
            <Popup class="share-popup">
                <div class="newquestion">
                    <div class="add-question">
                        <TextArea
                            id="questiontext"
                            name="questiontext"
                            maxlength="100"
                            value={self.text.clone()}
                            placeholder="password"
                            required=true
                            oninput={ctx.link().callback(Msg::InputChanged)}
                            >
                        </TextArea>

                        <div class="more-info">
                            {
                                html!{
                                    <div hidden={!self.try_again} class="invalid">
                                        <div>
                                        {"try again"}
                                        </div>
                                    </div>
                                }
                            }
                        </div>
                    </div>
                    <button class="dlg-button" onclick={on_click_ask}>
                        {"Ok"}
                    </button>
                </div>
            </Popup>
            }
        } else {
            html! {}
        }
    }
}
