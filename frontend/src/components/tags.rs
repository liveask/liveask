use shared::TagId;
use yew::prelude::*;

use super::SharableTags;

#[derive(PartialEq, Properties)]
pub struct TagsProps {
    pub tags: SharableTags,
    pub tag: Option<String>,
    pub tag_click: Callback<Option<TagId>>,
}

#[function_component]
pub fn Tags(props: &TagsProps) -> Html {
    //TODO: use_effect
    let mut tags = props
        .tags
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect::<Vec<_>>();
    tags.sort_by(|a, b| a.1.cmp(&b.1));

    html! {
        <div class="tags">
            {for tags.iter().map(|(id,tag)|{
                let is_current = props.tag.as_ref().is_some_and(|current| tag == current);

                let onclick = if is_current {
                    Callback::from({
                        let click = props.tag_click.clone();
                        move |_| {
                            click.emit(None);
                        }
                    })
                } else {
                    Callback::from({
                        let id = *id;
                        let click = props.tag_click.clone();
                        move |_| {
                            click.emit(Some(id));
                        }
                    })
                };

                let classes = if is_current {
                    "tag current"
                } else {
                    "tag"
                };

                html! {
                    <div class={classes} {onclick}>
                        {tag.clone()}
                    </div>
                }
            })}
        </div>
    }
}
