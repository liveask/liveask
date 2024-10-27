use yew::{function_component, html, Html};

#[function_component]
pub fn TrustedBy() -> Html {
    html! {
        <>
            <h1>{ "Trusted BY" }</h1>
            <div class="trustedby">
                <div class="item"><img src="assets/trustedby/ms.png" /></div>
                <div class="item"><img src="assets/trustedby/un.png" /></div>
                <div class="item"><img src="assets/trustedby/worldbank.png" /></div>
                <div class="item"><img src="assets/trustedby/canada.png" /></div>
                <div class="item"><img src="assets/trustedby/lifescience.png" /></div>
                <div class="item"><img src="assets/trustedby/alibaba.png" /></div>
            </div>
        </>
    }
}
