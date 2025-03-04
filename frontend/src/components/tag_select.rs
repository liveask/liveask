use crate::components::Tags;

use super::SharableTags;
use shared::TagId;
use yew::{function_component, prelude::*};

#[derive(PartialEq, Properties)]
pub struct TagSelectProps {
    pub tags: SharableTags,
    pub tag_selected: Callback<TagId>,
    pub tag: Option<TagId>,
}

#[function_component]
pub fn TagSelect(props: &TagSelectProps) -> Html {
    let tag_click: Callback<Option<TagId>> = Callback::from({
        let tag_selected = props.tag_selected.clone();
        move |tag: Option<TagId>| {
            if let Some(tag) = tag {
                tag_selected.emit(tag);
            }
        }
    });

    let tag = props.tag.and_then(|tag| props.tags.get(&tag).cloned());

    html! {
        <div class="tag-select" hidden={props.tags.is_empty()}>
            <div class="header">{"tag your question"}</div>
            <Tags tags={SharableTags::clone(&props.tags)} {tag} {tag_click} />
        </div>
    }
}
