use std::{cell::RefCell, rc::Rc};

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

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GlobalEvents {
    callbacks: Rc<RefCell<Vec<Callback<GlobalEvent>>>>,
}

impl GlobalEvents {
    pub fn emit(&self, e: GlobalEvent) {
        log::info!("event emit: {}", self.callbacks.borrow().len());

        for c in self.callbacks.borrow().iter() {
            c.emit(e);
        }
    }

    pub fn subscribe(&mut self, e: Callback<GlobalEvent>) {
        log::info!("event subscribe: {}", self.callbacks.borrow().len());

        self.callbacks.borrow_mut().push(e);
    }
}
