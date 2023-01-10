use shared::EventTokens;
use yew::prelude::*;
use yew_agent::Bridge;
use yew_agent::Bridged;

use crate::tracking;
use crate::{
    agents::{EventAgent, GlobalEvent},
    components::Spinner,
    not,
};

use super::payment_popup::PaymentPopup;

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct Props {
    pub tokens: EventTokens,
    pub pending: bool,
}

pub struct Upgrade {
    data: Props,
    collapsed: bool,
    events: Box<dyn Bridge<EventAgent>>,
}
pub enum Msg {
    ToggleExpansion,
    UpgradeClicked,
}
impl Component for Upgrade {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            data: ctx.props().clone(),
            collapsed: true,
            events: EventAgent::bridge(Callback::noop()),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ToggleExpansion => {
                self.collapsed = !self.collapsed;
                if !self.collapsed {
                    tracking::track_event(tracking::EVNT_PREMIUM_EXPAND);
                }
                true
            }
            Msg::UpgradeClicked => {
                tracking::track_event(tracking::EVNT_PREMIUM_UPGRADE);
                self.events.send(GlobalEvent::PayForUpgrade);
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
                        <span>{"Upgrade to PREMIUM EVENT"}</span>
                        <img class={classes!("dropdown",not(collapsed).then_some("rotated"))} src="/assets/dropdown.svg" />
                    </div>

                    {self.view_expanded(ctx)}

                    {Self::view_pending(ctx.props().pending)}
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

        html! {
            <Spinner />
        }
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
                    <li>{"Remove 7 Day Event Timeout"}</li>
                    <li>{"Live Stats (participants, likes ..)"}</li>
                    <li class="tbd">{"Prescreen Questions (coming soon..)"}</li>
                    <li class="tbd">{"Export Data (coming soon..)"}</li>
                    <li class="tbd">{"Word Cloud (coming soon..)"}</li>
                    <li class="tbd">{"Answer Questions (coming soon..)"}</li>
                </ul>
                </div>

                <button class="button" hidden={pending} onclick={ctx.link().callback(|_| Msg::UpgradeClicked)}>
                    {"upgrade"}
                </button>

                <PaymentPopup tokens={self.data.tokens.clone()}/>
            </div>
        }
    }
}
