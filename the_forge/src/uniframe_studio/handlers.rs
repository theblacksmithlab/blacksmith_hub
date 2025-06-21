use axum::extract::{Path, State};
use axum::{Extension, Json};
use core::models::uniframe_studio::uniframe_studio::{
    ApiError, DubbingPipelinePrepareRequest, DubbingPipelinePrepareResponse,
    DubbingPipelineRequest, DubbingPipelineResponse, DubbingPipelineStatus, UserJob,
};
use core::state::uniframe_studio::app_state::UniframeStudioAppState;
use http::StatusCode;
use std::sync::Arc;
use sqlx::Row;
use tracing::{error, info};
use core::models::uniframe_studio::uniframe_studio::ReviewUploadResponse;
use core::models::uniframe_studio::accounting_models::{ProcessingType, UserBalance};

pub async fn prepare_dubbing_pipeline(
    State(state): State<Arc<UniframeStudioAppState>>,
    Extension(user_id): Extension<String>,
    Json(request): Json<DubbingPipelinePrepareRequest>,
) -> Result<Json<DubbingPipelinePrepareResponse>, (StatusCode, Json<ApiError>)> {
    info!("Preparing dubbing pipeline...");

    if request.system_file_name.is_empty() || request.original_file_name.is_empty() {
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
    
    //
    let pipeline_info = match sqlx::query(
        "SELECT user_id, estimated_cost_usd FROM dubbing_pipelines WHERE job_id = ?"
    )
        .bind(&request.job_id)
        .fetch_optional(&state.local_db_pool)
        .await {
        Ok(Some(row)) => row,
        Ok(None) => return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "PIPELINE_NOT_FOUND".to_string(),
                message: "Pipeline not found".to_string(),
            }),
        )),
        Err(_) => return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: "Database error".to_string(),
            }),
        )),
    };

    let user_id_from_db: String = pipeline_info.get("user_id");
    let estimated_cost: f64 = pipeline_info.get("estimated_cost_usd");
    
    // Check in the real workflow, may be redundant
    if user_id != user_id_from_db {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError {
                code: "ACCESS_DENIED".to_string(),
                message: "You don't own this pipeline".to_string(),
            }),
        ));
    }
    
    // Проверяем баланс
    let mut user_balance = match UserBalance::get_or_create(&state.local_db_pool, &user_id_from_db).await {
        Ok(balance) => balance,
        Err(_) => return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "BALANCE_ERROR".to_string(),
                message: "Failed to get user balance".to_string(),
            }),
        )),
    };

    // Проверяем достаточность средств
    if !user_balance.has_sufficient_balance(estimated_cost) {
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            Json(ApiError {
                code: "INSUFFICIENT_BALANCE".to_string(),
                message: "Insufficient balance".to_string(),
            }),
        ));
    }

    // Проверяем возможность запуска
    if !user_balance.can_start_job(ProcessingType::Dubbing) {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiError {
                code: "JOB_ALREADY_RUNNING".to_string(),
                message: "Another dubbing job is already running".to_string(),
            }),
        ));
    }

    // Списываем средства
    if let Err(_) = user_balance.charge_and_reserve_job_slot(
        &state.local_db_pool,
        estimated_cost,
        ProcessingType::Dubbing,
        &format!("Dubbing job {}", request.job_id)
    ).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "PAYMENT_FAILED".to_string(),
                message: "Failed to process payment".to_string(),
            }),
        ));
    }
    //
    
    match state
        .dubbing_pipeline_service
        .start_pipeline(request, user_is_premium, state.clone(), user_id_from_db)
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
    true
}

pub async fn get_user_jobs(
    State(app_state): State<Arc<UniframeStudioAppState>>,
    Extension(user_id): Extension<String>,
) -> Result<Json<Vec<UserJob>>, StatusCode> {
    let db_pool = app_state.get_db_pool();

    let query = "
        SELECT 
            job_id,
            original_file_name,
            status,
            created_at
        FROM dubbing_pipelines
        WHERE user_id = ?
        ORDER BY created_at DESC
    ";

    let rows = sqlx::query_as::<_, (String, String, String, String)>(query)
        .bind(&user_id)
        .fetch_all(db_pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch user jobs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let jobs: Vec<UserJob> = rows
        .into_iter()
        .map(|(job_id, original_file_name, status, created_at)| UserJob {
            job_id,
            original_file_name,
            status,
            created_at,
        })
        .collect();

    Ok(Json(jobs))
}

pub async fn submit_review(
    Path(job_id): Path<String>,
    State(state): State<Arc<UniframeStudioAppState>>
) -> Result<Json<ReviewUploadResponse>, (StatusCode, Json<ApiError>)> {
    match state
        .dubbing_pipeline_service
        .get_review_upload_url(&job_id)
        .await
    {
        Ok(upload_url) => Ok(Json(ReviewUploadResponse { upload_url })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "REVIEW_UPLOAD_FAILED".to_string(),
                message: format!("Failed to get review upload URL: {}", e),
            }),
        )),
    }
}

pub async fn get_user_balance(
    State(app_state): State<Arc<UniframeStudioAppState>>,
    Extension(user_id): Extension<String>,
) -> Result<Json<UserBalance>, (StatusCode, String)> {
    let db_pool = app_state.get_db_pool();
    
    let balance = UserBalance::get_or_create(&db_pool, &user_id)
        .await
        .map_err(|e| {
            eprintln!("Database error getting user balance: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user balance".to_string())
        })?;

    Ok(Json(balance))
}
