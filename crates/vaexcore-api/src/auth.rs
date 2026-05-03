use std::sync::{Arc, RwLock};

use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

use crate::ApiError;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AuthConfig {
    pub token: Option<String>,
    pub dev_mode: bool,
}

impl AuthConfig {
    pub fn auth_required(&self) -> bool {
        !self.dev_mode && self.token.is_some()
    }
}

#[derive(Clone, Debug)]
pub struct SharedAuthConfig {
    inner: Arc<RwLock<AuthConfig>>,
}

impl SharedAuthConfig {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(config)),
        }
    }

    pub fn get(&self) -> AuthConfig {
        self.inner
            .read()
            .expect("auth config lock poisoned")
            .clone()
    }

    pub fn update(&self, config: AuthConfig) {
        *self.inner.write().expect("auth config lock poisoned") = config;
    }

    pub fn auth_required(&self) -> bool {
        self.get().auth_required()
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TokenQuery {
    pub token: Option<String>,
    pub limit: Option<usize>,
    pub client_id: Option<String>,
    pub client_name: Option<String>,
}

pub fn authorize_headers(headers: &HeaderMap, auth: &SharedAuthConfig) -> Result<(), ApiError> {
    let auth = auth.get();
    if auth.dev_mode {
        return Ok(());
    }

    let expected = auth.token.as_deref().ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "auth_not_configured",
            "API token is not configured",
        )
    })?;

    let provided = bearer_token(headers).or_else(|| header_token(headers));

    match provided {
        Some(token) if token == expected => Ok(()),
        _ => Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "valid localhost API token required",
        )),
    }
}

pub fn authorize_query(query: &TokenQuery, auth: &SharedAuthConfig) -> Result<(), ApiError> {
    let auth = auth.get();
    if auth.dev_mode {
        return Ok(());
    }

    let expected = auth.token.as_deref().ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "auth_not_configured",
            "API token is not configured",
        )
    })?;

    match query.token.as_deref() {
        Some(token) if token == expected => Ok(()),
        _ => Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "valid localhost API token required",
        )),
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

fn header_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-vaexcore-token")
        .and_then(|value| value.to_str().ok())
}
