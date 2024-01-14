use crate::eventsdb::Error;

use super::AttributeMap;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::{EventTags, Tag, TagId};

const ATTR_TAGS_TAGS: &str = "tags";
const ATTR_TAGS_CURRENT: &str = "current";

pub fn eventtags_to_attributes(value: EventTags) -> AttributeMap {
    let mut map = AttributeMap::new();

    map.insert(
        ATTR_TAGS_TAGS.into(),
        AttributeValue::L(
            value
                .tags
                .into_iter()
                .map(|i| AttributeValue::M(tag_to_attributes(i)))
                .collect(),
        ),
    );

    if let Some(tag) = value.current_tag {
        map.insert(
            ATTR_TAGS_CURRENT.into(),
            AttributeValue::N(tag.0.to_string()),
        );
    }

    map
}

pub fn attributes_to_eventtags(value: &AttributeMap) -> EventTags {
    let tags = value
        .get(ATTR_TAGS_TAGS)
        .and_then(|v| v.as_l().ok())
        .map(|list| {
            list.iter()
                .filter_map(|tag| tag.as_m().ok().and_then(|tag| attributes_to_tag(tag).ok()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let current_tag = value
        .get(ATTR_TAGS_CURRENT)
        .and_then(|value| value.as_n().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .map(TagId);

    EventTags { tags, current_tag }
}

const ATTR_TAG_ID: &str = "id";
const ATTR_TAG_NAME: &str = "name";

fn tag_to_attributes(value: Tag) -> AttributeMap {
    let mut map = AttributeMap::new();

    map.insert(
        ATTR_TAG_ID.into(),
        AttributeValue::N(value.id.0.to_string()),
    );
    map.insert(ATTR_TAG_NAME.into(), AttributeValue::S(value.name));

    map
}

fn attributes_to_tag(value: &AttributeMap) -> Result<Tag, super::Error> {
    let id = value[ATTR_TAG_ID]
        .as_n()
        .map_err(|_| Error::MalformedObject(ATTR_TAG_ID.into()))?
        .parse::<usize>()?;

    let name = value[ATTR_TAG_NAME]
        .as_s()
        .map_err(|_| Error::MalformedObject(ATTR_TAG_NAME.into()))?
        .clone();

    Ok(Tag {
        id: TagId(id),
        name,
    })
}
