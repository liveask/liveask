use axum::{
    extract::{ws::WebSocket, Path, State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    Json,
};
use axum_sessions::extractors::{ReadableSession, WritableSession};
use shared::EventPasswordResponse;
use tracing::instrument;

use crate::{
    app::SharedApp,
    auth::OptionalUser,
    error::InternalError,
    payment::{
        PaymentCaptureDeclinedResource, PaymentCaptureRefundedResource,
        PaymentCheckoutApprovedResource, PaymentWebhookBase,
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

#[instrument(skip(app, session))]
pub async fn getevent_handler(
    Path(id): Path<String>,
    OptionalUser(user): OptionalUser,
    session: ReadableSession,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("getevent_handler");

    let password = session.get_raw("pwd");

    Ok(Json(
        app.get_event(id, None, user.is_some(), password).await?,
    ))
}

#[instrument(skip(app, session))]
pub async fn set_event_password(
    Path(id): Path<String>,
    mut session: WritableSession,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::EventPasswordRequest>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("set_event_password");

    let response = EventPasswordResponse {
        ok: app.check_event_password(id, &payload.pwd).await?,
    };

    if response.ok {
        session.insert_raw("pwd", payload.pwd);
    }

    Ok(Json(response))
}

#[instrument(skip(app))]
pub async fn mod_get_event(
    Path((id, secret)): Path<(String, String)>,
    OptionalUser(user): OptionalUser,
    State(app): State<SharedApp>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_get_event");

    //TODO: special response type for mods to add more info
    Ok(Json(
        app.get_event(id, Some(secret), user.is_some(), None)
            .await?,
    ))
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
    } else if base.event_type == "PAYMENT.CAPTURE.DECLINED" {
        let resource: PaymentCaptureDeclinedResource = serde_json::from_value(base.resource)?;
        tracing::warn!("payment declined: {:?}", resource);
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

    Ok(Json(
        app.mod_edit_event(
            id,
            secret,
            shared::ModEvent {
                state: Some(payload.state),
                ..Default::default()
            },
        )
        .await?,
    ))
}

#[instrument(skip(app))]
pub async fn mod_edit_event(
    Path((id, secret)): Path<(String, String)>,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::ModEvent>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_edit_state");

    Ok(Json(app.mod_edit_event(id, secret, payload).await?))
}

#[instrument(skip(app))]
pub async fn mod_edit_screening(
    Path((id, secret)): Path<(String, String)>,
    State(app): State<SharedApp>,
    Json(payload): Json<shared::ModEditScreening>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("mod_edit_screening");

    Ok(Json(
        app.mod_edit_event(
            id,
            secret,
            shared::ModEvent {
                screening: Some(payload.screening),
                ..Default::default()
            },
        )
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
    #[tracing_test::traced_test]
    async fn test_conflicting_database_write() {
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
        eventsdb::{EventEntry, EventsDB, InMemoryEventsDB},
        payment::Payment,
        pubsub::PubSubInMemory,
        tracking::Tracking,
        viewers::MockViewers,
    };
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use axum_test::{TestServer, TestServerConfig};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use shared::{EventResponseFlags, TEST_EVENT_DESC, TEST_EVENT_NAME};
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

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_db_item_not_found() {
        let router = {
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
                .with_state(app.clone())
        };

        let response = router
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

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_event_fetch() {
        let (app, router) = {
            let events = Arc::new(InMemoryEventsDB::default());
            let app = Arc::new(App::new(
                events.clone(),
                Arc::new(PubSubInMemory::default()),
                Arc::new(MockViewers::new()),
                Arc::new(Payment::default()),
                Tracking::default(),
                String::new(),
            ));
            let (session, auth) = auth::setup_test();
            let router = Router::new()
                .route("/api/event/:id", get(getevent_handler))
                .layer(auth)
                .layer(session)
                .layer(TraceLayer::new_for_http())
                .with_state(app.clone());
            (app, router)
        };

        let e = app
            .create_event(shared::AddEvent {
                data: shared::EventData {
                    name: TEST_EVENT_NAME.into(),
                    description: TEST_EVENT_DESC.into(),
                    ..Default::default()
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        let response = router
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/api/event/{}", e.tokens.public_token))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_event_pwd() {
        let (app, router) = {
            let events = Arc::new(InMemoryEventsDB::default());
            let app = Arc::new(App::new(
                events.clone(),
                Arc::new(PubSubInMemory::default()),
                Arc::new(MockViewers::new()),
                Arc::new(Payment::default()),
                Tracking::default(),
                String::new(),
            ));
            let (session, auth) = auth::setup_test();
            let router = Router::new()
                .route("/api/event/:id", get(getevent_handler))
                .route("/api/event/:id/pwd", post(set_event_password))
                .layer(auth)
                .layer(session)
                .layer(TraceLayer::new_for_http())
                .with_state(app.clone());
            (app, router)
        };

        let e = app
            .create_event(shared::AddEvent {
                data: shared::EventData {
                    name: TEST_EVENT_NAME.into(),
                    description: TEST_EVENT_DESC.into(),
                    ..Default::default()
                },
                moderator_email: None,
                test: false,
            })
            .await
            .unwrap();

        app.mod_edit_event(
            e.tokens.public_token.clone(),
            e.tokens.moderator_token.clone().unwrap(),
            shared::ModEvent {
                password: Some(shared::EventPassword::Enabled("pwd".into())),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let server = TestServer::new_with_config(
            router,
            TestServerConfig::builder()
                .default_content_type("application/json")
                .save_cookies()
                .expect_success_by_default()
                .build(),
        )
        .unwrap();

        let response: shared::GetEventResponse = server
            .get(&format!("/api/event/{}", e.tokens.public_token))
            .await
            .json();

        assert!(response.flags.contains(EventResponseFlags::WRONG_PASSWORD));

        let res: shared::EventPasswordResponse = server
            .post(&format!("/api/event/{}/pwd", e.tokens.public_token))
            .json(&json!({
                "pwd": "pw",
            }))
            .await
            .json();
        assert!(!res.ok);

        let res: shared::EventPasswordResponse = server
            .post(&format!("/api/event/{}/pwd", e.tokens.public_token))
            .json(&json!({
                "pwd": "pwd",
            }))
            .await
            .json();
        assert!(res.ok);

        let response: shared::GetEventResponse = server
            .get(&format!("/api/event/{}", e.tokens.public_token))
            .await
            .json();

        assert!(!response.flags.contains(EventResponseFlags::WRONG_PASSWORD));
    }
}
