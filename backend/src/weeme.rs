use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateLinkRequest {
    pub url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CreatedLinkResponse {
    pub id: String,
    pub shortened_url: String,
    pub original_url: String,
}

pub async fn create_short_url(key: &str, original_link: &str) -> Option<String> {
    let url: Url = "https://weeme.io/link/create".parse().ok()?;

    let client = reqwest::Client::new();

    let resp = client
        .post(url)
        .header("Authorization", key)
        .json(&CreateLinkRequest {
            url: original_link.to_string(),
        })
        .send()
        .await
        .map_err(|e| {
            tracing::error!("weeme error: {e}");
        })
        .ok()?
        .json::<CreatedLinkResponse>()
        .await
        .map_err(|e| {
            tracing::error!("weeme deserialize error: {e}");
        })
        .ok()?;

    Some(resp.shortened_url)
}
