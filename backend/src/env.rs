pub const ENV_REDIS_URL: &str = "REDIS_URL";
pub const ENV_RELAX_CORS: &str = "RELAX_CORS";
pub const ENV_DB_LOCAL: &str = "DDB_LOCAL";
pub const ENV_ENV: &str = "LIVEASK_ENV";
pub const ENV_DB_URL: &str = "DDB_URL";
pub const ENV_BASE_URL: &str = "BASE_URL";
pub const ENV_TINY_TOKEN: &str = "TINY_URL_TOKEN";
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

pub fn session_secret() -> Option<Vec<u8>> {
    let vec: Vec<_> = std::env::var(ENV_SESSION_SECRET)
        .unwrap_or_else(|_| {
            String::from("0123456789012345678901234567890123456789012345678901234567890123")
        })
        .as_bytes()
        .into();

    (vec.len() >= 64).then_some(vec)
}
