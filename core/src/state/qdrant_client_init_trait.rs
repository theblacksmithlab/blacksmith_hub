use std::sync::Arc;
use qdrant_client::Qdrant;
use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::request_app::app_state::RequestAppState;
use crate::state::tg_bot::app_state::BotAppState;

pub trait QdrantClientInit {
    fn get_qdrant_client(&self) -> Arc<Qdrant>;
}

impl QdrantClientInit for BotAppState {
    fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.qdrant_client.clone()
    }
}

impl QdrantClientInit for BlacksmithWebAppState {
    fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.qdrant_client.clone()
    }
}

impl QdrantClientInit for RequestAppState {
    fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.qdrant_client.clone()
    }
}