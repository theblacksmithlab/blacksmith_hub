use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::models::common::system_messages::{AppsSystemMessages, ProbiotBotMessages};
use crate::temp_cache::temp_cache_traits::TempCacheInit;
use crate::utils::common::get_message;
use std::sync::Arc;

pub async fn get_cache_as_string<T: TempCacheInit + Send + Sync>(
    app_state: Arc<T>,
    user_id: &str,
) -> String {
    let cache = app_state.get_temp_cache().lock().await;
    cache
        .get(user_id)
        .map(|chat_cache| chat_cache.get_cache_as_string())
        .unwrap_or_else(|| "[]".to_string())
}

pub async fn add_user_message_to_cache<T: TempCacheInit + Send + Sync>(
    app_state: Arc<T>,
    user_id: &str,
    message: &str,
) {
    let mut cache = app_state.get_temp_cache().lock().await;
    let chat_cache = cache
        .entry(user_id.to_string())
        .or_insert_with(|| DialogueCache::new(10));
    chat_cache.add_user_message(message.to_string());
}

pub async fn add_llm_response_to_cache<T: TempCacheInit + Send + Sync>(
    app_state: Arc<T>,
    user_id: &str,
    llm_response: &str,
) {
    let mut cache = app_state.get_temp_cache().lock().await;
    let chat_cache = cache
        .entry(user_id.to_string())
        .or_insert_with(|| DialogueCache::new(10));
    chat_cache.add_llm_response_to_cache(llm_response.to_string());
}

pub async fn get_user_message_count<T: TempCacheInit + Send + Sync>(
    app_state: &Arc<T>,
    user_id: &str,
) -> usize {
    let cache = app_state.get_temp_cache().lock().await;
    cache
        .get(user_id)
        .map(|chat_cache| chat_cache.count_user_messages())
        .unwrap_or(0)
}

pub async fn append_footer_if_needed<T: TempCacheInit + Send + Sync>(
    llm_response: &str,
    app_state: Arc<T>,
    chat_id: &str,
    app_name: AppName,
) -> anyhow::Result<String> {
    let message_count = get_user_message_count(&app_state, chat_id).await;

    if message_count > 0 && message_count % 3 == 0 {
        let footer_message = match app_name {
            AppName::ProbiotBot => {
                get_message(AppsSystemMessages::ProbiotBot(
                    ProbiotBotMessages::ResponseFooter,
                ))
                .await?
            }
            AppName::W3AWeb => {
                "".to_string()
                // // Uncomment to use ResponseFooter for W3AWeb App
                // get_message(AppsSystemMessages::W3ABot(W3ABotMessages::ResponseFooter)).await?
            }
            _ => "".to_string(),
        };

        if !footer_message.is_empty() {
            return Ok(format!("{}\n{}", llm_response, footer_message));
        }
    }

    Ok(llm_response.to_string())
}
