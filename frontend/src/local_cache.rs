use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use shared::QuestionItem;
use std::collections::HashSet;
use wasm_bindgen::UnwrapThrowExt;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct EventStore {
    likes: HashSet<i64>,
    unscreened: Vec<QuestionItem>,
}

pub struct LocalCache;

impl LocalCache {
    pub fn is_liked(event: &str, id: i64) -> bool {
        Self::get_state(event).likes.contains(&id)
    }

    pub fn set_like_state(event: &str, id: i64, like: bool) {
        let mut store = Self::get_state(event);
        if like {
            store.likes.insert(id);
        } else {
            store.likes.remove(&id);
        }
        Self::set_state(event, store);
    }

    pub fn add_unscreened_question(event: &str, q: &QuestionItem) {
        // log::info!("question pending review: {}", q.id);
        let mut store = Self::get_state(event);
        store.unscreened.push(q.clone());
        Self::set_state(event, store);
    }

    pub fn unscreened_questions(event: &str, questions: &[QuestionItem]) -> Vec<QuestionItem> {
        let mut state = Self::get_state(event);

        let question_ids: HashSet<i64> = questions.iter().map(|q| q.id).collect();

        let amount_before_retain = state.unscreened.len();

        state.unscreened.retain(|q| !question_ids.contains(&q.id));

        if amount_before_retain != state.unscreened.len() {
            Self::set_state(event, state.clone());
        }

        state.unscreened
    }

    fn get_state(event: &str) -> EventStore {
        LocalStorage::get(event).unwrap_or_default()
    }

    fn set_state(event: &str, data: EventStore) {
        LocalStorage::set(event, data).unwrap_throw();
    }
}
