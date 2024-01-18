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

#[cfg(test)]
mod test {
    use super::*;
    use serde_dynamo::{from_item, to_item};
    use shared::ContextItem;

    #[test]
    #[tracing_test::traced_test]
    fn test_serde_dynamo_compare() {
        let original = ContextItem {
            label: String::from("label"),
            url: String::from("url"),
        };

        let serde: AttributeMap = to_item(original.clone()).unwrap();

        let manual = context_item_to_attributes(original.clone());

        assert_eq!(serde, manual);

        let from_serde: ContextItem = from_item(serde.clone()).unwrap();

        assert_eq!(from_serde, original);

        let from_manual = attributes_to_context_itemn(&serde).unwrap();

        assert_eq!(from_manual, original);

        assert_eq!(from_serde, from_manual);
    }
}
