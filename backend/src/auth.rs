use std::{sync::Arc, time::Duration};

use async_redis_session::RedisSessionStore;
use async_trait::async_trait;
use axum::{
    Extension, Json,
    extract::FromRequestParts,
    http::{HeaderMap, header, request::Parts},
    response::{AppendHeaders, IntoResponse},
};
use axum_sessions::{PersistencePolicy, SameSite, SessionLayer};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use shared::{GetUserInfo, UserInfo};
use tracing::instrument;

use crate::{env::admin_pwd_hash, error::InternalError};

/// Cookie carrying the admin JWT.
const AUTH_COOKIE: &str = "auth";
const ADMIN_NAME: &str = "admin";
/// Token / cookie lifetime (was the session ttl).
const COOKIE_TTL: Duration = Duration::from_hours(2);

/// JWT signing key + cookie flags, shared via request extension so the handlers and the
/// `OptionalUser` extractor can verify tokens without any session store.
#[derive(Clone)]
pub struct AuthConfig {
    secret: Arc<[u8]>,
    prod: bool,
}

impl AuthConfig {
    fn new(secret: Vec<u8>, prod: bool) -> Self {
        Self {
            secret: secret.into(),
            prod,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    /// token kind — `admin`; guards against a differently-scoped token being replayed here.
    sub: String,
    exp: u64,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn validation() -> Validation {
    let mut v = Validation::new(Algorithm::HS256);
    // we don't use `aud`; default-on validation would otherwise reject our tokens
    v.validate_aud = false;
    v
}

fn issue_admin_token(cfg: &AuthConfig) -> Result<String, InternalError> {
    let claims = Claims {
        sub: ADMIN_NAME.to_string(),
        exp: now_secs() + COOKIE_TTL.as_secs(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.secret.as_ref()),
    )
    .map_err(|e| InternalError::General(format!("jwt encode: {e}")))
}

fn verify_admin(cfg: &AuthConfig, token: &str) -> Option<AdminUser> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.secret.as_ref()),
        &validation(),
    )
    .ok()?;

    (data.claims.sub == ADMIN_NAME).then(|| AdminUser {
        name: data.claims.sub,
        expires: Duration::from_secs(data.claims.exp.saturating_sub(now_secs())),
    })
}

/// Read a single cookie value out of the `Cookie` request header.
fn read_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|kv| kv.split_once('='))
        .find_map(|(k, v)| (k.trim() == name).then_some(v.trim()))
}

/// `Set-Cookie` value for `name`, expiring in `max_age` (zero clears it).
fn set_cookie(cfg: &AuthConfig, name: &str, value: &str, max_age: Duration) -> String {
    let same_site = if cfg.prod { "Strict" } else { "None" };
    format!(
        "{name}={value}; HttpOnly; Path=/; Max-Age={}; SameSite={same_site}; Secure",
        max_age.as_secs()
    )
}

#[derive(Debug, Clone)]
pub struct AdminUser {
    name: String,
    expires: Duration,
}

#[instrument(skip_all, err)]
#[allow(clippy::unused_async)]
pub async fn login_handler(
    Extension(cfg): Extension<AuthConfig>,
    Json(payload): Json<shared::UserLogin>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    let expected = admin_pwd_hash();

    if !expected.is_empty() && payload.name == ADMIN_NAME && payload.pwd_hash == expected {
        tracing::info!("log in: {:?}", payload.name);

        let token = issue_admin_token(&cfg)?;
        Ok(AppendHeaders([(
            header::SET_COOKIE,
            set_cookie(&cfg, AUTH_COOKIE, &token, COOKIE_TTL),
        )]))
    } else {
        Err(InternalError::InvalidLogin)
    }
}

#[instrument(skip_all)]
#[allow(clippy::unused_async)]
pub async fn logout_handler(Extension(cfg): Extension<AuthConfig>) -> impl IntoResponse {
    tracing::info!("log out");
    // clearing the cookie is all logout can do for a stateless token
    AppendHeaders([(
        header::SET_COOKIE,
        set_cookie(&cfg, AUTH_COOKIE, "", Duration::ZERO),
    )])
}

