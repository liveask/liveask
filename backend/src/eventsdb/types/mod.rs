mod conversion;

use crate::utils::timestamp_now;
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use shared::{
    ContextItem, EventData, EventFlags, EventInfo, EventPassword, EventState, EventTags,
    EventTokens, QuestionItem,
};
use std::collections::HashMap;

use self::conversion::{attributes_to_event, event_to_attributes};

use super::{event_key, Error};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct ApiEventInfo {
    pub tokens: EventTokens,
    pub data: EventData,
    #[serde(rename = "createTimeUnix")]
    pub create_time_unix: i64,
    #[serde(rename = "deleteTimeUnix")]
    pub delete_time_unix: i64,
    pub deleted: bool,
    #[serde(rename = "lastEditUnix")]
    pub last_edit_unix: i64,
    pub questions: Vec<QuestionItem>,
    #[serde(default)]
    pub do_screening: bool,
    pub state: EventState,
    #[serde(default)]
    pub password: EventPassword,
    pub premium_order: Option<String>,
    #[serde(default)]
    pub context: Vec<ContextItem>,
    #[serde(default)]
    pub tags: EventTags,
}

const LOREM_IPSUM:&str = "Lorem ipsum dolor sit amet. Et adipisci repellendus id dolore molestiae sed quidem ratione! Aut itaque magnam eos corporis dolores ut repudiandae consequuntur et maiores accusantium. 33 quas illum vel cumque quisquam et possimus quaerat et nostrum galisum et similique dolorum quo earum earum et accusantium dignissimos!";

#[allow(clippy::string_slice)]
pub fn mask_string(s: &str) -> &str {
    //Note: as soon as `LOREM_IPSUM` contains utf8 this risks to panic inside utf8 codes
    let text_length = s.len().min(LOREM_IPSUM.len());
    &LOREM_IPSUM[..text_length]
}

impl ApiEventInfo {
    fn is_timed_out(&self) -> bool {
        Self::timestamp_to_datetime(self.create_time_unix).map_or_else(
            || {
                tracing::warn!("timeout calc error: {}", self.create_time_unix);
                true
            },
            |create_date| {
                let age: Duration = Utc::now() - create_date;
                age.num_days() > 7
            },
        )
    }

    pub fn is_timed_out_and_free(&self) -> bool {
        self.premium_order.is_none() && self.is_timed_out()
    }

    pub fn adapt_if_timedout(&mut self) -> bool {
        if self.is_timed_out_and_free() {
            self.mask_data();
            return true;
        }

        false
    }

    pub fn mask_data(&mut self) {
        for q in &mut self.questions {
            q.text = mask_string(&q.text).to_string();
        }
        self.data.description = mask_string(&self.data.description).to_string();
    }

    fn timestamp_to_datetime(timestamp: i64) -> Option<DateTime<Utc>> {
        Utc.timestamp_opt(timestamp, 0).latest()
    }
}

impl From<ApiEventInfo> for EventInfo {
    fn from(val: ApiEventInfo) -> Self {
        let mut flags = EventFlags::empty();

        flags.set(EventFlags::DELETED, val.deleted);
        flags.set(EventFlags::PREMIUM, val.premium_order.is_some());
        flags.set(EventFlags::SCREENING, val.do_screening);
        flags.set(EventFlags::PASSWORD, val.password.is_enabled());

        Self {
            tokens: val.tokens,
            data: val.data,
            create_time_unix: val.create_time_unix,
            delete_time_unix: val.delete_time_unix,
            last_edit_unix: val.last_edit_unix,
            questions: val.questions,
            state: val.state,
            flags,
            context: val.context,
            tags: val.tags,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct EventEntry {
    pub event: ApiEventInfo,
    pub version: usize,
    pub ttl: Option<i64>,
}

impl EventEntry {
    pub const fn new(event: ApiEventInfo, ttl: Option<i64>) -> Self {
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

#[cfg(test)]
mod test_serialization {
    use super::*;
    use pretty_assertions::assert_eq;
    use shared::{EventState, States, Tag};

    #[test]
    #[tracing_test::traced_test]
    fn test_ser_and_de_1() {
        let entry = EventEntry {
            event: ApiEventInfo {
                tokens: EventTokens {
                    public_token: String::from("token1"),
                    moderator_token: None,
                },
                data: EventData {
                    name: String::from("name"),
                    description: String::from("desc"),
                    short_url: String::from(""),
                    long_url: None,
                },
                create_time_unix: 1,
                delete_time_unix: 0,
                deleted: false,
                password: shared::EventPassword::Disabled,
                premium_order: Some(String::from("order")),
                last_edit_unix: 2,
                questions: vec![QuestionItem {
                    id: 0,
                    likes: 2,
                    text: String::from("q"),
                    hidden: false,
                    answered: true,
                    screening: false,
                    create_time_unix: 3,
                    tag: None,
                }],
                do_screening: true,
                state: EventState {
                    state: States::Closed,
                },
                context: Vec::new(),
                tags: EventTags::default(),
            },
            version: 2,
            ttl: None,
        };

        let map: AttributeMap = entry.clone().try_into().unwrap();

        let entry_deserialized: EventEntry = (&map).try_into().unwrap();

        assert_eq!(entry, entry_deserialized);
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_ser_and_de_2() {
        let entry = EventEntry {
            event: ApiEventInfo {
                tokens: EventTokens {
                    public_token: String::from("token1"),
                    moderator_token: Some(String::from("token2")),
                },
                data: EventData {
                    name: String::from("name"),
                    description: String::from("desc"),
                    short_url: String::from(""),
                    long_url: Some(String::from("foo")),
                },
                create_time_unix: 1,
                delete_time_unix: 0,
                deleted: false,
                password: shared::EventPassword::Enabled(String::from("pwd")),
                premium_order: Some(String::from("order")),
                last_edit_unix: 2,
                questions: vec![QuestionItem {
                    id: 0,
                    likes: 2,
                    text: String::from("q"),
                    hidden: false,
                    answered: true,
                    screening: true,
                    create_time_unix: 3,
                    tag: Some(TagId(0)),
                }],
                do_screening: false,
                state: EventState {
                    state: States::Closed,
                },
                context: vec![ContextItem {
                    label: String::new(),
                    url: String::from("foobar"),
                }],
                tags: EventTags {
                    tags: vec![Tag {
                        name: String::from("talk1"),
                        id: TagId(0),
                    }],
                    current_tag: Some(TagId(0)),
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
