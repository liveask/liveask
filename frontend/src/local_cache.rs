use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug)]
struct EventStore {
    likes: HashSet<i64>,
}

pub struct LocalCache;

impl LocalCache {
    pub fn is_liked(event: &str, id: i64) -> bool {
        Self::get_state(event).contains(&id)
    }

    pub fn set_like_state(event: &str, id: i64, like: bool) {
        let mut store = Self::get_state(event);
        if like {
            store.insert(id);
        } else {
            store.remove(&id);
        }
        Self::set_state(event, store);
    }

    fn get_state(event: &str) -> HashSet<i64> {
        LocalStorage::get(event)
            .map(|state: EventStore| state.likes)
            .unwrap_or_default()
    }

    fn set_state(event: &str, likes: HashSet<i64>) {
        LocalStorage::set(event, EventStore { likes }).unwrap();
    }
}
