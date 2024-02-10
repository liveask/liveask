use shared::ContextItem;
use yew::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct EventContextProps {
    pub context: Vec<ContextItem>,
    pub tokens: shared::EventTokens,
}

pub struct EventContext;
impl Component for EventContext {
    type Message = ();
    type Properties = EventContextProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
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
        let is_mod = ctx.props().tokens.moderator_token.is_some();

        if is_mod {
            html! {
                <div>
                    {"edit"}
                </div>
            }
        } else {
            html! {}
        }
    }
}
