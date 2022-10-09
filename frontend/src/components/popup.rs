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

        dispatch.reduce(|state| State {
            modal_open: true,
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
        self.dispatch.reduce(|state| State {
            modal_open: false,
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
