use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use yew_agent::{Agent, AgentLink, HandlerId};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum GlobalEvent {
    OpenSharePopup,
    OpenQuestionPopup,
}

pub struct EventAgent {
    link: AgentLink<Self>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for EventAgent {
    type Reach = yew_agent::Context<Self>;
    type Message = GlobalEvent;
    type Input = GlobalEvent;
    type Output = GlobalEvent;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        self.respond_to_all(&msg);
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }

    fn destroy(&mut self) {}
}

impl EventAgent {
    fn respond_to_all(&self, response: &GlobalEvent) {
        for sub in &self.subscribers {
            self.link.respond(*sub, response.clone());
        }
    }
}
