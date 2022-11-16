use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{PubSubPublish, PubSubReceiver};

#[derive(Clone, Default)]
pub struct PubSubInMemory {
    receiver: Arc<RwLock<Option<Arc<dyn PubSubReceiver>>>>,
}

impl PubSubInMemory {
    pub async fn set_receiver(&self, receiver: Arc<dyn PubSubReceiver>) {
        let mut r = self.receiver.write().await;
        *r = Some(receiver);
    }
}

#[async_trait]
impl PubSubPublish for PubSubInMemory {
    async fn publish(&self, topic: &str, payload: &str) {
        let receiver = self.receiver.read().await.clone();
        if let Some(receiver) = receiver {
            receiver.notify(topic, payload).await;
        } else {
            tracing::error!("no receiver registered");
        }
    }
}
