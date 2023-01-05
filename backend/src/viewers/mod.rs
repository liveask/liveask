mod redis;

pub use crate::viewers::redis::RedisViewers;

use async_trait::async_trait;

#[async_trait]
pub trait Viewers: Send + Sync {
    async fn count(&self, key: &str) -> isize;
    async fn add(&self, key: &str);
    async fn remove(&self, key: &str);
}
