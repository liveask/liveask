use shared::ContextItem;
use yew::prelude::*;

use crate::components::ContextPopup;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct EventContextProps {
    pub context: Vec<ContextItem>,
    pub tokens: shared::EventTokens,
    pub is_premium: bool,
}

pub enum Msg {
    EditClick,
    ClosePopup,
}

pub struct EventContext {
    show_popup: bool,
}

impl Component for EventContext {
    type Message = Msg;
    type Properties = EventContextProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self { show_popup: false }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::EditClick => {
                self.show_popup = true;
                true
            }
            Msg::ClosePopup => {
                self.show_popup = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let items = ctx.props().context.as_slice();

        html! {
            <div class="context">
            {
                for items.iter().map(Self::view_item)
            }
            {
                self.view_mod(ctx)
            }
            </div>
        }
    }
}

impl EventContext {
    fn view_item(item: &ContextItem) -> Html {
        html! {
            <a href={item.url.clone()} target="_blank">
                <img src="assets/context.svg" />
                <div class="label">{ item.label.clone() }</div>
            </a>
        }
    }

    fn view_mod(&self, ctx: &Context<Self>) -> Html {
        let is_mod_and_premium = ctx.props().tokens.is_mod() && ctx.props().is_premium;
        let tokens = ctx.props().tokens.clone();
        let context = ctx.props().context.clone();

        let on_click_edit = ctx.link().callback(|_| Msg::EditClick);
        let on_close_popup = ctx.link().callback(|()| Msg::ClosePopup);

        if is_mod_and_premium {
            html! {
                <>
                    <ContextPopup {tokens} on_close={on_close_popup} show={self.show_popup} {context} />
                    <button onclick={on_click_edit}>
                        {"edit"}
                    </button>
                </>
            }
        } else {
            html! {}
        }
    }
}
