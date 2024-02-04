use super::AttributeMap;
use crate::eventsdb::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::EventData;

const ATTR_EVENT_DATA_NAME: &str = "name";
const ATTR_EVENT_DATA_DESC: &str = "desc";
const ATTR_EVENT_DATA_URL_SHORT: &str = "short_url";
const ATTR_EVENT_DATA_URL_LONG: &str = "long_url";

pub fn eventdata_to_attributes(value: EventData) -> AttributeMap {
    let mut map = AttributeMap::new();

    map.insert(ATTR_EVENT_DATA_NAME.into(), AttributeValue::S(value.name));
    map.insert(
        ATTR_EVENT_DATA_DESC.into(),
        AttributeValue::S(value.description),
    );

    if !value.short_url.is_empty() {
        map.insert(
            ATTR_EVENT_DATA_URL_SHORT.into(),
            AttributeValue::S(value.short_url),
        );
    }

    if let Some(long_url) = value
        .long_url
        .and_then(|url| if url.is_empty() { None } else { Some(url) })
    {
        map.insert(ATTR_EVENT_DATA_URL_LONG.into(), AttributeValue::S(long_url));
    }

    map
}

pub fn attributes_to_eventdata(value: &AttributeMap) -> Result<EventData, super::Error> {
    let name = value[ATTR_EVENT_DATA_NAME]
        .as_s()
        .map_err(|_| Error::MalformedObject(ATTR_EVENT_DATA_NAME.into()))?
        .clone();

    let description = value[ATTR_EVENT_DATA_DESC]
        .as_s()
        .map_err(|_| Error::MalformedObject(ATTR_EVENT_DATA_DESC.into()))?
        .clone();

    let short_url = value
        .get(ATTR_EVENT_DATA_URL_SHORT)
        .and_then(|value| value.as_s().ok().cloned())
        .unwrap_or_default();

    let long_url = value
        .get(ATTR_EVENT_DATA_URL_LONG)
        .and_then(|value| value.as_s().ok().cloned());

    Ok(EventData {
        name,
        description,
        short_url,
        long_url,
    })
}
