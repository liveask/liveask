use serde::{Deserialize, Serialize};
use shared::QuestionItem;
use std::collections::HashSet;
use yew_agent::{Agent, AgentLink, HandlerId};

use crate::cloud::create_cloud;

#[derive(Serialize, Deserialize)]
pub struct WordCloudInput(pub Vec<QuestionItem>);

#[derive(Serialize, Deserialize)]
pub struct WordCloudOutput(pub String);

#[allow(dead_code)]
pub struct WordCloudAgent {
    link: AgentLink<Self>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for WordCloudAgent {
    type Reach = yew_agent::Context<Self>;
    type Message = ();
    type Input = WordCloudInput;
    type Output = WordCloudOutput;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, input: Self::Input, id: HandlerId) {
        log::info!("wordcloud requested");

        let text = input
            .0
            .into_iter()
            .map(|q| q.text)
            .collect::<Vec<_>>()
            .join(" ");

        match create_cloud(&text) {
            Ok(cloud) => {
                log::info!("wordcloud generated");

                self.link.respond(id, WordCloudOutput(cloud));

                log::info!("wordcloud send");
            }

            Err(e) => {
                log::error!("wordcloud error: {e}");
            }
        }
    }

    fn name_of_resource() -> &'static str {
        "worker.js"
    }

    fn resource_path_is_relative() -> bool {
        true
    }
}
