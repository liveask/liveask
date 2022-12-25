use chrono::Utc;

pub fn timestamp_now() -> i64 {
    let now = Utc::now();
    now.timestamp_millis() / 1000
}
