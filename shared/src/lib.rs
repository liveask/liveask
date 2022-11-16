mod validation;

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
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
    #[serde(rename = "maxLikes")]
    pub max_likes: i32,
    pub name: String,
    pub description: String,
    #[serde(rename = "shortUrl")]
    pub short_url: String,
    #[serde(rename = "longUrl")]
    pub long_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Item {
    pub id: i64,
    pub likes: i32,
    pub text: String,
    pub hidden: bool,
    pub answered: bool,
    #[serde(rename = "createTimeUnix")]
    pub create_time_unix: i64,
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
    //TODO: is this still needed in the new FE?
    #[serde(rename = "createTimeUTC")]
    pub create_time_utc: String,
    pub questions: Vec<Item>,
    pub state: EventState,
}

impl EventInfo {
    #[must_use]
    pub fn get_question(&self, id: i64) -> Option<Item> {
        self.questions.iter().find(|i| i.id == id).cloned()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddEvent {
    #[serde(rename = "eventData")]
    pub data: EventData,
    #[serde(rename = "moderatorEmail")]
    pub moderator_email: String,
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
}
