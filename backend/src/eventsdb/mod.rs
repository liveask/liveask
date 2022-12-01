mod dynamo;
mod error;
mod in_memory;

pub use dynamo::DynamoEventsDB;
pub use error::{Error, Result};
pub use in_memory::InMemoryEventsDB;

use crate::utils::timestamp_now;
use async_trait::async_trait;
use shared::EventInfo;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct EventEntry {
    pub event: EventInfo,
    pub version: usize,
}

impl EventEntry {
    pub const fn new(event: EventInfo) -> Self {
        Self { event, version: 0 }
    }

    pub fn bump(&mut self) {
        self.version += 1;
        self.event.last_edit_unix = timestamp_now();
    }
}

pub fn event_key(key: &str) -> String {
    format!("events/ev-{key}.json")
}

#[async_trait]
pub trait EventsDB: Send + Sync {
    async fn get(&self, key: &str) -> Result<EventEntry>;
    async fn put(&self, event: EventEntry) -> Result<()>;
}
