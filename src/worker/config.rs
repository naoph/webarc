use std::path::{Path, PathBuf};

use serde::Deserialize;
use snafu::prelude::*;

#[derive(Clone, Debug, Deserialize)]
pub struct WorkerConfig {
    listen: (String, u16),
    auth_tokens: Vec<String>,
    extractors: std::collections::HashMap<String, String>,
    blob_dir: PathBuf,
}

impl WorkerConfig {
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<WorkerConfig, WorkerConfigError> {
        let raw = tokio::fs::read_to_string(path.as_ref())
            .await
            .context(ReadConfigFileSnafu {
                path: path.as_ref().to_string_lossy(),
            })?;
        let conf: WorkerConfig = ron::from_str(&raw).context(DeserializeConfigFileSnafu)?;
        Ok(conf)
    }

    pub fn listen(&self) -> &(String, u16) {
        &self.listen
    }

    pub fn auth_tokens(&self) -> Vec<String> {
        self.auth_tokens.clone()
    }

    pub fn extractors(&self) -> std::collections::HashMap<String, String> {
        self.extractors.clone()
    }

    pub fn blob_dir(&self) -> PathBuf {
        self.blob_dir.clone()
    }
}

#[derive(Debug, Snafu)]
pub enum WorkerConfigError {
    #[snafu(display("Unable to read config file at {path}"))]
    ReadConfigFile {
        source: std::io::Error,
        path: String,
    },

    #[snafu(display("Unable to deserialize config file"))]
    DeserializeConfigFile { source: ron::de::SpannedError },
}
