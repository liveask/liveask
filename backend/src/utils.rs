use chrono::{TimeZone, Utc};

pub fn timestamp_now() -> i64 {
    let now = Utc::now();
    now.timestamp_millis() / 1000
}

pub fn format_timestamp(t: i64) -> String {
    let date = Utc.timestamp(t, 0);
    date.format("%Y%m%dT%H%M%S").to_string()
}

#[cfg(test)]
mod test {
    use super::format_timestamp;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_time_format() {
        assert_eq!(
            format_timestamp(1589961534),
            String::from("20200520T075854")
        );
    }
}
