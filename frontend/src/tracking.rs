use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::environment::{LiveAskEnv, la_env};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window,catch)]
    fn track_event_js(fathom: &str) -> Result<(), JsValue>;
}

pub fn track_event(fathom_idx: usize) {
    if !matches!(la_env(Some(env!("LA_ENV"))), LiveAskEnv::Local) {
        if let Err(e) = track_event_js(EVNT_FATHOM_IDS[fathom_idx]) {
            log::error!("track_event_js error: {:?}", e);
        }
    }
}

pub const EVNT_NEWEVENT_FINISH: usize = 0;
pub const EVNT_ASK_OPEN: usize = 1;
pub const EVNT_ASK_SENT: usize = 2;
pub const EVNT_EVENT_DELETE: usize = 3;
pub const EVNT_SHARE_OPEN: usize = 4;
pub const EVNT_QUESTION_LIKE: usize = 5;
pub const EVNT_QUESTION_UNLIKE: usize = 6;
pub const EVNT_PREMIUM_EXPAND: usize = 7;
pub const EVNT_PREMIUM_UPGRADE: usize = 8;
pub const EVNT_EXPORT: usize = 9;
pub const EVNT_SURVEY_OPENED: usize = 10;

const EVNT_FATHOM_IDS_BETA: &[&str] = &[
    "FGTHLILK", "PTYICP9D", "2QLZ08FA", "RPUPYLYB", "MNJ3ZBU9", "1O6TRFHR", "D56OBEJZ", "PZMXZBMP",
    "KW4PIK1U", "XMK8M2CD", "BQOVFVM5",
];
const EVNT_FATHOM_IDS_PROD: &[&str] = &[
    "CE3E5DQE", "YJOUOV25", "Z6JYJXLR", "IWTNGV5P", "KPLTI4YY", "BUFQIMQI", "VNYQXL7D", "ZHOUYH0B",
    "LYLSJMGT", "OAZVXYRC", "KH51IDSN",
];

const EVNT_FATHOM_IDS: &[&str] = fathom_ids();

const fn fathom_ids() -> &'static [&'static str] {
    match la_env(Some(env!("LA_ENV"))) {
        LiveAskEnv::Prod => EVNT_FATHOM_IDS_PROD,
        LiveAskEnv::Beta | LiveAskEnv::Local => EVNT_FATHOM_IDS_BETA,
    }
}
