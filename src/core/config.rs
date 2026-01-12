use std::path::Path;

use serde::Deserialize;
use snafu::prelude::*;

#[derive(Clone, Debug, Deserialize)]
pub struct CoreConfig {
    listen: (String, u16),
}

impl CoreConfig {
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<CoreConfig, CoreConfigError> {
        let raw = tokio::fs::read_to_string(path.as_ref())
            .await
            .context(ReadConfigFileSnafu {
                path: path.as_ref().to_string_lossy(),
            })?;
        let conf: CoreConfig = ron::from_str(&raw).context(DeserializeConfigFileSnafu)?;
        Ok(conf)
    }

    pub fn listen(&self) -> &(String, u16) {
        &self.listen
    }
}

#[derive(Debug, Snafu)]
pub enum CoreConfigError {
    #[snafu(display("Unable to read config file at {path}"))]
    ReadConfigFile {
        source: std::io::Error,
        path: String,
    },

    #[snafu(display("Unable to deserialize config file"))]
    DeserializeConfigFile { source: ron::de::SpannedError },
}
