#![allow(clippy::future_not_send)]

use gloo_utils::format::JsValueSerdeExt;
use shared::{
    AddEvent, AddQuestion, EditLike, EventData, EventInfo, EventState, Item, ModEventState,
    ModQuestion, States,
};
use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Something wrong has occurred while fetching an external resource.
#[derive(Debug)]
pub enum FetchError {
    JsonError(JsValue),
    SerdeError(serde_json::error::Error),
}
impl Display for FetchError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::JsonError(e) => Debug::fmt(e, f),
            Self::SerdeError(e) => Debug::fmt(e, f),
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

pub async fn fetch_event(
    base_api: &str,
    id: String,
    secret: Option<String>,
) -> Result<EventInfo, FetchError> {
    let url = secret.map_or_else(
        || format!("{base_api}/api/event/{id}"),
        |secret| format!("{base_api}/api/mod/event/{id}/{secret}"),
    );

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<EventInfo>(&json)?;

    Ok(res)
}

pub async fn mod_state_change(
    base_api: &str,
    id: String,
    secret: String,
    state: States,
) -> Result<EventInfo, FetchError> {
    let body = ModEventState {
        state: EventState { state },
    };
    let body = serde_json::to_string(&body)?;
    let body = JsValue::from_str(&body);

    let url = format!("{base_api}/api/mod/event/state/{id}/{secret}");

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));

    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<EventInfo>(&json)?;

    Ok(res)
}

pub async fn like_question(
    base_api: &str,
    event_id: String,
    question_id: i64,
    like: bool,
) -> Result<Item, FetchError> {
    let body = EditLike { question_id, like };
    let body = serde_json::to_string(&body)?;
    let body = JsValue::from_str(&body);

    let url = format!("{base_api}/api/event/editlike/{event_id}");

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));

    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<Item>(&json)?;

    Ok(res)
}

pub async fn mod_question(
    base_api: &str,
    event_id: String,
    event_secret: String,
    question_id: i64,
    modify: ModQuestion,
) -> Result<(), FetchError> {
    let body = serde_json::to_string(&modify)?;
    let body = JsValue::from_str(&body);

    let url =
        format!("{base_api}/api/mod/event/questionmod/{event_id}/{event_secret}/{question_id}");

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));

    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();
    let _resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    Ok(())
}

pub async fn add_question(
    base_api: &str,
    event_id: String,
    text: String,
) -> Result<Item, FetchError> {
    let body = AddQuestion { text };
    let body = serde_json::to_string(&body)?;
    let body = JsValue::from_str(&body);

    let url = format!("{base_api}/api/event/addquestion/{event_id}");

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));

    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<Item>(&json)?;

    Ok(res)
}

pub async fn create_event(
    base_api: &str,
    name: String,
    desc: String,
    email: String,
) -> Result<EventInfo, FetchError> {
    let body = AddEvent {
        data: EventData {
            name,
            description: desc,
            max_likes: 0,
            long_url: None,
            short_url: String::new(),
        },
        moderator_email: email,
    };
    let body = serde_json::to_string(&body)?;
    let body = JsValue::from_str(&body);

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));

    let request = Request::new_with_str_and_init(&format!("{base_api}/api/addevent"), &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<EventInfo>(&json)?;

    Ok(res)
}

pub async fn delete_event(
    base_api: &str,
    event_id: String,
    secret: String,
) -> Result<(), FetchError> {
    let url = format!("{base_api}/api/mod/event/delete/{event_id}/{secret}");

    let opts = {
        let mut opts = RequestInit::new();
        opts.method("GET").mode(RequestMode::Cors);
        opts
    };

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let req = window.fetch_with_request(&request);
    let _resp_value = JsFuture::from(req).await?;

    Ok(())
}
