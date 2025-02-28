use crate::local_db::local_db::save_message_to_db;
use crate::message_processing_flow::message_processing_flow::process_user_raw_request;
use crate::models::common::app_name::AppName;
use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::utils::common::markdown_to_html;
use crate::utils::tg_bot::tg_bot::add_llm_response_to_cache;
use crate::utils::tg_bot::tg_bot::append_footer_if_needed;
use std::sync::Arc;
use tracing::{error, info};

pub async fn default_message_handler(
    request_text: &str,
    app_state: Arc<BlacksmithWebAppState>,
    user_id: &str,
    app_name: &AppName,
) -> String {
    info!(
        "Message received from user: {} is a text message. Let's process it...",
        user_id
    );

    if let Err(e) = save_message_to_db(
        app_state.get_db_pool(),
        user_id,
        "user",
        request_text,
        &app_name.as_str(),
    )
    .await
    {
        error!("Failed to save user message to DB: {}", e);
    }

    match process_user_raw_request(user_id, request_text, app_state.clone(), app_name.clone()).await
    {
        Ok(llm_response) => {
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
                error!("Failed to save llm_response message to DB: {}", e);
            }

            add_llm_response_to_cache(app_state.clone(), user_id, &full_response).await;

            info!("Successfully processed text message from user: {}", user_id);

            htmled_full_response
        }
        Err(err) => {
            error!("Error processing action text from user: {}", err);
            err.to_string()
        }
    }
}
