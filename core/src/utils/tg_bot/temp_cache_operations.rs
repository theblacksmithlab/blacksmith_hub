use std::sync::Arc;
use teloxide::prelude::ChatId;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::state::tg_bot::app_state::BotAppState;

pub async fn add_user_message_to_cache(
    app_state: Arc<BotAppState>,
    user_id: ChatId,
    message: String,
) {
    let mut cache = app_state.temp_cache.lock().await;
    let chat_cache = cache.entry(user_id).or_insert_with(|| DialogueCache::new(20));
    chat_cache.add_user_message(message);
}

pub async fn add_llm_response_to_cache(
    app_state: Arc<BotAppState>,
    user_id: ChatId,
    llm_response: String,
) {
    let mut cache = app_state.temp_cache.lock().await;
    let chat_cache = cache.entry(user_id).or_insert_with(|| DialogueCache::new(20));
    chat_cache.add_llm_response_to_cache(llm_response);
}

pub async fn get_cache_as_string(
    app_state: Arc<BotAppState>,
    user_id: ChatId,
) -> String {
    let cache = app_state.temp_cache.lock().await;
    cache
        .get(&user_id)
        .map(|chat_cache| chat_cache.get_cache_as_string())
        .unwrap_or_else(|| "[]".to_string())
}
