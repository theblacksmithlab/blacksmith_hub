use crate::models::common::ai::OpenAIModel;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use anyhow::Result;
use async_openai::types::ResponseFormat::JsonObject;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use std::sync::Arc;
use tiktoken_rs::cl100k_base;
use tracing::info;

pub async fn raw_openai_processing_json<T: OpenAIClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: OpenAIModel,
) -> Result<String> {
    let openai_client = app_state.get_openai_client().clone();

    let mut builder = CreateChatCompletionRequestArgs::default();
    builder.model(model.as_str());

    if model.is_gpt5_model() {
        if let Some(effort) = model.reasoning_effort() {
            builder.reasoning_effort(effort);
        }
    } else {
        builder.temperature(0.0);
    }

    builder
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
        .response_format(JsonObject);

    let llm_request = builder.build()?;

    let response = openai_client.chat().create(llm_request).await?;

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

pub async fn raw_openai_processing<T: OpenAIClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: OpenAIModel,
) -> Result<String> {
    let openai_client = app_state.get_openai_client().clone();

    let mut builder = CreateChatCompletionRequestArgs::default();
    builder.model(model.as_str());

    if model.is_gpt5_model() {
        if let Some(effort) = model.reasoning_effort() {
            builder.reasoning_effort(effort);
        }
    } else {
        builder.temperature(0.2);
    }

    builder.messages([
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system_role)
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(request)
            .build()?
            .into(),
    ]);

    let llm_request = builder.build()?;

    let response = openai_client.chat().create(llm_request).await?;

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

pub async fn tokenize_and_truncate(
    data: &str,
    max_tokens: usize,
    keep_end: bool,
) -> Result<String> {
    let bpe = cl100k_base()?;
    let tokens = bpe.encode_ordinary(&*data);
    let token_count = tokens.len();

    let input_data_source = if keep_end == true {
        "chat history".to_string()
    } else {
        "vector db retriever".to_string()
    };

    info!("Input tokens: {} from: {}", token_count, input_data_source);

    if token_count > max_tokens {
        let truncated_tokens = if keep_end {
            tokens[token_count - max_tokens..].to_vec()
        } else {
            tokens[..max_tokens].to_vec()
        };

        let truncated_data = bpe.decode(truncated_tokens)?;
        let truncated_count = bpe.encode_ordinary(&truncated_data).len();

        info!(
            "Truncated input data: {} -> {} (keep_end: {})",
            token_count, truncated_count, keep_end
        );

        let result = if keep_end {
            format!(
                "...[начало обрезано в целях экономии контекстного окна]\n{}",
                truncated_data
            )
        } else {
            format!(
                "{}\n[конец обрезан в целях экономии контекстного окна]...",
                truncated_data
            )
        };

        Ok(result)
    } else {
        info!(
            "Input data tokens quantity ({}) < max_tokens, no need to truncate it",
            token_count
        );
        Ok(data.to_string())
    }
}
