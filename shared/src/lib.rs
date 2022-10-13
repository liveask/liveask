mod validation;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
pub use validation::{CreateEventErrors, ValidationError};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
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
    #[serde(rename = "createTimeUTC")]
    pub create_time_utc: String,
    pub questions: Vec<Item>,
    pub state: EventState,
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
#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum States {
    Open = 0,
    VotingOnly = 1,
    Closed = 2,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct EventState {
    pub state: States,
}
