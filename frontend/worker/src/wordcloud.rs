use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use yew_agent::{HandlerId, Worker, WorkerLink};

use crate::{cloud::create_cloud, pwd::pwd_hash};

static PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
static BUILD_TIME: &str = env!("VERGEN_BUILD_TIMESTAMP");

#[derive(Serialize, Deserialize)]
pub struct WordCloudInput(pub Vec<String>);

#[derive(Serialize, Deserialize)]
pub struct WordCloudOutput(pub String);

#[allow(dead_code)]
pub struct WordCloudAgent {
    link: WorkerLink<Self>,
    subscribers: HashSet<HandlerId>,
    cache: Option<(String, String)>,
}

impl Worker for WordCloudAgent {
    type Reach = yew_agent::Public<Self>;
    type Message = ();
    type Input = WordCloudInput;
    type Output = WordCloudOutput;

    fn create(link: WorkerLink<Self>) -> Self {
        let v = format!("{PACKAGE_VERSION} ({BUILD_TIME})",);

        log::info!(target: "worker", "[wc] worker created: {v}");

        Self {
            link,
            subscribers: HashSet::new(),
            cache: None,
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, input: Self::Input, id: HandlerId) {
        log::info!(target: "worker", "[wc] requested");

        let start = Utc::now();

        let text = input
            .0
            .into_iter()
            // .map(|q| q.text)
            .collect::<Vec<_>>()
            .join(" ");

        let hash = pwd_hash(&text);

        log::info!(target: "worker", "[wc] text generated [{hash}]: {}ms",elapsed(start));

        if let Some((cached_hash, data)) = &self.cache {
            if &hash == cached_hash {
                log::info!(target: "worker", "[wc] result from cache: {}ms",elapsed(start));

                self.link.respond(id, WordCloudOutput(data.clone()));

                log::info!(target: "worker", "[wc] send: {}ms",elapsed(start));

                return;
            }
        }

        match create_cloud(&text) {
            Ok(cloud) => {
                log::info!(target: "worker", "[wc] generated: {}ms",elapsed(start));

                self.link.respond(id, WordCloudOutput(cloud.clone()));

                log::info!(target: "worker", "[wc] send: {}ms",elapsed(start));

                self.cache = Some((hash, cloud));

                log::info!(target: "worker", "[wc] cached: {}ms",elapsed(start));
            }

            Err(e) => {
                log::error!(target: "worker", "[wc] error: {e}");
            }
        }
    }

    fn name_of_resource() -> &'static str {
        "worker2.js"
    }

    fn resource_path_is_relative() -> bool {
        false
    }
}

fn elapsed(start: DateTime<Utc>) -> i64 {
    let now = Utc::now();
    (now - start).num_milliseconds()
}
