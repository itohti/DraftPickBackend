use crate::Claims;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use tracing::error;

pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;
        let auth = headers.get("Authorization").and_then(|h| h.to_str().ok());
        let token = auth
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header"))?;

        let claims = decode::<Claims>(
            token,
            &DecodingKey::from_secret("sunnycup".as_ref()),
            &Validation::default(),
        )
        .map_err(|e| {
            error!("Token decoding failed: {:?}", e);
            (StatusCode::UNAUTHORIZED, "Invalid token")
        })?;

        Ok(AuthUser(claims.claims))
    }
}
