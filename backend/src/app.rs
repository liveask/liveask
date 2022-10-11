use std::{collections::HashMap, sync::Arc};

use anyhow::{bail, Result};
use shared::{AddEvent, EventInfo, EventTokens};
use tokio::sync::RwLock;
use ulid::Ulid;

#[derive(Clone, Default, Debug)]
pub struct App {
    events: Arc<RwLock<HashMap<String, EventInfo>>>,
}

impl App {
    pub async fn create_event(&self, request: AddEvent) -> Result<EventInfo> {
        let e = EventInfo {
            //TODO:
            create_time_unix: 0,
            delete_time_unix: 0,
            last_edit_unix: 0,
            create_time_utc: String::new(),
            deleted: false,
            questions: Vec::new(),
            data: request.data,
            tokens: EventTokens {
                public_token: Ulid::new().to_string(),
                moderator_token: Some(Ulid::new().to_string()),
            },
        };

        self.events
            .write()
            .await
            .insert(e.tokens.public_token.clone(), e.clone());

        Ok(e)
    }

    pub async fn get_event(&self, id: String, secret: Option<String>) -> Result<EventInfo> {
        let mut e = self
            .events
            .read()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

        if let Some(secret) = &secret {
            if e.tokens
                .moderator_token
                .as_ref()
                .map(|mod_token| mod_token != secret)
                .unwrap_or_default()
            {
                bail!("wrong mod token");
            }
        }

        if secret.is_none() {
            e.tokens.moderator_token = Some(String::new());
        }

        Ok(e)
    }
}
