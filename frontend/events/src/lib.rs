use std::{cell::RefCell, ops::Deref, rc::Rc};

use yew::prelude::*;

#[derive(Debug, PartialEq)]
pub struct EventCallback<T>
where
    T: PartialEq,
{
    id: usize,
    callback: Callback<T>,
}

impl<T: PartialEq> EventCallback<T> {
    pub fn emit(&self, e: T) {
        self.callback.emit(e);
    }
}

#[derive(Debug)]
pub struct EventBridge<T>
where
    T: PartialEq + Clone,
{
    id: usize,
    events: Events<T>,
}

impl<T: PartialEq + Clone> Deref for EventBridge<T> {
    type Target = Events<T>;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl<T: PartialEq + Clone> Drop for EventBridge<T> {
    fn drop(&mut self) {
        self.events.unsubscribe(self.id);

        // log::info!("event unsubscribed: {}", self.id);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Events<T>
where
    T: PartialEq + Clone,
{
    ids: Rc<RefCell<usize>>,
    callbacks: Rc<RefCell<Vec<EventCallback<T>>>>,
}

impl<T: PartialEq + Clone> Default for Events<T> {
    fn default() -> Self {
        Self {
            ids: Rc::default(),
            callbacks: Rc::default(),
        }
    }
}

#[must_use]
pub fn event_context<T: PartialEq + Clone + 'static, C: Component>(
    ctx: &Context<C>,
) -> Option<Events<T>> {
    let (events, _) = ctx.link().context::<Events<T>>(Callback::noop())?;

    Some(events)
}

impl<T: PartialEq + Clone> Events<T> {
    #[allow(clippy::needless_pass_by_value)]
    pub fn emit(&self, e: T) {
        // log::info!("event emit: {}", self.callbacks.borrow().len());

        for c in self.callbacks.borrow().iter() {
            c.emit(e.clone());
        }
    }

    pub fn subscribe(&mut self, e: Callback<T>) -> EventBridge<T> {
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
