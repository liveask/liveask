use serde::Deserialize;
use shared::{SubscriptionCheckout, SubscriptionResponse};
use yew::{prelude::*, suspense::use_future_with};
use yew_router::hooks::use_location;

use super::BASE_API;
use crate::fetch;

#[derive(Debug, Default, Deserialize)]
struct QueryParams {
    pub checkout: Option<String>,
    pub customer_email: Option<String>,
}

impl QueryParams {
    fn into_checkout(self) -> Option<SubscriptionCheckout> {
        if let Some(id) = self.checkout {
            Some(SubscriptionCheckout::CheckoutId(id))
        } else {
            self.customer_email.map(SubscriptionCheckout::CustomerEmail)
        }
    }
}

#[function_component]
pub fn Subscribe() -> Html {
    let location = use_location();

    let params: QueryParams = location
        .and_then(|loc| loc.query::<QueryParams>().ok())
        .unwrap_or_default();

    let checkout = params.into_checkout();

    // State for subscription URL
    let subscription_url: UseStateHandle<Option<Result<String, String>>> = use_state(|| None);

    // State for checkout response
    let response: UseStateHandle<Option<Result<SubscriptionResponse, String>>> = use_state(|| None);
    let copied = use_state(|| false);

    let _ = use_future_with(checkout.clone(), {
        let subscription_url = subscription_url.clone();
        let response = response.clone();

        |checkout| async move {
            if let Some(checkout) = (*checkout).clone() {
                let result = fetch::subscription_checkout(BASE_API, checkout)
                    .await
                    .map_err(|e| format!("{e:?}"));
                response.set(Some(result));
            } else {
                let result = fetch::subscription_url(BASE_API)
                    .await
                    .map_err(|e| format!("{e:?}"));
                subscription_url.set(Some(result));
            }
        }
    });

    if checkout.is_none() {
        return match &*subscription_url {
            None => html! { <div>{"Loading..."}</div> },
            Some(Ok(url)) => html! {
                <a href={url.clone()}>
                    <button class="button-red">{"Subscribe"}</button>
                </a>
            },
            Some(Err(e)) => html! {
                <div class="subscribe-success">
                    <div class="title">{"Subscription Not Available"}</div>
                    <div class="instructions">
                        {"Subscription feature is not configured. "}
                        {"Please create an active payment link in Stripe."}
                    </div>
                    <div style="margin-top: 20px; color: #666; font-size: 12px;">
                        {format!("Error: {}", e)}
                    </div>
                </div>
            },
        };
    }

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
                                if let Some(portal_url) = &res.portal_url {
                                    <div class="instructions" style="margin-top: 20px;">
                                        {"Manage your subscription:"}
                                    </div>
                                    <a href={portal_url.clone()}>
                                        <button class="button-red">{"Customer Portal"}</button>
                                    </a>
                                }
                            </div>
                        }
                    },
                    Some(Err(e)) => html! { <div>{format!("Error: {}", e)}</div> },
                }
            }
        </>
    }
}
