//! Messages passed between `webarc-core` and `webarc-worker`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct InitiateExtractRequest {
    url: url::Url,
    extractor: String,
}

impl InitiateExtractRequest {
    pub fn new(url: &url::Url, extractor: &str) -> Self {
        Self {
            url: url.clone(),
            extractor: extractor.to_string(),
        }
    }
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
pub enum InitiateExtractResponse {
    Initiated { ticket: Uuid },
    InvalidUrl,
    InvalidExtractor,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum QueryExtractProgressResponse {
    InProgress,
    UnsupportedUrl,
    Failed,
    Completed,
    NoSuchExtract,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfirmExtractRequest {
    ticket: Uuid,
    hash: String,
}

impl ConfirmExtractRequest {
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
pub enum ConfirmExtractResponse {
    CorrectHash,
    IncorrectHash,
    NoSuchExtract,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScrubExtractRequest {
    hash: String,
}

impl ScrubExtractRequest {
    pub fn hash(&self) -> &str {
        &self.hash
    }
}
