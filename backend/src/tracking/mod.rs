mod error;

pub use error::TrackingError;

use error::TrackingResult;

use async_posthog::{ClientOptions, Event};

use crate::GIT_HASH;

#[derive(Clone, Default)]
pub struct Tracking {
    key: Option<String>,
    server: String,
    env: String,
}

#[derive(Clone, Debug)]
pub enum EditEvent {
    Enabled,
    Changed,
    Disabled,
}

impl Tracking {
    pub const fn new(key: Option<String>, server: String, env: String) -> Self {
        Self { key, server, env }
    }

    pub async fn track_server_start(&self) -> TrackingResult<()> {
        self.log(Event::new("event-start", &self.server)).await?;
        Ok(())
    }

    pub async fn track_event_password_set(
        &self,
        event: String,
        edit: EditEvent,
    ) -> TrackingResult<()> {
        let mut e = Event::new("event-pwd", &self.server);

        e.insert_prop("event", event)?;
        e.insert_prop("edit", format!("{edit:?}"))?;

        self.log(e).await?;

        Ok(())
    }

    pub async fn track_event_tag_set(
        &self,
        event: String,
        edit: EditEvent,
        age: i64,
    ) -> TrackingResult<()> {
        let mut e = Event::new("event-tag", &self.server);

        e.insert_prop("event", event)?;
        e.insert_prop("edit", format!("{edit:?}"))?;
        e.insert_prop("age", age)?;

        self.log(e).await?;

        Ok(())
    }

    pub async fn track_event_create(
        &self,
        event: String,
        url: String,
        name: String,
    ) -> TrackingResult<()> {
        let mut e = Event::new("event-created", &self.server);

        e.insert_prop("event", event)?;
        e.insert_prop("name", name)?;
        e.insert_prop("url", url)?;

        self.log(e).await?;

        Ok(())
    }

    pub async fn track_event_upgrade(
        &self,
        event: String,
        name: String,
        long_url: String,
        age: i64,
        order_type: &str,
    ) -> TrackingResult<()> {
        let mut e = Event::new("event-upgraded", &self.server);

        e.insert_prop("event", event)?;
        e.insert_prop("name", name)?;
        e.insert_prop("url", long_url)?;
        e.insert_prop("age", age)?;
        e.insert_prop("order_type", order_type)?;

        self.log(e).await?;

        Ok(())
    }

    pub async fn track_event_context_set(
        &self,
        event: String,
        label: &str,
        url: &str,
    ) -> TrackingResult<()> {
        let mut e = Event::new("event-context", &self.server);

        e.insert_prop("event", event)?;
        e.insert_prop("context-label", label)?;
        e.insert_prop("context-url", url)?;

        self.log(e).await?;

        Ok(())
    }

    pub async fn track_event_meta_change(
        &self,
        event: String,
        meta: &shared::EditMetaData,
    ) -> TrackingResult<()> {
        let mut e = Event::new("event-meta-changed", &self.server);

        e.insert_prop("event", event)?;
        e.insert_prop("title", &meta.title)?;
        e.insert_prop("desc", &meta.description)?;

        self.log(e).await?;

        Ok(())
    }

    async fn log(&self, event: Event) -> TrackingResult<()> {
        if let Some(key) = &self.key {
            let mut client = ClientOptions::new(key);
            client.api_endpoint("https://eu.posthog.com");
            let client = client.build();

            let mut event = event;

            event.insert_prop("env", &self.env)?;
            event.insert_prop("git", GIT_HASH)?;

            if let Err(e) = client.capture(event).await {
                tracing::error!("posthog error: {e}");
            }
        }

        Ok(())
    }
}
