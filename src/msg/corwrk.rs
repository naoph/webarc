//! Messages passed between `webarc-core` and `webarc-worker`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct InitiateCaptureRequest {
    url: url::Url,
    extractor: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum InitiateCaptureResponse {
    Initiated { ticket: usize },
    InvalidUrl,
    InvalidExtractor,
}
