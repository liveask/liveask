use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename(deserialize = "Cluster"))]
    pub cluster: String,
    #[serde(rename(deserialize = "TaskARN"))]
    pub task_arn: String,
    #[serde(rename(deserialize = "Revision"))]
    pub revision: String,
    #[serde(rename(deserialize = "Family"))]
    pub family: String,
    #[serde(rename(deserialize = "DesiredStatus"))]
    pub desired_status: String,
    #[serde(rename(deserialize = "KnownStatus"))]
    pub known_status: String,
}

#[allow(clippy::cognitive_complexity)]
pub async fn server_id() -> Option<String> {
    let url = std::env::var("ECS_CONTAINER_METADATA_URI_V4").ok()?;

    let url = format!("{url}/task");
    let response = reqwest::Client::new().get(url).send().await.ok()?;

    let parsed: Response = response.json().await.ok()?;

    Some(
        parsed
            .task_arn
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string(),
    )
}
