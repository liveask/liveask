use axum::{
    extract::{ws::WebSocket, Path, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension, Json,
};
use tracing::instrument;

use crate::app::App;

//TODO: not sure why we need this
async fn socket_handler(ws: WebSocket, id: String, app: App) {
    app.push_subscriber(ws, id).await
}

#[instrument(skip(app, ws))]
pub async fn push_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    Extension(app): Extension<App>,
) -> impl IntoResponse {
    tracing::info!("push subscriber: {}", id);

    ws.on_upgrade(|ws| socket_handler(ws, id, app))
}

#[instrument(skip(app))]
pub async fn editlike_handler(
    Json(payload): Json<shared::EditLike>,
    Path(id): Path<String>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("edit like: {}/{}", payload.question_id, id);

    match app.edit_like(id, payload).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn addevent_handler(
    Json(payload): Json<shared::AddEvent>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("create event: {}", payload.data.name);

    match app.create_event(payload).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn addquestion_handler(
    Json(payload): Json<shared::AddQuestion>,
    Path(id): Path<String>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("add question: {} in event:  {}", payload.text, id);

    match app.add_question(id, payload).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn getevent_handler(
    Path(id): Path<String>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("get event:  {}", id);

    match app.get_event(id, None).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn mod_get_event(
    Path((id, secret)): Path<(String, String)>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    match app.get_event(id, Some(secret)).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn mod_delete_event(
    Path((id, secret)): Path<(String, String)>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    match app.delete_event(id, secret).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn mod_get_question(
    Path((id, secret, question_id)): Path<(String, String, i64)>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    match app.get_question(id, Some(secret), question_id).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn get_question(
    Path((id, question_id)): Path<(String, i64)>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    match app.get_question(id, None, question_id).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn mod_edit_question(
    Path((id, secret, question_id)): Path<(String, String, i64)>,
    Json(payload): Json<shared::ModQuestion>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    match app
        .mod_edit_question(id, secret, question_id, payload)
        .await
    {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(skip(app))]
pub async fn mod_edit_state(
    Path((id, secret)): Path<(String, String)>,
    Json(payload): Json<shared::EventState>,
    Extension(app): Extension<App>,
) -> Result<impl IntoResponse, StatusCode> {
    match app.edit_event_state(id, secret, payload).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("{}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument]
pub async fn ping_handler() -> Html<&'static str> {
    Html("pong")
}
