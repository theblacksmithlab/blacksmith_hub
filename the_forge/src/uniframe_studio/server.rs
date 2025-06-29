use crate::uniframe_studio::auth_handlers::{
    auth_middleware, handle_check_session, handle_send_magic_link, handle_verify_token,
};
use crate::uniframe_studio::handlers::{
    create_payment_invoice, get_dubbing_pipeline_status, get_user_balance, get_user_jobs,
    handle_submit_idea, prepare_dubbing_pipeline, refund_failed_job, start_dubbing_pipeline,
    submit_review,
};
use crate::uniframe_studio::local_db::setup_uniframe_studio_db;
use crate::uniframe_studio::payment_handlers::handle_payment_webhook;
use anyhow::{Context, Result};
use async_openai::Client as LLM_Client;
use axum::routing::{get, post};
use axum::Router;
use core::state::server_common::app_state::ServerAppState;
use core::state::uniframe_studio::app_state::UniframeStudioAppState;
use core::utils::server::server::start_server;
use http::StatusCode;
use std::sync::Arc;
use tracing::info;

pub async fn start_uniframe_studio_server(server_app_state: Arc<ServerAppState>) -> Result<()> {
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let llm_client = LLM_Client::new();

    let uniframe_studio_db_pool = setup_uniframe_studio_db().await?;

    let uniframe_studio_app_state = Arc::new(UniframeStudioAppState::new(
        s3_client,
        uniframe_studio_db_pool,
        llm_client,
    )?);

    info!("Initializing GPU instances...");
    uniframe_studio_app_state
        .initialize_gpu_instances()
        .await
        .context("Failed to initialize GPU instances")?;

    let router = get_uniframe_studio_router(uniframe_studio_app_state);

    info!("Starting Uniframe Studio server...");

    start_server(server_app_state, router).await
}

fn get_uniframe_studio_router(uniframe_studio_app_state: Arc<UniframeStudioAppState>) -> Router {
    let public_routes = Router::new()
        .route(
            "/api/uniframe/auth/send_magic_link",
            post(handle_send_magic_link).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/auth/verify_token",
            post(handle_verify_token).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/auth/check_session",
            get(handle_check_session).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/submit-idea",
            post(handle_submit_idea).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/payment/webhook",
            post(handle_payment_webhook).options(|| async { StatusCode::OK }),
        );

    let protected_routes = Router::new()
        .route(
            "/api/uniframe/dubbing/prepare",
            post(prepare_dubbing_pipeline).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/dubbing/start",
            post(start_dubbing_pipeline).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/dubbing/{job_id}/status",
            get(get_dubbing_pipeline_status).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/user/jobs",
            get(get_user_jobs).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/dubbing/{job_id}/submit_review",
            get(submit_review).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/user/balance",
            get(get_user_balance).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/user/refund/{jobId}",
            post(refund_failed_job).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/uniframe/payment/topup",
            post(create_payment_invoice).options(|| async { StatusCode::OK }),
        )
        .layer(axum::middleware::from_fn_with_state(
            uniframe_studio_app_state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(uniframe_studio_app_state)
}
