#![allow(dead_code)]

use reqwest::{header::CONTENT_TYPE, StatusCode};
use serde_json::json;
use shared::{EventInfo, GetEventResponse, TEST_VALID_QUESTION};

const MIN_DESC: &str = "minimum desc length possible!!";
const MIN_NAME: &str = "min name";

fn main() {}

fn server_rest() -> String {
    std::env::var("URL").unwrap_or_else(|_| "http://localhost:8090".into())
}
fn server_socket() -> String {
    std::env::var("SOCKET_URL").unwrap_or_else(|_| "ws://localhost:8090".into())
}

async fn get_event(public: String, secret: Option<String>) -> Option<GetEventResponse> {
    let url = if let Some(secret) = secret {
        format!("{}/api/mod/event/{}/{}", server_rest(), public, secret)
    } else {
        format!("{}/api/event/{}", server_rest(), public)
    };

    let res = reqwest::Client::new().get(url).send().await.unwrap();

    if res.status() == StatusCode::OK {
        assert!(res
            .headers()
            .get(CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("application/json"),);

        let e = res.json::<GetEventResponse>().await.unwrap();

        assert_eq!(e.info.tokens.public_token, public);

        Some(e)
    } else {
        None
    }
}

async fn add_event(name: String) -> EventInfo {
    let res = reqwest::Client::new()
        .post(format!("{}/api/event/add", server_rest()))
        .json(&json!({
            "eventData":{
                "maxLikes":0,
                "name":name,
                "description":MIN_DESC,
                "shortUrl":"",
                "longUrl":null},
            "test": true,
            "moderatorEmail": null,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert!(res
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("application/json"),);

    let e = res.json::<EventInfo>().await.unwrap();

    assert_eq!(e.data.name, name);

    e
}

async fn delete_event(id: String, secret: String) {
    let res = reqwest::Client::new()
        .get(format!(
            "{}/api/mod/event/delete/{}/{}",
            server_rest(),
            id,
            secret
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

async fn change_event_state(id: String, secret: String, state: u8) {
    let res = reqwest::Client::new()
        .post(format!(
            "{}/api/mod/event/state/{}/{}",
            server_rest(),
            id,
            secret
        ))
        .json(&json!({
            "state": {
                "state": state
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

async fn add_question(event: String) -> shared::QuestionItem {
    let res = reqwest::Client::new()
        .post(format!("{}/api/event/addquestion/{}", server_rest(), event))
        .json(&json!({
            "text":TEST_VALID_QUESTION
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert!(res
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("application/json"),);

    let q = res.json::<shared::QuestionItem>().await.unwrap();

    assert_eq!(q.text, TEST_VALID_QUESTION);

    q
}

async fn like_question(event: String, question_id: i64, like: bool) -> shared::QuestionItem {
    let body = shared::EditLike { question_id, like };
    let res = reqwest::Client::new()
        .post(format!("{}/api/event/editlike/{}", server_rest(), event))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert!(res
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("application/json"),);

    res.json::<shared::QuestionItem>().await.unwrap()
}

async fn hide_question(event: String, secret: String, question_id: i64) {
    let body = shared::ModQuestion {
        answered: false,
        hide: true,
        screened: false,
    };

    let res = reqwest::Client::new()
        .post(format!(
            "{}/api/mod/event/questionmod/{}/{}/{}",
            server_rest(),
            event,
            secret,
            question_id
        ))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
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
        let e = add_event(MIN_NAME.to_string()).await;
        assert_eq!(e.data.name, MIN_NAME);
    }

    #[tokio::test]
    async fn test_get_event() {
        // env_logger::init();

        let e = add_event(MIN_NAME.to_string()).await;
        assert_eq!(e.data.name, MIN_NAME);

        let e2 = get_event(
            e.tokens.public_token.clone(),
            e.tokens.moderator_token.clone(),
        )
        .await
        .unwrap()
        .info;

        assert_eq!(e2, e);
        let e3 = get_event(e.tokens.public_token, None).await.unwrap().info;
        assert_eq!(e3.tokens.moderator_token, Some(String::new()));
    }

    #[tokio::test]
    async fn test_like_question() {
        let e = add_event(MIN_NAME.to_string()).await;
        let q_before = add_question(e.tokens.public_token.clone()).await;
        let q_after = like_question(e.tokens.public_token, q_before.id, true).await;
        assert_eq!(q_after.likes, q_before.likes + 1);
    }

    #[tokio::test]
    async fn test_delete_event() {
        // env_logger::init();

        let e = add_event(MIN_NAME.to_string()).await;
        assert_eq!(e.deleted, false);

        delete_event(
            e.tokens.public_token.clone(),
            e.tokens.moderator_token.clone().unwrap(),
        )
        .await;

        let e = get_event(e.tokens.public_token.clone(), e.tokens.moderator_token).await;

        assert!(e.unwrap().info.deleted);
    }

    #[tokio::test]
    async fn test_hide_question() {
        let e_mod = add_event(MIN_NAME.to_string()).await;

        let q_before = add_question(e_mod.tokens.public_token.clone()).await;

        hide_question(
            e_mod.tokens.public_token.clone(),
            e_mod.tokens.moderator_token.clone().unwrap(),
            q_before.id,
        )
        .await;

        let e = get_event(e_mod.tokens.public_token.clone(), None)
            .await
            .unwrap()
            .info;
        assert_eq!(e.questions.len(), 0);
        let e = get_event(
            e_mod.tokens.public_token.clone(),
            e_mod.tokens.moderator_token,
        )
        .await
        .unwrap()
        .info;
        assert_eq!(e.questions.len(), 1);
    }

    #[tokio::test]
    async fn test_websockets() {
        // env_logger::init();

        let tokens = add_event(MIN_NAME.to_string()).await.tokens;

        let event = tokens.public_token;
        let secret = tokens.moderator_token.unwrap();

        let (mut socket, response) =
            connect(&format!("{}/push/{}", server_socket(), event)).expect("Can't connect");

        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

        let msg = socket.read().expect("Error reading message");
        assert_eq!(msg.into_text().unwrap(), "v:1".to_string());

        let question = add_question(event.clone()).await;

        let msg = socket.read().expect("Error reading message");
        assert_eq!(msg.into_text().unwrap(), format!("q:{}", question.id));

        like_question(event.clone(), question.id, true).await;

        let msg = socket.read().expect("Error reading message");
        assert_eq!(msg.into_text().unwrap(), format!("q:{}", question.id));

        change_event_state(event, secret, 1).await;
        let msg = socket.read().expect("Error reading message");
        assert_eq!(msg.into_text().unwrap(), "e");
    }
}
