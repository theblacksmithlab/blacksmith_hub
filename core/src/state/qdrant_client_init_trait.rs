use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::tg_bot::app_state::BotAppState;
use qdrant_client::Qdrant;
use std::sync::Arc;

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
