use super::{
    error::{Error, Result},
    event_key, EventEntry, EventsDB,
};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing::instrument;

#[derive(Default)]
pub struct InMemoryEventsDB {
    pub db: Arc<Mutex<HashMap<String, EventEntry>>>,
}

#[async_trait]
impl EventsDB for InMemoryEventsDB {
    #[instrument(skip(self), err)]
    async fn get(&self, key: &str) -> Result<EventEntry> {
        let db = self.db.lock().await;

        let key = event_key(key);

        db.get(&key)
            .cloned()
            .ok_or_else(|| Error::General("failed to get".to_string()))
    }

    #[instrument(skip(self), err)]
    #[allow(clippy::significant_drop_tightening)]
    async fn put(&self, event: EventEntry) -> Result<()> {
        let key = event_key(&event.event.tokens.public_token);

        let mut db = self.db.lock().await;

        if let Some(db_event) = db.get_mut(&key) {
            if event.version <= db_event.version {
                return Err(Error::Concurrency);
            }
            *db_event = event;
        } else {
            db.insert(key, event);
        }

        Ok(())
    }
}
