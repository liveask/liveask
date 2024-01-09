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
            html! {
                <div class="context">
                    {
                        for items.iter().map(|i|self.view_item(i))
                    }
                </div>
            }
        }
    }
}

impl EventContext {
    fn view_item(&self, item: &ContextItem) -> Html {
        html! {
            <a href={item.url.clone()} target="_blank" >
                <img src="assets/context.svg" />
                <div class="label">
                    {item.label.clone()}
                </div>
            </a>
        }
    }
}
