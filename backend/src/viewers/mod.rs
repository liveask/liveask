mod redis;

pub use crate::viewers::redis::RedisViewers;

use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Viewers: Send + Sync {
    async fn count(&self, key: &str) -> i64;
    async fn add(&self, key: &str);
    async fn remove(&self, key: &str);
}
