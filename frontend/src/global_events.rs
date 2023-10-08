#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    SocketManualReconnect,
}
