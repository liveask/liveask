use yew::{function_component, html, Html};

#[function_component]
pub fn TrustedBy() -> Html {
    html! {
        <>
            <div class="trustedby">
                <h2>{ "Trusted by" }</h2>
                <div class="items">
                    <div class="item"><img src="assets/trustedby/ms.png" /></div>
                    <div class="item"><img src="assets/trustedby/un.svg" /></div>
                    <div class="item"><img src="assets/trustedby/worldbank.png" /></div>
                    <div class="item"><img src="assets/trustedby/canada.png" /></div>
                    <div class="item"><img src="assets/trustedby/lifescience.png" /></div>
                    <div class="item"><img src="assets/trustedby/alibaba.png" /></div>
                </div>
            </div>
        </>
    }
}
