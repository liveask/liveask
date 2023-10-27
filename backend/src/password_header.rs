use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderName},
};

static PASSWORD_HEADER_NAME: HeaderName = HeaderName::from_static("la-password");

#[derive(Debug, Clone)]
pub struct ExtractPassword(pub Option<String>);

#[async_trait]
impl<S> FromRequestParts<S> for ExtractPassword
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts
                .headers
                .get(&PASSWORD_HEADER_NAME)
                .and_then(|pwd| pwd.to_str().ok())
                .map(std::string::ToString::to_string),
        ))
    }
}
