use crate::components::payment_popup::PaymentPopup;
use shared::EventTokens;
use yew::prelude::*;

#[derive(Eq, PartialEq, Properties)]
pub struct UpgradeButtonProps {
    pub tokens: EventTokens,
}

#[function_component]
pub fn UpgradeButton(props: &UpgradeButtonProps) -> Html {
    let onclick = Callback::from(|_| {
        log::info!("upgrade clicked");
    });

    html! {
        <>
            <button
                class="upgrade-button"
                // hidden={pending}
                {onclick}
            >
                { "upgrade for \u{20AC}7" }
            </button>
            <PaymentPopup tokens={props.tokens.clone()} />
        </>
    }
}
