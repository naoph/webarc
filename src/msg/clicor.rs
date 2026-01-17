//! Messages passed between a client and `webarc-core`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    username: String,
    password: String,
}

impl CreateUserRequest {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum CreateUserResponse {
    Created,
    InvalidUsername,
    InvalidPassword,
    UnavailableUsername,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthRequest {
    username: String,
    password: String,
}

impl AuthRequest {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum AuthResponse {
    Authenticated { token: String },
    UnacceptableCredentials,
    InvalidCredentials,
}
