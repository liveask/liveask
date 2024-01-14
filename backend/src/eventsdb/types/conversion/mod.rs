mod context;
mod eventdata;
mod questions;
mod tags;
mod tokens;

use self::{
    context::{attributes_to_contexts, contexts_to_attributes},
    eventdata::{attributes_to_eventdata, eventdata_to_attributes},
    questions::{attributes_to_questions, questions_to_attributes},
    tags::{attributes_to_eventtags, eventtags_to_attributes},
    tokens::{attributes_to_tokens, tokens_to_attributes},
};

use super::{ApiEventInfo, AttributeMap};
use crate::eventsdb::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::{EventPassword, EventState, EventTags, States};

const ATTR_EVENT_INFO_LAST_EDIT: &str = "last_edit";
const ATTR_EVENT_INFO_DELETE_TIME: &str = "delete_time";
const ATTR_EVENT_INFO_CREATE_TIME: &str = "create_time";
const ATTR_EVENT_INFO_DELETED: &str = "deleted";
const ATTR_EVENT_INFO_DO_SCREENING: &str = "do_screening";
const ATTR_EVENT_INFO_STATE: &str = "state";
const ATTR_EVENT_INFO_TOKENS: &str = "tokens";
const ATTR_EVENT_INFO_ITEMS: &str = "items";
const ATTR_EVENT_INFO_DATA: &str = "data";
const ATTR_EVENT_INFO_PREMIUM: &str = "premium";
const ATTR_EVENT_INFO_PASSWORD: &str = "password";
const ATTR_EVENT_INFO_CONTEXT: &str = "ctx";
const ATTR_EVENT_INFO_TAGS: &str = "tags";

pub fn event_to_attributes(value: ApiEventInfo) -> AttributeMap {
    let vec = vec![
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
            ATTR_EVENT_INFO_DO_SCREENING.into(),
            AttributeValue::Bool(value.do_screening),
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
        (
            ATTR_EVENT_INFO_CONTEXT.into(),
            AttributeValue::L(contexts_to_attributes(value.context)),
        ),
        (
            ATTR_EVENT_INFO_TAGS.into(),
            AttributeValue::M(eventtags_to_attributes(value.tags)),
        ),
    ];
    let mut map: AttributeMap = vec.into_iter().collect();

    if let Some(premium_order) = value.premium_order {
        map.insert(
            ATTR_EVENT_INFO_PREMIUM.into(),
            AttributeValue::S(premium_order),
        );
    }

    if let EventPassword::Enabled(password) = value.password {
        map.insert(ATTR_EVENT_INFO_PASSWORD.into(), AttributeValue::S(password));
    }

    map
}

pub fn attributes_to_event(value: &AttributeMap) -> Result<ApiEventInfo, super::Error> {
    let context = attributes_to_contexts(
        value
            .get(ATTR_EVENT_INFO_CONTEXT)
            .cloned()
            .unwrap_or_else(|| AttributeValue::L(Vec::new()))
            .as_l()
            .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_CONTEXT.into()))?,
    )?;

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

    let do_screening = value
        .get(ATTR_EVENT_INFO_DO_SCREENING)
        .and_then(|val| val.as_bool().ok())
        .copied()
        .unwrap_or_default();

    let premium_order = value
        .get(ATTR_EVENT_INFO_PREMIUM)
        .and_then(|value| value.as_s().ok().cloned());

    let password = value
        .get(ATTR_EVENT_INFO_PASSWORD)
        .and_then(|value| value.as_s().ok().cloned())
        .into();

    let state = EventState::from_value(
        value[ATTR_EVENT_INFO_STATE]
            .as_n()
            .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_STATE.into()))?
            .parse::<u8>()?,
    )
    .unwrap_or(EventState {
        state: States::Open,
    });

    let tags = if let Some(attr) = value.get(ATTR_EVENT_INFO_TAGS) {
        attributes_to_eventtags(
            attr.as_m()
                .map_err(|_| Error::MalformedObject(ATTR_EVENT_INFO_TAGS.into()))?,
        )
    } else {
        EventTags::default()
    };

    Ok(ApiEventInfo {
        tokens,
        data,
        create_time_unix,
        delete_time_unix,
        deleted,
        last_edit_unix,
        questions,
        do_screening,
        state,
        password,
        premium_order,
        context,
        tags,
    })
}
