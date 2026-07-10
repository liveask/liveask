#![forbid(unsafe_code)]

mod app;
mod auth;
mod ecs_task_id;
mod env;
mod error;
mod eventsdb;
mod handle;
mod mail;
mod payment;
mod pubsub;
mod redis_pool;
mod ses;
mod signals;
mod stripe_webhooks;
mod tracking;
mod utils;
mod viewers;

use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::config::Credentials;
use axum::{
    Extension, Router,
    http::{HeaderValue, Method, header},
    routing::{get, post},
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
    ecs_task_id::server_id,
    env::session_secret,
    error::Result,
    eventsdb::DynamoEventsDB,
    handle::{push_handler, subscription_handler, subscription_url_handler},
    payment::Payment,
    pubsub::PubSubRedis,
    redis_pool::{create_pool, ping_test_redis},
    tracking::Tracking,
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
        // The frontend is served from a different origin than the API
        // (e.g. www.live-ask.com -> prod.www.live-ask.com) and requests carry auth/pwd cookies,
        // so we must allow the site origin explicitly with credentials -- a wildcard origin is
        // invalid for credentialed requests, and CorsLayer::new() would send no CORS headers at all.
        // The site answers on both the apex and www host, so allow both variants.
        let origins = cors_origins();
        if origins.is_empty() {
            tracing::error!(base_url = %base_url(), "no valid CORS origin from BASE_URL");
            CorsLayer::new()
        } else {
            tracing::info!(?origins, "cors setup: allow site origins");
            CorsLayer::new()
                .allow_origin(origins)
                .allow_credentials(true)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::CONTENT_TYPE])
        }
    }
}

/// Allowed CORS origins derived from `BASE_URL`: the configured origin plus its
/// apex/www sibling, since the site answers on both `live-ask.com` and `www.live-ask.com`.
fn cors_origins() -> Vec<HeaderValue> {
    let base = base_url();
    let mut variants = vec![base.clone()];
    if let Some((scheme, host)) = base.split_once("://") {
        let sibling = host.strip_prefix("www.").map_or_else(
            || format!("{scheme}://www.{host}"),
            |apex| format!("{scheme}://{apex}"),
        );
        variants.push(sibling);
    }
    variants
        .into_iter()
        .filter_map(|origin| match origin.parse::<HeaderValue>() {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::error!(error = %e, %origin, "invalid CORS origin");
                None
            }
        })
        .collect()
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

/// Whether we run the local dev/test environment. Any other `LIVEASK_ENV` (prod, beta, ...)
/// is a publicly reachable deployment that must be hardened.
fn is_local() -> bool {
    production_env() == "local"
}

fn use_relaxed_cors() -> bool {
    let relaxed = std::env::var(env::ENV_RELAX_CORS).is_ok_and(|var| var == "1");
    // relaxed CORS mirrors any origin AND allows credentials; never permit that in prod,
    // regardless of RELAX_CORS, so a stray env var cannot open production up
    if relaxed && is_prod() {
        tracing::warn!("RELAX_CORS is ignored in production");
        return false;
    }
    relaxed
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
    std::env::var(env::ENV_REDIS_URL).unwrap_or_else(|_| "redis://localhost:6379".into())
}

fn posthog_key() -> Option<String> {
    std::env::var(env::ENV_POSTHOG_KEY).ok()
}

fn stripe_secret() -> String {
    std::env::var(env::ENV_STRIPE_SECRET).unwrap_or_else(|_| String::new())
}

async fn aws_ses_client() -> Result<aws_sdk_ses::Client> {
    let config = aws_config::defaults(BehaviorVersion::v2024_03_28());

    let config = config.load().await;

    Ok(aws_sdk_ses::Client::new(&config))
}

async fn dynamo_client() -> Result<aws_sdk_dynamodb::Client> {
    use aws_sdk_dynamodb::Client;

    let config = aws_config::defaults(BehaviorVersion::v2024_03_28());

    let config = if use_local_db() {
        let url = std::env::var(env::ENV_DB_URL).unwrap_or_else(|_| "http://localhost:8000".into());

        tracing::info!("ddb local url: {url}");

        config
            .credentials_provider(Credentials::new("aid", "sid", None, None, "local"))
            .region(aws_sdk_dynamodb::config::Region::new(""))
            .endpoint_url(&url)
    } else {
        config
    };

    let config = config.load().await;

    Ok(Client::new(&config))
}

async fn payment() -> Result<Arc<Payment>> {
    let is_test = !is_prod();
    let secret = stripe_secret();
    let mut payment = Payment::new(secret.clone());

    match payment.authenticate(!is_test).await {
        Err(e) => {
            tracing::error!(
                "payment auth error: [secret: {} ({}), test: {is_test}] {}",
                secret.get(0..8).unwrap_or("utf8 error in secret"),
                secret.len(),
                e
            );

            // panic if this is not working in live
            if !is_test {
                bail!("premium not configured")
            }
        }
        Ok(premium_id) => {
            tracing::info!("payment auth ok: [test: {is_test}, product: {premium_id}]");
        }
    }

    Ok(Arc::new(payment))
}

