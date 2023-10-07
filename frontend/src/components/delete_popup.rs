use crate::{
    components::Popup, fetch, pages::BASE_API, routes::Route, tracking, GlobalEvent, GlobalEvents,
};
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::{prelude::History, scope_ext::RouterScopeExt};

pub enum Msg {
    GlobalEvent(GlobalEvent),
    ConfirmedDelete,
    Sent,
    Close,
}

pub struct DeletePopup {
    show: bool,
    _events: GlobalEvents,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct DeletePopupProps {
    pub tokens: shared::EventTokens,
}

impl Component for DeletePopup {
    type Message = Msg;
    type Properties = DeletePopupProps;

    fn create(ctx: &Context<Self>) -> Self {
        let (mut events, _) = ctx
            .link()
            .context::<GlobalEvents>(Callback::noop())
            .expect_throw("context to be set");

        events.subscribe(ctx.link().callback(Msg::GlobalEvent));

        Self {
            show: false,
            _events: events,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::GlobalEvent(e) => {
                if matches!(e, GlobalEvent::DeletePopup) {
                    self.show = true;
                    return true;
                }
                false
            }
            Msg::Close => {
                self.show = false;
                true
            }
            Msg::ConfirmedDelete => {
                self.show = false;
                let event_id = ctx.props().tokens.public_token.clone();
                let secret = ctx
                    .props()
                    .tokens
                    .moderator_token
                    .clone()
                    .unwrap_or_default();

                tracking::track_event(tracking::EVNT_EVENT_DELETE);

                ctx.link().send_future(async move {
                    let _res = fetch::delete_event(BASE_API, event_id.clone(), secret).await;

                    Msg::Sent
                });
                true
            }
            Msg::Sent => {
                ctx.link().history().unwrap_throw().push(Route::Home);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.show {
            let on_close = ctx.link().callback(|()| Msg::Close);
            let on_click_ok = ctx.link().callback(|_| Msg::ConfirmedDelete);
            let on_click_no = ctx.link().callback(|_| Msg::Close);

            html! {
            <Popup class="delete-popup" {on_close}>
                <div class="title">
                    {"Delete event permanently"}
                </div>

                <div class="text">
                    {"This action is irreversable. Only you as the moderator can delete an event. Users you shared this event with will not be
                      able to see it anymore."}
                </div>

                <div class="buttons">
                    <div class="btn-yes" onclick={on_click_ok}>
                         {"yes"}
                    </div>
                    <div class="btn-yes" onclick={on_click_no}>
                         {"no"}
                    </div>
                </div>
            </Popup>
            }
        } else {
            html! {}
        }
    }
}
