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
    clippy::expect_used,
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
mod auth;
mod env;
mod error;
mod eventsdb;
mod handle;
mod mail;
mod payment;
mod pubsub;
mod redis_pool;
mod signals;
mod utils;
mod viewers;

use async_redis_session::RedisSessionStore;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::config::Credentials;
use axum::{
    http::header,
    routing::{get, post},
    Router,
};
use sentry::integrations::{
    tower::{NewSentryLayer, SentryHttpLayer},
    tracing::EventFilter,
};
use std::{iter::once, net::SocketAddr, sync::Arc};
use tower_http::{
    cors::CorsLayer, sensitive_headers::SetSensitiveRequestHeadersLayer, trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    app::App,
    auth::{admin_user_handler, login_handler, logout_handler},
    env::session_secret,
    error::Result,
    eventsdb::DynamoEventsDB,
    handle::push_handler,
    payment::Payment,
    pubsub::PubSubRedis,
    redis_pool::{create_pool, ping_test_redis},
    viewers::RedisViewers,
};

pub const GIT_HASH: &str = env!("VERGEN_GIT_SHA");
pub const GIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");

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
        CorsLayer::very_permissive().allow_credentials(true)
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

fn base_url() -> String {
    std::env::var(env::ENV_BASE_URL).unwrap_or_else(|_| "https://www.live-ask.com".into())
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
            .endpoint_url(&url)
    } else {
        config
    };

    let config = config.load().await;

    Ok(Client::new(&config))
}

#[allow(clippy::too_many_lines)]
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

    let base_url = base_url();

    tracing::info!(
        git= %GIT_HASH,
        env= prod_env,
        is_prod= is_prod(),
        log_level,
        redis_url,
        base_url,
        "server-starting",
    );

    let redis_pool = create_pool(&redis_url)?;
    ping_test_redis(&redis_pool).await?;

    let payment = Arc::new(Payment::new(
        std::env::var(env::ENV_PAYPAL_ID).unwrap_or_default(),
        std::env::var(env::ENV_PAYPAL_SECRET).unwrap_or_default(),
        //only use sandbox on non-prod
        !is_prod(),
    )?);

    if let Err(e) = payment.authenticate().await {
        tracing::error!("payment auth error: {}", e);
    }

    let pubsub = Arc::new(PubSubRedis::new(redis_pool.clone(), redis_url.clone()));
    let viewers = Arc::new(RedisViewers::new(redis_pool));

    let eventsdb = Arc::new(DynamoEventsDB::new(dynamo_client().await?, use_local_db()).await?);
    let app = Arc::new(App::new(
        eventsdb,
        pubsub.clone(),
        viewers,
        payment,
        base_url,
    ));

    pubsub.set_receiver(app.clone()).await;

    let secret = session_secret()
        .ok_or_else(|| error::InternalError::General(String::from("invalid session secret")))?;

    let (session_layer, auth_layer) = auth::setup(
        secret.as_ref(),
        RedisSessionStore::new(redis_url)?.with_prefix("session/"),
    );

    let admin_routes = Router::new()
        .route("/user", get(admin_user_handler))
        .route("/login", post(login_handler))
        .route("/logout", get(logout_handler));

    let event_routes = Router::new()
        .route("/add", post(handle::addevent_handler))
        .route("/editlike/:id", post(handle::editlike_handler))
        .route("/addquestion/:id", post(handle::addquestion_handler))
        .route("/question/:id/:question_id", get(handle::get_question))
        .route("/:id", get(handle::getevent_handler));

    #[rustfmt::skip]
    let mod_routes = Router::new()
        .route("/:id/:secret", get(handle::mod_get_event))
        .route("/upgrade/:id/:secret", get(handle::mod_premium_upgrade))
        .route("/capture/:id/:order", get(handle::mod_premium_capture))
        .route("/delete/:id/:secret", get(handle::mod_delete_event))
        .route("/question/:id/:secret/:question_id", get(handle::mod_get_question))
        .route("/questionmod/:id/:secret/:question_id", post(handle::mod_edit_question))
        .route("/screening/:id/:secret", post(handle::mod_edit_screening))
        .route("/state/:id/:secret", post(handle::mod_edit_state));

    #[rustfmt::skip]
    let router = Router::new()
        .route("/api/ping", get(handle::ping_handler))
        .route("/api/version", get(handle::version_handler))
        .route("/api/panic", get(handle::panic_handler))
        .route("/api/error", get(handle::error_handler))
        .route("/api/payment/webhook", post(handle::payment_webhook))
        .route("/push/:id", get(push_handler))
        .nest("/api/event", event_routes)
        .nest("/api/mod/event", mod_routes)
        .nest("/api/admin", admin_routes)
        .layer(auth_layer)
        .layer(session_layer)
        .layer(SetSensitiveRequestHeadersLayer::new(once(header::COOKIE)))
        .layer(SentryHttpLayer::with_transaction())
        .layer(NewSentryLayer::new_from_top())
        .layer(TraceLayer::new_for_http())
        .layer(setup_cors())
        .with_state(Arc::clone(&app));

    let addr = SocketAddr::from(([0, 0, 0, 0], get_port()));

    tracing::info!("listening on {}", addr);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    signals::create_term_signal_handler(tx);

    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    let graceful = server.with_graceful_shutdown(async {
        rx.await.ok();
    });

    if let Err(e) = graceful.await {
        tracing::error!("server error: {}", e);
    }

    if let Err(e) = app.shutdown().await {
        tracing::error!("app shutdown error: {}", e);
    }

    Ok(())
}
