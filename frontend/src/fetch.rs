#![allow(clippy::future_not_send)]

use gloo_utils::format::JsValueSerdeExt;
use shared::{
    AddEvent, AddQuestion, EditLike, EventData, EventInfo, EventPasswordRequest,
    EventPasswordResponse, EventState, EventUpgrade, GetEventResponse, GetUserInfo, ModEvent,
    ModQuestion, PaymentCapture, QuestionItem, States, UserLogin,
};
use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestCredentials, RequestInit, Response};

/// Something wrong has occurred while fetching an external resource.
#[derive(Debug)]
pub enum FetchError {
    Generic(String),
    JsonError(JsValue),
    SerdeError(serde_json::error::Error),
}
impl Display for FetchError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::JsonError(e) => Debug::fmt(e, f),
            Self::SerdeError(e) => Debug::fmt(e, f),
            Self::Generic(e) => Debug::fmt(e, f),
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

pub async fn fetch_version(base_api: &str) -> Result<String, FetchError> {
    let url = format!("{base_api}/api/version");

    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;
    let resp = JsFuture::from(resp.text()?).await?;

    resp.as_string()
        .ok_or_else(|| FetchError::Generic(String::from("string error")))
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

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.credentials(RequestCredentials::Include);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<GetEventResponse>(&json)?;

    Ok(res)
}

pub async fn mod_state_change(
    base_api: &str,
    id: String,
    secret: String,
    state: States,
) -> Result<EventInfo, FetchError> {
    let res = mod_edit_event(
        base_api,
        id,
        secret,
        ModEvent {
            state: Some(EventState { state }),
            ..Default::default()
        },
    )
    .await?;

    Ok(res)
}

pub async fn mod_edit_event(
    base_api: &str,
    id: String,
    secret: String,
    change: ModEvent,
) -> Result<EventInfo, FetchError> {
    let body = serde_json::to_string(&change)?;
    let body = JsValue::from_str(&body);

    let url = format!("{base_api}/api/mod/event/{id}/{secret}");

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

pub async fn event_set_password(
    base_api: &str,
    id: String,
    pwd: String,
) -> Result<bool, FetchError> {
    let body = serde_json::to_string(&EventPasswordRequest { pwd })?;
    let body = JsValue::from_str(&body);

    let url = format!("{base_api}/api/event/{id}/pwd");

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));
    opts.credentials(RequestCredentials::Include);

    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<EventPasswordResponse>(&json)?;

    Ok(res.ok)
}

pub async fn mod_edit_screening(
    base_api: &str,
    id: String,
    secret: String,
    screening: bool,
) -> Result<EventInfo, FetchError> {
    let res = mod_edit_event(
        base_api,
        id,
        secret,
        ModEvent {
            screening: Some(screening),
            ..Default::default()
        },
    )
    .await?;

    Ok(res)
}

pub async fn mod_upgrade(
    base_api: &str,
    id: String,
    secret: String,
) -> Result<EventUpgrade, FetchError> {
    let url = format!("{base_api}/api/mod/event/upgrade/{id}/{secret}");

    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<EventUpgrade>(&json)?;

    Ok(res)
}

pub async fn mod_premium_capture(
    base_api: &str,
    id: String,
    order_id: String,
) -> Result<PaymentCapture, FetchError> {
    let url = format!("{base_api}/api/mod/event/capture/{id}/{order_id}");

    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<PaymentCapture>(&json)?;

    Ok(res)
}

pub async fn like_question(
    base_api: &str,
    event_id: String,
    question_id: i64,
    like: bool,
) -> Result<QuestionItem, FetchError> {
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
    let res = JsValueSerdeExt::into_serde::<QuestionItem>(&json)?;

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
) -> Result<QuestionItem, FetchError> {
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
    let res = JsValueSerdeExt::into_serde::<QuestionItem>(&json)?;

    Ok(res)
}

pub async fn create_event(
    base_api: &str,
    name: String,
    desc: String,
    email: Option<String>,
) -> Result<EventInfo, FetchError> {
    let body = AddEvent {
        data: EventData {
            name,
            description: desc,
            long_url: None,
            short_url: String::new(),
        },
        test: false,
        moderator_email: email,
    };
    let body = serde_json::to_string(&body)?;
    let body = JsValue::from_str(&body);

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));

    let request = Request::new_with_str_and_init(&format!("{base_api}/api/event/add"), &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<EventInfo>(&json)?;

    Ok(res)
}

pub async fn admin_login(base_api: &str, name: String, pwd_hash: String) -> Result<(), FetchError> {
    let body = UserLogin { name, pwd_hash };
    let body = serde_json::to_string(&body)?;
    let body = JsValue::from_str(&body);

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body));
    opts.credentials(RequestCredentials::Include);

    let request = Request::new_with_str_and_init(&format!("{base_api}/api/admin/login"), &opts)?;
    request.headers().set("content-type", "application/json")?;

    let window = gloo_utils::window();

    let resp = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp.dyn_into()?;

    if resp.ok() {
        Ok(())
    } else {
        Err(FetchError::Generic("request failed".into()))
    }
}

pub async fn fetch_user(base_api: &str) -> Result<GetUserInfo, FetchError> {
    let url = format!("{base_api}/api/admin/user");

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.credentials(RequestCredentials::Include);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    let res = JsValueSerdeExt::into_serde::<GetUserInfo>(&json)?;

    Ok(res)
}

pub async fn admin_logout(base_api: &str) -> Result<(), FetchError> {
    let url = format!("{base_api}/api/admin/logout");

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.credentials(RequestCredentials::Include);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let resp = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp.dyn_into()?;

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

    let opts = {
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts
    };

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = gloo_utils::window();
    let req = window.fetch_with_request(&request);
    let _resp_value = JsFuture::from(req).await?;

    Ok(())
}
