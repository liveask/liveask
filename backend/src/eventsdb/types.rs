use std::collections::HashMap;

use crate::utils::{format_timestamp, timestamp_now};
use aws_sdk_dynamodb::model::AttributeValue;
use shared::{EventData, EventInfo, EventState, EventTokens, QuestionItem, States};

use super::{event_key, Error};

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct EventEntry {
    pub event: EventInfo,
    pub version: usize,
    pub ttl: Option<i64>,
}

impl EventEntry {
    pub const fn new(event: EventInfo, ttl: Option<i64>) -> Self {
        Self {
            event,
            version: 0,
            ttl,
        }
    }

    pub fn bump(&mut self) {
        self.version += 1;
        self.event.last_edit_unix = timestamp_now();
    }
}

pub type AttributeMap = HashMap<std::string::String, AttributeValue>;

const CURRENT_FORMAT: usize = 1;

impl TryFrom<&AttributeMap> for EventEntry {
    type Error = super::Error;

    fn try_from(value: &AttributeMap) -> Result<Self, Error> {
        let version = value["v"]
            .as_n()
            .map_err(|_| Error::General("malformed event: `v`".into()))?
            .parse::<usize>()?;

        let event = value["event"]
            .as_m()
            .map_err(|_| Error::MalformedObject("event".into()))?;

        let ttl = value
            .get("ttl")
            .and_then(|ttl| ttl.as_n().ok())
            .and_then(|ttl| ttl.parse::<i64>().ok());

        let event = attributes_to_event(event)?;

        Ok(Self {
            event,
            version,
            ttl,
        })
    }
}

impl From<EventEntry> for AttributeMap {
    fn from(value: EventEntry) -> Self {
        let mut map = Self::new();
        let event_key = event_key(&value.event.tokens.public_token);

        let event_av = event_to_attributes(value.event);
        let version_av = AttributeValue::N(value.version.to_string());
        let format_av = AttributeValue::N(CURRENT_FORMAT.to_string());
        let key_av = AttributeValue::S(event_key);

        map.insert("key".into(), key_av);
        map.insert("format".into(), format_av);
        map.insert("v".into(), version_av);
        map.insert("event".into(), AttributeValue::M(event_av));

        if let Some(ttl) = value.ttl {
            map.insert("ttl".into(), AttributeValue::N(ttl.to_string()));
        }

        map
    }
}

const ATTR_EVENT_INFO_LAST_EDIT: &str = "last_edit";
const ATTR_EVENT_INFO_DELETE_TIME: &str = "delete_time";
const ATTR_EVENT_INFO_CREATE_TIME: &str = "create_time";
const ATTR_EVENT_INFO_DELETED: &str = "deleted";
const ATTR_EVENT_INFO_STATE: &str = "state";
const ATTR_EVENT_INFO_TOKENS: &str = "tokens";
const ATTR_EVENT_INFO_ITEMS: &str = "items";
const ATTR_EVENT_INFO_DATA: &str = "data";

fn event_to_attributes(value: EventInfo) -> AttributeMap {
    let map: AttributeMap = vec![
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

    map
}

fn attributes_to_event(value: &AttributeMap) -> Result<EventInfo, super::Error> {
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

    let state = EventState::from_value(
        value[ATTR_EVENT_INFO_STATE]
            .as_n()
            .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_STATE.into()))?
            .parse::<u8>()?,
    )
    .unwrap_or(EventState {
        state: States::Open,
    });

    Ok(EventInfo {
        last_edit_unix,
        delete_time_unix,
        create_time_unix,
        deleted,
        state,
        tokens,
        data,
        questions,
        create_time_utc: format_timestamp(create_time_unix),
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
const ATTR_EVENT_DATA_MAIL: &str = "mail";

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

    if let Some(mail) = value
        .mail
        .and_then(|url| if url.is_empty() { None } else { Some(url) })
    {
        map.insert(ATTR_EVENT_DATA_MAIL.into(), AttributeValue::S(mail));
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

    let mail = value
        .get(ATTR_EVENT_DATA_MAIL)
        .and_then(|value| value.as_s().ok().cloned());

    Ok(EventData {
        name,
        description,
        short_url,
        long_url,
        mail,
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

#[cfg(test)]
mod test_serialization {
    use super::*;
    use pretty_assertions::assert_eq;
    use shared::{EventState, States};

    #[test]
    fn test_ser_and_de_1() {
        // env_logger::init();

        let entry = EventEntry {
            event: EventInfo {
                tokens: EventTokens {
                    public_token: String::from("token1"),
                    moderator_token: None,
                },
                data: EventData {
                    name: String::from("name"),
                    description: String::from("desc"),
                    short_url: String::from(""),
                    long_url: None,
                    mail: None,
                },
                create_time_unix: 1,
                delete_time_unix: 0,
                deleted: false,
                last_edit_unix: 2,
                create_time_utc: String::from("19700101T000001"),
                questions: vec![QuestionItem {
                    id: 0,
                    likes: 2,
                    text: String::from("q"),
                    hidden: false,
                    answered: true,
                    create_time_unix: 3,
                }],
                state: EventState {
                    state: States::Closed,
                },
            },
            version: 2,
            ttl: None,
        };

        let map: AttributeMap = entry.clone().try_into().unwrap();

        let entry_deserialized: EventEntry = (&map).try_into().unwrap();

        assert_eq!(entry, entry_deserialized);
    }

    #[test]
    fn test_ser_and_de_2() {
        // env_logger::init();

        let entry = EventEntry {
            event: EventInfo {
                tokens: EventTokens {
                    public_token: String::from("token1"),
                    moderator_token: Some(String::from("token2")),
                },
                data: EventData {
                    name: String::from("name"),
                    description: String::from("desc"),
                    short_url: String::from(""),
                    long_url: Some(String::from("foo")),
                    mail: Some(String::from("mail")),
                },
                create_time_unix: 1,
                delete_time_unix: 0,
                deleted: false,
                last_edit_unix: 2,
                create_time_utc: String::from("19700101T000001"),
                questions: vec![QuestionItem {
                    id: 0,
                    likes: 2,
                    text: String::from("q"),
                    hidden: false,
                    answered: true,
                    create_time_unix: 3,
                }],
                state: EventState {
                    state: States::Closed,
                },
            },
            version: 2,
            ttl: Some(12345),
        };

        let map: AttributeMap = entry.clone().try_into().unwrap();

        let entry_deserialized: EventEntry = (&map).try_into().unwrap();

        assert_eq!(entry, entry_deserialized);
    }
}
