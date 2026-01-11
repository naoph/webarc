use std::path::PathBuf;

use log::*;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::config::WorkerConfig;
use crate::msg::corwrk::QueryCaptureProgressResponse;

pub struct State {
    auth_tokens: RwLock<Vec<String>>,
    extractors: RwLock<std::collections::HashMap<String, String>>,
    tasks: RwLock<std::collections::HashMap<Uuid, QueryCaptureProgressResponse>>,
    blob_hashes: RwLock<std::collections::HashMap<Uuid, String>>,
    blob_dir: PathBuf,
}

impl State {
    /// Initiate state from config
    pub async fn from_config(config: WorkerConfig) -> Self {
        Self {
            auth_tokens: RwLock::new(config.auth_tokens()),
            extractors: RwLock::new(config.extractors()),
            tasks: RwLock::new(std::collections::HashMap::new()),
            blob_hashes: RwLock::new(std::collections::HashMap::new()),
            blob_dir: config.blob_dir(),
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

    /// Get the blob storage directory
    pub fn blob_dir(&self) -> &PathBuf {
        &self.blob_dir
    }

    /// Get the status of an ongoing capture
    pub async fn capture_status(&self, ticket: &Uuid) -> QueryCaptureProgressResponse {
        self.tasks
            .read()
            .await
            .get(ticket)
            .unwrap_or(&QueryCaptureProgressResponse::NoSuchCapture)
            .clone()
    }

    /// Register a newly-spawned capture
    pub async fn register_capture(&self, ticket: Uuid) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(ticket, QueryCaptureProgressResponse::InProgress);
        debug!("Task list:\n{:#?}", tasks);
    }

    /// Mark capture as failed
    pub async fn abort_capture(&self, ticket: Uuid) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(ticket, QueryCaptureProgressResponse::Failed);
        debug!("Task {ticket} failed");
    }

    /// Mark capture as completed
    pub async fn finalize_capture(&self, ticket: Uuid, hash: String) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(ticket, QueryCaptureProgressResponse::Completed);
        let mut hashes = self.blob_hashes.write().await;
        hashes.insert(ticket, hash);
        debug!("Task {ticket} completed");
    }

    /// Get the hash of a completed capture
    pub async fn get_hash(&self, ticket: &Uuid) -> Option<String> {
        let hashes = self.blob_hashes.read().await;
        hashes.get(ticket).map(|h| h.to_string())
    }
}
