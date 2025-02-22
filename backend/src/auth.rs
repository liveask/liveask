use async_redis_session::RedisSessionStore;
use async_trait::async_trait;
use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::IntoResponse,
};
use axum_login::{AuthLayer, AuthUser, UserStore, axum_sessions::SessionLayer, secrecy::SecretVec};
use shared::{GetUserInfo, UserInfo};
use tracing::instrument;

use crate::{env::admin_pwd_hash, error::InternalError};

#[derive(Debug, Clone)]
pub struct User {
    id: String,
    password_hash: String,
}

impl AuthUser<String> for User {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_password_hash(&self) -> SecretVec<u8> {
        SecretVec::new(self.password_hash.clone().into())
    }
}

type AuthContext = axum_login::extractors::AuthContext<String, User, DumbAdminUserStore>;

#[instrument(skip(auth), err)]
pub async fn login_handler(
    mut auth: AuthContext,
    Json(payload): Json<shared::UserLogin>,
) -> std::result::Result<impl IntoResponse, InternalError> {
    let admin = admin_user();
    if payload.name == admin.id && payload.pwd_hash == admin.password_hash {
        tracing::info!("log in: {:?}", payload.name);

        auth.login(&User {
            id: String::from("admin"),
            password_hash: admin_pwd_hash(),
        })
        .await?;

        Ok(())
    } else {
        Err(InternalError::InvalidLogin)
    }
}

fn admin_user() -> User {
    User {
        id: String::from("admin"),
        password_hash: admin_pwd_hash(),
    }
}

#[instrument(skip(auth))]
pub async fn logout_handler(mut auth: AuthContext) {
    tracing::info!("log out: {:?}", &auth.current_user);
    auth.logout().await;
}

#[instrument(skip(session), err)]
#[allow(clippy::unused_async)]
pub async fn admin_user_handler(
    session: axum_sessions::extractors::ReadableSession,
    OptionalUser(user): OptionalUser,
) -> std::result::Result<impl IntoResponse, InternalError> {
    tracing::info!("[admin] user handler {user:?}");
    Ok(Json(GetUserInfo {
        user: user.map(|user| UserInfo {
            name: user.id,
            expires: session.expires_in().unwrap_or_default(),
        }),
    }))
}

pub struct OptionalUser(pub Option<User>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth: AuthContext = AuthContext::from_request_parts(parts, state)
            .await
            .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Self(auth.current_user))
    }
}

#[derive(Clone, Debug, Default)]
pub struct DumbAdminUserStore;

#[async_trait]
impl<Role> UserStore<String, Role> for DumbAdminUserStore
where
    Role: PartialOrd + PartialEq + Clone + Send + Sync + 'static,
    User: AuthUser<String, Role>,
{
    type User = User;

    type Error = std::convert::Infallible;

    async fn load_user(
        &self,
        user_id: &String,
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        tracing::debug!("load_user: {}", user_id);

        let admin = admin_user();
        if user_id == &admin.id {
            Ok(Some(admin))
        } else {
            Ok(None)
        }
    }
}

//Note: session livetime
const COOKIE_TTL: std::time::Duration = std::time::Duration::from_secs(60 * 60 * 2);

pub fn setup(
    secret: &[u8],
    session_store: RedisSessionStore,
    is_prod: bool,
) -> (
    SessionLayer<RedisSessionStore>,
    AuthLayer<DumbAdminUserStore, String, User>,
) {
    let same_site_policy = if is_prod {
        axum_sessions::SameSite::Strict
    } else {
        axum_sessions::SameSite::None
    };
    let session_layer = SessionLayer::new(session_store, secret)
        .with_cookie_name("sid")
        .with_persistence_policy(axum_login::axum_sessions::PersistencePolicy::ExistingOnly)
        .with_same_site_policy(same_site_policy)
        .with_session_ttl(Some(COOKIE_TTL));

    let auth_layer = AuthLayer::new(DumbAdminUserStore {}, secret);

    (session_layer, auth_layer)
}

#[cfg(test)]
pub fn setup_test() -> (
    SessionLayer<axum_login::axum_sessions::async_session::MemoryStore>,
    AuthLayer<DumbAdminUserStore, String, User>,
) {
    let secret = "0123456789012345678901234567890123456789012345678901234567890123";
    let session_store = axum_login::axum_sessions::async_session::MemoryStore::new();
    let session_layer = SessionLayer::new(session_store, secret.as_bytes())
        .with_cookie_name("sid")
        .with_persistence_policy(axum_login::axum_sessions::PersistencePolicy::ExistingOnly)
        .with_session_ttl(Some(COOKIE_TTL));

    let auth_layer = AuthLayer::new(DumbAdminUserStore {}, secret.as_bytes());

    (session_layer, auth_layer)
}
