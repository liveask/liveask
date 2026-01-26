use serde::Deserialize;
use shared::SubscriptionResponse;
use yew::{prelude::*, suspense::use_future_with};
use yew_router::hooks::use_location;

use super::BASE_API;
use crate::fetch;

const STRIPE_PURCHASE_URL: &str = "https://buy.stripe.com/test_5kQbJ1bUHdh52ntb9A7Vm00";

#[derive(Debug, Default, Deserialize)]
struct QueryParams {
    pub checkout: Option<String>,
}

#[function_component]
pub fn Subscribe() -> Html {
    let location = use_location();

    let params: QueryParams = location
        .and_then(|loc| loc.query::<QueryParams>().ok())
        .unwrap_or_default();

    let checkout_id = params.checkout;

    // No checkout param - show purchase button
    if checkout_id.is_none() {
        return html! {
            <a href={STRIPE_PURCHASE_URL}>
                <button class="button-red">{"Subscribe"}</button>
            </a>
        };
    }

    let response: UseStateHandle<Option<Result<SubscriptionResponse, String>>> = use_state(|| None);
    let copied = use_state(|| false);

    let _ = use_future_with(checkout_id, {
        let response = response.clone();

        |checkout_id| async move {
            if let Some(checkout) = (*checkout_id).clone() {
                let result = fetch::subscription_checkout(BASE_API, checkout)
                    .await
                    .map_err(|e| format!("{e:?}"));
                response.set(Some(result));
            }
        }
    });

    html! {
        <>
            {
                match &*response {
                    None => html! { <div>{"Loading..."}</div> },
                    Some(Ok(res)) => {
                        let origin = gloo_utils::window().location().origin().unwrap_or_default();
                        let newevent_url = format!("{}/newevent?customer={}", origin, res.customer);
                        let on_click_copy = {
                            let copied = copied.clone();
                            let url = newevent_url.clone();
                            Callback::from(move |_| {
                                let _ = gloo_utils::window()
                                    .navigator()
                                    .clipboard()
                                    .write_text(&url);
                                copied.set(true);
                            })
                        };
                        html! {
                            <div class="subscribe-success">
                                <div class="title">{"Thank you for subscribing!"}</div>
                                <div class="instructions">
                                    {"Save this URL to create premium events with your subscription:"}
                                </div>
                                <div class="link-box" onclick={on_click_copy}>
                                    <div class="link">{ newevent_url.clone() }</div>
                                    <div class="copy">
                                        { if *copied { "Copied" } else { "Copy" } }
                                    </div>
                                </div>
                                <a href={newevent_url}>
                                    <button class="button-red">{"Open"}</button>
                                </a>
                            </div>
                        }
                    },
                    Some(Err(e)) => html! { <div>{format!("Error: {}", e)}</div> },
                }
            }
        </>
    }
}
