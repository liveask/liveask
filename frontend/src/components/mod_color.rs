use crate::components::{RedButton, buttons::WhiteButton};
use shared::{Color, EventTokens};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlElement;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct ColorPopupProps {
    pub tokens: EventTokens,
    pub on_close: Callback<()>,
    pub color: Option<Color>,
    pub is_premium: bool,
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

    let click_save = Callback::from({
        let on_close = props.on_close.clone();
        move |()| {
            on_close.emit(());
        }
    });
    let click_cancel = Callback::from({
        let on_close = props.on_close.clone();
        move |()| {
            on_close.emit(());
        }
    });

    // let _ = use_future_with(tag_to_add, {
    //     let tokens = props.tokens.clone();

    //     |tag_to_add| async move {
    //         if let Some(tag) = &**tag_to_add {
    //             tag_to_add.set(None);

    //             if let Err(e) = fetch::mod_edit_event(
    //                 BASE_API,
    //                 tokens.public_token.clone(),
    //                 tokens.moderator_token.clone().unwrap_throw(),
    //                 ModEvent {
    //                     current_tag: Some(shared::CurrentTag::Enabled(tag.clone())),
    //                     ..Default::default()
    //                 },
    //             )
    //             .await
    //             {
    //                 log::error!("mod_edit_event error: {e}");
    //             }
    //         }
    //     }
    // });

    let color = props
        .color
        .as_ref()
        .map(|c| c.0.clone())
        .unwrap_or(String::from("#282828"));

    html! {
        <div class="popup-bg" ref={bg_ref} onclick={click_bg}>
            <div class="color-popup">
                <div class="header">{"Select Event Color"}</div>

                <div class="colors">
                    <div class="color" style="background-color: #282828;"></div>
                    <div class="color" style="background-color: #FF2C5E;"></div>
                    <div class="color" style="background-color: #7BBE31;"></div>
                </div>

                <div class="buttons" style={format!("background-color: {}",color)}>
                    <WhiteButton label="Cancel" on_click={click_cancel} />
                    <RedButton label="Save" on_click={click_save} />
                </div>
            </div>
        </div>
    }
}
