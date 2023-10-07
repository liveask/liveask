use std::{cell::RefCell, ops::Deref, rc::Rc};

use serde::{Deserialize, Serialize};
use yew::prelude::*;

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum GlobalEvent {
    SocketStatus {
        connected: bool,
        timeout_secs: Option<i64>,
    },
    OpenSharePopup,
    OpenQuestionPopup,
    DeletePopup,
    QuestionCreated(i64),
    PayForUpgrade,
}

#[derive(Debug, PartialEq)]
pub struct EventCallback {
    id: usize,
    callback: Callback<GlobalEvent>,
}

impl EventCallback {
    pub fn emit(&self, e: GlobalEvent) {
        self.callback.emit(e);
    }
}

#[derive(Debug)]
pub struct EventBridge {
    id: usize,
    events: GlobalEvents,
}

impl Deref for EventBridge {
    type Target = GlobalEvents;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl Drop for EventBridge {
    fn drop(&mut self) {
        self.events.unsubscribe(self.id);

        // log::info!("event unsubscribed: {}", self.id);
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GlobalEvents {
    ids: Rc<RefCell<usize>>,
    callbacks: Rc<RefCell<Vec<EventCallback>>>,
}

impl GlobalEvents {
    pub fn emit(&self, e: GlobalEvent) {
        // log::info!("event emit: {}", self.callbacks.borrow().len());

        for c in self.callbacks.borrow().iter() {
            c.emit(e);
        }
    }

    pub fn subscribe(&mut self, e: Callback<GlobalEvent>) -> EventBridge {
        let id = *self.ids.borrow();

        (*self.ids.borrow_mut()) += 1;

        // log::info!("event subscribe: {}", id);

        let bridge = EventBridge {
            events: self.clone(),
            id,
        };

        self.callbacks
            .borrow_mut()
            .push(EventCallback { id, callback: e });

        bridge
    }

    fn unsubscribe(&mut self, id: usize) {
        let pos = self.callbacks.borrow().iter().position(|e| e.id == id);
        if let Some(idx) = pos {
            self.callbacks.borrow_mut().remove(idx);
        }
    }
}
