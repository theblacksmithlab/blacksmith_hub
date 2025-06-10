use axum::extract::{Path, State};
use axum::Json;
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
        .prepare_pipeline(request)
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
    Json(request): Json<DubbingPipelineRequest>,
) -> Result<Json<DubbingPipelineResponse>, (StatusCode, Json<ApiError>)> {
    info!("Starting dubbing pipeline {}...", request.pipeline_id);

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
    let user_is_premium = is_premium_user(None).await;

    match state
        .dubbing_pipeline_service
        .start_pipeline(request, user_is_premium)
        .await
    {
        Ok(response) => {
            tracing::Span::current().record("pipeline_id", &response.pipeline_id);
            info!(
                "Successfully started dubbing pipeline {}",
                response.pipeline_id
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

#[instrument(skip(state))]
pub async fn get_dubbing_pipeline_status(
    State(state): State<Arc<UniframeStudioAppState>>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<DubbingPipelineStatus>, (StatusCode, Json<ApiError>)> {
    info!(
        "Retrieving dubbing pipeline status for pipeline_id={}",
        pipeline_id
    );

    match state
        .dubbing_pipeline_service
        .get_pipeline_status(&pipeline_id)
        .await
    {
        Ok(status) => {
            info!("Successfully retrieved pipeline status");
            Ok(Json(status))
        }
        Err(e) => {
            error!("Failed to get pipeline status: {}", e);
            if e.to_string().contains("Pipeline not found") {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiError {
                        code: "PIPELINE_NOT_FOUND".to_string(),
                        message: format!("Pipeline not found: {}", pipeline_id),
                    }),
                ))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        code: "PIPELINE_STATUS_ERROR".to_string(),
                        message: format!("Failed to get pipeline status: {}", e),
                    }),
                ))
            }
        }
    }
}

pub async fn is_premium_user(_user_id: Option<&str>) -> bool {
    // TODO: Implement user's subscription tier detection fn
    false
}
