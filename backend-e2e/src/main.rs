#![allow(dead_code)]

use reqwest::{header::CONTENT_TYPE, StatusCode};
use serde_json::json;
use shared::EventInfo;

fn main() {}

fn server_rest() -> String {
    std::env::var("URL").unwrap_or_else(|_| "http://localhost:8090".into())
}
fn server_socket() -> String {
    std::env::var("SOCKET_URL").unwrap_or_else(|_| "ws://localhost:8090".into())
}

async fn get_event(public: String, secret: Option<String>) -> EventInfo {
    let url = if let Some(secret) = secret {
        format!("{}/api/mod/event/{}/{}", server_rest(), public, secret)
    } else {
        format!("{}/api/event/{}", server_rest(), public)
    };

    let res = reqwest::Client::new().get(url).send().await.unwrap();

    assert!(dbg!(res.headers().get(CONTENT_TYPE).unwrap().to_str())
        .unwrap()
        .starts_with("application/json"),);
    assert_eq!(res.status(), StatusCode::OK);

    let e = res.json::<EventInfo>().await.unwrap();

    assert_eq!(e.tokens.public_token, public);

    e
}

async fn add_event(name: String) -> EventInfo {
    let res = reqwest::Client::new()
        .post(format!("{}/api/addevent", server_rest()))
        .json(&json!({
            "eventData":{
                "maxLikes":0,
                "name":name,
                "description":"fancy description",
                "shortUrl":"",
                "longUrl":null},
            "moderatorEmail": "",
        }))
        .send()
        .await
        .unwrap();

    assert!(res
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("application/json"),);
    assert_eq!(res.status(), StatusCode::OK);

    let e = res.json::<EventInfo>().await.unwrap();

    assert_eq!(e.data.name, name);

    e
}

async fn add_question(event: String) -> shared::Item {
    let res = reqwest::Client::new()
        .post(format!("{}/api/event/addquestion/{}", server_rest(), event))
        .json(&json!({
            "text":"test"
        }))
        .send()
        .await
        .unwrap();

    assert!(res
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("application/json"),);
    assert_eq!(res.status(), StatusCode::OK);

    let q = res.json::<shared::Item>().await.unwrap();

    assert_eq!(q.text, "test");

    q
}

async fn like_question(event: String, question_id: i64, like: bool) -> shared::Item {
    let body = shared::EditLike { question_id, like };

    let res = reqwest::Client::new()
        .post(format!("{}/api/event/editlike/{}", server_rest(), event))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert!(res
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("application/json"),);
    assert_eq!(res.status(), StatusCode::OK);

    let q = res.json::<shared::Item>().await.unwrap();

    q
}

#[cfg(test)]
mod test {
    use super::*;
    use reqwest::StatusCode;
    use tungstenite::connect;

    #[tokio::test]
    async fn test_status() {
        let res = reqwest::get(format!("{}/api/ping", server_rest()))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_event() {
        let e = add_event("foo".to_string()).await;
        assert_eq!(e.data.name, "foo");
    }

    #[tokio::test]
    async fn test_get_event() {
        // env_logger::init();

        let e = add_event("foo".to_string()).await;
        assert_eq!(e.data.name, "foo");

        eprintln!("e: {:?}", e);

        let e2 = get_event(
            e.tokens.public_token.clone(),
            e.tokens.moderator_token.clone(),
        )
        .await;

        eprintln!("e2: {:?}", e2);

        assert_eq!(e2, e);
        let e3 = get_event(e.tokens.public_token, None).await;
        assert_eq!(e3.tokens.moderator_token, Some(String::new()));
    }

    #[tokio::test]
    async fn test_like_question() {
        let e = add_event("foo".to_string()).await;
        let q_before = add_question(e.tokens.public_token.clone()).await;
        let q_after = like_question(e.tokens.public_token, q_before.id, true).await;
        assert_eq!(q_after.likes, q_before.likes + 1);
    }

    #[tokio::test]
    async fn test_websockets() {
        // env_logger::init();

        let event = add_event("foo".to_string()).await.tokens.public_token;

        let (mut socket, response) =
            connect(&format!("{}/push/{}", server_socket(), event)).expect("Can't connect");

        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

        let question = add_question(event).await;

        let msg = socket.read_message().expect("Error reading message");
        assert_eq!(msg.into_text().unwrap(), format!("q:{}", question.id));
    }
}
