mod app;
mod handle;
mod mail;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{app::App, handle::push_handler};

#[cfg(not(debug_assertions))]
fn setup_cors() -> CorsLayer {
    CorsLayer::new()
}

#[cfg(debug_assertions)]
fn setup_cors() -> CorsLayer {
    tracing::info!("cors setup");

    use axum::http::HeaderValue;
    use tower_http::cors::Any;

    CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods(Any)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = App::default();

    #[rustfmt::skip]
    let mod_routes = Router::new()
        .route("/:id/:secret", get(handle::mod_get_event))
        .route("/delete/:id/:secret", get(handle::mod_delete_event))
        .route("/question/:id/:secret/:question_id", get(handle::mod_get_question))
        .route("/questionmod/:id/:secret/:question_id", get(handle::mod_edit_question))
        .route("/state/:id/:secret", post(handle::mod_edit_state));

    #[rustfmt::skip]
    let router = Router::new()
        .route("/api/ping", get(handle::ping_handler))
        .route("/api/addevent", post(handle::addevent_handler))
        .route("/api/event/editlike/:id", post(handle::editlike_handler))
        .route("/api/event/addquestion/:id", post(handle::addquestion_handler))
        .route("/api/event/question/:id/:question_id", get(handle::get_question))
        .route("/api/event/:id", get(handle::getevent_handler))
        .route("/push/:id", get(push_handler))
        .nest("/api/mod/event",mod_routes)
        .layer(TraceLayer::new_for_http())
        .layer(setup_cors())
        .layer(Extension(app));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8090));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
