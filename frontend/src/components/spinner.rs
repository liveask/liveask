use yew::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct SpinnerProps {
    #[prop_or_default]
    pub id: Option<AttrValue>,
}

#[function_component(Spinner)]
pub fn spinner(props: &SpinnerProps) -> Html {
    html! {
        <div class="lds-roller" id={props.id.clone()}>
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
