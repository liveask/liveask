#![allow(clippy::future_not_send)]

use gloo_net::http::Request;
use shared::{
    AddEvent, AddQuestion, EditLike, EventData, EventInfo, EventPasswordRequest,
    EventPasswordResponse, EventUpgradeResponse, GetEventResponse, GetUserInfo, ModEvent,
    ModQuestion, PaymentCapture, QuestionItem, UserLogin,
};
use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};
use wasm_bindgen::JsValue;
use web_sys::RequestCredentials;

//TODO: switch to `thiserror`
/// Something wrong has occurred while fetching an external resource.
#[derive(Debug)]
pub enum FetchError {
    Generic(String),
    JsonError(JsValue),
    SerdeError(serde_json::error::Error),
    Gloo(gloo_net::Error),
}
impl Display for FetchError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::JsonError(e) => Debug::fmt(e, f),
            Self::SerdeError(e) => Debug::fmt(e, f),
            Self::Generic(e) => Debug::fmt(e, f),
            Self::Gloo(e) => Debug::fmt(e, f),
        }
    }
}
impl Error for FetchError {}

impl From<JsValue> for FetchError {
    fn from(v: JsValue) -> Self {
        Self::JsonError(v)
    }
}
impl From<serde_json::error::Error> for FetchError {
    fn from(v: serde_json::error::Error) -> Self {
        Self::SerdeError(v)
    }
}
impl From<gloo_net::Error> for FetchError {
    fn from(v: gloo_net::Error) -> Self {
        Self::Gloo(v)
    }
}

fn set_content_type_json(request: &Request) {
    request.headers().set("content-type", "application/json");
}

pub async fn fetch_version(base_api: &str) -> Result<String, FetchError> {
    let url = format!("{base_api}/api/version");

    Ok(Request::get(&url).send().await?.text().await?)
}

pub async fn fetch_event(
    base_api: &str,
    id: String,
    secret: Option<String>,
) -> Result<GetEventResponse, FetchError> {
    let url = secret.map_or_else(
        || format!("{base_api}/api/event/{id}"),
        |secret| format!("{base_api}/api/mod/event/{id}/{secret}"),
    );

    Ok(Request::get(&url)
        .credentials(RequestCredentials::Include)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn mod_edit_event(
    base_api: &str,
    id: String,
    secret: String,
    change: ModEvent,
) -> Result<EventInfo, FetchError> {
    let url = format!("{base_api}/api/mod/event/{id}/{secret}");

    let body = JsValue::from_str(&serde_json::to_string(&change)?);

    let request = Request::post(&url).body(body)?;
    set_content_type_json(&request);
    Ok(request.send().await?.json().await?)
}

pub async fn event_set_password(
    base_api: &str,
    id: String,
    pwd: String,
) -> Result<bool, FetchError> {
    let url = format!("{base_api}/api/event/{id}/pwd");

    let body = JsValue::from_str(&serde_json::to_string(&EventPasswordRequest { pwd })?);

    let request = Request::post(&url)
        .credentials(RequestCredentials::Include)
        .body(body)?;
    set_content_type_json(&request);
    let response: EventPasswordResponse = request.send().await?.json().await?;
    Ok(response.ok)
}

pub async fn mod_upgrade(
    base_api: &str,
    id: String,
    secret: String,
) -> Result<EventUpgradeResponse, FetchError> {
    let url = format!("{base_api}/api/mod/event/upgrade/{id}/{secret}");

    Ok(Request::get(&url)
        .credentials(RequestCredentials::Include)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn mod_premium_capture(
    base_api: &str,
    id: String,
    order_id: String,
) -> Result<PaymentCapture, FetchError> {
    let url = format!("{base_api}/api/mod/event/capture/{id}/{order_id}");

    Ok(Request::get(&url).send().await?.json().await?)
}

pub async fn like_question(
    base_api: &str,
    event_id: String,
    question_id: i64,
    like: bool,
) -> Result<QuestionItem, FetchError> {
    let url = format!("{base_api}/api/event/editlike/{event_id}");

    let body = JsValue::from_str(&serde_json::to_string(&EditLike { question_id, like })?);

    let request = Request::post(&url).body(body)?;
    set_content_type_json(&request);
    Ok(request.send().await?.json().await?)
}

pub async fn mod_question(
    base_api: &str,
    event_id: String,
    event_secret: String,
    question_id: i64,
    modify: ModQuestion,
) -> Result<(), FetchError> {
    let url =
        format!("{base_api}/api/mod/event/questionmod/{event_id}/{event_secret}/{question_id}");

    let body = JsValue::from_str(&serde_json::to_string(&modify)?);

    let request = Request::post(&url).body(body)?;
    set_content_type_json(&request);
    let _ = request.send().await?;
    Ok(())
}

pub async fn add_question(
    base_api: &str,
    event_id: String,
    text: String,
) -> Result<QuestionItem, FetchError> {
    let url = format!("{base_api}/api/event/addquestion/{event_id}");

    let body = JsValue::from_str(&serde_json::to_string(&AddQuestion { text })?);

    let request = Request::post(&url).body(body)?;
    set_content_type_json(&request);
    Ok(request.send().await?.json().await?)
}

pub async fn create_event(
    base_api: &str,
    name: String,
    desc: String,
    email: Option<String>,
) -> Result<EventInfo, FetchError> {
    let url = format!("{base_api}/api/event/add");

    let body = JsValue::from_str(&serde_json::to_string(&AddEvent {
        data: EventData {
            name,
            description: desc,
            long_url: None,
            short_url: String::new(),
        },
        test: false,
        moderator_email: email,
    })?);

    let request = Request::post(&url).body(body)?;
    set_content_type_json(&request);
    Ok(request.send().await?.json().await?)
}

pub async fn admin_login(base_api: &str, name: String, pwd_hash: String) -> Result<(), FetchError> {
    let url = format!("{base_api}/api/admin/login");
    let body = JsValue::from_str(&serde_json::to_string(&UserLogin { name, pwd_hash })?);

    let request = Request::post(&url)
        .credentials(RequestCredentials::Include)
        .body(body)?;
    set_content_type_json(&request);
    let resp = request.send().await?;

    if resp.ok() {
        Ok(())
    } else {
        Err(FetchError::Generic("request failed".into()))
    }
}

pub async fn fetch_user(base_api: &str) -> Result<GetUserInfo, FetchError> {
    let url = format!("{base_api}/api/admin/user");

    Ok(Request::get(&url)
        .credentials(RequestCredentials::Include)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn admin_logout(base_api: &str) -> Result<(), FetchError> {
    let url = format!("{base_api}/api/admin/logout");

    let resp = Request::get(&url)
        .credentials(RequestCredentials::Include)
        .send()
        .await?;

    if resp.ok() {
        Ok(())
    } else {
        Err(FetchError::Generic("request failed".into()))
    }
}

pub async fn delete_event(
    base_api: &str,
    event_id: String,
    secret: String,
) -> Result<(), FetchError> {
    let url = format!("{base_api}/api/mod/event/delete/{event_id}/{secret}");

    let resp = Request::get(&url).send().await?;

    if resp.ok() {
        Ok(())
    } else {
        Err(FetchError::Generic("request failed".into()))
    }
}
