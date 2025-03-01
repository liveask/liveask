use super::SharableTags;
use crate::{
    GlobalEvent,
    components::{Popup, TagSelect, TextArea},
    fetch,
    local_cache::LocalCache,
    pages::BASE_API,
    tracking,
};
use events::{EventBridge, event_context};
use shared::{AddQuestionError, AddQuestionValidation, TagId, ValidationState};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{HtmlTextAreaElement, KeyboardEvent};
use yew::prelude::*;

pub enum Msg {
    GlobalEvent(GlobalEvent),
    Send,
    QuestionCreated(Option<i64>),
    Close,
    InputChanged(InputEvent),
    KeyEvent(KeyboardEvent),
    Tag(TagId),
}

pub struct QuestionPopup {
    show: bool,
    text: String,
    tag: Option<TagId>,
    errors: AddQuestionValidation,
    events: EventBridge<GlobalEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct AddQuestionProps {
    pub event_id: AttrValue,
    pub current_tag: Option<TagId>,
    pub tags: SharableTags,
}

impl Component for QuestionPopup {
    type Message = Msg;
    type Properties = AddQuestionProps;

    fn create(ctx: &Context<Self>) -> Self {
        let events = event_context(ctx)
            .unwrap_throw()
            .subscribe(ctx.link().callback(Msg::GlobalEvent));

        Self {
            show: false,
            events,
            tag: None,
            errors: AddQuestionValidation::default(),
            text: String::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::GlobalEvent(e) => {
                if matches!(e, GlobalEvent::OpenQuestionPopup) {
                    tracking::track_event(tracking::EVNT_ASK_OPEN);
                    self.tag = ctx.props().current_tag;
                    self.show = true;
                    self.errors = AddQuestionValidation::default();
                    return true;
                }
                false
            }
            Msg::Close => {
                self.show = false;
                true
            }
            Msg::Send => {
                let event_id: String = ctx.props().event_id.to_string();
                let text = self.text.clone();
                let tag = self.tag;

                tracking::track_event(tracking::EVNT_ASK_SENT);

                ctx.link().send_future(async move {
                    if let Ok(item) =
                        fetch::add_question(BASE_API, event_id.clone(), text, tag).await
                    {
                        LocalCache::set_like_state(&event_id, item.id, true);
                        if item.screening {
                            LocalCache::add_unscreened_question(&event_id, &item);
                        }
                        Msg::QuestionCreated(Some(item.id))
                    } else {
                        Msg::QuestionCreated(None)
                    }
                });

                self.show = false;
                self.text.clear();

                true
            }
            Msg::QuestionCreated(id) => {
                if let Some(id) = id {
                    self.events.emit(GlobalEvent::QuestionCreated(id));
                }
                true
            }
            Msg::InputChanged(ev) => {
                let target: HtmlTextAreaElement = ev.target_dyn_into().unwrap_throw();
                self.text = target.value();
                self.errors.check(&self.text);
                true
            }
            Msg::KeyEvent(e) => {
                if &e.key() == "Enter" && e.meta_key() && !self.errors.has_any() {
                    ctx.link().callback(|()| Msg::Send).emit(());
                }
                true
            }
            Msg::Tag(id) => {
                self.tag = Some(id);
                true
            }
        }
    }

    #[allow(clippy::if_not_else)]
    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.show {
            let tags = SharableTags::clone(&ctx.props().tags);

            let on_close = ctx.link().callback(|()| Msg::Close);
            let on_click_ask = ctx.link().callback(|_| Msg::Send);
            let on_key = ctx.link().callback(Msg::KeyEvent);
            let on_tag = ctx.link().callback(|id: TagId| Msg::Tag(id));

            html! {
                <Popup class="share-popup" {on_close}>
                    <div class="newquestion">
                        <div class="add-question" onkeydown={on_key}>
                            <TextArea
                                id="questiontext"
                                name="questiontext"
                                maxlength="200"
                                value={self.text.clone()}
                                placeholder="What’s your question?"
                                required=true
                                oninput={ctx.link().callback(Msg::InputChanged)}
                                autosize=true
                            />
                            <div class="more-info">
                                <div class="chars-info">
                                    <code>{ format!("{}",200 - self.text.len()) }</code>
                                </div>
                                { html!{
                                <div hidden={!self.errors.has_any()} class="invalid">
                                    <div>
                                    {self.error_text().unwrap_or_default()}
                                    </div>
                                </div>
                            } }
                            </div>
                            <TagSelect {tags} tag_selected={on_tag} tag={self.tag}/>
                        </div>
                        <button
                            class="dlg-button"
                            onclick={on_click_ask}
                            disabled={self.errors.has_any()}
                        >
                            { "Ask!" }
                        </button>
                    </div>
                </Popup>
            }
        } else {
            html! {}
        }
    }
}

impl QuestionPopup {
    fn error_text(&self) -> Option<String> {
        match self.errors.content {
            ValidationState::Invalid(AddQuestionError::MinLength(_, _)) => {
                Some("Question too short.".to_string())
            }
            ValidationState::Invalid(AddQuestionError::MaxLength(_, max)) => {
                Some(format!("Question too long. Max: {max})"))
            }
            ValidationState::Invalid(AddQuestionError::MinWordCount(_, min)) => {
                Some(format!("Minimum words required: {min}."))
            }
            ValidationState::Invalid(AddQuestionError::WordLengthMax(max)) => {
                Some(format!("No word can be longer than: {max}."))
            }
            _ => None,
        }
    }
}
