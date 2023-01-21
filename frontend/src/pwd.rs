use base64::{engine::general_purpose, Engine};
use sha2::{Digest, Sha256};

pub fn pwd_hash(pwd: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pwd.as_bytes());
    general_purpose::STANDARD_NO_PAD.encode(hasher.finalize())
}
