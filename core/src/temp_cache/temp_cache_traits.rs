use std::collections::HashMap;
use tokio::sync::Mutex;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::tg_bot::app_state::BotAppState;

pub trait TempCacheInit {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>>;
}

impl TempCacheInit for BotAppState {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>> {
        &self.temp_cache
    }
}

impl TempCacheInit for BlacksmithWebAppState {
    fn get_temp_cache(&self) -> &Mutex<HashMap<String, DialogueCache>> {
        &self.temp_cache
    }
}