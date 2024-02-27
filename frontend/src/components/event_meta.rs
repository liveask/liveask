use crate::components::{EventContext, MetaPopup};
use shared::{ContextItem, EditMetaData, EventData, EventTokens};
use yew::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct EventMetaProps {
    pub context: Vec<ContextItem>,
    pub tokens: EventTokens,
    pub data: EventData,
    pub is_premium: bool,
    pub is_masked: bool,
    pub is_first_24h: bool,
}

pub struct EventMeta {
    show_meta_popup: bool,
}

pub enum Msg {
    EditClick,
    ClosePopup,
}

impl Component for EventMeta {
    type Message = Msg;
    type Properties = EventMetaProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            show_meta_popup: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::EditClick => {
                self.show_meta_popup = true;
                true
            }
            Msg::ClosePopup => {
                self.show_meta_popup = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let name = ctx.props().data.name.clone();
        let desc = ctx.props().data.description.clone();
        let context = ctx.props().context.clone();
        let is_premium = ctx.props().is_premium;
        let is_masked = ctx.props().is_masked;
        let meta = EditMetaData {
            title: name.clone(),
            description: desc.clone(),
        };

        let on_close_popup = ctx.link().callback(|()| Msg::ClosePopup);

        html! {
            <>
                <div class="event-name-label">{"The Event"}{ Self::mod_view_edit(ctx) }</div>
                <div class="event-name">{name}</div>
                <EventContext {context} tokens={ctx.props().tokens.clone()} {is_premium} />
                <MetaPopup tokens={ctx.props().tokens.clone()} on_close={on_close_popup} show={self.show_meta_popup} {meta} />
                //TODO: collapsable event desc
                <div
                    class={classes!("event-desc",is_masked.then_some("blurr"))}
                >
                    { {desc} }
                </div>
            </>
        }
    }
}

impl EventMeta {
    fn mod_view_edit(ctx: &Context<Self>) -> Html {
        //TODO: show clock icon with tooltip that only in first 24h the text can be edited
        let is_mod = ctx.props().tokens.is_mod() && ctx.props().is_first_24h;
        let on_click = ctx.link().callback(|_| Msg::EditClick);

        if is_mod {
            html! {
                <button class="button-icon" onclick={on_click}>
                    <img src="assets/edit.svg" alt="edit"/>
                </button>
            }
        } else {
            html! {}
        }
    }
}
