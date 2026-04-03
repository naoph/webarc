use std::collections::HashMap;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, mobc::Pool};
use log::*;
use tokio::sync::{Mutex, RwLock};

use crate::msg;

use super::config::CoreConfig;

type PgPool = Pool<AsyncPgConnection>;

pub struct State {
    db_pool: PgPool,
    token_map: RwLock<HashMap<u128, i32>>,
    http_client: reqwest::Client,
    extractor_map: ExtractorMap,
    capture_map: CaptureMap,
    worker_dispatch: WorkerDispatch,
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
    progress: msg::clicor::QueryCaptureResponse,
    user_restriction: Option<i32>,
}

impl CaptureStatus {
    pub fn new(extract_quantity: usize, user_restriction: Option<i32>) -> Self {
        Self {
            progress: msg::clicor::QueryCaptureResponse::new_from_quantity(extract_quantity),
            user_restriction,
        }
    }

    /// Return a clone of the progress
    pub fn get_progress(&self) -> msg::clicor::QueryCaptureResponse {
        self.progress.clone()
    }

    /// Determine if a specific user is allowed to check this capture's progress
    pub fn allows_user(&self, user: i32) -> bool {
        self.user_restriction == None || self.user_restriction == Some(user)
    }
}

/// Mediate assignment of workers to extracts
#[derive(Debug)]
pub struct WorkerDispatch {
    worker_map: RwLock<HashMap<String, WorkerDescriptor>>,
    selector: Mutex<WorkerSelector>,
}

impl WorkerDispatch {
    fn from_config(config: &CoreConfig) -> WorkerDispatch {
        let mut worker_map = HashMap::new();
        let mut worker_vec = Vec::new();
        for w in config.workers().iter() {
            let shortname = w.0.clone();
            worker_vec.push(shortname.clone());
            let worker = WorkerDescriptor {
                url: w.2.clone(),
                token: w.1.clone(),
            };
            if worker.url.scheme() != "https" {
                println!("URL for worker `{shortname}` is not HTTPS, consider upgrading it.");
            }
            worker_map.insert(shortname, worker);
        }
        Self {
            worker_map: RwLock::new(worker_map),
            selector: Mutex::new(WorkerSelector::RoundRobin {
                worker_vec,
                next_index: 0,
            }),
        }
    }

    pub async fn select_worker(&self, extractor: &str, target_url: &url::Url) -> String {
        let mut selector = self.selector.lock().await;
        let worker_name = selector.select_worker(&extractor, &target_url);
        worker_name.to_string()
    }

    /// Retrieve descriptor for a specified worker name
    pub async fn describe_worker(&self, name: &str) -> WorkerDescriptor {
        self.worker_map.read().await.get(name).unwrap().clone()
    }
}

#[derive(Clone, Debug)]
pub struct WorkerDescriptor {
    url: url::Url,
    token: String,
}

impl WorkerDescriptor {
    pub fn url(&self) -> &url::Url {
        &self.url
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Debug)]
enum WorkerSelector {
    RoundRobin {
        worker_vec: Vec<String>,
        next_index: usize,
    },
}

impl WorkerSelector {
    pub fn select_worker(&mut self, extractor: &str, target: &url::Url) -> &str {
        let _extractor = extractor;
        let _target = target;
        let selection;
        match self {
            WorkerSelector::RoundRobin {
                worker_vec,
                next_index,
            } => {
                selection = worker_vec.get(*next_index).unwrap();
                *next_index = (*next_index + 1) % worker_vec.len();
            }
        }
        selection
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
        let worker_dispatch = WorkerDispatch::from_config(&config);
        Self {
            db_pool,
            token_map,
            http_client,
            extractor_map,
            capture_map,
            worker_dispatch,
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

    pub fn worker_dispatch(&self) -> &WorkerDispatch {
        &self.worker_dispatch
    }

    /// Return a clone of the preestablished HTTP client
    pub fn http_client(&self) -> reqwest::Client {
        self.http_client.clone()
    }
}
