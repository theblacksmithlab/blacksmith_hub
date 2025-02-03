use std::sync::Arc;
use tracing::{error, info};
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::message_processing_flow::message_processing_flow::process_user_raw_request;
use core::utils::tg_bot::tg_bot::append_footer_if_needed;
use core::models::common::app_name::AppName;
use uuid::Uuid;
use core::utils::tg_bot::tg_bot::save_tts_payload;
use core::utils::tg_bot::tg_bot::add_llm_response_to_cache;
use core::utils::common::markdown_to_html;
use core::local_db::local_db::save_message_to_db;

pub(crate) async fn default_message_handler(
    action_text: &str,
    app_state: Arc<BlacksmithWebAppState>,
    user_id: &str,
    app_name: AppName,
) -> String {
    info!("Got message: '{}' from user: {}", action_text, user_id);
    info!("Message received from user: {} is text message. Let's process it...", user_id);

    info!(
    "Saving message: user_id={}, sender={}, message={}, app_name={}",
    user_id, "user", "action_text", app_name
);
    if let Err(e) = save_message_to_db(
        app_state.get_db_pool(),
        user_id,
        "user",
        action_text,
        &app_name.as_str(),
    ).await {
        error!("Failed to save user message to DB: {}", e);
    }

    match process_user_raw_request(
        user_id,
        action_text,
        app_state.clone(),
        app_name.clone(),
    )
        .await
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

            // let message_id = Uuid::new_v4().to_string();

            // save_tts_payload(
            //     app_state.clone(),
            //     user_id,
            //     message_id.clone(),
            //     llm_response.clone(),
            // )
            //     .await;
            

            info!("Successfully processed action text from: {}", user_id);

            if let Err(e) = save_message_to_db(
                &app_state.get_db_pool(),
                user_id,
                "server",
                &full_response,
                &app_name.as_str(),
            ).await {
                error!("Failed to save llm_response message to DB: {}", e);
            }
            
            add_llm_response_to_cache(app_state.clone(), user_id, &full_response)
                .await;

            htmled_full_response
        }
        Err(err) => {
            error!("Error processing action text from user: {}", err);
            err.to_string()
        }
    }
}
