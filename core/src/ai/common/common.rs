use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::request_app::app_state::RequestAppState;
use anyhow::Result;
use async_openai::types::ResponseFormat::JsonObject;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs, CreateEmbeddingRequestArgs, CreateEmbeddingResponse,
};
use std::sync::Arc;
use tiktoken_rs::cl100k_base;
use tracing::info;
use crate::models::common::ai::LlmModel;

pub async fn raw_llm_processing_json<T: OpenAIClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: LlmModel,
) -> Result<String> {
    let llm_client = app_state.get_llm_client().clone();

    let llm_request = CreateChatCompletionRequestArgs::default()
        .max_tokens(4095u32)
        .model(model.as_str())
        .temperature(0.2)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_role)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(request)
                .build()?
                .into(),
        ])
        .response_format(JsonObject)
        .build()?;

    let response = llm_client.chat().create(llm_request).await?;

    if let Some(choice) = response.choices.get(0) {
        let content = choice
            .message
            .content
            .clone()
            .unwrap_or_else(|| "Error generating response... Please try again".to_string());
        Ok(content)
    } else {
        Ok("Error generating response... Please try again".to_string())
    }
}

pub async fn raw_llm_processing<T: OpenAIClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: LlmModel,
) -> Result<String> {
    let llm_client = app_state.get_llm_client().clone();

    let llm_request = CreateChatCompletionRequestArgs::default()
        .max_tokens(4095u32)
        .model(model.as_str())
        .temperature(0.2)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_role)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(request)
                .build()?
                .into(),
        ])
        .build()?;

    let response = llm_client.chat().create(llm_request).await?;

    if let Some(choice) = response.choices.get(0) {
        let content = choice
            .message
            .content
            .clone()
            .unwrap_or_else(|| "Error generating response... Please try again".to_string());
        Ok(content)
    } else {
        Ok("Error generating response... Please try again".to_string())
    }
}

pub async fn vectorize(data: String, app_state: Arc<RequestAppState>) -> Result<Vec<f32>> {
    let llm_client = app_state.llm_client.clone();

    let request = CreateEmbeddingRequestArgs::default()
        .model(LlmModel::TextEmbedding3Large.as_str())
        .input(data)
        .build()?;

    let response: CreateEmbeddingResponse = llm_client.embeddings().create(request).await?;
    let embedding = response.data.into_iter().next().unwrap().embedding;

    Ok(embedding)
}

pub async fn tokenize_and_truncate(data: &str, max_tokens: usize) -> Result<(String, usize)> {
    let bpe = cl100k_base()?;
    let tokens = bpe.encode_ordinary(&*data);
    let token_count = tokens.len();

    info!("Tokenize_and_truncate fn | Input tokens: {:?}", token_count);

    if token_count > max_tokens {
        let truncated_tokens = tokens[..max_tokens].to_vec();
        let truncated_data = bpe.decode(truncated_tokens)?;
        let truncated_text_tokens = bpe.encode_ordinary(&*truncated_data);
        let truncated_count = truncated_text_tokens.len();

        info!("Truncated input tokens: {:?}", truncated_count);

        Ok((truncated_data, truncated_count))
    } else {
        info!(
            "Input tokens {} < max_tokens, no need to truncate",
            token_count
        );
        Ok((data.to_string(), token_count))
    }
}
