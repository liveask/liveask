use chrono::Utc;
use sha2::{Digest, Sha256};

pub fn timestamp_now() -> i64 {
    let now = Utc::now();
    now.timestamp_millis().saturating_div(1000)
}

/// Stable, non-reversible fingerprint of an event password. Embedded in a pwd grant so the
/// grant is bound to the password value: rotating the password changes the fingerprint and
/// re-locks outstanding grants (matching the old re-check-every-request behaviour).
pub fn pwd_fingerprint(pwd: &str) -> String {
    hex::encode(Sha256::digest(pwd.as_bytes()))
}
