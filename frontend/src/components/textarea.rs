#![allow(unused_imports)]

use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlTextAreaElement;
use yew::{prelude::*, virtual_dom::AttrValue};

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct TextAreaProps {
    pub id: AttrValue,
    pub name: AttrValue,
    pub placeholder: AttrValue,
    pub value: AttrValue,
    pub maxlength: AttrValue,
    #[prop_or_default]
    pub required: bool,
    #[prop_or_default]
    pub autosize: bool,
    pub oninput: Callback<InputEvent>,
}

pub enum Msg {
    Input(InputEvent),
}

pub struct TextArea;
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

                if ctx.props().autosize {
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
                required={props.required}
                oninput={ctx.link().callback(Msg::Input)}>
            </textarea>
        }
    }
}
