use async_trait::async_trait;
use redis::AsyncCommands;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::instrument;

use super::{PubSubPublish, PubSubReceiver};
use crate::error::Result;

#[derive(Clone)]
pub struct PubSubRedis {
    receiver: Arc<RwLock<Option<Arc<dyn PubSubReceiver>>>>,
    redis: deadpool_redis::Pool,
    url: String,
}

impl PubSubRedis {
    pub async fn new(redis: deadpool_redis::Pool, url: String) -> Self {
        let new = Self {
            redis,
            url,
            receiver: Arc::new(RwLock::new(None)),
        };

        let new_res = new.clone();

        tokio::spawn(async move {
            let sub = new.clone();
            loop {
                if let Err(e) = sub.subscriber_task().await {
                    tracing::error!("subscriber err: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        new_res
    }

    pub async fn set_receiver(&self, receiver: Arc<dyn PubSubReceiver>) {
        let mut r = self.receiver.write().await;
        *r = Some(receiver);
    }

    #[instrument(skip(self), err)]
    async fn subscriber_task(&self) -> Result<()> {
        tracing::info!("subscriber_task");

        let client = redis::Client::open(self.url.clone())?;
        let client = client.get_tokio_connection().await?;

        let mut pubsub = client.into_pubsub();
        pubsub.psubscribe("la/*").await?;

        tracing::info!("subscribed");

        let mut pubsub_stream = pubsub.on_message();

        while let Some(msg) = pubsub_stream.next().await {
            let payload: String = msg.get_payload()?;
            let topic = msg.get_channel_name();

            tracing::debug!(target: "received", bytes = payload.len(), topic = ?topic);

            if let Some(topic) = topic.strip_prefix("la/") {
                self.forward_to_receiver(topic.to_string(), payload).await;
            }
        }

        Ok(())
    }

    async fn forward_to_receiver(&self, topic: String, payload: String) {
        let receiver = self.receiver.clone();

        tokio::spawn(async move {
            let topic = topic.clone();
            let payload = payload.clone();

            let receiver = receiver.read().await.clone();
            if let Some(receiver) = receiver {
                receiver.notify(&topic, &payload).await;
            } else {
                tracing::error!("no receiver registered");
            }
        });
    }
}

#[async_trait]
impl PubSubPublish for PubSubRedis {
    async fn publish(&self, topic: &str, payload: &str) {
        if let Ok(mut db) = self.redis.get().await {
            if let Err(e) = db.publish::<_, _, ()>(format!("la/{topic}"), payload).await {
                tracing::error!("publish err: {}", e);
            }
        }
    }
}
