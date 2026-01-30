use crate::models::common::ai::GoogleModel;
use crate::state::llm_client_init_trait::GoogleClientInit;
use anyhow::Result;
use std::sync::Arc;

pub async fn raw_google_processing<T: GoogleClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: GoogleModel,
) -> Result<String> {
    let google_client = app_state.get_google_client();

    google_client
        .chat_completion(system_role, request, &model, 0.2)
        .await
}

pub async fn raw_google_processing_json<T: GoogleClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: GoogleModel,
) -> Result<String> {
    let google_client = app_state.get_google_client();

    google_client
        .chat_completion_json(system_role, request, &model)
        .await
}