#[instrument(skip_all)]
#[allow(clippy::unused_async)]
pub async fn admin_user_handler(OptionalUser(user): OptionalUser) -> impl IntoResponse {
    tracing::info!("[admin] user handler {user:?}");
    Json(GetUserInfo {
        user: user.map(|user| UserInfo {
            name: user.name,
            expires: user.expires,
        }),
    })
}

/// `Some` iff the request carries a valid, unexpired admin JWT cookie.
pub struct OptionalUser(pub Option<AdminUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts.extensions.get::<AuthConfig>().and_then(|cfg| {
            read_cookie(&parts.headers, AUTH_COOKIE).and_then(|token| verify_admin(cfg, token))
        });

        Ok(Self(user))
    }
}

/// Builds the pwd-session layer (still Redis-backed) and the stateless JWT config.
pub fn setup(
    secret: Vec<u8>,
    session_store: RedisSessionStore,
    is_prod: bool,
) -> (SessionLayer<RedisSessionStore>, AuthConfig) {
    let same_site_policy = if is_prod {
        SameSite::Strict
    } else {
        SameSite::None
    };
    let session_layer = SessionLayer::new(session_store, &secret)
        .with_cookie_name("sid")
        .with_persistence_policy(PersistencePolicy::ExistingOnly)
        .with_same_site_policy(same_site_policy)
        .with_session_ttl(Some(COOKIE_TTL));

    (session_layer, AuthConfig::new(secret, is_prod))
}

#[cfg(test)]
pub fn setup_test() -> (
    SessionLayer<axum_sessions::async_session::MemoryStore>,
    AuthConfig,
) {
    let secret = b"0123456789012345678901234567890123456789012345678901234567890123".to_vec();
    let session_store = axum_sessions::async_session::MemoryStore::new();
    let session_layer = SessionLayer::new(session_store, &secret)
        .with_cookie_name("sid")
        .with_persistence_policy(PersistencePolicy::ExistingOnly)
        .with_session_ttl(Some(COOKIE_TTL));

    (session_layer, AuthConfig::new(secret, false))
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    fn cfg() -> AuthConfig {
        AuthConfig::new(
            b"0123456789012345678901234567890123456789012345678901234567890123".to_vec(),
            false,
        )
    }

    fn sign(cfg: &AuthConfig, claims: &Claims) -> String {
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(cfg.secret.as_ref()),
        )
        .unwrap()
    }

    #[test]
    fn admin_token_roundtrips() {
        let cfg = cfg();
        let user = verify_admin(&cfg, &issue_admin_token(&cfg).unwrap()).expect("valid token");
        assert_eq!(user.name, ADMIN_NAME);
        assert!(user.expires <= COOKIE_TTL && user.expires.as_secs() + 5 > COOKIE_TTL.as_secs());
    }

    #[test]
    fn rejects_token_signed_with_other_secret() {
        let token = issue_admin_token(&cfg()).unwrap();
        let other = AuthConfig::new(
            b"9999999999999999999999999999999999999999999999999999999999999999".to_vec(),
            false,
        );
        assert!(verify_admin(&other, &token).is_none());
    }

    #[test]
    fn rejects_non_admin_subject() {
        // a validly-signed token whose subject isn't the admin (e.g. a future pwd grant) must
        // never authenticate as admin
        let cfg = cfg();
        let token = sign(
            &cfg,
            &Claims {
                sub: "pwd".to_string(),
                exp: now_secs() + 60,
            },
        );
        assert!(verify_admin(&cfg, &token).is_none());
    }

    #[test]
    fn rejects_expired_token() {
        let cfg = cfg();
        let token = sign(
            &cfg,
            &Claims {
                sub: ADMIN_NAME.to_string(),
                exp: now_secs().saturating_sub(3600),
            },
        );
        assert!(verify_admin(&cfg, &token).is_none());
    }

    #[test]
    fn reads_named_cookie_among_many() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "foo=1; auth=the-token; bar=2".parse().unwrap(),
        );
        assert_eq!(read_cookie(&headers, AUTH_COOKIE), Some("the-token"));
        assert_eq!(read_cookie(&headers, "missing"), None);
    }
}
