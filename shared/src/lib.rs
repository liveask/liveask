mod flags;
mod validation;

use std::{str::FromStr, time::Duration};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub use flags::{EventFlags, EventResponseFlags};
pub use validation::{
    add_question::{AddQuestionError, AddQuestionValidation},
    create_event::{CreateEventError, CreateEventValidation},
    pwd_validation::{PasswordError, PasswordValidation},
    ValidationState,
};

//TODO: validate in unittest against validator
pub const TEST_VALID_QUESTION: &str = "1 2 3fourfive";
pub const TEST_EVENT_DESC: &str = "minimum desc length possible!!";
pub const TEST_EVENT_NAME: &str = "min name";

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct EventTokens {
    #[serde(rename = "publicToken")]
    pub public_token: String,
    #[serde(rename = "moderatorToken")]
    pub moderator_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
pub struct EventData {
    pub name: String,
    pub description: String,
    #[serde(rename = "shortUrl")]
    pub short_url: String,
    #[serde(rename = "longUrl")]
    pub long_url: Option<String>,
}

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
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct EventUpgrade {
    pub url: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct PaymentCapture {
    pub order_captured: bool,
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModQuestion {
    pub hide: bool,
    pub answered: bool,
    pub screened: bool,
}

///
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

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct ModEvent {
    pub password: Option<EventPassword>,
    pub state: Option<EventState>,
    pub description: Option<String>,
    pub screening: Option<bool>,
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
