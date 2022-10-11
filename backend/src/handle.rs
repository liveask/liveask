use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
    Json,
};
use shared::{EventData, EventInfo, EventTokens};

pub async fn getevent_handler(Path(_id): Path<String>) -> Result<impl IntoResponse, StatusCode> {
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

pub async fn ping_handler() -> Html<&'static str> {
    Html("pong")
}
