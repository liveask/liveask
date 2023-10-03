use axum::{
    extract::{ws::WebSocket, Path, State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    Json,
};
use tracing::instrument;

use crate::{
    app::SharedApp,
    auth::OptionalUser,
    error::InternalError,
    payment::{
        PaymentCaptureRefundedResource, PaymentCheckoutApprovedResource, PaymentWebhookBase,
    },
    GIT_HASH,
};

async fn socket_handler(ws: WebSocket, id: String, app: SharedApp) {
    app.push_subscriber(ws, id).await;
}

#[instrument(skip(app, ws))]
pub async fn push_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(app): State<SharedApp>,
) -> impl IntoResponse {
    tracing::info!("push subscriber: {}", id);

    ws.on_upgrade(|ws| socket_handler(ws, id, app))
}

#[instrument(skip(app))]
pub async fn editlike_handler(
    Path(id): Path<String>,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::EditLike>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("edit like: {}/{}", payload.question_id, id);

    Ok(Json(app.edit_like(id, payload).await?))
}

#[instrument(skip(app))]
pub async fn addevent_handler(
    State(app): State<SharedApp>,
    Json(payload): Json<shared::AddEvent>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("create event");

    Ok(Json(app.create_event(payload).await?))
}

#[instrument(skip(app))]
pub async fn addquestion_handler(
    Path(id): Path<String>,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::AddQuestion>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("add question: {} in event:  {}", payload.text, id);

    Ok(Json(app.add_question(id, payload).await?))
}

#[instrument(skip(app))]
pub async fn getevent_handler(
    Path(id): Path<String>,
    OptionalUser(user): OptionalUser,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("getevent_handler");

    Ok(Json(app.get_event(id, None, user.is_some()).await?))
}

#[instrument(skip(app))]
pub async fn mod_get_event(
    Path((id, secret)): Path<(String, String)>,
    OptionalUser(user): OptionalUser,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_get_event");

    Ok(Json(app.get_event(id, Some(secret), user.is_some()).await?))
}

#[instrument(skip(app))]
pub async fn mod_delete_event(
    Path((id, secret)): Path<(String, String)>,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_delete_event");

    Ok(Json(app.delete_event(id, secret).await?))
}

#[instrument(skip(app))]
pub async fn mod_premium_upgrade(
    Path((id, secret)): Path<(String, String)>,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_premium_upgrade");

    Ok(Json(app.request_premium_upgrade(id, secret).await?))
}

#[instrument(skip(app))]
pub async fn mod_premium_capture(
    Path((id, order)): Path<(String, String)>,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_premium_capture");

    Ok(Json(app.premium_capture(id, order).await?))
}

#[instrument(skip(app, body))]
pub async fn payment_webhook(
    State(app): State<SharedApp>,
    body: String,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("payment_webhook");

    let base: PaymentWebhookBase = serde_json::from_str(&body)?;

    if base.event_type == "CHECKOUT.ORDER.APPROVED" {
        let resource: PaymentCheckoutApprovedResource = serde_json::from_value(base.resource)?;

        app.payment_webhook(resource.id).await?;
    } else if base.event_type == "PAYMENT.CAPTURE.COMPLETED" {
        tracing::info!(base.id, "payment capture completed: {}", body);
    } else if base.event_type == "PAYMENT.CAPTURE.REFUNDED" {
        let resource: PaymentCaptureRefundedResource = serde_json::from_value(base.resource)?;

        //TODO: make event not-premium again
        tracing::warn!("refund: {:?}", resource);
    } else {
        tracing::warn!("unknown payment hook: {}", body);
    }

    Ok(Html(""))
}

#[instrument(skip(app))]
pub async fn mod_get_question(
    Path((id, secret, question_id)): Path<(String, String, i64)>,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_get_question");

    Ok(Json(app.get_question(id, Some(secret), question_id).await?))
}

#[instrument(skip(app))]
pub async fn get_question(
    Path((id, question_id)): Path<(String, i64)>,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("get_question");

    Ok(Json(app.get_question(id, None, question_id).await?))
}

#[instrument(skip(app))]
pub async fn mod_edit_question(
    Path((id, secret, question_id)): Path<(String, String, i64)>,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::ModQuestion>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_edit_question");

    Ok(Json(
        app.mod_edit_question(id, secret, question_id, payload)
            .await?,
    ))
}

