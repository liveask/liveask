use std::ops::Not;

use events::event_context;
use shared::EventTokens;
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

use crate::local_cache::LocalCache;
use crate::tracking;
use crate::{Events, GlobalEvent, components::Spinner};

use super::payment_popup::PaymentPopup;

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct Props {
    pub tokens: EventTokens,
    pub pending: bool,
}

pub struct Upgrade {
    data: Props,
    collapsed: bool,
    events: Events<GlobalEvent>,
}
pub enum Msg {
    ToggleExpansion,
    UpgradeClicked,
}
impl Component for Upgrade {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let event_id: &String = &ctx.props().tokens.public_token;
        let collapsed = LocalCache::is_premium_banner_collapsed(event_id);

        Self {
            data: ctx.props().clone(),
            collapsed,
            events: event_context(ctx).unwrap_throw(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ToggleExpansion => {
                let event_id: &String = &ctx.props().tokens.public_token;
                self.collapsed = LocalCache::toggle_premium_banner_collapsed(event_id);
                if !self.collapsed {
                    tracking::track_event(tracking::EVNT_PREMIUM_EXPAND);
                }
                true
            }
            Msg::UpgradeClicked => {
                tracking::track_event(tracking::EVNT_PREMIUM_UPGRADE);
                self.events.emit(GlobalEvent::PayForUpgrade);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let collapsed = self.collapsed;
        html! {
            <div class="premium-banner">
                <div class="rectangle">
                    <div class="toprow" onclick={ctx.link().callback(|_| Msg::ToggleExpansion)}>
                        <span>
                            { "Upgrade now to " }
                            <strong>{ "PREMIUM EVENT" }</strong>
                        </span>
                        <img
                            alt="dropdown"
                            class={classes!("dropdown",collapsed.not().then_some("rotated"))}
                            src="/assets/dropdown.svg"
                        />
                    </div>
                    { self.view_expanded(ctx) }
                    { Self::view_pending(ctx.props().pending) }
                </div>
            </div>
        }
    }
}

impl Upgrade {
    fn view_pending(pending: bool) -> Html {
        if !pending {
            return html! {};
        }

        html! { <Spinner /> }
    }
    fn view_expanded(&self, ctx: &Context<Self>) -> Html {
        let pending = ctx.props().pending;

        if self.collapsed && !pending {
            return html! {};
        }

        html! {
            <div class="expanded">
                <div class="features">
                {"To unlock the following features:"}
                <ul>
                    <li>{"Unlimited access to your event"}</li>
                    <li>{"Realtime statistics (participants, likes ..)"}</li>
                    <li>{"Export your event data"}</li>
                    <li>{"Pre-screen questions before they appear"}</li>
                    <li>{"Automatically tag questions"}</li>
                    <li>{"Add context link to your event"}</li>
                    <li>{"Plus much more .."}</li>
                </ul>
                </div>
                <div class="tmp-subscription">
                    { "Are you planning to host multiple events? " }
                    <a href="mailto:mail@live-ask.com">{ "Contact us" }</a>
                    { " for special discounts." }
                </div>
                <button
                    class="button"
                    hidden={pending}
                    onclick={ctx.link().callback(|_| Msg::UpgradeClicked)}
                >
                    { "upgrade for \u{20AC}7" }
                </button>
                <PaymentPopup tokens={self.data.tokens.clone()} />
            </div>
        }
    }
}
