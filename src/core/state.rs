use super::config::CoreConfig;

pub struct State {}

impl State {
    /// Initiate state from config
    pub async fn from_config(config: CoreConfig) -> Self {
        Self {}
    }
}
