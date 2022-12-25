mod in_memory;
mod redis;

pub use self::redis::PubSubRedis;
pub use in_memory::{PubSubInMemory, PubSubReceiverInMemory};

use async_trait::async_trait;

#[async_trait]
pub trait PubSubPublish: Send + Sync {
    async fn publish(&self, topic: &str, payload: &str);
}

#[async_trait]
pub trait PubSubReceiver: Send + Sync {
    async fn notify(&self, topic: &str, payload: &str);
}
