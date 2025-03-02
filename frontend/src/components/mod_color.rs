use crate::{
    components::{RedButton, buttons::WhiteButton},
    fetch,
    pages::BASE_API,
};
use shared::{Color, EditColor, EventTokens, ModEvent};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlElement;
use yew::{prelude::*, suspense::use_future_with};

#[derive(PartialEq, Properties)]
pub struct ColorPopupProps {
    pub tokens: EventTokens,
    pub on_close: Callback<()>,
    pub color: Option<Color>,
    pub is_premium: bool,
    pub open: bool,
}

#[allow(unused_braces)]
#[function_component]
pub fn ColorPopup(props: &ColorPopupProps) -> Html {
    let bg_ref = use_node_ref();

    let click_bg = Callback::from({
        let on_close = props.on_close.clone();
        let bg_ref = bg_ref.clone();
        move |e: MouseEvent| {
            let div = bg_ref
                .cast::<HtmlElement>()
                .expect_throw("div_ref not attached to div element");

            let target = e.target().unwrap_throw();
            let target: HtmlElement = target.dyn_into().unwrap_throw();

            if div == target {
                on_close.emit(());
            }
        }
    });

    let color_state = use_state(|| {
        props
            .color
            .as_ref()
            .map_or_else(|| String::from("#282828"), |c| c.0.clone())
    });

    let color_save = use_state(|| None::<String>);

    let click_save = Callback::from({
        let on_close = props.on_close.clone();
        let color_state = color_state.clone();
        let color_save = color_save.clone();
        move |()| {
            color_save.set(Some((*color_state).clone()));
            on_close.emit(());
        }
    });
    let click_cancel = Callback::from({
        let on_close = props.on_close.clone();
        move |()| {
            on_close.emit(());
        }
    });

    let _ = use_future_with(color_save, {
        let tokens = props.tokens.clone();

        |color_save| async move {
            if let Some(color) = &**color_save {
                color_save.set(None);

                if let Err(e) = fetch::mod_edit_event(
                    BASE_API,
                    tokens.public_token.clone(),
                    tokens.moderator_token.clone().unwrap_throw(),
                    ModEvent {
                        color: Some(EditColor(color.clone())),
                        ..Default::default()
                    },
                )
                .await
                {
                    log::error!("mod_edit_event error: {e}");
                }
            }
        }
    });

    if props.open {
        html! {
            <div class="popup-bg" ref={bg_ref} onclick={click_bg}>
                <div class="color-popup">
                    <div class="header">{"Select Event Color"}</div>

                    <div class="colors">
                        <ColorButton color="#282828" state={color_state.clone()} />
                        <ColorButton color="#FF2C5E" state={color_state.clone()} />
                        <ColorButton color="#7BBE31" state={color_state.clone()} />
                    </div>

                    <div class="buttons" style={format!("background-color: {}",*color_state)}>
                        <WhiteButton label="Cancel" on_click={click_cancel} />
                        <RedButton label="Save" on_click={click_save} />
                    </div>
                </div>
            </div>
        }
    } else {
        html! {}
    }
}

#[derive(PartialEq, Properties)]
pub struct ColorButtonProps {
    pub state: UseStateHandle<String>,
    pub color: String,
}

#[function_component]
fn ColorButton(props: &ColorButtonProps) -> Html {
    let onclick = Callback::from({
        let state = props.state.clone();
        let color = props.color.clone();
        move |_| {
            state.set(color.clone());
        }
    });

    html! {
        <div class="color" style={format!("background-color: {}",props.color.clone())} {onclick}></div>
    }
}
