mod in_memory;

pub use in_memory::PubSubInMemory;

use async_trait::async_trait;

#[async_trait]
pub trait PubSubPublish: Send + Sync {
    async fn publish(&self, topic: &str, payload: &str);
}

#[async_trait]
pub trait PubSubReceiver: Send + Sync {
    async fn notify(&self, topic: &str, payload: &str);
}
