use crate::{
    GlobalEvent,
    components::{Popup, Spinner},
    fetch,
    pages::BASE_API,
};
use events::{EventBridge, event_context};
use gloo_timers::callback::Timeout;
use shared::{EventTokens, EventUpgradeResponse};
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct PaymentProps {
    pub tokens: EventTokens,
}

pub enum Msg {
    GlobalEvent(GlobalEvent),
    UpgradeRequested(Option<EventUpgradeResponse>),
    TimerDone(String),
}

pub struct PaymentPopup {
    show: bool,
    timeout: Option<Timeout>,
    _events: EventBridge<GlobalEvent>,
}

impl Component for PaymentPopup {
    type Message = Msg;
    type Properties = PaymentProps;

    fn create(ctx: &Context<Self>) -> Self {
        let events = event_context(ctx)
            .unwrap_throw()
            .subscribe(ctx.link().callback(Msg::GlobalEvent));

        Self {
            show: false,
            timeout: None,
            _events: events,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::GlobalEvent(e) => {
                if matches!(e, GlobalEvent::PayForUpgrade) {
                    log::info!("open payment popup");

                    let tokens = ctx.props().tokens.clone();
                    request_upgrade(
                        tokens.public_token.clone(),
                        tokens.moderator_token,
                        ctx.link(),
                    );
                    return true;
                }
                false
            }
            Msg::TimerDone(url) => {
                self.show = false;
                log::info!("redirect to: {}", url);
                gloo_utils::window().location().assign(&url).unwrap_throw();
                true
            }
            Msg::UpgradeRequested(u) => {
                if let Some(u) = u {
                    if let EventUpgradeResponse::Redirect { url } = u {
                        let handle = {
                            let link = ctx.link().clone();

                            Timeout::new(1000, move || link.send_message(Msg::TimerDone(url)))
                        };

                        self.timeout = Some(handle);
                        self.show = true;
                    } else {
                        log::info!("admin upgrade");
                    }
                } else {
                    self.show = false;
                }
                true
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        if self.show {
            html! {
                <Popup class="payment-popup">
                    <img alt="pay via stripe" class="payment-logo" src="/assets/stripe.svg" />
                    <Spinner />
                </Popup>
            }
        } else {
            html! {}
        }
    }
}

fn request_upgrade(id: String, secret: Option<String>, link: &html::Scope<PaymentPopup>) {
    link.send_future(async move {
        match fetch::mod_upgrade(BASE_API, id, secret.unwrap_throw()).await {
            Err(e) => {
                log::error!("request_upgrade error: {e}");
                Msg::UpgradeRequested(None)
            }
            Ok(u) => Msg::UpgradeRequested(Some(u)),
        }
    });
}
