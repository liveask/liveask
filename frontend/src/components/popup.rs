use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PopupProps {
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub on_close: Callback<()>,
}

pub enum Msg {
    ClickOutside,
    ClickInside,
}

pub struct Popup {
    body: HtmlElement,
}

impl Component for Popup {
    type Message = Msg;
    type Properties = PopupProps;

    fn create(_ctx: &Context<Self>) -> Self {
        let body: HtmlElement = gloo_utils::document()
            .body()
            .expect_throw("no body node found");

        let result = Self { body };
        result.toggle_modal(true);
        result
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ClickOutside => {
                ctx.props().on_close.emit(());
                true
            }
            Msg::ClickInside => {
                //do nothing
                false
            }
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.toggle_modal(false);
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let click_outside = ctx.link().callback(|_| Msg::ClickOutside);
        let click_inside = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            e.stop_immediate_propagation();
            Msg::ClickInside
        });

        let PopupProps {
            class, children, ..
        } = &ctx.props();

        let mut c = class.clone();
        c.push(classes!("popup"));

        html! {
            <div class="popup-bg" onclick={click_outside}>
                <div class={c} onclick={click_inside}>
                    { for children.iter() }
                </div>
            </div>
        }
    }
}

impl Popup {
    fn toggle_modal(&self, enable: bool) {
        if enable {
            self.body
                .class_list()
                .add_1("modal-open")
                .expect_throw("toggle_modal error");
        } else {
            self.body
                .class_list()
                .remove_1("modal-open")
                .expect_throw("toggle_modal error 2");
        }
    }
}
