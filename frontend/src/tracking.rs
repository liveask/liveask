use wasm_bindgen::prelude::wasm_bindgen;

use crate::environment::{la_env, LiveAskEnv};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn track_event_js(metrical: &str, fathom: &str);
}

pub fn track_event((metrical, fathom_idx): (&str, usize)) {
    if !matches!(la_env(Some(env!("LA_ENV"))), LiveAskEnv::Local) {
        track_event_js(metrical, EVNT_FATHOM_IDS[fathom_idx]);
    }
}

pub const EVNT_NEWEVENT_FINISH: (&str, usize) = ("newevent_finish", 0);
pub const EVNT_ASK_OPEN: (&str, usize) = ("ask_open", 1);
pub const EVNT_ASK_SENT: (&str, usize) = ("ask_sent", 2);
pub const EVNT_EVENT_DELETE: (&str, usize) = ("event_delete", 3);
pub const EVNT_SHARE_OPEN: (&str, usize) = ("share_open", 4);
pub const EVNT_QUESTION_LIKE: (&str, usize) = ("question_like", 5);
pub const EVNT_QUESTION_UNLIKE: (&str, usize) = ("question_unlike", 6);
pub const EVNT_PREMIUM_EXPAND: (&str, usize) = ("premium_expand", 7);
pub const EVNT_PREMIUM_UPGRADE: (&str, usize) = ("premium_upgrade", 8);

const EVNT_FATHOM_IDS_BETA: &[&str] = &[
    "FGTHLILK", "PTYICP9D", "2QLZ08FA", "RPUPYLYB", "MNJ3ZBU9", "1O6TRFHR", "D56OBEJZ", "PZMXZBMP",
    "KW4PIK1U",
];
const EVNT_FATHOM_IDS_PROD: &[&str] = &[
    "CE3E5DQE", "YJOUOV25", "Z6JYJXLR", "IWTNGV5P", "KPLTI4YY", "BUFQIMQI", "VNYQXL7D", "ZHOUYH0B",
    "LYLSJMGT",
];

const EVNT_FATHOM_IDS: &[&str] = fathom_ids();

const fn fathom_ids() -> &'static [&'static str] {
    match la_env(Some(env!("LA_ENV"))) {
        LiveAskEnv::Prod => EVNT_FATHOM_IDS_PROD,
        LiveAskEnv::Beta | LiveAskEnv::Local => EVNT_FATHOM_IDS_BETA,
    }
}
