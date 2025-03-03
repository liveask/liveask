use crate::components::{ColorPopup, EventContext, MetaPopup};
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
    show_color_edit: bool,
}

pub enum Msg {
    EditClick,
    EditColorClick,
    ClosePopup,
}

impl Component for EventMeta {
    type Message = Msg;
    type Properties = EventMetaProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            show_meta_popup: false,
            show_color_edit: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::EditClick => {
                self.show_meta_popup = true;
                true
            }
            Msg::EditColorClick => {
                self.show_color_edit = true;
                true
            }
            Msg::ClosePopup => {
                self.show_meta_popup = false;
                self.show_color_edit = false;
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
        let color = ctx.props().data.color.clone();

        let on_close_popup = ctx.link().callback(|()| Msg::ClosePopup);

        html! {
            <>
                <div class="event-name-label">{"The Event"}{ Self::mod_view_edit(ctx) }</div>
                <div class="event-name">{name}</div>
                <EventContext {context} tokens={ctx.props().tokens.clone()} {is_premium} />
                <MetaPopup tokens={ctx.props().tokens.clone()} on_close={on_close_popup.clone()} show={self.show_meta_popup} {meta} />
                <ColorPopup tokens={ctx.props().tokens.clone()} on_close={on_close_popup} open={self.show_color_edit} {color} {is_premium} />

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
        let is_first_24h = ctx.props().tokens.is_mod() && ctx.props().is_first_24h;
        let on_click_text = ctx.link().callback(|_| Msg::EditClick);
        let on_click_color = ctx.link().callback(|_| Msg::EditColorClick);

        if is_first_24h {
            html! {
                <>
                    <button class="button-icon" onclick={on_click_text}>
                        <img src="assets/edit.svg" alt="edit"/>
                    </button>
                    <button class="button-icon" onclick={on_click_color}>
                        <img src="assets/color-pick.svg" alt="edit"/>
                    </button>
                </>
            }
        } else {
            html! {
                <button class="button-icon" onclick={on_click_color}>
                    <img src="assets/color-pick.svg" alt="edit"/>
                </button>
            }
        }
    }
}
