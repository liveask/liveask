use super::AttributeMap;
use crate::eventsdb::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::ContextItem;

pub fn contexts_to_attributes(value: Vec<ContextItem>) -> Vec<AttributeValue> {
    value
        .into_iter()
        .map(|i| AttributeValue::M(context_item_to_attributes(i)))
        .collect()
}

fn context_item_to_attributes(value: ContextItem) -> AttributeMap {
    let mut map = AttributeMap::new();

    map.insert("label".into(), AttributeValue::S(value.label));
    map.insert("url".into(), AttributeValue::S(value.url));

    map
}

pub fn attributes_to_contexts(
    value: &Vec<AttributeValue>,
) -> Result<Vec<ContextItem>, super::Error> {
    let mut result = Vec::with_capacity(value.len());

    for e in value {
        let f = e
            .as_m()
            .as_ref()
            .map(|e| attributes_to_context_itemn(e))
            .map_err(|_| Error::MalformedObject(String::from("context")))??;

        result.push(f);
    }

    Ok(result)
}

fn attributes_to_context_itemn(value: &AttributeMap) -> Result<ContextItem, super::Error> {
    let label = value["label"]
        .as_s()
        .map_err(|_| Error::MalformedObject("label".into()))?
        .clone();

    let url = value["url"]
        .as_s()
        .map_err(|_| Error::MalformedObject("url".into()))?
        .clone();

    Ok(ContextItem { label, url })
}
