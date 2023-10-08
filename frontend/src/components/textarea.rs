#![allow(unused_imports)]

use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, Properties)]
pub struct TextAreaProps {
    pub id: Option<String>,
    pub name: String,
    pub placeholder: String,
    pub value: String,
    pub maxlength: Option<String>,
    pub required: Option<bool>,
    pub autosize: Option<bool>,
    pub oninput: Callback<InputEvent>,
}

pub enum Msg {
    Input(InputEvent),
}

pub struct TextArea {}
impl Component for TextArea {
    type Message = Msg;
    type Properties = TextAreaProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Input(e) => {
                let target: HtmlTextAreaElement = e.target_dyn_into().unwrap_throw();
                ctx.props().oninput.emit(e);

                let style = target.style();
                style.set_property("height", "auto").unwrap_throw();

                if ctx.props().autosize == Some(true) {
                    let height = target.scroll_height();

                    style
                        .set_property("height", &format!("{height}px"))
                        .unwrap_throw();
                }

                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props().clone();
        html! {
            <textarea
                id={props.id}
                name={props.name}
                placeholder={props.placeholder}
                value={props.value}
                maxlength={props.maxlength}
                required={props.required.unwrap_or_default()}
                oninput={ctx.link().callback(Msg::Input)}>
            </textarea>
        }
    }
}
