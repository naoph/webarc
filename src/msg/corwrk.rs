//! Messages passed between `webarc-core` and `webarc-worker`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct InitiateCaptureRequest {
    url: url::Url,
    extractor: String,
}

impl InitiateCaptureRequest {
    pub fn url(&self) -> &url::Url {
        &self.url
    }

    pub fn extractor(&self) -> &str {
        &self.extractor
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum InitiateCaptureResponse {
    Initiated { ticket: Uuid },
    InvalidUrl,
    InvalidExtractor,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QueryCaptureProgressRequest {
    ticket: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum QueryCaptureProgressResponse {
    InProgress,
    UnsupportedUrl,
    Failed,
    Completed,
    NoSuchCapture,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfirmCaptureRequest {
    ticket: Uuid,
    hash: String,
}

impl ConfirmCaptureRequest {
    pub fn ticket(&self) -> &Uuid {
        &self.ticket
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum ConfirmCaptureResponse {
    CorrectHash,
    IncorrectHash,
    NoSuchCapture,
}
