use yew::{Html, function_component, html};

#[function_component]
pub fn TrustedBy() -> Html {
    html! {
        <>
            <div class="trustedby">
                <h2>{ "Trusted by" }</h2>
                <div class="items">
                    <div class="item"><img src="assets/trustedby/microsoft.svg" /></div>
                    <div class="item"><img src="assets/trustedby/un.svg" /></div>
                    <div class="item"><img src="assets/trustedby/worldbank.svg" /></div>
                    <div class="item"><img src="assets/trustedby/canada.svg" /></div>
                    <div class="item"><img src="assets/trustedby/l2s.svg" /></div>
                    <div class="item"><img src="assets/trustedby/alibaba.svg" /></div>
                </div>
            </div>
        </>
    }
}
