use base64::{Engine, engine::general_purpose};
use sha2::{Digest, Sha256};

#[must_use]
pub fn pwd_hash(pwd: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pwd.as_bytes());
    general_purpose::STANDARD_NO_PAD.encode(hasher.finalize())
}
