#![deny(clippy::unwrap_used)]

mod app;
mod env;
mod eventsdb;
mod handle;
mod mail;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::{Credentials, Endpoint};
use axum::{
    http::Uri,
    routing::{get, post},
    Extension, Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{app::App, eventsdb::DynamoEventsDB, handle::push_handler};

#[cfg(not(debug_assertions))]
fn setup_cors() -> CorsLayer {
    CorsLayer::new()
}

#[cfg(debug_assertions)]
fn setup_cors() -> CorsLayer {
    tracing::info!("cors setup");
    CorsLayer::very_permissive()
}

fn use_local_db() -> bool {
    std::env::var(env::ENV_DB_LOCAL).is_ok()
}

async fn dynamo_client() -> aws_sdk_dynamodb::Client {
    use aws_sdk_dynamodb::Client;

    let region_provider = RegionProviderChain::default_provider().or_else("us-west-1");
    let config = aws_config::from_env().region(region_provider);

    let config = if use_local_db() {
        let url = if let Ok(env) = std::env::var(env::ENV_DB_URL) {
            env
        } else {
            "http://localhost:8000".into()
        };

        tracing::info!("ddb url: {}", url);

        config
            .credentials_provider(Credentials::new("aid", "sid", None, None, "local"))
            .endpoint_resolver(Endpoint::immutable(Uri::from_static(
                "http://localhost:8000",
            )))
    } else {
        config
    };

    let config = config.load().await;

    Client::new(&config)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = App::new(Arc::new(DynamoEventsDB::new(dynamo_client().await).await?));

    #[rustfmt::skip]
    let mod_routes = Router::new()
        .route("/:id/:secret", get(handle::mod_get_event))
        .route("/delete/:id/:secret", get(handle::mod_delete_event))
        .route("/question/:id/:secret/:question_id", get(handle::mod_get_question))
        .route("/questionmod/:id/:secret/:question_id", post(handle::mod_edit_question))
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
        .await?;

    Ok(())
}
