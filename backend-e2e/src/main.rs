#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::if_then_some_else_none,
    clippy::option_if_let_else
)]

use reqwest::{header::CONTENT_TYPE, StatusCode};
use serde_json::json;
use shared::{EventInfo, GetEventResponse, TEST_EVENT_DESC, TEST_VALID_QUESTION};

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
                "maxLikes":0_i32,
                "name":name,
                "description": TEST_EVENT_DESC,
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
        .post(format!("{}/api/mod/event/{}/{}", server_rest(), id, secret))
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

/// reqwest client that keeps cookies across requests (carries the admin / event-password
/// JWT cookies between calls).
fn cookie_client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap()
}

async fn get_version() -> shared::VersionInfo {
    let res = reqwest::get(format!("{}/api/version", server_rest()))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await.unwrap()
}

async fn get_public_question(event: &str, question_id: i64) -> shared::QuestionItem {
    let res = reqwest::get(format!(
        "{}/api/event/question/{}/{}",
        server_rest(),
        event,
        question_id
    ))
    .await
    .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await.unwrap()
}

async fn get_mod_question(event: &str, secret: &str, question_id: i64) -> shared::QuestionItem {
    let res = reqwest::get(format!(
        "{}/api/mod/event/question/{}/{}/{}",
        server_rest(),
        event,
        secret,
        question_id
    ))
    .await
    .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await.unwrap()
}

/// POST a question returning only the status code — for validation/error cases.
async fn add_question_status(event: &str, text: &str) -> StatusCode {
    reqwest::Client::new()
        .post(format!("{}/api/event/addquestion/{}", server_rest(), event))
        .json(&json!({ "text": text }))
        .send()
        .await
        .unwrap()
        .status()
}

async fn edit_event(id: &str, secret: &str, changes: &shared::ModEvent) -> StatusCode {
    reqwest::Client::new()
        .post(format!("{}/api/mod/event/{}/{}", server_rest(), id, secret))
        .json(changes)
        .send()
        .await
        .unwrap()
        .status()
}

async fn mod_question(
    event: &str,
    secret: &str,
    question_id: i64,
    change: shared::ModQuestion,
) -> StatusCode {
    reqwest::Client::new()
        .post(format!(
            "{}/api/mod/event/questionmod/{}/{}/{}",
            server_rest(),
            event,
            secret,
            question_id
        ))
        .json(&change)
        .send()
        .await
        .unwrap()
        .status()
}

async fn delete_event_status(id: &str, secret: &str) -> StatusCode {
    reqwest::Client::new()
        .get(format!(
            "{}/api/mod/event/delete/{}/{}",
            server_rest(),
            id,
            secret
        ))
        .send()
        .await
        .unwrap()
        .status()
}

/// GET an event through a specific client so the session cookie is carried.
async fn get_event_with(client: &reqwest::Client, public: &str) -> GetEventResponse {
    let res = client
        .get(format!("{}/api/event/{}", server_rest(), public))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await.unwrap()
}

