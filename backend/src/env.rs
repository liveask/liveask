pub const ENV_REDIS_URL: &str = "REDIS_URL";
pub const ENV_RELAX_CORS: &str = "RELAX_CORS";
pub const ENV_DB_LOCAL: &str = "DDB_LOCAL";
pub const ENV_ENV: &str = "LIVEASK_ENV";
pub const ENV_DB_URL: &str = "DDB_URL";
pub const ENV_BASE_URL: &str = "BASE_URL";
pub const ENV_WEEME_KEY: &str = "WEEME_KEY";
pub const ENV_SENTRY_DSN: &str = "LA_SENTRY_DSN";
pub const ENV_PORT: &str = "LA_PORT";
pub const ENV_POSTHOG_KEY: &str = "LA_POSTHOG_KEY";
const ENV_ADMIN_PWD_HASH: &str = "LA_ADMIN_PWD_HASH";
const ENV_SESSION_SECRET: &str = "LA_SESSION_SECRET";
pub const ENV_STRIPE_SECRET: &str = "LA_STRIPE_SECRET";
pub const ENV_STRIPE_HOOK_SECRET: &str = "LA_STRIPE_HOOK_SECRET";

pub fn admin_pwd_hash() -> String {
    std::env::var(ENV_ADMIN_PWD_HASH).unwrap_or_default()
}

/// Insecure well-known fallback used only for local dev / tests when `LA_SESSION_SECRET`
/// is unset. Since auth is now a stateless JWT, this key MUST NOT be used in production —
/// anyone who reads the repo could otherwise forge an admin token (see [`is_default_session_secret`]).
const DEFAULT_DEV_SECRET: &str = "0123456789012345678901234567890123456789012345678901234567890123";

pub fn session_secret() -> Option<Vec<u8>> {
    let vec: Vec<_> = std::env::var(ENV_SESSION_SECRET)
        .unwrap_or_else(|_| String::from(DEFAULT_DEV_SECRET))
        .as_bytes()
        .into();

    (vec.len() >= 64).then_some(vec)
}

/// Whether `secret` is the built-in dev fallback; the caller refuses to start with it in prod.
pub fn is_default_session_secret(secret: &[u8]) -> bool {
    secret == DEFAULT_DEV_SECRET.as_bytes()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dev_fallback_is_recognised_and_long_enough() {
        // the fallback must pass the >=64 gate (otherwise session_secret would reject it in dev)
        assert!(DEFAULT_DEV_SECRET.len() >= 64);
        assert!(is_default_session_secret(DEFAULT_DEV_SECRET.as_bytes()));
        assert!(!is_default_session_secret(
            b"some other 64+ byte secret ................................"
        ));
    }
}
