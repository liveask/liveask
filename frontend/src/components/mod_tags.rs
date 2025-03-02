use crate::{
    components::{DarkButton, RedButton, Tags},
    fetch,
    pages::BASE_API,
};
use shared::{EventTokens, ModEvent, TagId, TagValidation};
use std::{collections::HashMap, rc::Rc};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{HtmlElement, HtmlInputElement};
use yew::{prelude::*, suspense::use_future_with};

pub type SharableTags = Rc<HashMap<TagId, String>>;

#[derive(Eq, PartialEq, Properties)]
pub struct ModTagsProps {
    pub tokens: EventTokens,
    pub tags: SharableTags,
    pub tag: Option<String>,
}

#[function_component]
pub fn ModTags(props: &ModTagsProps) -> Html {
    let add_popup_open = use_state(|| false);

    let on_click: Callback<()> = Callback::from({
        let add_popup_open = add_popup_open.clone();
        move |()| {
            add_popup_open.set(true);
        }
    });

    let set_current_tag = use_state(|| None::<shared::CurrentTag>);

    let tag_click: Callback<Option<TagId>> = Callback::from({
        let set_current_tag = set_current_tag.clone();
        let tags = SharableTags::clone(&props.tags);
        move |tag: Option<TagId>| {
            if let Some(tag) = tag {
                let tag = tags.get(&tag).unwrap_throw().clone();
                set_current_tag.set(Some(shared::CurrentTag::Enabled(tag)));
            } else {
                set_current_tag.set(Some(shared::CurrentTag::Disabled));
            }
        }
    });

    //TODO: make hook and reuse in add-tag
    let _ = use_future_with(set_current_tag, {
        let tokens = props.tokens.clone();

        |set_current_tag| async move {
            if let Some(tag) = &**set_current_tag {
                set_current_tag.set(None);

                if let Err(e) = fetch::mod_edit_event(
                    BASE_API,
                    tokens.public_token.clone(),
                    tokens.moderator_token.clone().unwrap_throw(),
                    ModEvent {
                        current_tag: Some(tag.clone()),
                        ..Default::default()
                    },
                )
                .await
                {
                    log::error!("mod_edit_event error: {e}");
                }
            }
        }
    });

    html! {
        <div class="mod-tags">

            <div class="questions-seperator">{"TAGS"}</div>

            <Tags tags={SharableTags::clone(&props.tags)} tag={props.tag.clone()} {tag_click} />

            {
                if props.tags.len() < shared::MAX_TAGS {
                    html! {
                        <>
                        <AddTag open={add_popup_open.clone()} tokens={props.tokens.clone()} />
                        <DarkButton label="add tag" {on_click}/>
                        </>
                    }
                } else {
                    html! {}
                }
            }
        </div>
    }
}

#[derive(PartialEq, Properties)]
pub struct AddTagProps {
    pub open: UseStateHandle<bool>,
    pub tokens: EventTokens,
}

#[function_component]
fn AddTag(props: &AddTagProps) -> Html {
    let bg_ref = use_node_ref();
    let input_ref = use_node_ref();

    let click_bg = Callback::from({
        let open = props.open.clone();
        let bg_ref = bg_ref.clone();
        move |e: MouseEvent| {
            let div = bg_ref
                .cast::<HtmlElement>()
                .expect_throw("div_ref not attached to div element");

            let target = e.target().unwrap_throw();
            let target: HtmlElement = target.dyn_into().unwrap_throw();

            if div == target {
                open.set(false);
            }
        }
    });

    let tag_to_add = use_state(|| None::<String>);

    let on_click: Callback<()> = Callback::from({
        let open = props.open.clone();
        let input_ref = input_ref.clone();
        let tag_to_add = tag_to_add.clone();
        move |()| {
            let input = input_ref
                .cast::<HtmlInputElement>()
                .expect_throw("div_ref not attached to div element");

            let value = input.value();

            let mut valid = TagValidation::default();
            valid.check(&value);

            if !valid.has_any() {
                tag_to_add.set(Some(value));
                open.set(false);
            }
        }
    });

    let _ = use_future_with(tag_to_add, {
        let tokens = props.tokens.clone();

        |tag_to_add| async move {
            if let Some(tag) = &**tag_to_add {
                tag_to_add.set(None);

                if let Err(e) = fetch::mod_edit_event(
                    BASE_API,
                    tokens.public_token.clone(),
                    tokens.moderator_token.clone().unwrap_throw(),
                    ModEvent {
                        current_tag: Some(shared::CurrentTag::Enabled(tag.clone())),
                        ..Default::default()
                    },
                )
                .await
                {
                    log::error!("mod_edit_event error: {e}");
                }
            }
        }
    });

    if *props.open {
        html! {
            <div class="popup-bg" ref={bg_ref} onclick={click_bg}>
                <div class="add-tag-popup">
                    <div class="">
                        <input
                            ref={input_ref}
                            type="text"
                            placeholder="tag"
                            maxlength="15"
                        />
                    </div>
                    <RedButton label="Add" {on_click} />
                </div>
            </div>
        }
    } else {
        html! {}
    }
}
