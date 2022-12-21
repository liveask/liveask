#![deny(
    warnings,
    unused_imports,
    unused_must_use,
    unused_variables,
    unused_mut,
    dead_code
)]
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::dbg_macro,
    clippy::unwrap_used,
    clippy::panic,
    clippy::needless_update,
    clippy::match_like_matches_macro,
    clippy::from_over_into,
    clippy::useless_conversion,
    clippy::float_cmp_const,
    clippy::lossy_float_literal,
    clippy::string_to_string,
    clippy::unneeded_field_pattern,
    clippy::verbose_file_reads
)]
#![allow(clippy::module_name_repetitions)]
//TODO: get rid of having to allow this
#![allow(clippy::result_large_err)]
mod app;
mod env;
mod error;
mod eventsdb;
mod handle;
mod mail;
mod payment;
mod pubsub;
mod redis_pool;
mod utils;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::{Credentials, Endpoint};
use axum::{
    http::Uri,
    routing::{get, post},
    Router,
};
use sentry::integrations::{
    tower::{NewSentryLayer, SentryHttpLayer},
    tracing::EventFilter,
};
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    app::App,
    error::Result,
    eventsdb::DynamoEventsDB,
    handle::push_handler,
    payment::Payment,
    pubsub::PubSubRedis,
    redis_pool::{create_pool, ping_test_redis},
};

pub const GIT_HASH: &str = env!("GIT_HASH");
pub const GIT_BRANCH: &str = env!("GIT_BRANCH");

#[cfg(not(debug_assertions))]
#[must_use]
pub const fn is_debug() -> bool {
    false
}

#[cfg(debug_assertions)]
#[must_use]
pub const fn is_debug() -> bool {
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

fn production_env() -> String {
    std::env::var(env::ENV_ENV).unwrap_or_else(|_| String::from("local"))
}

fn is_prod() -> bool {
    production_env() == "prod"
}

fn use_relaxed_cors() -> bool {
    std::env::var(env::ENV_RELAX_CORS)
        .map(|var| var == "1")
        .unwrap_or_default()
}

fn get_port() -> u16 {
    std::env::var(env::ENV_PORT)
        .ok()
        .and_then(|var| var.parse().ok())
        .unwrap_or(8090)
}

fn get_redis_url() -> String {
    std::env::var(env::ENV_REDIS_URL).map_or_else(|_| "redis://localhost:6379".into(), |env| env)
}

async fn dynamo_client() -> Result<aws_sdk_dynamodb::Client> {
    use aws_sdk_dynamodb::Client;

    let region_provider = RegionProviderChain::default_provider().or_else("us-west-1");
    let config = aws_config::from_env().region(region_provider);

    let config = if use_local_db() {
        let url = std::env::var(env::ENV_DB_URL)
            .map_or_else(|_| "http://localhost:8000".into(), |env| env);

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
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,liveask_server=debug,tower_http=debug".into());

    let prod_env = production_env();

    let _guard = sentry::init((
        std::env::var(env::ENV_SENTRY_DSN).unwrap_or_default(),
        sentry::ClientOptions {
            release: Some(GIT_HASH.into()),
            attach_stacktrace: true,
            traces_sample_rate: if is_debug() { 1.0 } else { 0.01 },
            environment: Some(prod_env.clone().into()),
            ..Default::default()
        },
    ));

    let sentry_layer = if is_prod() {
        sentry::integrations::tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR | &tracing::Level::WARN => EventFilter::Event,
            _ => EventFilter::Ignore,
        })
    } else {
        sentry::integrations::tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::ERROR => EventFilter::Event,
            _ => EventFilter::Ignore,
        })
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(log_level.clone()))
        .with(tracing_subscriber::fmt::layer().with_ansi(is_debug()))
        .with(sentry_layer)
        .init();

    let redis_url = get_redis_url();

    tracing::info!(
        git= %GIT_HASH,
        env= prod_env,
        is_prod= is_prod(),
        log_level,
        redis_url,
        "server-starting",
    );

    let redis_pool = create_pool(&redis_url)?;
    ping_test_redis(&redis_pool).await?;

    let payment = Arc::new(Payment::new(
        std::env::var(env::ENV_PAYPAL_ID).unwrap_or_default(),
        std::env::var(env::ENV_PAYPAL_SECRET).unwrap_or_default(),
        //TODO: derive from env
        true,
    ));

    if let Err(e) = payment.authenticate().await {
        tracing::error!("payment auth error: {}", e);
    }

    let pubsub = Arc::new(PubSubRedis::new(redis_pool, redis_url).await);

    let eventsdb = Arc::new(DynamoEventsDB::new(dynamo_client().await?, use_local_db()).await?);
    let app = Arc::new(App::new(eventsdb, pubsub.clone(), payment));

    pubsub.set_receiver(app.clone()).await;

    #[rustfmt::skip]
    let mod_routes = Router::new()
        .route("/:id/:secret", get(handle::mod_get_event))
        .route("/upgrade/:id/:secret", get(handle::mod_premium_upgrade))
        .route("/delete/:id/:secret", get(handle::mod_delete_event))
        .route("/question/:id/:secret/:question_id", get(handle::mod_get_question))
        .route("/questionmod/:id/:secret/:question_id", post(handle::mod_edit_question))
        .route("/state/:id/:secret", post(handle::mod_edit_state));

    #[rustfmt::skip]
    let router = Router::new()
        .route("/api/ping", get(handle::ping_handler))
        .route("/api/panic", get(handle::panic_handler))
        .route("/api/addevent", post(handle::addevent_handler))
        .route("/api/payment/webhook", post(handle::payment_webhook))
        .route("/api/event/editlike/:id", post(handle::editlike_handler))
        .route("/api/event/addquestion/:id", post(handle::addquestion_handler))
        .route("/api/event/question/:id/:question_id", get(handle::get_question))
        .route("/api/event/:id", get(handle::getevent_handler))
        .route("/push/:id", get(push_handler))
        .nest("/api/mod/event",mod_routes)
        .layer(SentryHttpLayer::with_transaction())
        .layer(NewSentryLayer::new_from_top())
        .layer(TraceLayer::new_for_http())
        .layer(setup_cors())
        .with_state(app);

    let addr = SocketAddr::from(([0, 0, 0, 0], get_port()));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
