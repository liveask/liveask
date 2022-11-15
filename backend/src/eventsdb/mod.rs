mod dynamo;
mod in_memory;

pub use dynamo::DynamoEventsDB;
pub use in_memory::InMemoryEventsDB;

use anyhow::Result;
use async_trait::async_trait;
use shared::EventInfo;

use crate::utils::timestamp_now;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct EventEntry {
    pub event: EventInfo,
    pub version: usize,
}

impl EventEntry {
    pub fn new(event: EventInfo) -> Self {
        Self { event, version: 0 }
    }

    pub fn bump(&mut self) {
        self.version += 1;
        self.event.last_edit_unix = timestamp_now();
    }
}

pub fn event_key(key: &str) -> String {
    format!("events/ev-{}.json", key)
}

#[async_trait]
pub trait EventsDB: Send + Sync {
    async fn get(&self, key: &str) -> Result<EventEntry>;
    async fn put(&self, event: EventEntry) -> Result<()>;
}
