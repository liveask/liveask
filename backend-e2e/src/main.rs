fn main() {}

#[cfg(test)]
mod test {
    use grillon::{
        header::{self, HeaderValue, CONTENT_TYPE},
        json, Grillon, StatusCode,
    };
    use tungstenite::connect;

    fn server_rest() -> String {
        std::env::var("URL").unwrap_or_else(|_| "http://localhost:8090".into())
    }
    fn server_socket() -> String {
        std::env::var("SOCKET_URL").unwrap_or_else(|_| "ws://localhost:8090".into())
    }

    #[tokio::test]
    async fn test_status() {
        Grillon::new(&server_rest())
            .unwrap()
            .get("api/ping")
            .assert()
            .await
            .status_success()
            .status(StatusCode::OK)
            .headers_exist(vec![(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=UTF-8"),
            )])
            .body(json!("pong"));
    }

    #[tokio::test]
    async fn test_add_event() {
        let res = Grillon::new(&server_rest())
            .unwrap()
            .post("api/addevent")
            .payload(json!({
                "eventData":{
                    "maxLikes":0,
                    "name":"foobar foo",
                    "description":"fancy description",
                    "shortUrl":"",
                    "longUrl":null},
                "moderatorEmail": "",
            }))
            .headers(vec![(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )])
            .assert()
            .await
            .status_success()
            .status(StatusCode::OK)
            .headers_exist(vec![(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=UTF-8"),
            )])
            .assert_fn(|assert| {
                assert!(assert.json.is_some());
            });

        let json = res.json.unwrap();

        assert_eq!(json.get("data").unwrap().get("name").unwrap(), "foobar foo");
    }

    async fn add_event() -> String {
        let res = Grillon::new(&server_rest())
            .unwrap()
            .post("api/addevent")
            .payload(json!({
                "eventData":{
                    "maxLikes":0,
                    "name":"foobar foo",
                    "description":"fancy description",
                    "shortUrl":"",
                    "longUrl":null},
                "moderatorEmail": "",
            }))
            .headers(vec![(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )])
            .assert()
            .await
            .status_success()
            .status(StatusCode::OK)
            .headers_exist(vec![(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=UTF-8"),
            )])
            .assert_fn(|assert| {
                assert!(assert.json.is_some());
            });

        let res = res.json.unwrap();

        res.get("tokens")
            .unwrap()
            .get("publicToken")
            .unwrap()
            .as_str()
            .unwrap()
            .into()
    }

    async fn add_question(event: String) -> String {
        let res = Grillon::new(&server_rest())
            .unwrap()
            .post(&format!("api/event/addquestion/{}", event))
            .payload(json!({
                "text":"test"
            }))
            .headers(vec![(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            )])
            .assert()
            .await
            .status_success()
            .status(StatusCode::OK)
            .headers_exist(vec![(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=UTF-8"),
            )])
            .assert_fn(|assert| {
                assert!(assert.json.is_some());
                assert_eq!(
                    assert
                        .json
                        .as_ref()
                        .unwrap()
                        .get("text")
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    "test"
                );
            });

        let res = res.json.unwrap();

        res.get("id").unwrap().to_string()
    }

    #[tokio::test]
    async fn test_websockets() {
        let event = add_event().await;

        let (mut socket, response) =
            connect(&format!("{}/push/{}", server_socket(), event)).expect("Can't connect");

        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

        let question = add_question(event).await;

        let msg = socket.read_message().expect("Error reading message");
        assert_eq!(msg.into_text().unwrap(), format!("q:{}", question));
    }
}
