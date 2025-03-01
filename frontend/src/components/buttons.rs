use yew::{AttrValue, Callback, Html, Properties, function_component, html};

#[derive(PartialEq, Properties)]
pub struct ButtonProps {
    pub label: AttrValue,
    pub on_click: Callback<()>,
}

#[function_component]
pub fn DarkButton(props: &ButtonProps) -> Html {
    let onclick = Callback::from({
        let on_click = props.on_click.clone();
        move |_| {
            on_click.emit(());
        }
    });

    html! {
        <button class="button-dark" {onclick}>
            {props.label.clone()}
        </button>
    }
}

#[function_component]
pub fn RedButton(props: &ButtonProps) -> Html {
    let onclick = Callback::from({
        let on_click = props.on_click.clone();
        move |_| {
            on_click.emit(());
        }
    });

    html! {
        <button class="button-red" {onclick}>
            {props.label.clone()}
        </button>
    }
}

#[function_component]
pub fn WhiteButton(props: &ButtonProps) -> Html {
    let onclick = Callback::from({
        let on_click = props.on_click.clone();
        move |_| {
            on_click.emit(());
        }
    });

    html! {
        <button class="button-white" {onclick}>
            {props.label.clone()}
        </button>
    }
}

#[function_component]
pub fn BlueButton(props: &ButtonProps) -> Html {
    let onclick = Callback::from({
        let on_click = props.on_click.clone();
        move |_| {
            on_click.emit(());
        }
    });

    html! {
        <button class="button-blue" {onclick}>
            {props.label.clone()}
        </button>
    }
}
