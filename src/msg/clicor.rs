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

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCaptureRequest {
    url: url::Url,
    public: bool,
}

impl CreateCaptureRequest {
    pub fn url(&self) -> &url::Url {
        &self.url
    }

    pub fn public(&self) -> bool {
        self.public
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum CreateCaptureResponse {
    Initiated { capture_id: uuid::Uuid },
    NoExtractors,
    Unauthenticated,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryCaptureResponse {
    in_progress: usize,
    completed: usize,
    failed: usize,
}

impl QueryCaptureResponse {
    pub fn new_from_quantity(qty: usize) -> Self {
        Self {
            in_progress: qty,
            completed: 0,
            failed: 0,
        }
    }

    pub fn incr_completed(&mut self) {
        self.in_progress -= 1;
        self.completed += 1;
    }

    pub fn incr_failed(&mut self) {
        self.in_progress -= 1;
        self.failed += 1;
    }
}
