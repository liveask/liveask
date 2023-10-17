use chrono::Utc;

pub fn timestamp_now() -> i64 {
    let now = Utc::now();
    now.timestamp_millis().saturating_div(1000)
}
