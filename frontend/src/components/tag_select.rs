use shared::TagId;
use yew::{function_component, prelude::*};

use super::SharableTags;

#[derive(PartialEq, Properties)]
pub struct TagSelectProps {
    pub tags: SharableTags,
    pub tag_selected: Callback<TagId>,
    pub tag: Option<TagId>,
}

#[function_component]
pub fn TagSelect(props: &TagSelectProps) -> Html {
    let mut tags = props
        .tags
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect::<Vec<_>>();
    tags.sort_by(|a, b| a.1.cmp(&b.1));

    html! {
        <div class="tag-select">
            <div class="header">{"tag your question"}</div>

            <div class="tags">
            {
                for tags.into_iter().map(|(id,tag)|{
                    let is_current = props.tag.is_some_and(|current_id| id == current_id);
                    if  is_current {
                        html! {
                            <div class="tag current">
                                {tag.clone()}
                            </div>
                        }
                    } else {
                        html! {
                            <div class="tag" onclick={
                                let tag_selected = props.tag_selected.clone();
                                move |_| {
                                    tag_selected.emit(id);
                                }
                            }>
                                {tag.clone()}
                            </div>
                        }
                    }
                })
            }
            </div>
        </div>
    }
}
