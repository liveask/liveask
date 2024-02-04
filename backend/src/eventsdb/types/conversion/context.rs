use super::AttributeMap;
use crate::eventsdb::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use serde_dynamo::{from_item, to_item};
use shared::ContextItem;

pub fn contexts_to_attributes(value: Vec<ContextItem>) -> Vec<AttributeValue> {
    value
        .into_iter()
        .filter_map(|i| to_item(i).ok())
        .map(AttributeValue::M)
        .collect()
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
    let v = value.clone();

    Ok(from_item(v)?)
}
