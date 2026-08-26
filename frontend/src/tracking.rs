use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::environment::{LiveAskEnv, la_env};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window,catch)]
    fn track_event_js(name: &str) -> Result<(), JsValue>;
}

pub fn track_event(name: &str) {
    if !matches!(la_env(Some(env!("LA_ENV"))), LiveAskEnv::Local)
        && let Err(e) = track_event_js(name)
    {
        log::error!("track_event_js error: {:?}", e);
    }
}

// Central list of Fathom event names. With the new event system these are plain
// strings sent via `trackEvent` - no dashboard pre-setup and no per-env goal ids.
// Adding an event is just a new const here; names are permanent once fired.
pub const EVNT_NEWEVENT_FINISH: &str = "new-event-finish";
pub const EVNT_ASK_OPEN: &str = "ask-open";
pub const EVNT_ASK_SENT: &str = "ask-sent";
pub const EVNT_EVENT_DELETE: &str = "event-delete";
pub const EVNT_SHARE_OPEN: &str = "share-open";
pub const EVNT_QUESTION_LIKE: &str = "question-like";
pub const EVNT_QUESTION_UNLIKE: &str = "question-unlike";
pub const EVNT_PREMIUM_EXPAND: &str = "premium-expand";
pub const EVNT_PREMIUM_UPGRADE: &str = "premium-upgrade";
pub const EVNT_EXPORT: &str = "export";
pub const EVNT_SURVEY_OPENED: &str = "survey-opened";
