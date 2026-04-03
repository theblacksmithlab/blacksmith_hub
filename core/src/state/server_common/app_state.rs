use crate::server_config::server_config::AppConfig;
use std::sync::Arc;

pub struct ServerAppState {
    pub config: Arc<AppConfig>,
}

impl ServerAppState {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }
}
