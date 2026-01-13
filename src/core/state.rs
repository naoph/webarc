use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, mobc::Pool};

use super::config::CoreConfig;

type PgPool = Pool<AsyncPgConnection>;

pub struct State {
    db_pool: PgPool,
}

impl State {
    /// Initiate state from config
    pub async fn from_config(config: CoreConfig) -> Self {
        let cm = AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.database_url());
        let db_pool = Pool::new(cm);
        Self { db_pool }
    }

    /// Return a copy of the database pool
    pub async fn db_pool(&self) -> PgPool {
        self.db_pool.clone()
    }
}
