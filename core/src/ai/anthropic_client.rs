use crate::models::common::ai::AnthropicModel;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 8192;

#[derive(Clone)]
pub struct AnthropicClient {
    client: Client,
    api_key: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicError,
}

#[derive(Deserialize)]
struct AnthropicError {
    message: String,
}

impl AnthropicClient {
    /// Create a new AnthropicClient using ANTHROPIC_API_KEY environment variable
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;

        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    /// Create a new AnthropicClient with explicit API key
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Send a chat completion request to Anthropic API
    pub async fn chat_completion(
        &self,
        system_role: &str,
        user_message: &str,
        model: &AnthropicModel,
        temperature: f32,
    ) -> Result<String> {
        let request_body = AnthropicRequest {
            model: model.as_str().to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system: if system_role.is_empty() {
                None
            } else {
                Some(system_role.to_string())
            },
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
            temperature: Some(temperature),
        };

        self.send_request(request_body).await
    }

    /// Send a chat completion request expecting JSON response
    pub async fn chat_completion_json(
        &self,
        system_role: &str,
        user_message: &str,
        model: &AnthropicModel,
    ) -> Result<String> {
        if system_role.is_empty() {
            warn!("Anthropic chat_completion_json called with empty system_role");
            return Ok("Error generating response... Please try again".to_string());
        }

        let request_body = AnthropicRequest {
            model: model.as_str().to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system: Some(system_role.to_string()),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
            temperature: Some(0.0),
        };

        self.send_request(request_body).await
    }

    async fn send_request(&self, request_body: AnthropicRequest) -> Result<String> {
        info!(
            "Sending request to Anthropic API, model: {}",
            request_body.model
        );

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(&error_text)
            {
                return Err(anyhow!(
                    "Anthropic API error ({}): {}",
                    status,
                    error_response.error.message
                ));
            }

            return Err(anyhow!("Anthropic API error ({}): {}", status, error_text));
        }

        let response_body: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic API response")?;

        let text = response_body
            .content
            .into_iter()
            .filter(|block| block.content_type == "text")
            .filter_map(|block| block.text)
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(anyhow!("Empty response from Anthropic API"));
        }

        Ok(text)
    }
}
