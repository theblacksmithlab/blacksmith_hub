use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::tg_bot::{GrootBotState, ProbiotBotState, TheViperRoomBotState};
use qdrant_client::Qdrant;
use std::sync::Arc;

pub trait QdrantClientInit {
    fn get_qdrant_client(&self) -> Arc<Qdrant>;
}

impl QdrantClientInit for BlacksmithWebAppState {
    fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.qdrant_client.clone()
    }
}

// Реализации для новых bot states
impl QdrantClientInit for ProbiotBotState {
    fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.core.qdrant_client.clone()
    }
}

impl QdrantClientInit for GrootBotState {
    fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.core.qdrant_client.clone()
    }
}

impl QdrantClientInit for TheViperRoomBotState {
    fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.core.qdrant_client.clone()
    }
}
