use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct DubbingPipelinePrepareRequest {
    pub filename: String,
    pub content_type: String,
}

#[derive(Debug, Serialize)]
pub struct DubbingPipelinePrepareResponse {
    pub pipeline_id: String,
    pub job_id: String,
    pub upload_url: String,
    pub video_s3_url: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct DubbingPipelineRequest {
    pub pipeline_id: String,
    pub job_id: String,
    pub video_url: String,
    pub target_language: String,
    pub tts_provider: String,
    pub tts_voice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,
    pub api_keys: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct DubbingPipelineResponse {
    pub pipeline_id: String,
    pub job_id: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct DubbingPipelineStatus {
    pub pipeline_id: String,
    pub job_id: String,
    pub status: String,
    pub step_description: String,
    pub progress_percentage: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub result_urls: Option<HashMap<String, String>>,
    pub error_message: Option<String>,
    pub processing_steps: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct DubbingJobRequest {
    pub job_id: String,
    pub video_url: String,
    pub target_language: String,
    pub tts_provider: String,
    pub tts_voice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,
    pub is_premium: bool,
    pub api_keys: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DubbingJobStatus {
    pub job_id: String,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_steps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_percentage: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_steps: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DubbingJobResult {
    pub job_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_urls: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}
