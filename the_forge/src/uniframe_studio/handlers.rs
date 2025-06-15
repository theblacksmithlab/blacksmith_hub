use axum::extract::{Path, State};
use axum::{Extension, Json};
use core::models::uniframe_studio::uniframe_studio::{
    ApiError, DubbingPipelinePrepareRequest, DubbingPipelinePrepareResponse,
    DubbingPipelineRequest, DubbingPipelineResponse, DubbingPipelineStatus,
};
use core::state::uniframe_studio::app_state::UniframeStudioAppState;
use http::StatusCode;
use std::sync::Arc;
use tracing::{error, info, instrument};

#[instrument(skip(state, request))]
pub async fn prepare_dubbing_pipeline(
    State(state): State<Arc<UniframeStudioAppState>>,
    Extension(user_id): Extension<String>,
    Json(request): Json<DubbingPipelinePrepareRequest>,
) -> Result<Json<DubbingPipelinePrepareResponse>, (StatusCode, Json<ApiError>)> {
    info!("Preparing dubbing pipeline...");

    if request.filename.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_FILENAME".to_string(),
                message: "Filename cannot be empty".to_string(),
            }),
        ));
    }

    match state
        .dubbing_pipeline_service
        .prepare_pipeline(request, Some(user_id))
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "PREPARE_FAILED".to_string(),
                message: format!("Failed to prepare pipeline: {}", e),
            }),
        )),
    }
}

#[instrument(skip(state, request), fields(pipeline_id))]
pub async fn start_dubbing_pipeline(
    State(state): State<Arc<UniframeStudioAppState>>,
    Extension(user_id): Extension<String>,
    Json(request): Json<DubbingPipelineRequest>,
) -> Result<Json<DubbingPipelineResponse>, (StatusCode, Json<ApiError>)> {
    info!("Starting dubbing pipeline for job: {}...", request.job_id);

    if !request.video_url.starts_with("s3://") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_VIDEO_URL".to_string(),
                message: "Video URL must be in the format s3://bucket/key".to_string(),
            }),
        ));
    }

    if !["openai", "elevenlabs"].contains(&request.tts_provider.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_TTS_PROVIDER".to_string(),
                message: "TTS provider must be either 'openai' or 'elevenlabs'".to_string(),
            }),
        ));
    }

    // TODO: Implement user's subscription tier detection
    let user_is_premium = is_premium_user(Some(&user_id)).await;

    match state
        .dubbing_pipeline_service
        .start_pipeline(request, user_is_premium)
        .await
    {
        Ok(response) => {
            info!(
                "Successfully started dubbing pipeline for job: {} initiated by user {}",
                response.job_id, user_id
            );
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to start dubbing pipeline: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    code: "PIPELINE_START_FAILED".to_string(),
                    message: format!("Failed to start dubbing pipeline: {}", e),
                }),
            ))
        }
    }
}

pub async fn get_dubbing_pipeline_status(
    State(state): State<Arc<UniframeStudioAppState>>,
    Extension(user_id): Extension<String>,
    Path(job_id): Path<String>,
) -> Result<Json<DubbingPipelineStatus>, (StatusCode, Json<ApiError>)> {
    info!(
        "Getting pipeline status for job {} by user {}",
        job_id, user_id
    );

    match state
        .dubbing_pipeline_service
        .get_pipeline_status(&job_id)
        .await
    {
        Ok(status) => {
            info!("Retrieved pipeline status for job {}", job_id);
            Ok(Json(status))
        }
        Err(e) => {
            error!("Failed to get pipeline status: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    code: "PIPELINE_NOT_FOUND".to_string(),
                    message: format!("Pipeline not found: {}", e),
                }),
            ))
        }
    }
}

pub async fn is_premium_user(_user_id: Option<&str>) -> bool {
    // TODO: Implement user's subscription tier detection fn
    false
}
