use crate::uniframe_studio::local_utils::{send_idea_email, verify_turnstile_token};
use blacksmith_core::utils::uniframe_studio::heleket_client::{HeleketClient, HeleketConfig};
use axum::extract::{Path, State};
use axum::{Extension, Json};
use blacksmith_core::models::uniframe_studio::accounting_models::{ProcessingType, UserBalance};
use blacksmith_core::models::uniframe_studio::payment_models::{TopUpRequest, TopUpResponse};
use blacksmith_core::models::uniframe_studio::uniframe_studio::ReviewUploadResponse;
use blacksmith_core::models::uniframe_studio::uniframe_studio::{
    ApiError, DubbingPipelinePrepareRequest, DubbingPipelinePrepareResponse,
    DubbingPipelineRequest, DubbingPipelineResponse, DubbingPipelineStatus, SubmitIdeaRequest,
    SubmitIdeaResponse, UserJob,
};
use blacksmith_core::state::uniframe_studio::app_state::UniframeStudioAppState;
use http::StatusCode;
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info, warn};

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

    let pipeline_info = match sqlx::query(
        "SELECT user_id, estimated_cost_usd FROM dubbing_pipelines WHERE job_id = ?",
    )
    .bind(&request.job_id)
    .fetch_optional(&state.local_db_pool)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    code: "PIPELINE_NOT_FOUND".to_string(),
                    message: "Pipeline not found".to_string(),
                }),
            ))
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: "Database error".to_string(),
                }),
            ))
        }
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

    let mut user_balance = match UserBalance::get_or_create(&state.local_db_pool, &user_id).await {
        Ok(balance) => balance,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    code: "BALANCE_ERROR".to_string(),
                    message: "Failed to get user balance".to_string(),
                }),
            ))
        }
    };

    if !user_balance.has_sufficient_balance(estimated_cost) {
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            Json(ApiError {
                code: "INSUFFICIENT_BALANCE".to_string(),
                message: "Insufficient balance".to_string(),
            }),
        ));
    }

    if !user_balance.can_start_job(ProcessingType::Dubbing) {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiError {
                code: "JOB_ALREADY_RUNNING".to_string(),
                message: "Another dubbing job is already running".to_string(),
            }),
        ));
    }

    match state
        .dubbing_pipeline_service
        .start_pipeline(request.clone(), state.clone(), user_id.clone())
        .await
    {
        Ok(response) => {
            info!(
                "Successfully started dubbing pipeline for job: {} initiated by user {}",
                response.job_id, user_id
            );

            if let Err(_) = user_balance
                .charge_and_reserve_job_slot(
                    &state.local_db_pool,
                    estimated_cost,
                    ProcessingType::Dubbing,
                    &format!("Dubbing job {}", request.job_id),
                )
                .await
            {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        code: "PAYMENT_FAILED".to_string(),
                        message: "Failed to process payment".to_string(),
                    }),
                ));
            }

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
    State(state): State<Arc<UniframeStudioAppState>>,
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get user balance".to_string(),
            )
        })?;

    Ok(Json(balance))
}

pub async fn refund_failed_job(
    Path(job_id): Path<String>,
    State(app_state): State<Arc<UniframeStudioAppState>>,
    Extension(user_id): Extension<String>,
) -> Result<StatusCode, StatusCode> {
    let db_pool = app_state.get_db_pool();

    let pipeline_row = sqlx::query(
        "SELECT user_id, status, refund_status, estimated_cost_usd FROM dubbing_pipelines WHERE job_id = ?"
    )
        .bind(&job_id)
        .fetch_one(db_pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let pipeline_user_id: String = pipeline_row.get("user_id");
    let status: String = pipeline_row.get("status");
    let refund_status: Option<String> = pipeline_row.get("refund_status");
    let estimated_cost: Option<f64> = pipeline_row.get("estimated_cost_usd");

    if pipeline_user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(ref_status) = refund_status {
        if ref_status == "refunded" {
            return Ok(StatusCode::OK);
        }
    }

    if status != "failed" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut user_balance = UserBalance::get_or_create(db_pool, &user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(cost) = estimated_cost {
        user_balance
            .add_funds(db_pool, cost, &format!("Refund for failed job: {}", job_id))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    user_balance
        .complete_job(db_pool, ProcessingType::Dubbing)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE dubbing_pipelines SET refund_status = 'refunded' WHERE job_id = ?")
        .bind(&job_id)
        .execute(db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

pub async fn handle_submit_idea(
    Json(request): Json<SubmitIdeaRequest>,
) -> Result<Json<SubmitIdeaResponse>, (StatusCode, Json<SubmitIdeaResponse>)> {
    if request.idea.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(SubmitIdeaResponse {
                success: false,
                message: "The message can't be empty".to_string(),
            }),
        ));
    }

    if request.idea.len() > 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(SubmitIdeaResponse {
                success: false,
                message: "The message ios too long (max 1000 symbols)".to_string(),
            }),
        ));
    }

    match verify_turnstile_token(&request.captcha_token).await {
        Ok(true) => {
            info!("Turnstile verification successful");
        }
        Ok(false) => {
            error!("Turnstile verification failed");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(SubmitIdeaResponse {
                    success: false,
                    message: "Captcha verification failed".to_string(),
                }),
            ));
        }
        Err(e) => {
            error!("Turnstile verification error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SubmitIdeaResponse {
                    success: false,
                    message: "Captcha verification error".to_string(),
                }),
            ));
        }
    }

    info!(
        "Idea received: {}",
        request.idea.chars().take(50).collect::<String>()
    );

    if let Err(e) = send_idea_email(&request.idea).await {
        error!("Failed to send idea email: {}", e);
        warn!("Idea submission completed but email delivery failed");
    } else {
        info!("Idea email sent successfully");
    }

    Ok(Json(SubmitIdeaResponse {
        success: true,
        message: "Submission successful".to_string(),
    }))
}

pub async fn create_payment_invoice(
    Extension(user_id): Extension<String>,
    Json(request): Json<TopUpRequest>,
) -> Result<Json<TopUpResponse>, (StatusCode, String)> {
    if request.amount_usd <= 0.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Amount must be greater than 0".to_string(),
        ));
    }

    if request.amount_usd < 10.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Minimum top-up amount is $10.00".to_string(),
        ));
    }

    if request.amount_usd > 1000.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Maximum top-up amount is $1,000.00".to_string(),
        ));
    }

    let config = HeleketConfig::default();
    let client = HeleketClient::new(config);

    let invoice = client
        .create_invoice(request.amount_usd, &user_id)
        .await
        .map_err(|e| {
            eprintln!("Failed to create Heleket invoice: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create payment invoice".to_string(),
            )
        })?;

    Ok(Json(TopUpResponse {
        payment_url: invoice.url,
        order_id: invoice.order_id,
        amount_usd: request.amount_usd,
    }))
}