async fn setup_app(
    redis_url: &str,
    prod_env: &str,
    log_level: &str,
) -> std::result::Result<Arc<App>, Box<dyn std::error::Error>> {
    let base_url = base_url();

    let server_id = server_id().await.unwrap_or_else(|| "server".to_string());

    let posthog_key = posthog_key();

    tracing::info!(
        git= %GIT_HASH,
        env= prod_env,
        is_prod= is_prod(),
        posthog_key = posthog_key.is_some(),
        log_level,
        redis_url,
        base_url,
        server_id,
        "server-starting",
    );

    let tracking = Tracking::new(posthog_key, server_id.clone(), prod_env.to_string());

    tracking.track_server_start().await?;

    let redis_pool = create_pool(redis_url)?;
    ping_test_redis(&redis_pool).await?;

    let payment = payment().await?;

    let pubsub = Arc::new(PubSubRedis::new(redis_pool.clone(), redis_url.to_string()));
    let viewers = Arc::new(RedisViewers::new(redis_pool));

    let eventsdb = Arc::new(DynamoEventsDB::new(dynamo_client().await?, use_local_db()).await?);
    let app = Arc::new(App::new(
        eventsdb,
        Arc::<PubSubRedis>::clone(&pubsub),
        viewers,
        payment,
        tracking,
        base_url,
    ));

    pubsub.set_receiver(Arc::<App>::clone(&app)).await;

    Ok(app)
}

#[allow(clippy::unwrap_in_result)]
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
            traces_sample_rate: if is_debug() { 1.0 } else { 0.0 },
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

    let app = setup_app(&redis_url, &prod_env, &log_level).await?;

    let secret = session_secret()
        .ok_or_else(|| error::InternalError::General(String::from("invalid session secret")))?;

    // auth is now a stateless JWT: knowing the signing key is enough to forge an admin token,
    // so refuse to start on any public (non-local) env with the well-known dev fallback.
    if !is_local() && env::is_default_session_secret(&secret) {
        return Err(
            "LA_SESSION_SECRET must be set to a non-default value outside local dev".into(),
        );
    }

    // harden cookies (SameSite=Strict) on every public env, not just prod
    let auth_config = auth::setup(secret, !is_local());

    let admin_routes = Router::new()
        .route("/user", get(admin_user_handler))
        .route("/login", post(login_handler))
        .route("/logout", get(logout_handler));

    let event_routes = Router::new()
        .route("/:id", get(handle::getevent_handler))
        .route("/:id/pwd", post(handle::set_event_password))
        .route("/add", post(handle::addevent_handler))
        .route("/editlike/:id", post(handle::editlike_handler))
        .route("/addquestion/:id", post(handle::addquestion_handler))
        .route("/question/:id/:question_id", get(handle::get_question));

    #[rustfmt::skip]
    let mod_routes = Router::new()
        .route("/:id/:secret", get(handle::mod_get_event))
        .route("/upgrade/:id/:secret", post(handle::mod_premium_upgrade))
        .route("/capture/:id/:order", get(handle::mod_premium_capture))
        .route("/delete/:id/:secret", get(handle::mod_delete_event))
        .route("/question/:id/:secret/:question_id", get(handle::mod_get_question))
        .route("/questionmod/:id/:secret/:question_id", post(handle::mod_edit_question))
        .route("/:id/:secret", post(handle::mod_edit_event));

    let (prometheus_layer, metrics_handler) = axum_prometheus::PrometheusMetricLayer::pair();

    #[rustfmt::skip]
    let router = Router::new()
        .route("/api/ping", get(handle::ping_handler))
        .route("/api/version", get(handle::version_handler))
        .route("/api/payment/stripe/webhook", post(stripe_webhooks::handle_webhook))
        .route("/push/:id", get(push_handler))
        .route("/api/subscription", post(subscription_handler))
        .route("/api/subscription/url", get(subscription_url_handler))
        .nest("/api/event", event_routes)
        .nest("/api/mod/event", mod_routes)
        .nest("/api/admin", admin_routes)
        .route("/metrics", get(async move || metrics_handler.render()))
        .layer(Extension(auth_config))
        .layer(SetSensitiveRequestHeadersLayer::new(once(header::COOKIE)))
        .layer(SentryHttpLayer::with_transaction())
        .layer(NewSentryLayer::new_from_top())
        .layer(TraceLayer::new_for_http())
        .layer(setup_cors())
        .layer(prometheus_layer)
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
