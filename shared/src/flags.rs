#![allow(unknown_lints, clippy::iter_without_into_iter)]

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize, Clone, Debug, Copy, Eq, PartialEq, Default)]
    pub struct EventResponseFlags: u32 {
        const TIMED_OUT = 1 << 0;
        const WRONG_PASSWORD = 1 << 1;
    }
}

bitflags! {

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct EventFlags: u32 {
        const DELETED = 1 << 0;
        const PREMIUM = 1 << 1;
        const SCREENING = 1 << 2;
        const PASSWORD = 1 << 3;
    }
}
