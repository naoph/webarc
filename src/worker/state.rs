use tokio::sync::RwLock;

use super::config::WorkerConfig;

pub struct State {
    auth_tokens: RwLock<Vec<String>>,
    extractors: RwLock<std::collections::HashMap<String, String>>,
}

impl State {
    /// Initiate state from config
    pub async fn from_config(config: WorkerConfig) -> Self {
        Self {
            auth_tokens: RwLock::new(config.auth_tokens()),
            extractors: RwLock::new(config.extractors()),
        }
    }

    /// Check a provided auth token against the allowlist
    pub async fn validate_auth_token(&self, token: Option<String>) -> bool {
        let tokens = self.auth_tokens.read().await;
        match token {
            None => false,
            Some(a) => tokens.contains(&a),
        }
    }

    /// Determine which executable to use for a given extractor name
    pub async fn locate_extractor(&self, extractor: &str) -> Option<String> {
        let extractors = self.extractors.read().await;
        extractors.get(extractor).map(|a| a.clone())
    }
}
