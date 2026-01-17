use std::collections::HashMap;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, mobc::Pool};
use tokio::sync::RwLock;

use super::config::CoreConfig;

type PgPool = Pool<AsyncPgConnection>;

pub struct State {
    db_pool: PgPool,
    token_map: RwLock<HashMap<u128, i32>>,
}

impl State {
    /// Initiate state from config
    pub async fn from_config(config: CoreConfig) -> Self {
        let cm = AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.database_url());
        let db_pool = Pool::new(cm);
        let token_map = RwLock::new(HashMap::new());
        Self { db_pool, token_map }
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
}
