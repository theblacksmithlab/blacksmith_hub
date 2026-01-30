use crate::models::common::ai::GoogleModel;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// Using v1beta for access to latest models (Gemini 3).
// Note: Response structure may change without notice.
const GOOGLE_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/";

#[derive(Clone)]
pub struct GoogleClient {
    client: Client,
    api_key: String,
}

#[derive(Serialize)]
struct GoogleRequest {
    contents: Vec<GoogleContent>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GoogleSystemInstruction>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GoogleGenerationConfig>,
}

#[derive(Serialize)]
struct GoogleContent {
    role: String,
    parts: Vec<GooglePart>,
}

#[derive(Serialize)]
struct GoogleSystemInstruction {
    parts: Vec<GooglePart>,
}

#[derive(Serialize)]
struct GooglePart {
    text: String,
}

#[derive(Serialize)]
struct GoogleGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(rename = "responseMimeType", skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Deserialize)]
struct GoogleResponse {
    candidates: Option<Vec<GoogleCandidate>>,
    error: Option<GoogleError>,
}

#[derive(Deserialize)]
struct GoogleCandidate {
    content: Option<GoogleResponseContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GoogleResponseContent {
    parts: Vec<GoogleResponsePart>,
}

#[derive(Deserialize)]
struct GoogleResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GoogleError {
    message: String,
}

impl GoogleClient {
    /// Create a new GoogleClient using GOOGLE_API_KEY environment variable
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("GOOGLE_API_KEY")
            .context("GOOGLE_API_KEY environment variable not set")?;

        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    /// Create a new GoogleClient with explicit API key
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Send a chat completion request to Google Gemini API
    pub async fn chat_completion(
        &self,
        system_role: &str,
        user_message: &str,
        model: &GoogleModel,
        temperature: f32,
    ) -> Result<String> {
        let request_body = GoogleRequest {
            contents: vec![GoogleContent {
                role: "user".to_string(),
                parts: vec![GooglePart {
                    text: user_message.to_string(),
                }],
            }],
            system_instruction: if system_role.is_empty() {
                None
            } else {
                Some(GoogleSystemInstruction {
                    parts: vec![GooglePart {
                        text: system_role.to_string(),
                    }],
                })
            },
            generation_config: Some(GoogleGenerationConfig {
                temperature: Some(temperature),
                response_mime_type: None,
            }),
        };

        self.send_request(request_body, model).await
    }

    /// Send a chat completion request expecting JSON response
    pub async fn chat_completion_json(
        &self,
        system_role: &str,
        user_message: &str,
        model: &GoogleModel,
    ) -> Result<String> {
        if system_role.is_empty() {
            warn!("Google chat_completion_json called with empty system_role");
            return Ok("Error generating response... Please try again".to_string());
        }

        let request_body = GoogleRequest {
            contents: vec![GoogleContent {
                role: "user".to_string(),
                parts: vec![GooglePart {
                    text: user_message.to_string(),
                }],
            }],
            system_instruction: Some(GoogleSystemInstruction {
                parts: vec![GooglePart {
                    text: system_role.to_string(),
                }],
            }),
            generation_config: Some(GoogleGenerationConfig {
                temperature: Some(0.0),
                response_mime_type: Some("application/json".to_string()),
            }),
        };

        self.send_request(request_body, model).await
    }

    async fn send_request(&self, request_body: GoogleRequest, model: &GoogleModel) -> Result<String> {
        let url = format!(
            "{}{}:generateContent?key={}",
            GOOGLE_API_BASE_URL,
            model.as_str(),
            self.api_key
        );

        info!("Sending request to Google Gemini API, model: {}", model.as_str());

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Google Gemini API error ({}): {}", status, error_text));
        }

        let response_body: GoogleResponse = response
            .json()
            .await
            .context("Failed to parse Google Gemini API response")?;

        if let Some(error) = response_body.error {
            return Err(anyhow!("Google Gemini API error: {}", error.message));
        }

        let candidates = response_body
            .candidates
            .ok_or_else(|| anyhow!("No candidates in Google Gemini API response"))?;

        // Check if response was blocked by safety filter
        if let Some(candidate) = candidates.first() {
            if candidate.content.is_none() {
                let reason = candidate.finish_reason.as_deref().unwrap_or("unknown");
                warn!("Google Gemini response blocked, finish_reason: {}", reason);
                return Ok("Error generating response... Please try again".to_string());
            }
        }

        let text = candidates
            .into_iter()
            .filter_map(|candidate| candidate.content)
            .flat_map(|content| content.parts)
            .filter_map(|part| part.text)
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(anyhow!("Empty response from Google Gemini API"));
        }

        Ok(text)
    }
}
