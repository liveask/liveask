use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename(serialize = "Cluster"))]
    pub cluster: String,
    #[serde(rename(serialize = "TaskARN"))]
    pub task_arn: String,
    #[serde(rename(serialize = "Revision"))]
    pub revision: String,
    #[serde(rename(serialize = "Family"))]
    pub family: String,
    #[serde(rename(serialize = "DesiredStatus"))]
    pub desired_status: String,
    #[serde(rename(serialize = "KnownStatus"))]
    pub known_status: String,
}

#[allow(clippy::cognitive_complexity)]
pub async fn server_id() -> Option<String> {
    let url = std::env::var("ECS_CONTAINER_METADATA_URI_V4").ok()?;

    tracing::info!(url, "[server-starting] read env",);

    let url = format!("{url}/task");
    let response = reqwest::Client::new().get(url).send().await.ok()?;

    tracing::info!("[server-starting] fetched");

    let parsed: Response = response.json().await.ok()?;

    tracing::info!("[server-starting] parsed: {:?}", parsed);

    Some(
        parsed
            .task_arn
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string(),
    )
}
