use std::sync::Arc;
use tracing::{error, info};
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::message_processing_flow::message_processing_flow::process_user_raw_request;
use core::utils::tg_bot::tg_bot::append_footer_if_needed;
use core::models::common::app_name::AppName;
use uuid::Uuid;
use core::utils::tg_bot::tg_bot::save_tts_payload;
use core::utils::tg_bot::tg_bot::add_llm_response_to_cache;

pub(crate) async fn default_message_handler(
    msg: String,
    app_state: Arc<BlacksmithWebAppState>,
    chat_id: String,
    app_name: AppName,
) -> String {
    info!("Message received from user: {} is text message. Let's process it...", chat_id);

    let chat_id = 12345i64;

    match process_user_raw_request(
        chat_id,
        msg,
        app_state.clone(),
        app_name.clone(),
    )
        .await
    {
        Ok(llm_response) => {
            let full_response = append_footer_if_needed(
                llm_response.clone(),
                app_state.clone(),
                chat_id,
                app_name.clone(),
            )
                .await
                .unwrap_or_else(|_| llm_response.clone());

            // let htmled_full_response = markdown_to_html(&full_response);

            let message_id = Uuid::new_v4().to_string();

            save_tts_payload(
                app_state.clone(),
                chat_id,
                message_id.clone(),
                llm_response.clone(),
            )
                .await;
            

            info!("Successfully processed text message from: {}", chat_id);

            add_llm_response_to_cache(app_state.clone(), chat_id, full_response.clone())
                .await;
            
            return llm_response;
        }
        Err(err) => {
            error!("Error in process_user_raw_request: {}", err);
            return err.to_string();
        }
    }

}