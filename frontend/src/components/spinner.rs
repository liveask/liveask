use yew::prelude::*;

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct SpinnerProps {
    #[prop_or_default]
    pub id: Option<AttrValue>,
}

pub struct Spinner;
impl Component for Spinner {
    type Message = ();
    type Properties = SpinnerProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="lds-roller" id={ctx.props().id.clone()}>
                <div />
                <div />
                <div />
                <div />
                <div />
                <div />
                <div />
                <div />
            </div>
        }
    }
}