/// Submit a password attempt through a cookie-carrying client; returns whether it was accepted.
async fn submit_password(client: &reqwest::Client, event: &str, pwd: &str) -> bool {
    let res = client
        .post(format!("{}/api/event/{}/pwd", server_rest(), event))
        .json(&json!({ "pwd": pwd }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.json::<shared::EventPasswordResponse>()
        .await
        .unwrap()
        .ok
}

/// The login name the backend checks against (see backend `auth::login_handler`).
const ADMIN_NAME: &str = "admin";

/// The admin password hash the server under test was booted with, if any. `None` means we do
/// not control the server's credentials — controlled-server-only tests skip in that case.
fn admin_pwd_hash() -> Option<String> {
    std::env::var("LA_ADMIN_PWD_HASH")
        .ok()
        .filter(|h| !h.trim().is_empty())
}

async fn admin_login(client: &reqwest::Client, name: &str, pwd_hash: &str) -> StatusCode {
    client
        .post(format!("{}/api/admin/login", server_rest()))
        .json(&shared::UserLogin {
            name: name.to_string(),
            pwd_hash: pwd_hash.to_string(),
        })
        .send()
        .await
        .unwrap()
        .status()
}

async fn admin_get_user(client: &reqwest::Client) -> shared::GetUserInfo {
    let res = client
        .get(format!("{}/api/admin/user", server_rest()))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await.unwrap()
}

async fn admin_logout(client: &reqwest::Client) -> StatusCode {
    client
        .get(format!("{}/api/admin/logout", server_rest()))
        .send()
        .await
        .unwrap()
        .status()
}

/// Request a premium upgrade for an event. With an admin session on `client` this hits the
/// no-Stripe `AdminUpgrade` path.
async fn premium_upgrade(
    client: &reqwest::Client,
    event: &str,
    secret: &str,
) -> shared::EventUpgradeResponse {
    let res = client
        .post(format!(
            "{}/api/mod/event/upgrade/{}/{}",
            server_rest(),
            event,
            secret
        ))
        .json(&shared::ModRequestPremium {
            context: shared::ModRequestPremiumContext::Regular,
        })
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await.unwrap()
}

/// GET a single question via the public route, returning only the status code.
async fn public_get_question_status(event: &str, question_id: i64) -> StatusCode {
    reqwest::get(format!(
        "{}/api/event/question/{}/{}",
        server_rest(),
        event,
        question_id
    ))
    .await
    .unwrap()
    .status()
}

/// GET a single question via the moderator route, returning only the status code.
async fn mod_get_question_status(event: &str, secret: &str, question_id: i64) -> StatusCode {
    reqwest::get(format!(
        "{}/api/mod/event/question/{}/{}/{}",
        server_rest(),
        event,
        secret,
        question_id
    ))
    .await
    .unwrap()
    .status()
}

/// POST an editlike returning only the status code — for state-gate error cases.
async fn edit_like_status(event: &str, question_id: i64, like: bool) -> StatusCode {
    reqwest::Client::new()
        .post(format!("{}/api/event/editlike/{}", server_rest(), event))
        .json(&shared::EditLike { question_id, like })
        .send()
        .await
        .unwrap()
        .status()
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use reqwest::StatusCode;
    use shared::TEST_EVENT_NAME;
    use tungstenite::{connect, Message};

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_status() {
        let res = reqwest::get(format!("{}/api/ping", server_rest()))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_add_event() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        assert_eq!(e.data.name, TEST_EVENT_NAME);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_get_event() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        assert_eq!(e.data.name, TEST_EVENT_NAME);

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
    #[tracing_test::traced_test]
    async fn test_like_question() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let q_before = add_question(e.tokens.public_token.clone()).await;
        let q_after = like_question(e.tokens.public_token, q_before.id, true).await;
        assert_eq!(q_after.likes, q_before.likes + 1);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_delete_event() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        assert!(!e.is_deleted());

        delete_event(
            e.tokens.public_token.clone(),
            e.tokens.moderator_token.clone().unwrap(),
        )
        .await;

        let e = get_event(e.tokens.public_token.clone(), e.tokens.moderator_token).await;

        assert!(e.unwrap().info.is_deleted());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_hide_question() {
        let e_mod = add_event(TEST_EVENT_NAME.to_string()).await;

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
    #[tracing_test::traced_test]
    async fn test_websockets() {
        let tokens = add_event(TEST_EVENT_NAME.to_string()).await.tokens;

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

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_version() {
        let version = get_version().await;
        assert!(!version.version.trim().is_empty());
        assert!(!version.git_hash.trim().is_empty());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_get_single_question_public_and_mod() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let q = add_question(e.tokens.public_token.clone()).await;

        let public = get_public_question(&e.tokens.public_token, q.id).await;
        assert_eq!(public.id, q.id);
        assert_eq!(public.text, TEST_VALID_QUESTION);

        let moderated = get_mod_question(&e.tokens.public_token, &secret, q.id).await;
        assert_eq!(moderated.id, q.id);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_get_unknown_event_is_none() {
        // an id that was never created must not resolve to an event
        let e = get_event("unknown0000000000000000000".to_string(), None).await;
        assert!(e.is_none());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_edit_event_state() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();

        let status = edit_event(
            &public,
            &secret,
            &shared::ModEvent {
                state: Some(shared::EventState {
                    state: shared::States::Closed,
                }),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let after = get_event(public, None).await.unwrap();
        assert!(after.info.state.is_closed());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_unlike_question() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let public = e.tokens.public_token.clone();
        let q = add_question(public.clone()).await;

        let liked = like_question(public.clone(), q.id, true).await;
        assert_eq!(liked.likes, q.likes + 1);

        let unliked = like_question(public, q.id, false).await;
        assert_eq!(unliked.likes, q.likes);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_add_question_too_short_rejected() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let status = add_question_status(&e.tokens.public_token, "a b").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_duplicate_question_rejected() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let public = e.tokens.public_token.clone();

        add_question(public.clone()).await;
        let status = add_question_status(&public, TEST_VALID_QUESTION).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_answer_question() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();
        let q = add_question(public.clone()).await;

        let status = mod_question(
            &public,
            &secret,
            q.id,
            shared::ModQuestion {
                hide: false,
                answered: true,
                screened: false,
            },
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let moderated = get_mod_question(&public, &secret, q.id).await;
        assert!(moderated.answered);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_delete_wrong_secret_rejected() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let public = e.tokens.public_token.clone();

        let status = delete_event_status(&public, "definitely-not-the-secret").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // the event must still be alive
        let after = get_event(public, None).await.unwrap();
        assert!(!after.info.is_deleted());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_password_protection() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();

        add_question(public.clone()).await;

        // enable a password on the (free) event
        let status = edit_event(
            &public,
            &secret,
            &shared::ModEvent {
                password: Some(shared::EventPassword::Enabled("secret-pwd".into())),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let client = cookie_client();

        // without the password the event is masked (question text is obscured)
        let masked = get_event_with(&client, &public).await;
        assert!(masked.masked);
        assert!(masked
            .flags
            .contains(shared::EventResponseFlags::WRONG_PASSWORD));
        assert_ne!(masked.info.questions[0].text, TEST_VALID_QUESTION);

        // wrong password is rejected
        assert!(!submit_password(&client, &public, "nope").await);

        // correct password is accepted and unlocks the event for this session
        assert!(submit_password(&client, &public, "secret-pwd").await);

        let unlocked = get_event_with(&client, &public).await;
        assert!(!unlocked
            .flags
            .contains(shared::EventResponseFlags::WRONG_PASSWORD));
        assert_eq!(unlocked.info.questions[0].text, TEST_VALID_QUESTION);

        // rotating the password re-locks the still-held grant (revocation parity with the old
        // server-side session): the same client is masked again without re-entering a password
        let status = edit_event(
            &public,
            &secret,
            &shared::ModEvent {
                password: Some(shared::EventPassword::Enabled("rotated-pwd".into())),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let relocked = get_event_with(&client, &public).await;
        assert!(relocked
            .flags
            .contains(shared::EventResponseFlags::WRONG_PASSWORD));
        assert_ne!(relocked.info.questions[0].text, TEST_VALID_QUESTION);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_screening_approve_flow() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();

        // enable screening
        let status = edit_event(
            &public,
            &secret,
            &shared::ModEvent {
                screening: Some(true),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // a new question is held for screening
        let q = add_question(public.clone()).await;
        assert!(q.screening);

        // public viewers don't see it yet, moderators do
        let public_view = get_event(public.clone(), None).await.unwrap();
        assert!(public_view.info.questions.is_empty());
        let mod_view = get_event(public.clone(), e.tokens.moderator_token.clone())
            .await
            .unwrap();
        assert_eq!(mod_view.info.questions.len(), 1);

        // moderator approves -> now visible publicly
        let status = mod_question(
            &public,
            &secret,
            q.id,
            shared::ModQuestion {
                hide: false,
                answered: false,
                screened: true,
            },
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let public_view = get_event(public, None).await.unwrap();
        assert_eq!(public_view.info.questions.len(), 1);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_stripe_webhook_bad_signature_rejected() {
        let res = reqwest::Client::new()
            .post(format!("{}/api/payment/stripe/webhook", server_rest()))
            .header("stripe-signature", "t=0,v1=deadbeef")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_websocket_viewer_count_broadcast() {
        // Viewer counts are pushed as `v:<count>` frames; a fresh subscriber first sees `v:1`
        // (itself). A second subscriber must bump the first socket to `v:2`, which also
        // exercises the redis pub/sub fan-out to already-connected clients.
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let public = e.tokens.public_token.clone();

        let (mut socket1, resp1) =
            connect(&format!("{}/push/{}", server_socket(), public)).expect("connect 1");
        assert_eq!(resp1.status(), StatusCode::SWITCHING_PROTOCOLS);
        assert_eq!(socket1.read().unwrap().into_text().unwrap(), "v:1");

        let (mut _socket2, resp2) =
            connect(&format!("{}/push/{}", server_socket(), public)).expect("connect 2");
        assert_eq!(resp2.status(), StatusCode::SWITCHING_PROTOCOLS);

        assert_eq!(socket1.read().unwrap().into_text().unwrap(), "v:2");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "seeds the server's redis directly; run via `just e2e-test-local`"]
    async fn test_viewer_count_never_reports_negative() {
        // The counter can drift below zero (a missed INCR / expired key) and the WS `v:` frame
        // is computed from `count()`, which now clamps to >= 0. The drift can only be forced by
        // seeding redis, so this needs the local redis the server we booted talks to.
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
        let Ok(client) = redis::Client::open(redis_url) else {
            eprintln!("skipping test_viewer_count_never_reports_negative: invalid REDIS_URL");
            return;
        };
        let Ok(mut con) = client.get_async_connection().await else {
            eprintln!("skipping test_viewer_count_never_reports_negative: no redis reachable");
            return;
        };

        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let public = e.tokens.public_token.clone();

        // force the stored counter negative (backend key format is `viewers/<id>`)
        let _: () = redis::cmd("SET")
            .arg(format!("viewers/{public}"))
            .arg(-5_i64)
            .query_async(&mut con)
            .await
            .unwrap();

        // a fresh subscriber INCRs to -4, then the broadcast runs count() -> clamped to 0.
        // without the clamp this first frame would be `v:-4`.
        let (mut socket, resp) =
            connect(&format!("{}/push/{}", server_socket(), public)).expect("connect");
        assert_eq!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
        assert_eq!(socket.read().unwrap().into_text().unwrap(), "v:0");
    }

    // ---------------------------------------------------------------------------------------
    // Controlled-server-only tests.
    //
    // These require a server WE booted with a known admin credential (`LA_ADMIN_PWD_HASH`),
    // so they cannot run against the shared beta/prod deployments. They are `#[ignore]`d and
    // therefore skipped by the plain `cargo test` used for `e2e-test-beta`/`-remote`/`-legacy`.
    // Run them with `just e2e-test-local` (which boots a local server and passes
    // `--include-ignored` plus the matching admin credential).
    // ---------------------------------------------------------------------------------------

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "needs a server we control (known admin creds); run via `just e2e-test-local`"]
    async fn test_admin_login_and_logout() {
        let Some(pwd_hash) = admin_pwd_hash() else {
            eprintln!("skipping test_admin_login_and_logout: LA_ADMIN_PWD_HASH not set");
            return;
        };

        let client = cookie_client();

        // not authenticated yet
        assert!(admin_get_user(&client).await.user.is_none());

        // valid credentials establish an authenticated session
        assert_eq!(
            admin_login(&client, ADMIN_NAME, &pwd_hash).await,
            StatusCode::OK
        );

        let user = admin_get_user(&client)
            .await
            .user
            .expect("expected an authenticated admin user after login");
        assert_eq!(user.name, ADMIN_NAME);

        // logging out clears the session
        assert_eq!(admin_logout(&client).await, StatusCode::OK);
        assert!(admin_get_user(&client).await.user.is_none());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "needs a server we control (known admin creds); run via `just e2e-test-local`"]
    async fn test_admin_login_wrong_password_rejected() {
        let client = cookie_client();

        // a bad password hash is rejected and must not establish a session
        assert_eq!(
            admin_login(&client, ADMIN_NAME, "definitely-not-the-admin-hash").await,
            StatusCode::FORBIDDEN
        );
        assert!(admin_get_user(&client).await.user.is_none());
    }

    // ---- state gates ---------------------------------------------------------------------

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_closed_event_rejects_add_and_like() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();
        let q = add_question(public.clone()).await; // while still Open

        assert_eq!(
            edit_event(
                &public,
                &secret,
                &shared::ModEvent {
                    state: Some(shared::EventState {
                        state: shared::States::Closed,
                    }),
                    ..Default::default()
                },
            )
            .await,
            StatusCode::OK
        );

        // both mutations are gated off once the event is Closed (currently → 500)
        assert_eq!(
            add_question_status(&public, "a different valid question here").await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            edit_like_status(&public, q.id, true).await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_voting_only_blocks_add_but_allows_like() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();
        let q = add_question(public.clone()).await; // while still Open

        assert_eq!(
            edit_event(
                &public,
                &secret,
                &shared::ModEvent {
                    state: Some(shared::EventState {
                        state: shared::States::VotingOnly,
                    }),
                    ..Default::default()
                },
            )
            .await,
            StatusCode::OK
        );

        // adding a question is blocked in VotingOnly (currently → 500) ...
        assert_eq!(
            add_question_status(&public, "another valid question here").await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        // ... but liking is still allowed
        let liked = like_question(public, q.id, true).await;
        assert_eq!(liked.likes, q.likes + 1);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_mod_get_question_access_control() {
        // Prod runs a pre-fix build until promoted; gate the fix-specific assertions on the
        // reported version. Delete the `pre_fix` branch once prod stops reporting this SHA.
        const PRE_FIX_PROD_SHA: &str = "70f4350";

        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();
        let q = add_question(public.clone()).await;

        // hide the question — now only a moderator may fetch it (same on both versions)
        assert_eq!(
            mod_question(
                &public,
                &secret,
                q.id,
                shared::ModQuestion {
                    hide: true,
                    answered: false,
                    screened: false,
                },
            )
            .await,
            StatusCode::OK
        );

        let pre_fix = get_version().await.git_hash.trim() == PRE_FIX_PROD_SHA;

        if pre_fix {
            // pre-fix: the real moderator is not recognized, so the correct secret is wrongly
            // refused (500), while a wrong secret wrongly LEAKS the hidden question (200).
            assert_eq!(
                mod_get_question_status(&public, &secret, q.id).await,
                StatusCode::INTERNAL_SERVER_ERROR
            );
            assert_eq!(
                mod_get_question_status(&public, "definitely-not-the-secret", q.id).await,
                StatusCode::OK
            );
        } else {
            // the correct moderator secret can fetch the hidden question
            let fetched = get_mod_question(&public, &secret, q.id).await;
            assert_eq!(fetched.id, q.id);
            assert!(fetched.hidden);

            // a wrong secret must NOT leak the hidden question — it is rejected
            assert_eq!(
                mod_get_question_status(&public, "definitely-not-the-secret", q.id).await,
                StatusCode::BAD_REQUEST
            );
        }

        // the public route cannot see the hidden question either
        assert_eq!(
            public_get_question_status(&public, q.id).await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    // ---- premium (free-event rejection is un-gated; the allow-side needs admin below) -----

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_premium_only_feature_rejected_on_free_event() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();

        // current_tag requires premium → 400 on a free event
        assert_eq!(
            edit_event(
                &public,
                &secret,
                &shared::ModEvent {
                    current_tag: Some(shared::CurrentTag::Enabled("keynote".into())),
                    ..Default::default()
                },
            )
            .await,
            StatusCode::BAD_REQUEST
        );

        // context also requires premium → 400
        assert_eq!(
            edit_event(
                &public,
                &secret,
                &shared::ModEvent {
                    context: Some(shared::EditContextLink::Enabled(shared::ContextItem {
                        label: "Slides".into(),
                        url: "https://example.com/slides".into(),
                    })),
                    ..Default::default()
                },
            )
            .await,
            StatusCode::BAD_REQUEST
        );
    }

    // ---- websocket protocol / fan-out ----------------------------------------------------

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_websocket_keepalive_and_disconnect_on_bad_frame() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let public = e.tokens.public_token.clone();

        let (mut socket, _) =
            connect(&format!("{}/push/{}", server_socket(), public)).expect("connect");
        assert_eq!(socket.read().unwrap().into_text().unwrap(), "v:1");

        // "p" is the keepalive: ignored, connection stays open (a later question still arrives)
        socket.send(Message::Text("p".into())).unwrap();
        let q = add_question(public.clone()).await;
        assert_eq!(
            socket.read().unwrap().into_text().unwrap(),
            format!("q:{}", q.id)
        );

        // any other frame is treated as a protocol violation → server disconnects us
        socket.send(Message::Text("garbage".into())).unwrap();
        let disconnected = loop {
            match socket.read() {
                Ok(Message::Close(_)) => break true,
                Ok(_) => continue,
                Err(_) => break true,
            }
        };
        assert!(
            disconnected,
            "server should disconnect after an unexpected frame"
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_websocket_viewer_decrement_on_disconnect() {
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let public = e.tokens.public_token.clone();

        let (mut socket1, _) =
            connect(&format!("{}/push/{}", server_socket(), public)).expect("connect 1");
        assert_eq!(socket1.read().unwrap().into_text().unwrap(), "v:1");

        let (socket2, _) =
            connect(&format!("{}/push/{}", server_socket(), public)).expect("connect 2");
        // the second viewer bumps socket1 to v:2 (also our barrier that socket2 is counted)
        assert_eq!(socket1.read().unwrap().into_text().unwrap(), "v:2");

        // dropping socket2 closes it; the survivor must observe the decrement
        drop(socket2);
        assert_eq!(socket1.read().unwrap().into_text().unwrap(), "v:1");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_websocket_multi_event_isolation() {
        let a = add_event(TEST_EVENT_NAME.to_string()).await;
        let b = add_event(TEST_EVENT_NAME.to_string()).await;
        let a_secret = a.tokens.moderator_token.clone().unwrap();

        let (mut sa, _) = connect(&format!(
            "{}/push/{}",
            server_socket(),
            a.tokens.public_token
        ))
        .expect("ws a");
        assert_eq!(sa.read().unwrap().into_text().unwrap(), "v:1");
        let (mut sb, _) = connect(&format!(
            "{}/push/{}",
            server_socket(),
            b.tokens.public_token
        ))
        .expect("ws b");
        assert_eq!(sb.read().unwrap().into_text().unwrap(), "v:1");

        // mutate event A (state change → "e"); this must NOT reach event B's socket
        assert_eq!(
            edit_event(
                &a.tokens.public_token,
                &a_secret,
                &shared::ModEvent {
                    state: Some(shared::EventState {
                        state: shared::States::Closed,
                    }),
                    ..Default::default()
                },
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(sa.read().unwrap().into_text().unwrap(), "e");

        // mutate event B; sb's next frame must be B's "q:" — if A had leaked, it would be "e"
        let qb = add_question(b.tokens.public_token.clone()).await;
        assert_eq!(
            sb.read().unwrap().into_text().unwrap(),
            format!("q:{}", qb.id)
        );
    }

    // ---- premium allow-side (controlled-server-only: needs admin to reach the no-Stripe
    //      AdminUpgrade path, so these are #[ignore]d like the admin login tests) -----------

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "needs a server we control (admin creds); run via `just e2e-test-local`"]
    async fn test_admin_premium_upgrade() {
        let Some(pwd_hash) = admin_pwd_hash() else {
            eprintln!("skipping test_admin_premium_upgrade: LA_ADMIN_PWD_HASH not set");
            return;
        };
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();

        // subscribe so we can observe the upgrade notification
        let (mut socket, _) =
            connect(&format!("{}/push/{}", server_socket(), public)).expect("connect");
        assert_eq!(socket.read().unwrap().into_text().unwrap(), "v:1");

        // an admin session takes the no-Stripe AdminUpgrade path
        let client = cookie_client();
        assert_eq!(
            admin_login(&client, ADMIN_NAME, &pwd_hash).await,
            StatusCode::OK
        );
        assert_eq!(
            premium_upgrade(&client, &public, &secret).await,
            shared::EventUpgradeResponse::AdminUpgrade
        );

        // the event is now premium (real serde_dynamo round-trip of premium_id) ...
        let ev = get_event(public.clone(), Some(secret)).await.unwrap();
        assert!(ev.info.is_premium());

        // ... and subscribers were notified with an "e" frame
        assert_eq!(socket.read().unwrap().into_text().unwrap(), "e");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "needs a server we control (admin creds); run via `just e2e-test-local`"]
    async fn test_premium_feature_edits_persist() {
        let Some(pwd_hash) = admin_pwd_hash() else {
            eprintln!("skipping test_premium_feature_edits_persist: LA_ADMIN_PWD_HASH not set");
            return;
        };
        let e = add_event(TEST_EVENT_NAME.to_string()).await;
        let secret = e.tokens.moderator_token.clone().unwrap();
        let public = e.tokens.public_token.clone();

        let client = cookie_client();
        assert_eq!(
            admin_login(&client, ADMIN_NAME, &pwd_hash).await,
            StatusCode::OK
        );
        assert_eq!(
            premium_upgrade(&client, &public, &secret).await,
            shared::EventUpgradeResponse::AdminUpgrade
        );

        // premium-gated (tag, context) + un-gated (color, meta) fields in one edit; this is
        // the real-DB round-trip of the event fields that free events never persist.
        assert_eq!(
            edit_event(
                &public,
                &secret,
                &shared::ModEvent {
                    current_tag: Some(shared::CurrentTag::Enabled("keynote".into())),
                    context: Some(shared::EditContextLink::Enabled(shared::ContextItem {
                        label: "Slides".into(),
                        url: "https://example.com/slides".into(),
                    })),
                    color: Some(shared::EditColor("#ff2c5e".into())),
                    meta: Some(shared::EditMetaData {
                        title: "Premium Title".into(),
                        description: "a premium event description well over thirty chars".into(),
                    }),
                    ..Default::default()
                },
            )
            .await,
            StatusCode::OK
        );

        let info = get_event(public.clone(), Some(secret)).await.unwrap().info;
        assert!(info.tags.tags.iter().any(|t| t.name == "keynote"));
        assert_eq!(info.tags.current_tag, Some(shared::TagId(0)));
        assert!(info.context.iter().any(|c| c.label == "Slides"));
        assert_eq!(info.data.color, Some(shared::Color("#ff2c5e".into())));
        assert_eq!(info.data.name, "Premium Title");
    }
}
