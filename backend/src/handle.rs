use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
    Json,
};
use shared::{EventData, EventInfo, EventTokens, Item};
use tracing::instrument;
use ulid::Ulid;

#[instrument]
pub async fn editlike_handler(
    Json(payload): Json<shared::EditLike>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("edit like: {}/{}", payload.question_id, id);

    let res = Item {
        answered: false,
        create_time_unix: 0,
        hidden: false,
        id: 0,
        likes: 1,
        text: String::new(),
    };

    Ok(Json(res))
}

#[instrument]
pub async fn addevent_handler(
    Json(payload): Json<shared::AddEvent>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("create event: {}", payload.data.name);

    let ev = EventInfo {
        create_time_unix: 0,
        delete_time_unix: 0,
        last_edit_unix: 0,
        create_time_utc: String::new(),
        deleted: false,
        questions: Vec::new(),
        data: payload.data,
        tokens: EventTokens {
            public_token: Ulid::new().to_string(),
            moderator_token: Some(Ulid::new().to_string()),
        },
    };

    Ok(Json(ev))
}

#[instrument]
pub async fn addquestion_handler(
    Json(payload): Json<shared::AddQuestion>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("add question: {} in event:  {}", payload.text, id);

    let res = Item {
        answered: false,
        create_time_unix: 0,
        hidden: false,
        id: 0,
        likes: 1,
        text: payload.text,
    };

    Ok(Json(res))
}

#[instrument]
pub async fn getevent_handler(Path(id): Path<String>) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("get event:  {}", id);

    let ev = EventInfo {
        create_time_unix: 0,
        delete_time_unix: 0,
        last_edit_unix: 0,
        create_time_utc: String::new(),
        deleted: false,
        questions: Vec::new(),
        data: EventData {
            max_likes: 10,
            name: String::from("foo"),
            description: String::from("bar"),
            short_url: String::new(),
            long_url: None,
        },
        tokens: EventTokens {
            public_token: String::new(),
            moderator_token: None,
        },
    };

    Ok(Json(ev))
}

#[instrument]
pub async fn get_modevent_handler(
    Path(id): Path<String>,
    Path(secret): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("get mod event:  {}/{}", id, secret);

    let ev = EventInfo {
        create_time_unix: 0,
        delete_time_unix: 0,
        last_edit_unix: 0,
        create_time_utc: String::new(),
        deleted: false,
        questions: Vec::new(),
        data: EventData {
            max_likes: 10,
            name: String::from("foo"),
            description: String::from("bar"),
            short_url: String::new(),
            long_url: None,
        },
        tokens: EventTokens {
            public_token: String::new(),
            moderator_token: None,
        },
    };

    Ok(Json(ev))
}

#[instrument]
pub async fn ping_handler() -> Html<&'static str> {
    Html("pong")
}
