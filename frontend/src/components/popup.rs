use web_sys::HtmlElement;
use yew::prelude::*;
use yewdux::prelude::Dispatch;

use crate::State;

#[derive(Properties, PartialEq)]
pub struct PopupProps {
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub children: Children,
    pub on_close: Callback<()>,
}

pub enum Msg {
    ClickOutside,
    ClickInside,
}

pub struct Popup {
    dispatch: Dispatch<State>,
}

impl Component for Popup {
    type Message = Msg;
    type Properties = PopupProps;

    fn create(_ctx: &Context<Self>) -> Self {
        let dispatch = Dispatch::<State>::subscribe(Callback::noop());

        toggle_modal(true);

        dispatch.reduce(|state| State {
            event: state.event.clone(),
        });

        Self { dispatch }
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
        toggle_modal(false);

        self.dispatch.reduce(|state| State {
            event: state.event.clone(),
        });
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let click_outside = ctx
            .link()
            .callback_with_passive(Some(true), |_| Msg::ClickOutside);
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
                <div class={classes!(c)} onclick={click_inside}>
                    { for children.iter() }
                </div>
            </div>
        }
    }
}

fn toggle_modal(enable: bool) {
    let body: HtmlElement = gloo_utils::document()
        .body()
        .expect("no body node found")
        .into();

    if enable {
        body.class_list().add_1("modal-open").unwrap();
    } else {
        body.class_list().remove_1("modal-open").unwrap();
    }
}
