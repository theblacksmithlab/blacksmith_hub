use crate::models::common::ai::AnthropicModel;
use crate::state::llm_client_init_trait::AnthropicClientInit;
use anyhow::Result;
use std::sync::Arc;

pub async fn raw_anthropic_processing<T: AnthropicClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: AnthropicModel,
) -> Result<String> {
    let anthropic_client = app_state.get_anthropic_client();

    anthropic_client
        .chat_completion(system_role, request, &model, 0.2)
        .await
}

pub async fn raw_anthropic_processing_json<T: AnthropicClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: AnthropicModel,
) -> Result<String> {
    let anthropic_client = app_state.get_anthropic_client();

    anthropic_client
        .chat_completion_json(system_role, request, &model)
        .await
}
