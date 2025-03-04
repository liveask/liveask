mod flags;
mod validation;

use std::{str::FromStr, time::Duration};

use chrono::{DateTime, TimeZone as _, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub use flags::{EventFlags, EventResponseFlags};
pub use validation::{
    add_question::{AddQuestionError, AddQuestionValidation},
    context_validation::{ContextLabelError, ContextUrlError, ContextValidation},
    create_event::{CreateEventError, CreateEventValidation},
    pwd_validation::{PasswordError, PasswordValidation},
    tag_validation::{TagError, TagValidation},
    ValidationState,
};

//TODO: validate in unittest against validator
pub const TEST_VALID_QUESTION: &str = "1 2 3fourfive";
pub const TEST_EVENT_DESC: &str = "minimum desc length possible!!";
pub const TEST_EVENT_NAME: &str = "min name";

pub const MAX_TAGS: usize = 15;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct EventTokens {
    #[serde(rename = "publicToken")]
    pub public_token: String,
    #[serde(rename = "moderatorToken")]
    pub moderator_token: Option<String>,
}

impl EventTokens {
    #[must_use]
    pub fn is_mod(&self) -> bool {
        self.moderator_token.as_ref().is_some_and(|t| !t.is_empty())
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
pub struct Color(pub String);

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
pub struct EventData {
    pub name: String,
    pub description: String,
    #[serde(rename = "shortUrl")]
    pub short_url: String,
    #[serde(rename = "longUrl")]
    pub long_url: Option<String>,
    #[serde(default)]
    pub color: Option<Color>,
}

#[derive(Serialize, Deserialize, Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TagId(pub usize);

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct QuestionItem {
    pub id: i64,
    pub likes: i32,
    pub text: String,
    pub hidden: bool,
    pub answered: bool,
    #[serde(default)]
    pub screening: bool,
    #[serde(rename = "createTimeUnix")]
    pub create_time_unix: i64,
    #[serde(default)]
    pub tag: Option<TagId>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum EventUpgradeResponse {
    Redirect { url: String },
    AdminUpgrade,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct ContextItem {
    pub label: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct PaymentCapture {
    pub order_captured: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct Tag {
    pub name: String,
    pub id: TagId,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct EventTags {
    pub current_tag: Option<TagId>,
    pub tags: Vec<Tag>,
}
impl EventTags {
    #[must_use]
    pub fn get_current_tag_label(&self) -> Option<String> {
        self.current_tag
            .as_ref()
            .and_then(|current| self.tags.iter().find(|tag| tag.id == *current))
            .map(|tag| tag.name.clone())
    }

    /// returns `false` if max tags is reached and a new tag would have to be added
    pub fn set_or_add_tag(&mut self, tag: &str) -> bool {
        let tag = tag.to_lowercase();

        if let Some(i) = self.tags.iter().find(|e| *e.name == tag) {
            self.current_tag = Some(i.id);
        } else {
            if self.tags.len() >= MAX_TAGS {
                return false;
            }

            let id = TagId(self.tags.len());
            self.tags.push(Tag { name: tag, id });
            self.current_tag = Some(id);
        }

        true
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct EventInfo {
    pub tokens: EventTokens,
    pub data: EventData,
    #[serde(rename = "createTimeUnix")]
    pub create_time_unix: i64,
    #[serde(rename = "deleteTimeUnix")]
    pub delete_time_unix: i64,
    #[serde(rename = "lastEditUnix")]
    pub last_edit_unix: i64,
    pub questions: Vec<QuestionItem>,
    pub state: EventState,
    #[serde(default)]
    pub flags: EventFlags,
    #[serde(default)]
    pub context: Vec<ContextItem>,
    #[serde(default)]
    pub tags: EventTags,
}

impl EventInfo {
    #[must_use]
    pub fn deleted(id: String) -> Self {
        Self {
            tokens: EventTokens {
                public_token: id,
                ..Default::default()
            },
            flags: EventFlags::DELETED,
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn is_premium(&self) -> bool {
        self.flags.contains(EventFlags::PREMIUM)
    }
    #[must_use]
    pub const fn is_deleted(&self) -> bool {
        self.flags.contains(EventFlags::DELETED)
    }
    #[must_use]
    pub const fn is_screening(&self) -> bool {
        self.flags.contains(EventFlags::SCREENING)
    }
    #[must_use]
    pub const fn has_password(&self) -> bool {
        self.flags.contains(EventFlags::PASSWORD)
    }

    #[must_use]
    pub fn timestamp_to_datetime(timestamp: i64) -> Option<DateTime<Utc>> {
        Utc.timestamp_opt(timestamp, 0).latest()
    }

    #[must_use]
    pub fn age_in_seconds(create_time_unix: i64) -> i64 {
        Self::timestamp_to_datetime(create_time_unix)
            .map(|create| Utc::now() - create)
            .map(|age| age.num_seconds())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn during_first_day(create_time_unix: i64) -> bool {
        Self::age_in_seconds(create_time_unix) <= (60 * 60 * 24)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct ModInfo {
    pub pwd: EventPassword,
    pub private_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct GetEventResponse {
    //TODO: remove mod token from inside here
    pub info: EventInfo,
    pub viewers: i64,
    //TODO: not needed if client becomes aware of its user role via header
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub masked: bool,
    #[serde(default)]
    pub flags: EventResponseFlags,
    pub mod_info: Option<ModInfo>,
}

impl GetEventResponse {
    #[must_use]
    pub fn get_question(&self, id: i64) -> Option<QuestionItem> {
        self.info.questions.iter().find(|i| i.id == id).cloned()
    }

    #[must_use]
    pub fn get_likes(&self) -> i32 {
        self.info.questions.iter().map(|q| q.likes).sum()
    }

    #[must_use]
    pub const fn is_closed(&self) -> bool {
        matches!(self.info.state.state, States::Closed) || self.is_timed_out()
    }

    #[must_use]
    pub const fn is_deleted(&self) -> bool {
        self.info.is_deleted()
    }

    #[must_use]
    pub const fn is_timed_out(&self) -> bool {
        self.flags.contains(EventResponseFlags::TIMED_OUT)
    }

    #[must_use]
    pub const fn is_wrong_pwd(&self) -> bool {
        self.flags.contains(EventResponseFlags::WRONG_PASSWORD)
    }

    #[must_use]
    pub fn deleted(id: String) -> Self {
        Self {
            info: EventInfo::deleted(id),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn any_questions(&self) -> bool {
        !self.info.questions.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddEvent {
    #[serde(rename = "eventData")]
    pub data: EventData,
    #[serde(rename = "moderatorEmail", default)]
    pub moderator_email: Option<String>,
    pub test: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EditLike {
    #[serde(rename = "questionid")]
    pub question_id: i64,
    pub like: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddQuestion {
    pub text: String,
    #[serde(default)]
    pub tag: Option<TagId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModQuestion {
    pub hide: bool,
    pub answered: bool,
    pub screened: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ModRequestPremiumContext {
    Regular,
    ColorPicker,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModRequestPremium {
    pub context: ModRequestPremiumContext,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum States {
    #[default]
    Open = 0,
    VotingOnly = 1,
    Closed = 2,
}

impl FromStr for States {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::Open),
            "1" => Ok(Self::VotingOnly),
            "2" => Ok(Self::Closed),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum EventPassword {
    Disabled,
    Enabled(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum CurrentTag {
    Disabled,
    Enabled(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum EditContextLink {
    Disabled,
    Enabled(ContextItem),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct EditColor(pub String);

impl CurrentTag {
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled(_))
    }
}

impl Default for EventPassword {
    fn default() -> Self {
        Self::Disabled
    }
}

impl From<Option<String>> for EventPassword {
    fn from(value: Option<String>) -> Self {
        value
            .as_ref()
            .map_or(Self::Disabled, |v| Self::Enabled(v.clone()))
    }
}

impl EventPassword {
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled(_))
    }

    #[must_use]
    pub fn matches(&self, v: &Option<String>) -> bool {
        if v.is_none() && !self.is_enabled() {
            return true;
        } else if let Some(v) = v {
            if let Self::Enabled(pwd) = self {
                return pwd == v;
            }
        }

        false
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct EditMetaData {
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct ModEvent {
    pub current_tag: Option<CurrentTag>,
    pub password: Option<EventPassword>,
    pub state: Option<EventState>,
    pub meta: Option<EditMetaData>,
    pub screening: Option<bool>,
    pub context: Option<EditContextLink>,
    pub color: Option<EditColor>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct EventState {
    pub state: States,
}

impl EventState {
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, States::Open)
    }

    #[must_use]
    pub const fn is_vote_only(&self) -> bool {
        matches!(self.state, States::VotingOnly)
    }

    #[must_use]
    pub const fn is_closed(&self) -> bool {
        matches!(self.state, States::Closed)
    }

    #[must_use]
    pub const fn to_value(&self) -> u8 {
        match self.state {
            States::Open => 0,
            States::VotingOnly => 1,
            States::Closed => 2,
        }
    }

    #[must_use]
    pub fn from_value(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self {
                state: States::Open,
            },
            1 => Self {
                state: States::VotingOnly,
            },
            2 => Self {
                state: States::Closed,
            },
            _ => None?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserLogin {
    pub name: String,
    pub pwd_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    pub name: String,
    pub expires: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetUserInfo {
    pub user: Option<UserInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct EventPasswordRequest {
    pub pwd: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct EventPasswordResponse {
    pub ok: bool,
}
