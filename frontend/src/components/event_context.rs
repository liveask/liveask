use shared::ContextItem;
use yew::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct EventContextProps {
    pub context: Vec<ContextItem>,
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

        if items.is_empty() {
            html! {}
        } else {
            html! { <div class="context">{ for items.iter().map(Self::view_item) }</div> }
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
}
