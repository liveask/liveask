fn main() {}

#[cfg(test)]
mod test {
    // use grillon::{
    //     header::{self, HeaderValue, CONTENT_TYPE},
    //     json, Grillon, StatusCode,
    // };
    use reqwest::{
        header::{HeaderValue, CONTENT_TYPE},
        StatusCode,
    };
    use serde_json::json;
    use tungstenite::connect;

    fn server_rest() -> String {
        std::env::var("URL").unwrap_or_else(|_| "http://localhost:8090".into())
    }
    fn server_socket() -> String {
        std::env::var("SOCKET_URL").unwrap_or_else(|_| "ws://localhost:8090".into())
    }

    #[tokio::test]
    async fn test_status() {
        // env_logger::init();

        let resp = reqwest::get(format!("{}/api/ping", server_rest()))
            .await
            .unwrap();

        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("application/json; charset=UTF-8")
        );
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.json::<String>().await.unwrap(), "pong");
    }

    #[tokio::test]
    async fn test_add_event() {
        let res = reqwest::Client::new()
            .post(format!("{}/api/addevent", server_rest()))
            .json(&json!({
                "eventData":{
                    "maxLikes":0,
                    "name":"foobar foo",
                    "description":"fancy description",
                    "shortUrl":"",
                    "longUrl":null},
                "moderatorEmail": "",
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("application/json; charset=UTF-8")
        );
        assert_eq!(res.status(), StatusCode::OK);

        let json = res.json::<serde_json::Value>().await.unwrap();

        assert_eq!(json.get("data").unwrap().get("name").unwrap(), "foobar foo");
    }

    async fn add_event() -> String {
        let res = reqwest::Client::new()
            .post(format!("{}/api/addevent", server_rest()))
            .json(&json!({
                "eventData":{
                    "maxLikes":0,
                    "name":"foobar foo",
                    "description":"fancy description",
                    "shortUrl":"",
                    "longUrl":null},
                "moderatorEmail": "",
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("application/json; charset=UTF-8")
        );
        assert_eq!(res.status(), StatusCode::OK);

        let json = res.json::<serde_json::Value>().await.unwrap();

        json.get("tokens")
            .unwrap()
            .get("publicToken")
            .unwrap()
            .as_str()
            .unwrap()
            .into()
    }

    async fn add_question(event: String) -> String {
        let res = reqwest::Client::new()
            .post(format!("{}/api/event/addquestion/{}", server_rest(), event))
            .json(&json!({
                "text":"test"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("application/json; charset=UTF-8")
        );
        assert_eq!(res.status(), StatusCode::OK);

        let json = res.json::<serde_json::Value>().await.unwrap();

        assert_eq!(json.get("text").unwrap().as_str().unwrap(), "test");

        json.get("id").unwrap().to_string()
    }

    #[tokio::test]
    async fn test_websockets() {
        env_logger::init();

        let event = add_event().await;

        let (mut socket, response) =
            connect(&format!("{}/push/{}", server_socket(), event)).expect("Can't connect");

        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

        let question = add_question(event).await;

        let msg = socket.read_message().expect("Error reading message");
        assert_eq!(msg.into_text().unwrap(), format!("q:{}", question));
    }
}