#[instrument(skip(app))]
pub async fn mod_edit_state(
    Path((id, secret)): Path<(String, String)>,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::ModEventState>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_edit_state");

    Ok(Json(app.edit_event_state(id, secret, payload.state).await?))
}

#[instrument(skip(app))]
pub async fn mod_edit_screening(
    Path((id, secret)): Path<(String, String)>,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::ModEditScreening>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_edit_screening");

    Ok(Json(
        app.edit_event_screening(id, secret, payload.screening)
            .await?,
    ))
}

#[instrument]
pub async fn ping_handler() -> Html<&'static str> {
    Html("pong")
}

#[instrument]
pub async fn version_handler() -> Html<&'static str> {
    Html(GIT_HASH)
}

#[instrument]
pub async fn error_handler() -> Html<&'static str> {
    tracing::error!("error handler");
    Html("error!")
}

#[cfg(test)]
mod test_db_conflicts {
    use super::*;
    use crate::eventsdb::{ApiEventInfo, EventEntry, EventsDB};
    use crate::payment::Payment;
    use crate::tracking::Tracking;
    use crate::utils::timestamp_now;
    use crate::viewers::MockViewers;
    use crate::{app::App, pubsub::PubSubInMemory};
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::post,
        Router,
    };
    use pretty_assertions::assert_eq;
    use shared::QuestionItem;
    use std::sync::Arc;
    use tower::util::ServiceExt;
    use tower_http::trace::TraceLayer;

    #[derive(Default)]
    pub struct ConflictDB;
    #[async_trait]
    impl EventsDB for ConflictDB {
        async fn get(&self, key: &str) -> crate::eventsdb::Result<EventEntry> {
            tracing::info!("fake db get: {key}");
            Ok(EventEntry {
                event: ApiEventInfo {
                    questions: vec![QuestionItem {
                        id: 1,
                        ..Default::default()
                    }],
                    create_time_unix: timestamp_now(),
                    ..Default::default()
                },
                version: 1,
                ttl: None,
            })
        }
        async fn put(&self, event: EventEntry) -> crate::eventsdb::Result<()> {
            tracing::info!("fake db put: {}", event.event.tokens.public_token);
            Err(crate::eventsdb::Error::Concurrency)
        }
    }

    fn app() -> Router {
        let app = Arc::new(App::new(
            Arc::new(ConflictDB::default()),
            Arc::new(PubSubInMemory::default()),
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        ));

        Router::new()
            .route("/api/event/editlike/:id", post(editlike_handler))
            .layer(TraceLayer::new_for_http())
            .with_state(app)
    }

    #[tokio::test]
    async fn test_conflicting_database_write() {
        // env_logger::init();

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/event/editlike/test")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&shared::EditLike {
                            like: true,
                            question_id: 1,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}

#[cfg(test)]
mod test_db_item_not_found {
    use super::*;
    use crate::{
        app::App,
        auth,
        eventsdb::{EventEntry, EventsDB},
        payment::Payment,
        pubsub::PubSubInMemory,
        tracking::Tracking,
        viewers::MockViewers,
    };
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::get,
        Router,
    };
    use pretty_assertions::assert_eq;
    use std::sync::Arc;
    use tower::util::ServiceExt;
    use tower_http::trace::TraceLayer;

    #[derive(Default)]
    pub struct ItemNotFoundDB;
    #[async_trait]
    impl EventsDB for ItemNotFoundDB {
        async fn get(&self, _key: &str) -> crate::eventsdb::Result<EventEntry> {
            Err(crate::eventsdb::Error::ItemNotFound)
        }
        async fn put(&self, _event: EventEntry) -> crate::eventsdb::Result<()> {
            Ok(())
        }
    }

    fn app() -> Router {
        let app = Arc::new(App::new(
            Arc::new(ItemNotFoundDB::default()),
            Arc::new(PubSubInMemory::default()),
            Arc::new(MockViewers::new()),
            Arc::new(Payment::default()),
            Tracking::default(),
            String::new(),
        ));

        let (session, auth) = auth::setup_test();

        Router::new()
            .route("/api/event/:id", get(getevent_handler))
            .layer(auth)
            .layer(session)
            .layer(TraceLayer::new_for_http())
            .with_state(app)
    }

    #[tokio::test]
    async fn test_db_item_not_found() {
        // env_logger::init();

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .uri("/api/event/test")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
