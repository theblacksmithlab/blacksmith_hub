use crate::models::tg_bot::groot_bot::groot_bot::ResourcesDialogState;
use crate::state::tg_bot::app_state::BotAppState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct GrootBotAppState {
    pub base_bot_app_state: Arc<BotAppState>,
    pub dialog_states: Mutex<HashMap<u64, ResourcesDialogState>>,
}

impl GrootBotAppState {
    pub fn new(base_bot_app_state: Arc<BotAppState>) -> Self {
        Self {
            base_bot_app_state,
            dialog_states: Mutex::new(HashMap::new()),
        }
    }
}
