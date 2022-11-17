mod app;
mod env;
mod eventsdb;
mod handle;
mod mail;
mod pubsub;
mod redis_pool;
mod utils;

use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::{Credentials, Endpoint};
use axum::{
    http::Uri,
    routing::{get, post},
    Extension, Router,
};
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    app::App,
    eventsdb::DynamoEventsDB,
    handle::push_handler,
    pubsub::PubSubRedis,
    redis_pool::{create_pool, ping_test_redis},
};

pub const GIT_HASH: &str = env!("GIT_HASH");
pub const GIT_BRANCH: &str = env!("GIT_BRANCH");

#[cfg(not(debug_assertions))]
#[must_use]
pub fn is_debug() -> bool {
    false
}

#[cfg(debug_assertions)]
#[must_use]
pub fn is_debug() -> bool {
    true
}

fn setup_cors() -> CorsLayer {
    if use_relaxed_cors() {
        tracing::info!("cors setup: very_permissive");
        CorsLayer::very_permissive()
    } else {
        tracing::info!("cors setup: default");
        CorsLayer::new()
    }
}

fn use_local_db() -> bool {
    std::env::var(env::ENV_DB_LOCAL).is_ok()
}

fn use_relaxed_cors() -> bool {
    std::env::var(env::ENV_RELAX_CORS)
        .map(|var| var == "1")
        .unwrap_or_default()
}

fn get_redis_url() -> String {
    if let Ok(env) = std::env::var(env::ENV_REDIS_URL) {
        env
    } else {
        "redis://localhost:6379".into()
    }
}

async fn dynamo_client() -> Result<aws_sdk_dynamodb::Client> {
    use aws_sdk_dynamodb::Client;

    let region_provider = RegionProviderChain::default_provider().or_else("us-west-1");
    let config = aws_config::from_env().region(region_provider);

    let config = if use_local_db() {
        let url = if let Ok(env) = std::env::var(env::ENV_DB_URL) {
            env
        } else {
            "http://localhost:8000".into()
        };

        tracing::info!("ddb local url: {url}");

        config
            .credentials_provider(Credentials::new("aid", "sid", None, None, "local"))
            .endpoint_resolver(Endpoint::immutable(Uri::from_str(&url)?))
    } else {
        config
    };

    let config = config.load().await;

    Ok(Client::new(&config))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,liveask_server=debug,tower_http=debug".into());

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(log_level.clone()))
        .with(tracing_subscriber::fmt::layer().with_ansi(is_debug()))
        .init();

    let redis_url = get_redis_url();

    tracing::info!(
        target: "server-starting",
        git = %GIT_HASH,
        log_level,
        redis_url,
    );

    let redis_pool = create_pool(&redis_url)?;
    ping_test_redis(&redis_pool).await?;

    let pubsub = Arc::new(PubSubRedis::new(redis_pool, redis_url).await);

    let app = Arc::new(App::new(
        Arc::new(DynamoEventsDB::new(dynamo_client().await?, use_local_db()).await?),
        pubsub.clone(),
    ));

    pubsub.set_receiver(app.clone()).await;

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
