mod app;
mod eventsdb;
mod handle;
mod mail;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::{
    model::{
        AttributeDefinition, KeySchemaElement, KeyType, ProvisionedThroughput, ScalarAttributeType,
    },
    Credentials, Endpoint,
};
use axum::{
    http::Uri,
    routing::{get, post},
    Extension, Router,
};
use eventsdb::InMemoryEventsDB;
use std::{net::SocketAddr, sync::Arc};
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
    CorsLayer::very_permissive()
}

async fn test_aws() {
    use aws_sdk_dynamodb::Client;

    let region_provider = RegionProviderChain::default_provider().or_else("us-west-1");
    let config = aws_config::from_env()
        .region(region_provider)
        .credentials_provider(Credentials::new("aid", "sid", None, None, "local"))
        .endpoint_resolver(Endpoint::immutable(Uri::from_static(
            "http://localhost:8000",
        )))
        .load()
        .await;

    let client = Client::new(&config);

    let resp = client.list_tables().send().await.unwrap();
    let names = resp.table_names().unwrap_or_default();

    tracing::trace!("tables: {}", names.join(","));

    if !names.contains(&"liveask".into()) {
        tracing::info!("table not found, creating now");
        create_table(&client, "liveask".into(), "key".into()).await;
    }
}

//TODO: error handling
async fn create_table(client: &aws_sdk_dynamodb::Client, table_name: String, key_name: String) {
    let ad = AttributeDefinition::builder()
        .attribute_name(&key_name)
        .attribute_type(ScalarAttributeType::S)
        .build();

    let ks = KeySchemaElement::builder()
        .attribute_name(&key_name)
        .key_type(KeyType::Hash)
        .build();

    let pt = ProvisionedThroughput::builder()
        .read_capacity_units(5)
        .write_capacity_units(5)
        .build();

    client
        .create_table()
        .table_name(table_name)
        .attribute_definitions(ad)
        .key_schema(ks)
        .provisioned_throughput(pt)
        .send()
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    test_aws().await;

    let app = App::new(Arc::new(InMemoryEventsDB::default()));

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
        .await
        .unwrap();
}
