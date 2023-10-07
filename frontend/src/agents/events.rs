use serde::{Deserialize, Serialize};

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
