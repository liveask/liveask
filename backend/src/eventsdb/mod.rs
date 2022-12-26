mod dynamo;
mod error;
mod in_memory;
mod types;

pub use dynamo::DynamoEventsDB;
pub use error::{Error, Result};
pub use in_memory::InMemoryEventsDB;

use async_trait::async_trait;

pub use self::types::{ApiEventInfo, EventEntry};

pub fn event_key(key: &str) -> String {
    format!("events/ev-{key}.json")
}

#[async_trait]
pub trait EventsDB: Send + Sync {
    async fn get(&self, key: &str) -> Result<EventEntry>;
    async fn put(&self, event: EventEntry) -> Result<()>;
}
