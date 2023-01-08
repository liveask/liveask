use super::Viewers;
use async_trait::async_trait;
use redis::AsyncCommands;
use tracing::instrument;

pub struct RedisViewers {
    redis: deadpool_redis::Pool,
}

impl RedisViewers {
    pub const fn new(pool: deadpool_redis::Pool) -> Self {
        Self { redis: pool }
    }
}

#[async_trait]
impl Viewers for RedisViewers {
    #[instrument(skip(self))]
    async fn count(&self, key: &str) -> i64 {
        if let Ok(mut db) = self.redis.get().await {
            db.get::<_, i64>(create_key(key)).await.unwrap_or_default()
        } else {
            0
        }
    }

    #[instrument(skip(self))]
    async fn add(&self, key: &str) {
        if let Ok(mut db) = self.redis.get().await {
            db.incr::<_, isize, isize>(create_key(key), 1).await.ok();
        }
    }

    #[instrument(skip(self))]
    async fn remove(&self, key: &str) {
        if let Ok(mut db) = self.redis.get().await {
            db.decr::<_, isize, isize>(create_key(key), 1).await.ok();
        }
    }
}

fn create_key(key: &str) -> String {
    format!("viewers/{key}")
}
