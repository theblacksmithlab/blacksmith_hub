use crate::models::common::dialogue_cache::DialogueCache;
use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::tg_bot::{GrootBotState, ProbiotBotState, TheViperRoomBotState};
use std::collections::HashMap;
use tokio::sync::Mutex;

pub trait TempCacheInit {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>>;
}

impl TempCacheInit for BlacksmithWebAppState {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>> {
        &self.temp_cache
    }
}

// Реализации для новых bot states
impl TempCacheInit for ProbiotBotState {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>> {
        &self.core.temp_cache
    }
}

impl TempCacheInit for GrootBotState {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>> {
        &self.core.temp_cache
    }
}

impl TempCacheInit for TheViperRoomBotState {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>> {
        &self.core.temp_cache
    }
}
