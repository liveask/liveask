mod handle;
mod mail;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(debug_assertions))]
fn setup_cors() -> CorsLayer {
    CorsLayer::new()
}

#[cfg(debug_assertions)]
fn setup_cors() -> CorsLayer {
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

    let app = Router::new()
        .route("/ping", get(handle::ping_handler))
        .route("/api/event/:id", get(handle::getevent_handler))
        .layer(TraceLayer::new_for_http())
        .layer(setup_cors());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8090));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
