use std::{collections::HashMap, sync::Arc};

use anyhow::{bail, Result};
use shared::{AddEvent, EventInfo, EventTokens, Item};
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

    pub async fn add_question(&self, id: String, question: shared::AddQuestion) -> Result<Item> {
        let mut events = self.events.write().await;

        let question_id = events
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .questions
            .len() as i64;

        let question = shared::Item {
            text: question.text,
            answered: false,
            create_time_unix: 0,
            hidden: false,
            id: question_id,
            likes: 1,
        };

        events
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .questions
            .push(question.clone());

        Ok(question)
    }

    pub async fn edit_like(&self, id: String, edit: shared::EditLike) -> Result<Item> {
        let mut e = self
            .events
            .read()
            .await
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("ev not found"))?
            .clone();

        if let Some(f) = e.questions.iter_mut().find(|e| e.id == edit.question_id) {
            f.likes = if edit.like {
                f.likes + 1
            } else {
                f.likes.saturating_sub(1)
            };

            let res = f.clone();

            self.events.write().await.insert(id, e.clone());

            Ok(res)
        } else {
            bail!("question not found")
        }
    }
}
