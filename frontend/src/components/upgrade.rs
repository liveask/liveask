#![allow(dead_code)]
use shared::{EventTokens, EventUpgrade};
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

use crate::{fetch, not, pages::BASE_API};

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct Props {
    pub tokens: EventTokens,
}

pub struct Upgrade {
    data: Props,
    collapsed: bool,
}
pub enum Msg {
    Expand,
    Collapse,
    ToggleExpansion,
    Upgrade,
    UpgradeRequested(Option<EventUpgrade>),
}
impl Component for Upgrade {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            data: ctx.props().clone(),
            collapsed: true,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Expand => todo!(),
            Msg::Collapse => todo!(),
            Msg::ToggleExpansion => {
                self.collapsed = !self.collapsed;
                true
            }
            Msg::Upgrade => {
                let tokens = self.data.tokens.clone();
                request_upgrade(
                    tokens.public_token.clone(),
                    tokens.moderator_token,
                    ctx.link(),
                );

                false
            }
            Msg::UpgradeRequested(u) => {
                if let Some(u) = u {
                    log::info!("redirect to: {}", u.url);
                    gloo::utils::window()
                        .location()
                        .assign(&u.url)
                        .unwrap_throw();
                    true
                } else {
                    false
                }
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
                </div>
            </div>
        }
    }
}

impl Upgrade {
    fn view_expanded(&self, ctx: &Context<Self>) -> Html {
        if self.collapsed {
            return html! {};
        }

        html! {
            <div class="expanded">
                <div class="features">
                {"To unlock the following features:"}
                <ul>
                    <li>{"Remove 7 Day Event Timeout"}</li>
                    <li class="tbd">{"Prescreen Questions (coming soon..)"}</li>
                    <li class="tbd">{"Export Questions as CSV (coming soon..)"}</li>
                </ul>
                </div>
                <button class="button" onclick={ctx.link().callback(|_| Msg::Upgrade)}>
                    {"upgrade"}
                </button>
            </div>
        }
    }
}

fn request_upgrade(id: String, secret: Option<String>, link: &html::Scope<Upgrade>) {
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
