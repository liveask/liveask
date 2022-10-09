use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use mailjet_rs::common::Recipient;
use mailjet_rs::v3::Message;
use mailjet_rs::{Client, SendAPIVersion};
use mailjet_rs::{Map, Value};
use shared::{EventData, EventInfo, EventTokens};
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
        .route("/ping", get(ping_handler))
        .route("/api/event/:id", get(getevent_handler))
        .layer(TraceLayer::new_for_http())
        .layer(setup_cors());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8090));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn getevent_handler(Path(_id): Path<String>) -> Result<impl IntoResponse, StatusCode> {
    let ev = EventInfo {
        create_time_unix: 0,
        delete_time_unix: 0,
        last_edit_unix: 0,
        create_time_utc: String::new(),
        deleted: false,
        questions: Vec::new(),
        data: EventData {
            max_likes: 10,
            name: String::from("foo"),
            description: String::from("bar"),
            short_url: String::new(),
            long_url: None,
        },
        tokens: EventTokens {
            public_token: String::new(),
            moderator_token: None,
        },
    };

    Ok(Json(ev))
}

async fn ping_handler() -> Html<&'static str> {
    Html("pong")
}

pub struct MailJetCredentials {
    pub public_key: String,
    pub private_key: String,
}

#[allow(dead_code)]
async fn send_mail(
    receiver: String,
    event_name: String,
    public_link: String,
    mod_link: String,
    mailjet_template_id: usize, //std::env::var("MAILJET_TEMPLATE_ID").unwrap().parse::<usize>().unwrap()
    //TODO:
    //std::env::var("MAILJET_KEY").unwrap()
    //std::env::var("MAILJET_SECRET").unwrap()
    mailjet_credentials: MailJetCredentials,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new(
        SendAPIVersion::V3,
        &mailjet_credentials.public_key,
        &mailjet_credentials.private_key,
    );

    // Create your a `Message` instance with the minimum required values
    let mut message = Message::new(
        "mail@live-ask.com",
        "liveask",
        Some("New Event Created".to_string()),
        None,
    );
    message.push_recipient(Recipient::new(&receiver));

    message.set_template_id(mailjet_template_id);

    let mut vars = Map::new();

    vars.insert(String::from("eventname"), Value::from(event_name));
    vars.insert(String::from("publiclink"), Value::from(public_link));
    vars.insert(String::from("moderatorlink"), Value::from(mod_link));

    message.vars = Some(vars);

    let response = client.send(message).await;

    tracing::debug!("mailjet response: {:?}", response);

    Ok(())
}
