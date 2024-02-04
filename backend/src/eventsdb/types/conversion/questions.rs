use super::AttributeMap;
use crate::eventsdb::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::{QuestionItem, TagId};

pub fn questions_to_attributes(value: Vec<QuestionItem>) -> Vec<AttributeValue> {
    value
        .into_iter()
        .map(|i| AttributeValue::M(question_to_attributes(i)))
        .collect()
}

pub fn attributes_to_questions(
    value: &Vec<AttributeValue>,
) -> Result<Vec<QuestionItem>, super::Error> {
    let mut list = Vec::with_capacity(value.len());

    for i in value {
        list.push(attributes_to_question(
            i.as_m()
                .map_err(|_| Error::General("question is not a map".into()))?,
        )?);
    }

    Ok(list)
}

const ATTR_QUESTION_TEXT: &str = "text";
const ATTR_QUESTION_ID: &str = "id";
const ATTR_QUESTION_LIKES: &str = "likes";
const ATTR_QUESTION_CREATED: &str = "created";
const ATTR_QUESTION_ANSWERED: &str = "answered";
const ATTR_QUESTION_SCREENING: &str = "screening";
const ATTR_QUESTION_HIDDEN: &str = "hidden";
const ATTR_QUESTION_TAG: &str = "tag";

fn question_to_attributes(value: QuestionItem) -> AttributeMap {
    let mut map = AttributeMap::new();

    map.insert(
        ATTR_QUESTION_ID.into(),
        AttributeValue::N(value.id.to_string()),
    );
    map.insert(ATTR_QUESTION_TEXT.into(), AttributeValue::S(value.text));
    map.insert(
        ATTR_QUESTION_LIKES.into(),
        AttributeValue::N(value.likes.to_string()),
    );
    map.insert(
        ATTR_QUESTION_CREATED.into(),
        AttributeValue::N(value.create_time_unix.to_string()),
    );

    if value.answered {
        map.insert(ATTR_QUESTION_ANSWERED.into(), AttributeValue::Bool(true));
    }
    if value.hidden {
        map.insert(ATTR_QUESTION_HIDDEN.into(), AttributeValue::Bool(true));
    }
    if value.screening {
        map.insert(ATTR_QUESTION_SCREENING.into(), AttributeValue::Bool(true));
    }
    if let Some(tag) = value.tag {
        map.insert(
            ATTR_QUESTION_TAG.into(),
            AttributeValue::N(tag.0.to_string()),
        );
    }

    map
}

fn attributes_to_question(value: &AttributeMap) -> Result<QuestionItem, super::Error> {
    let id = value[ATTR_QUESTION_ID]
        .as_n()
        .map_err(|_| Error::MalformedObject(ATTR_QUESTION_ID.into()))?
        .parse::<i64>()?;

    let text = value[ATTR_QUESTION_TEXT]
        .as_s()
        .map_err(|_| Error::MalformedObject(ATTR_QUESTION_TEXT.into()))?
        .clone();

    let likes = value[ATTR_QUESTION_LIKES]
        .as_n()
        .map_err(|_| Error::MalformedObject(ATTR_QUESTION_LIKES.into()))?
        .parse::<i32>()?;

    let create_time_unix = value[ATTR_QUESTION_CREATED]
        .as_n()
        .map_err(|_| Error::MalformedObject(ATTR_QUESTION_CREATED.into()))?
        .parse::<i64>()?;

    let answered = value
        .get(ATTR_QUESTION_ANSWERED)
        .and_then(|value| value.as_bool().ok().copied())
        .unwrap_or_default();

    let screening = value
        .get(ATTR_QUESTION_SCREENING)
        .and_then(|value| value.as_bool().ok().copied())
        .unwrap_or_default();

    let hidden = value
        .get(ATTR_QUESTION_HIDDEN)
        .and_then(|value| value.as_bool().ok().copied())
        .unwrap_or_default();

    let tag = value
        .get(ATTR_QUESTION_TAG)
        .and_then(|v| v.as_n().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .map(TagId);

    Ok(QuestionItem {
        id,
        likes,
        text,
        hidden,
        answered,
        screening,
        create_time_unix,
        tag,
    })
}
