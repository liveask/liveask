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

pub async fn server_id() -> Option<String> {
    let url = std::env::var("ECS_CONTAINER_METADATA_URI_V4").ok()?;
    let url = format!("{url}/task");
    let res: Response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    Some(
        res.task_arn
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string(),
    )
}
