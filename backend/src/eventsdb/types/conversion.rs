use super::{ApiEventInfo, AttributeMap};
use crate::eventsdb::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::{EventData, EventState, EventTokens, QuestionItem, States};

const ATTR_EVENT_INFO_LAST_EDIT: &str = "last_edit";
const ATTR_EVENT_INFO_DELETE_TIME: &str = "delete_time";
const ATTR_EVENT_INFO_CREATE_TIME: &str = "create_time";
const ATTR_EVENT_INFO_DELETED: &str = "deleted";
const ATTR_EVENT_INFO_STATE: &str = "state";
const ATTR_EVENT_INFO_TOKENS: &str = "tokens";
const ATTR_EVENT_INFO_ITEMS: &str = "items";
const ATTR_EVENT_INFO_DATA: &str = "data";
const ATTR_EVENT_INFO_PREMIUM: &str = "premium";
const ATTR_EVENT_INFO_MODMAIL: &str = "mod_email";

pub fn event_to_attributes(value: ApiEventInfo) -> AttributeMap {
    let mut map: AttributeMap = vec![
        (
            ATTR_EVENT_INFO_TOKENS.into(),
            AttributeValue::M(tokens_to_attributes(value.tokens)),
        ),
        (
            ATTR_EVENT_INFO_ITEMS.into(),
            AttributeValue::L(questions_to_attributes(value.questions)),
        ),
        (
            ATTR_EVENT_INFO_DATA.into(),
            AttributeValue::M(eventdata_to_attributes(value.data)),
        ),
        (
            ATTR_EVENT_INFO_STATE.into(),
            AttributeValue::N(value.state.to_value().to_string()),
        ),
        (
            ATTR_EVENT_INFO_DELETED.into(),
            AttributeValue::Bool(value.deleted),
        ),
        (
            ATTR_EVENT_INFO_CREATE_TIME.into(),
            AttributeValue::N(value.create_time_unix.to_string()),
        ),
        (
            ATTR_EVENT_INFO_DELETE_TIME.into(),
            AttributeValue::N(value.delete_time_unix.to_string()),
        ),
        (
            ATTR_EVENT_INFO_LAST_EDIT.into(),
            AttributeValue::N(value.last_edit_unix.to_string()),
        ),
    ]
    .into_iter()
    .collect();

    if let Some(premium_order) = value.premium_order {
        map.insert(
            ATTR_EVENT_INFO_PREMIUM.into(),
            AttributeValue::S(premium_order),
        );
    }

    if let Some(mod_email) = value.mod_email {
        map.insert(ATTR_EVENT_INFO_MODMAIL.into(), AttributeValue::S(mod_email));
    }

    map
}

pub fn attributes_to_event(value: &AttributeMap) -> Result<ApiEventInfo, super::Error> {
    let tokens = attributes_to_tokens(
        value[ATTR_EVENT_INFO_TOKENS]
            .as_m()
            .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_TOKENS.into()))?,
    )?;

    let data = attributes_to_eventdata(
        value[ATTR_EVENT_INFO_DATA]
            .as_m()
            .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_DATA.into()))?,
    )?;

    let questions = attributes_to_questions(
        value[ATTR_EVENT_INFO_ITEMS]
            .as_l()
            .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_ITEMS.into()))?,
    )?;

    let last_edit_unix = value[ATTR_EVENT_INFO_LAST_EDIT]
        .as_n()
        .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_LAST_EDIT.into()))?
        .parse::<i64>()?;

    let delete_time_unix = value[ATTR_EVENT_INFO_DELETE_TIME]
        .as_n()
        .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_DELETE_TIME.into()))?
        .parse::<i64>()?;

    let create_time_unix = value[ATTR_EVENT_INFO_CREATE_TIME]
        .as_n()
        .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_CREATE_TIME.into()))?
        .parse::<i64>()?;

    let deleted = value[ATTR_EVENT_INFO_DELETED]
        .as_bool()
        .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_DELETED.into()))?
        .to_owned();

    let premium_order = value
        .get(ATTR_EVENT_INFO_PREMIUM)
        .and_then(|value| value.as_s().ok().cloned());

    let mod_email = value
        .get(ATTR_EVENT_INFO_MODMAIL)
        .and_then(|value| value.as_s().ok().cloned());

    let state = EventState::from_value(
        value[ATTR_EVENT_INFO_STATE]
            .as_n()
            .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_STATE.into()))?
            .parse::<u8>()?,
    )
    .unwrap_or(EventState {
        state: States::Open,
    });

    Ok(ApiEventInfo {
        tokens,
        data,
        create_time_unix,
        delete_time_unix,
        deleted,
        last_edit_unix,
        questions,
        state,
        premium_order,
        mod_email,
    })
}

const ATTR_EVENT_TOKENS_PUBLIC: &str = "pub";
const ATTR_EVENT_TOKENS_PRIVATE: &str = "priv";

fn tokens_to_attributes(value: EventTokens) -> AttributeMap {
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

fn attributes_to_tokens(value: &AttributeMap) -> Result<EventTokens, super::Error> {
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

const ATTR_EVENT_DATA_NAME: &str = "name";
const ATTR_EVENT_DATA_DESC: &str = "desc";
const ATTR_EVENT_DATA_URL_SHORT: &str = "short_url";
const ATTR_EVENT_DATA_URL_LONG: &str = "long_url";

fn eventdata_to_attributes(value: EventData) -> AttributeMap {
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

fn attributes_to_eventdata(value: &AttributeMap) -> Result<EventData, super::Error> {
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

const ATTR_QUESTION_TEXT: &str = "text";
const ATTR_QUESTION_ID: &str = "id";
const ATTR_QUESTION_LIKES: &str = "likes";
const ATTR_QUESTION_CREATED: &str = "created";
const ATTR_QUESTION_ANSWERED: &str = "answered";
const ATTR_QUESTION_HIDDEN: &str = "hidden";

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

    let hidden = value
        .get(ATTR_QUESTION_HIDDEN)
        .and_then(|value| value.as_bool().ok().copied())
        .unwrap_or_default();

    Ok(QuestionItem {
        id,
        likes,
        text,
        hidden,
        answered,
        create_time_unix,
    })
}

fn questions_to_attributes(value: Vec<QuestionItem>) -> Vec<AttributeValue> {
    value
        .into_iter()
        .map(|i| AttributeValue::M(question_to_attributes(i)))
        .collect()
}

fn attributes_to_questions(value: &Vec<AttributeValue>) -> Result<Vec<QuestionItem>, super::Error> {
    let mut list = Vec::with_capacity(value.len());

    for i in value {
        list.push(attributes_to_question(
            i.as_m()
                .map_err(|_| Error::General("question is not a map".into()))?,
        )?);
    }

    Ok(list)
}
