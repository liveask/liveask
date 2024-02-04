use super::AttributeMap;
use crate::eventsdb::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::EventTokens;

const ATTR_EVENT_TOKENS_PUBLIC: &str = "pub";
const ATTR_EVENT_TOKENS_PRIVATE: &str = "priv";

pub fn tokens_to_attributes(value: EventTokens) -> AttributeMap {
    let mut map = AttributeMap::new();

    map.insert(
        ATTR_EVENT_TOKENS_PUBLIC.into(),
        AttributeValue::S(value.public_token),
    );

    if let Some(moderator_token) = value.moderator_token {
        map.insert(
            ATTR_EVENT_TOKENS_PRIVATE.into(),
            AttributeValue::S(moderator_token),
        );
    }

    map
}

pub fn attributes_to_tokens(value: &AttributeMap) -> Result<EventTokens, super::Error> {
    let public_token = value[ATTR_EVENT_TOKENS_PUBLIC]
        .as_s()
        .map_err(|_| Error::MalformedObject(ATTR_EVENT_TOKENS_PUBLIC.into()))?
        .clone();

    let moderator_token = value
        .get(ATTR_EVENT_TOKENS_PRIVATE)
        .and_then(|value| value.as_s().ok().cloned());

    Ok(EventTokens {
        public_token,
        moderator_token,
    })
}
