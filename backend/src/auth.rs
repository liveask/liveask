use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    Extension, Json,
    extract::FromRequestParts,
    http::{HeaderMap, header, request::Parts},
    response::{AppendHeaders, IntoResponse},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use shared::{GetUserInfo, UserInfo};
use tracing::instrument;

use crate::{env::admin_pwd_hash, error::InternalError};

/// Cookie carrying the admin JWT.
const AUTH_COOKIE: &str = "auth";
/// Cookie carrying a per-event "password proven" grant JWT.
const PWD_COOKIE: &str = "pwd";
/// `sub` values that scope a token to one purpose so it cannot be replayed as another.
const ADMIN_NAME: &str = "admin";
const PWD_KIND: &str = "pwd";
/// Token / cookie lifetime (was the session ttl).
const COOKIE_TTL: Duration = Duration::from_secs(2 * 60 * 60);

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
    /// token kind (`admin` / `pwd`); guards against a token being replayed for another purpose.
    sub: String,
    /// event a `pwd` grant is scoped to; absent on admin tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event: Option<String>,
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

fn encode_token(cfg: &AuthConfig, claims: &Claims) -> Result<String, InternalError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(cfg.secret.as_ref()),
    )
    .map_err(|e| InternalError::General(format!("jwt encode: {e}")))
}

fn decode_token(cfg: &AuthConfig, token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.secret.as_ref()),
        &validation(),
    )
    .ok()
    .map(|data| data.claims)
}

fn issue_admin_token(cfg: &AuthConfig) -> Result<String, InternalError> {
    encode_token(
        cfg,
        &Claims {
            sub: ADMIN_NAME.to_string(),
            event: None,
            exp: now_secs() + COOKIE_TTL.as_secs(),
        },
    )
}

fn verify_admin(cfg: &AuthConfig, token: &str) -> Option<AdminUser> {
    let claims = decode_token(cfg, token)?;

    (claims.sub == ADMIN_NAME).then(|| AdminUser {
        name: claims.sub,
        expires: Duration::from_secs(claims.exp.saturating_sub(now_secs())),
    })
}

/// `Set-Cookie` value granting the caller access to the (already password-validated) event.
pub fn pwd_grant_cookie(cfg: &AuthConfig, event: &str) -> Result<String, InternalError> {
    let token = encode_token(
        cfg,
        &Claims {
            sub: PWD_KIND.to_string(),
            event: Some(event.to_string()),
            exp: now_secs() + COOKIE_TTL.as_secs(),
        },
    )?;
    Ok(set_cookie(cfg, PWD_COOKIE, &token, COOKIE_TTL))
}

/// `true` iff the request carries a valid, unexpired pwd grant scoped to `event`.
pub fn pwd_unlocked(cfg: &AuthConfig, headers: &HeaderMap, event: &str) -> bool {
    read_cookie(headers, PWD_COOKIE)
        .and_then(|token| decode_token(cfg, token))
        .is_some_and(|claims| claims.sub == PWD_KIND && claims.event.as_deref() == Some(event))
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

/// Stateless JWT config shared into the router as a request extension.
pub fn setup(secret: Vec<u8>, is_prod: bool) -> AuthConfig {
    AuthConfig::new(secret, is_prod)
}

#[cfg(test)]
pub fn setup_test() -> AuthConfig {
    AuthConfig::new(
        b"0123456789012345678901234567890123456789012345678901234567890123".to_vec(),
        false,
    )
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
                sub: PWD_KIND.to_string(),
                event: Some("EVENT".to_string()),
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
                event: None,
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

    /// wraps the token from a `Set-Cookie` into a `Cookie` request header for verification.
    fn headers_with(cookie: &str) -> HeaderMap {
        let pair = cookie.split(';').next().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, pair.parse().unwrap());
        headers
    }

    #[test]
    fn pwd_grant_is_event_scoped() {
        let cfg = cfg();
        let headers = headers_with(&pwd_grant_cookie(&cfg, "EVENT_A").unwrap());
        assert!(pwd_unlocked(&cfg, &headers, "EVENT_A"));
        // a grant for one event must not unlock another
        assert!(!pwd_unlocked(&cfg, &headers, "EVENT_B"));
    }

    #[test]
    fn admin_token_is_not_accepted_as_pwd_grant() {
        let cfg = cfg();
        let headers = headers_with(&format!("pwd={}", issue_admin_token(&cfg).unwrap()));
        assert!(!pwd_unlocked(&cfg, &headers, "EVENT_A"));
    }

    #[test]
    fn pwd_grant_is_not_accepted_as_admin() {
        let cfg = cfg();
        let cookie = pwd_grant_cookie(&cfg, "EVENT_A").unwrap();
        let token = cookie
            .split(';')
            .next()
            .and_then(|p| p.split_once('='))
            .unwrap()
            .1;
        assert!(verify_admin(&cfg, token).is_none());
    }
}
