use crate::local_db::blacksmith_web::chat_history_storage::save_message_to_db;
use crate::message_processing_flow::message_processing_flow::process_user_query;
use crate::models::common::app_name::AppName;
use crate::models::common::system_messages::{AppsSystemMessages, CommonMessages};
use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::utils::common::{get_message, markdown_to_html};
use crate::utils::tg_bot::tg_bot::add_llm_response_to_cache;
use crate::utils::tg_bot::tg_bot::append_footer_if_needed;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub async fn default_message_handler(
    request_text: &str,
    app_state: Arc<BlacksmithWebAppState>,
    user_id: &str,
    app_name: &AppName,
) -> (String, HashMap<String, String>) {
    let request_id = Uuid::new_v4();
    info!(request_id = %request_id, user_id = %user_id, "Request processing started");

    if let Err(e) = save_message_to_db(
        app_state.get_db_pool(),
        user_id,
        "user",
        request_text,
        &app_name.as_str(),
    )
    .await
    {
        error!("Failed to save user message to local db: {}", e);
    }

    match process_user_query(user_id, request_text, app_state.clone(), app_name.clone()).await {
        Ok((llm_response, extra_data)) => {
            let full_response = append_footer_if_needed(
                &llm_response,
                app_state.clone(),
                user_id,
                app_name.clone(),
            )
            .await
            .unwrap_or_else(|_| llm_response.clone());

            let htmled_full_response = markdown_to_html(&full_response);

            if let Err(e) = save_message_to_db(
                &app_state.get_db_pool(),
                user_id,
                "server",
                &htmled_full_response,
                &app_name.as_str(),
            )
            .await
            {
                error!("Failed to save llm_response message to local db: {}", e);
            }

            add_llm_response_to_cache(app_state.clone(), user_id, &full_response).await;

            info!(request_id = %request_id, user_id = %user_id, "User request processed successfully");

            (htmled_full_response, extra_data)
        }
        Err(err) => {
            error!(request_id = %request_id, user_id = %user_id, "User request processing failed with error: {}", err);

            let error_msg_for_user = get_message(AppsSystemMessages::Common(
                CommonMessages::ServiceUnavailable,
            ))
            .await
            .unwrap_or_else(|_| {
                "В данный момент на сервере проводятся технические работы.\n\
                Пожалуйста, повторите попытку позднее, мы работаем для Вас 🙏"
                    .to_string()
            });

            (error_msg_for_user, HashMap::new())
        }
    }
}
