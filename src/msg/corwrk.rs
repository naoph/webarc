//! Messages passed between `webarc-core` and `webarc-worker`.

use serde::{Deserialize, Serialize};

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
    Initiated { ticket: usize },
    InvalidUrl,
    InvalidExtractor,
}
