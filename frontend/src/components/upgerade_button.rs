use shared::{EventTokens, EventUpgradeResponse};
use wasm_bindgen::UnwrapThrowExt;
use yew::{prelude::*, suspense::use_future_with};

use crate::{components::Spinner, fetch, local_cache::LocalCache, pages::BASE_API, tracking};

#[derive(Eq, PartialEq, Properties)]
pub struct UpgradeButtonProps {
    pub tokens: EventTokens,
    pub pending: bool,
}

#[function_component]
pub fn UpgradeButton(props: &UpgradeButtonProps) -> Html {
    let popup_open = use_state(|| false);

    let onclick = Callback::from({
        let popup_open = popup_open.clone();
        let event_id = props.tokens.public_token.clone();
        move |_| {
            tracking::track_event(tracking::EVNT_PREMIUM_UPGRADE);
            LocalCache::set_mod_color_picker_shown(&event_id, false);
            popup_open.set(true);
        }
    });

    let pending = props.pending;

    html! {
        <>
            <button
                class="upgrade-button"
                hidden={pending}
                {onclick}
            >
                { "upgrade for \u{20AC}7" }
            </button>

            <PaymentOverlay tokens={props.tokens.clone()} open={popup_open} />
        </>
    }
}

#[derive(PartialEq, Properties)]
pub struct PaymentOverlayProps {
    pub tokens: EventTokens,
    pub open: UseStateHandle<bool>,
}

#[function_component]
fn PaymentOverlay(props: &PaymentOverlayProps) -> Html {
    let _ = use_future_with(props.open.clone(), {
        let tokens = props.tokens.clone();
        let open = props.open.clone();

        |_| async move {
            if *open {
                match fetch::mod_upgrade(
                    BASE_API,
                    tokens.public_token.clone(),
                    tokens.moderator_token.clone().unwrap_throw(),
                )
                .await
                {
                    Err(e) => {
                        log::error!("mod_edit_event error: {e}");
                        open.set(false);
                    }
                    Ok(EventUpgradeResponse::Redirect { url }) => {
                        log::info!("redirect to: {}", url);
                        gloo_utils::window().location().assign(&url).unwrap_throw();
                    }
                    Ok(EventUpgradeResponse::AdminUpgrade) => {
                        open.set(false);
                    }
                }
            }
        }
    });

    if *props.open {
        html! {
            <div class="popup-bg" >
                <div class="payment-popup">
                    <img alt="pay via stripe" class="payment-logo" src="/assets/stripe.svg" />
                    <Spinner />
                </div>
            </div>
        }
    } else {
        html! {}
    }
}
