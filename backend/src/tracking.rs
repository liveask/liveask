use std::collections::HashMap;

use posthog_rs::{ClientOptions, Event};
use shared::EventData;

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

    pub fn track_server_start(&self) {
        let tracking = self.clone();

        tokio::task::spawn_blocking(move || {
            if let Err(e) = tracking.logger("server-start", None) {
                tracing::error!("posthog error: {e}");
            }
        });
    }

    pub fn track_event_password_set(&self, event: String, edit: EditEvent) {
        let tracking = self.clone();

        tokio::task::spawn_blocking(move || {
            let mut data = HashMap::with_capacity(1);
            data.insert("event".to_string(), event);
            data.insert("edit".to_string(), format!("{edit:?}"));

            if let Err(e) = tracking.logger("event-pwd", Some(data)) {
                tracing::error!("posthog error: {e}");
            }
        });
    }

    pub fn track_event_create(&self, event: String, url: String, name: String) {
        let tracking = self.clone();

        tokio::task::spawn_blocking(move || {
            let mut data = HashMap::with_capacity(1);
            data.insert("event".to_string(), event);
            data.insert("url".to_string(), url);
            data.insert("name".to_string(), name);
            if let Err(e) = tracking.logger("event-created", Some(data)) {
                tracing::error!("posthog error: {e}");
            }
        });
    }

    pub fn track_event_upgrade(&self, event: &str, data: &EventData) {
        let tracking = self.clone();
        let name = data.name.clone();
        let url = data.long_url.clone().unwrap_or_default();
        let event = event.to_string();

        tokio::task::spawn_blocking(move || {
            let mut data = HashMap::with_capacity(1);
            data.insert("event".to_string(), event);
            data.insert("url".to_string(), url);
            data.insert("name".to_string(), name);
            if let Err(e) = tracking.logger("event-upgraded", Some(data)) {
                tracing::error!("posthog error: {e}");
            }
        });
    }

    fn logger(
        &self,
        event: &str,
        properties: Option<HashMap<String, String>>,
    ) -> std::result::Result<(), posthog_rs::Error> {
        if let Some(key) = &self.key {
            let mut client = ClientOptions::new(key);
            client.api_endpoint("https://eu.posthog.com");
            let client = client.build();

            let mut event = Event::new(event, &self.server);
            event
                .insert_prop("env", &self.env)
                .map_err(|e| posthog_rs::Error::PostHogCore { source: e })?;
            event
                .insert_prop("git", GIT_HASH)
                .map_err(|e| posthog_rs::Error::PostHogCore { source: e })?;

            if let Some(properties) = properties {
                for (k, v) in properties {
                    event
                        .insert_prop(k, v)
                        .map_err(|e| posthog_rs::Error::PostHogCore { source: e })?;
                }
            }

            client.capture(event)?;
        }

        Ok(())
    }
}
