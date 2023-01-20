mod validation;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::str::FromStr;

pub use validation::{CreateEventErrors, ValidationError};

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
    pub mail: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct QuestionItem {
    pub id: i64,
    pub likes: i32,
    pub text: String,
    pub hidden: bool,
    pub answered: bool,
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
    pub deleted: bool,
    #[serde(rename = "lastEditUnix")]
    pub last_edit_unix: i64,
    pub questions: Vec<QuestionItem>,
    pub state: EventState,
    #[serde(default)]
    pub premium: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct GetEventResponse {
    pub info: EventInfo,
    #[serde(default)]
    pub timed_out: bool,
    pub viewers: i64,
    //TODO: not needed if client becomes aware of its user role
    #[serde(default)]
    pub admin: bool,
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
        matches!(self.info.state.state, States::Closed) || self.timed_out
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddEvent {
    #[serde(rename = "eventData")]
    pub data: EventData,
    #[serde(rename = "moderatorEmail")]
    pub moderator_email: String,
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct ModEventState {
    pub state: EventState,
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

    pub fn to_value(&self) -> u8 {
        match self.state {
            States::Open => 0,
            States::VotingOnly => 1,
            States::Closed => 2,
        }
    }

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
