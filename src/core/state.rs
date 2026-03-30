use std::collections::HashMap;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, mobc::Pool};
use log::*;
use tokio::sync::RwLock;

use crate::msg;

use super::config::CoreConfig;

type PgPool = Pool<AsyncPgConnection>;

pub struct State {
    db_pool: PgPool,
    token_map: RwLock<HashMap<u128, i32>>,
    http_client: reqwest::Client,
    extractor_map: ExtractorMap,
    capture_map: CaptureMap,
}

#[derive(Debug)]
pub struct ExtractorMap {
    map: RwLock<HashMap<String, ExtractorConfig>>,
}

impl ExtractorMap {
    /// Create new ExtractorMap from a HashMap
    fn from_map(map: HashMap<String, ExtractorConfig>) -> Self {
        Self {
            map: RwLock::new(map),
        }
    }

    /// Determine appropriate extractors for a given URL
    pub async fn extractors_for_url(&self, url: &url::Url) -> Vec<String> {
        let url_string = url.to_string();
        let mut matches = Vec::new();
        let emap = self.map.read().await;
        for e in emap.keys() {
            if emap.get(e).unwrap().url_matches(&url_string) {
                matches.push(e.to_string());
            }
        }
        matches
    }
}

#[derive(Debug)]
struct ExtractorConfig {
    url_regex: regex::Regex,
}

impl ExtractorConfig {
    fn from_regex(url_regex: regex::Regex) -> Self {
        Self { url_regex }
    }

    fn url_matches(&self, url: &String) -> bool {
        self.url_regex.is_match(url)
    }
}

#[derive(Debug)]
pub struct CaptureMap {
    map: RwLock<HashMap<uuid::Uuid, CaptureStatus>>,
}

impl CaptureMap {
    fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Insert a clean status for a newly-initiated capture
    pub async fn new_status(
        &self,
        capture: &uuid::Uuid,
        extract_quantity: usize,
        user_id: i32,
        public: bool,
    ) {
        let user_restriction = match public {
            true => None,
            false => Some(user_id),
        };
        self.map.write().await.insert(
            capture.clone(),
            CaptureStatus::new(extract_quantity, user_restriction),
        );
    }

    /// Get the status of an ongoing capture
    pub async fn get_status(&self, capture: &uuid::Uuid) -> Option<CaptureStatus> {
        self.map.read().await.get(capture).map(|a| a.clone())
    }
}

#[derive(Clone, Debug)]
pub struct CaptureStatus {
    progress: crate::msg::clicor::QueryCaptureResponse,
    user_restriction: Option<i32>,
}

impl CaptureStatus {
    pub fn new(extract_quantity: usize, user_restriction: Option<i32>) -> Self {
        Self {
            progress: crate::msg::clicor::QueryCaptureResponse::new_from_quantity(extract_quantity),
            user_restriction,
        }
    }

    /// Return a clone of the progress
    pub fn get_progress(&self) -> crate::msg::clicor::QueryCaptureResponse {
        self.progress.clone()
    }

    /// Determine if a specific user is allowed to check this capture's progress
    pub fn allows_user(&self, user: i32) -> bool {
        self.user_restriction == None || self.user_restriction == Some(user)
    }
}

impl State {
    /// Initiate state from config
    pub async fn from_config(config: CoreConfig) -> Self {
        let cm = AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.database_url());
        let db_pool = Pool::new(cm);
        let token_map = RwLock::new(HashMap::new());
        let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        let http_client = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .build()
            .unwrap();
        let mut extractor_map = HashMap::new();
        for (e, r) in config.extractors().iter() {
            let rex = match regex::Regex::new(r) {
                Ok(r) => r,
                Err(e) => {
                    error!("Bad regex for extractor {e}");
                    panic!();
                }
            };
            extractor_map.insert(e.clone(), ExtractorConfig::from_regex(rex));
        }
        let extractor_map = ExtractorMap::from_map(extractor_map);
        let capture_map = CaptureMap::new();
        Self {
            db_pool,
            token_map,
            http_client,
            extractor_map,
            capture_map,
        }
    }

    /// Return a copy of the database pool
    pub async fn db_pool(&self) -> PgPool {
        self.db_pool.clone()
    }

    /// Create a new token->user association
    pub async fn register_token(&self, token: u128, user_id: i32) {
        let mut token_map = self.token_map.write().await;
        token_map.insert(token, user_id);
    }

    /// Derive a user from an associated token
    pub async fn user_from_token(&self, token: u128) -> Option<i32> {
        let token_map = self.token_map.read().await;
        token_map.get(&token).map(|i| *i)
    }

    pub async fn extractor_map(&self) -> &ExtractorMap {
        &self.extractor_map
    }

    pub async fn capture_map(&self) -> &CaptureMap {
        &self.capture_map
    }
}
