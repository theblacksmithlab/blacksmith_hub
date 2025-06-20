use crate::gpu_client::immers_cloud_client::ImmersCloudClient;
use crate::models::uniframe_studio::dubbing_client::DubbingClient;
use crate::models::uniframe_studio::uniframe_studio::{
    DubbingJobRequest, DubbingJobResult, DubbingPipelinePrepareRequest,
    DubbingPipelinePrepareResponse, DubbingPipelineRequest, DubbingPipelineResponse,
    DubbingPipelineStatus, PipelineStage, StepInfo,
};
use crate::state::uniframe_studio::app_state::UniframeStudioAppState;
use crate::utils::uniframe_studio::uniframe_studio::{get_adaptive_interval, validate_transcription_keywords};
use anyhow::{Context, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;
use chrono::Utc;
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info};
use uuid::Uuid;

pub struct DubbingPipelineService {
    dubbing_client: DubbingClient,
    s3_client: Arc<S3Client>,
    db_pool: SqlitePool,
}

impl DubbingPipelineService {
    pub fn new(
        dubbing_client: DubbingClient,
        s3_client: Arc<S3Client>,
        db_pool: SqlitePool,
    ) -> Self {
        Self {
            dubbing_client,
            s3_client,
            db_pool,
        }
    }

    const BACKEND_INNER_STEPS: &'static [StepInfo] = &[
        StepInfo {
            description: "Preparing pipeline...",
            stage: PipelineStage::Preparation,
        },
        StepInfo {
            description: "Setting up technical environment...",
            stage: PipelineStage::Preparation,
        },
        StepInfo {
            description: "Resurrecting system components...",
            stage: PipelineStage::Preparation,
        },
        StepInfo {
            description: "Warming up GPUs...",
            stage: PipelineStage::Preparation,
        },
        StepInfo {
            description: "Checking technical readiness...",
            stage: PipelineStage::Preparation,
        },
        StepInfo {
            description: "Launching processing pipeline...",
            stage: PipelineStage::Preparation,
        },
        StepInfo {
            description: "Retrieving result urls...",
            stage: PipelineStage::Finalization,
        },
        StepInfo {
            description: "Processing pipeline successfully completed!",
            stage: PipelineStage::Finalization,
        },
    ];

    fn determine_stage_info(
        &self,
        step_description: &str,
        processing_steps: &Option<Vec<String>>,
    ) -> (String, Option<i32>) {
        let stage = Self::BACKEND_INNER_STEPS
            .iter()
            .find(|step| step.description == step_description)
            .map(|step| step.stage.as_str())
            .unwrap_or("processing");

        let current_step_index = processing_steps.as_ref().and_then(|steps| {
            steps
                .iter()
                .position(|s| s == step_description)
                .map(|pos| pos as i32)
        });

        (stage.to_string(), current_step_index)
    }
    
    pub async fn prepare_pipeline(
        &self,
        request: DubbingPipelinePrepareRequest,
        user_id: Option<String>,
    ) -> Result<DubbingPipelinePrepareResponse> {
        let job_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        info!("Generated job_id for dubbing pipeline: {}", job_id);

        let s3_key = format!("uploads/{}/input/{}", job_id, request.system_file_name);
        let video_s3_url = format!(
            "s3://{}/{}",
            std::env::var("S3_BUCKET").unwrap_or("default-bucket".to_string()),
            s3_key
        );

        let upload_url = self
            .s3_client
            .put_object()
            .bucket(std::env::var("S3_BUCKET").unwrap_or("default-bucket".to_string()))
            .key(&s3_key)
            .content_type(&request.content_type)
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_secs(3600))
                    .build()?,
            )
            .await?;

        sqlx::query(
            "INSERT INTO dubbing_pipelines (
                               job_id, user_id, status, step_description,
                               progress_percentage, original_video_s3_url, system_file_name,
                               original_file_name, created_at, updated_at) 
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&job_id)
        .bind(&user_id)
        .bind("preparing")
        .bind("Preparing pipeline...")
        .bind(0i32)
        .bind(&video_s3_url)
        .bind(&request.system_file_name)
        .bind(&request.original_file_name)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db_pool)
        .await?;

        let response = DubbingPipelinePrepareResponse {
            job_id: job_id.clone(),
            upload_url: upload_url.uri().to_string(),
            video_s3_url,
            expires_in: 3600,
        };

        info!(
            "Dubbing pipeline prepared successfully for job: {}!",
            job_id
        );
        Ok(response)
    }

    pub async fn start_pipeline(
        &self,
        request: DubbingPipelineRequest,
        is_premium: bool,
        app_state: Arc<UniframeStudioAppState>,
    ) -> Result<DubbingPipelineResponse> {
        let job_id = request.job_id.clone();
        let now = Utc::now();

        Self::update_pipeline_status(
            &self.db_pool,
            &job_id,
            "initializing",
            "Setting up technical environment...",
            Some(1),
            None,
            None,
            None,
            None,
            None,
        )
        .await;

        let response = DubbingPipelineResponse {
            job_id: job_id.clone(),
            status: "initializing".to_string(),
            created_at: now.to_rfc3339(),
        };

        let request_clone = request;
        let dubbing_client = self.dubbing_client.clone();
        let s3_client = self.s3_client.clone();
        let db_pool = self.db_pool.clone();

        tokio::spawn(async move {
            Self::pipeline_processor(
                job_id,
                request_clone,
                is_premium,
                dubbing_client,
                s3_client,
                db_pool,
                app_state,
            )
            .await;
        });

        Ok(response)
    }

    async fn pipeline_processor(
        job_id: String,
        request: DubbingPipelineRequest,
        is_premium: bool,
        dubbing_client: DubbingClient,
        s3_client: Arc<S3Client>,
        db_pool: Pool<Sqlite>,
        app_state: Arc<UniframeStudioAppState>,
    ) {
        let validated_transcription_keywords = match request.transcription_keywords {
            Some(transcription_keywords) => {
                Some(validate_transcription_keywords(transcription_keywords, app_state).await)
            }
            None => None,
        };

        let gpu_processing_instance_init = async {
            info!("Checking GPU processing instance status...");

            let immers_cloud_client = ImmersCloudClient::new(
                &std::env::var("IMMERS_USERNAME").context("IMMERS_USERNAME not set")?,
                &std::env::var("IMMERS_PASSWORD").context("IMMERS_PASSWORD not set")?,
                &std::env::var("IMMERS_PROJECT").context("IMMERS_PROJECT not set")?,
                std::env::var("IMMERS_AI_SERVER_ID").context("IMMERS_AI_SERVER_ID not set")?,
            )
            .await
            .context("Failed to initialize Immers.Cloud client")?;

            let gpu_processing_instance_status = immers_cloud_client.get_service_status().await?;

            info!(
                "GPU processing instance status: {}",
                gpu_processing_instance_status
            );

            if gpu_processing_instance_status == "SHELVED_OFFLOADED"
                || gpu_processing_instance_status == "SHELVED"
            {
                info!("GPU processing instance is sleeping, initiating wake-up process...");

                Self::update_pipeline_status(
                    &db_pool,
                    &job_id,
                    "initializing",
                    "Resurrecting system components...",
                    Some(1),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await;

                immers_cloud_client.unshelve_server().await?;

                Self::update_pipeline_status(
                    &db_pool,
                    &job_id,
                    "initializing",
                    "Warming up GPUs...",
                    Some(1),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await;

                immers_cloud_client.wait_for_instance_active(600).await?;

                info!(
                    "GPU processing service is now active, waiting for it's components to start..."
                );

                Self::update_pipeline_status(
                    &db_pool,
                    &job_id,
                    "initializing",
                    "Checking technical readiness...",
                    Some(1),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await;

                tokio::time::sleep(Duration::from_secs(90)).await;

                let max_attempts = 30;
                for attempt in 1..=max_attempts {
                    info!(
                        "Checking GPU processing instance readiness, attempt {}/{}",
                        attempt, max_attempts
                    );

                    match dubbing_client.health_check().await {
                        Ok(_) => {
                            info!("GPU processing instance is ready!");
                            break;
                        }
                        Err(e) => {
                            if attempt == max_attempts {
                                return Err(anyhow::anyhow!(
                                    "GPU processing instance failed to become ready: {}",
                                    e
                                ));
                            }
                            tokio::time::sleep(Duration::from_secs(10)).await;
                        }
                    }
                }

                info!("GPU processing instance and it's components prepared successfully!");
            } else if gpu_processing_instance_status == "ACTIVE" {
                info!("GPU processing service is already active!");

                if let Err(e) = dubbing_client.health_check().await {
                    return Err(anyhow::anyhow!(
                        "GPU processing instance is not responding: {}",
                        e
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "GPU processing service is in unexpected state: {}",
                    gpu_processing_instance_status
                ));
            }

            Ok(())
        }
        .await;

        if let Err(e) = gpu_processing_instance_init {
            error!("Failed to prepare GPU instance: {}", e);
            Self::update_pipeline_status(
                &db_pool,
                &job_id,
                "failed",
                "Technical environment initialization failed",
                Some(0),
                None,
                Some(&format!("Failed to prepare GPU service: {}", e)),
                None,
                None,
                None,
            )
            .await;
            return;
        }

        info!("GPU instance ready, submitting job to dubbing_client...");

        Self::update_pipeline_status(
            &db_pool,
            &job_id,
            "initializing",
            "Launching processing pipeline...",
            Some(1),
            None,
            None,
            None,
            None,
            None,
        )
        .await;

        let dubbing_job_request = DubbingJobRequest {
            job_id: job_id.clone(),
            video_url: request.video_url,
            target_language: request.target_language,
            tts_provider: request.tts_provider,
            tts_voice: request.tts_voice,
            source_language: request.source_language,
            is_premium,
            transcription_keywords: validated_transcription_keywords,
        };

        let job_submission_result = dubbing_client.process_video(dubbing_job_request).await;

        match job_submission_result {
            Ok(job_status) => {
                info!("Successfully submitted job to dubbing client!");

                Self::update_pipeline_status(
                    &db_pool,
                    &job_id,
                    &job_status.status,
                    &job_status
                        .step_description
                        .unwrap_or_else(|| "Processing started".to_string()),
                    job_status.progress_percentage,
                    None,
                    job_status.error_message.as_deref(),
                    job_status.processing_steps,
                    None,
                    job_status.step,
                )
                .await;

                info!("Starting dubbing pipeline monitoring process...");

                Self::run_dubbing_pipeline_process(job_id, dubbing_client, s3_client, db_pool)
                    .await;
            }
            Err(e) => {
                error!("Failed to submit job to processing service: {}", e);
                Self::update_pipeline_status(
                    &db_pool,
                    &job_id,
                    "failed",
                    "Processing pipeline launch failed",
                    Some(0),
                    None,
                    Some(&format!("Failed to submit job: {}", e)),
                    None,
                    None,
                    None,
                )
                .await;
            }
        }
    }

    async fn run_dubbing_pipeline_process(
        job_id: String,
        dubbing_client: DubbingClient,
        s3_client: Arc<S3Client>,
        db_pool: Pool<Sqlite>,
    ) {
        let max_checks = 600;
        let mut result: Option<Result<DubbingJobResult>> = None;

        for check_number in 1..=max_checks {
            let interval = get_adaptive_interval(check_number);
            
            info!(
                "Monitoring job status, check {}/{}",
                check_number, max_checks
            );
            
            match dubbing_client.get_job_status(&job_id).await {
                Ok(status) => {
                    let step_description = status
                        .step_description
                        .clone()
                        .unwrap_or_else(|| format!("Processing step {}", status.step.unwrap_or(0)));

                    let presigned_review_url = if let Some(s3_uri) = &status.review_required_url {
                        Self::generate_single_presigned_url(&s3_client, s3_uri).await.ok()
                    } else {
                        None
                    };
                    
                    Self::update_pipeline_status(
                        &db_pool,
                        &job_id,
                        &status.status,
                        &step_description,
                        status.progress_percentage,
                        None,
                        status.error_message.as_deref(),
                        status.processing_steps,
                        presigned_review_url,
                        status.step,
                    )
                    .await;

                    if status.status == "completed" || status.status == "failed" {
                        if status.status == "completed" {
                            info!("Dubbing job completed successfully, retrieving results...");

                            Self::update_pipeline_status(
                                &db_pool,
                                &job_id,
                                "preparing_results",
                                "Retrieving result urls...",
                                Some(99),
                                None,
                                None,
                                None,
                                None,
                                None,
                            )
                            .await;

                            result = Some(dubbing_client.get_job_result(&job_id).await);
                        } else {
                            info!("Dubbing job failed with error: {:?}", status.error_message);
                            Self::update_pipeline_status(
                                &db_pool,
                                &job_id,
                                "failed",
                                "Dubbing job failed",
                                Some(0),
                                None,
                                status.error_message.as_deref(),
                                None,
                                None,
                                None,
                            )
                            .await;
                            break;
                        }
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to get job status: {}", e);
                    if check_number >= 3 {
                        Self::update_pipeline_status(
                            &db_pool,
                            &job_id,
                            "failed",
                            "Failed to get dubbing job status",
                            Some(0),
                            None,
                            Some(&format!("Failed to get dubbing job status: {}", e)),
                            None,
                            None,
                            None,
                        )
                        .await;
                        return;
                    }
                }
            }

            sleep(interval).await;
        }

        match result {
            Some(Ok(job_result)) => {
                info!("Processing successful dubbing job result...");

                if let Some(result_urls) = job_result.result_urls {
                    let processed_urls = Self::process_result_urls(s3_client, result_urls).await;

                    match processed_urls {
                        Ok(urls) => {
                            info!(
                                "Pipeline completed successfully with {} result URLs",
                                urls.len()
                            );
                            Self::update_pipeline_status(
                                &db_pool,
                                &job_id,
                                "completed",
                                "Processing pipeline successfully completed!",
                                Some(100),
                                Some(urls),
                                None,
                                None,
                                None,
                                None,
                            )
                            .await;
                        }
                        Err(e) => {
                            error!("Failed to process result URLs: {}", e);
                            Self::update_pipeline_status(
                                &db_pool,
                                &job_id,
                                "failed",
                                "Failed to process result urls",
                                Some(0),
                                None,
                                Some(&format!("Failed to process result URLs: {}", e)),
                                None,
                                None,
                                None,
                            )
                            .await;
                        }
                    }
                } else {
                    error!("Job completed but no result URLs provided");
                    Self::update_pipeline_status(
                        &db_pool,
                        &job_id,
                        "failed",
                        "No result urls",
                        Some(0),
                        None,
                        Some("Job completed but no result URLs provided"),
                        None,
                        None,
                        None,
                    )
                    .await;
                }
            }
            Some(Err(e)) => {
                error!("Failed to get job results: {}", e);
                Self::update_pipeline_status(
                    &db_pool,
                    &job_id,
                    "failed",
                    "Failed to get dubbing job results",
                    Some(0),
                    None,
                    Some(&format!("Failed to get dubbing job results: {}", e)),
                    None,
                    None,
                    None,
                )
                .await;
            }
            None => {
                error!("Maximum waiting time exceeded");
                Self::update_pipeline_status(
                    &db_pool,
                    &job_id,
                    "failed",
                    "Timeout",
                    Some(0),
                    None,
                    Some("Maximum waiting time exceeded"),
                    None,
                    None,
                    None,
                )
                .await;
            }
        }

        info!(
            "Dubbing pipeline successfully completed for job: {}",
            job_id
        );
    }
    
    async fn process_result_urls(
        s3_client: Arc<S3Client>,
        result_urls: HashMap<String, String>,
    ) -> Result<HashMap<String, String>> {
        let mut processed_urls = HashMap::new();

        for (key, url) in result_urls {
            if url.starts_with("s3://") {
                let s3_path = url
                    .strip_prefix("s3://")
                    .ok_or_else(|| anyhow::anyhow!("Invalid S3 URL format"))?;

                let parts: Vec<&str> = s3_path.splitn(2, '/').collect();
                if parts.len() != 2 {
                    return Err(anyhow::anyhow!("Invalid S3 URL format: missing key"));
                }

                let bucket = parts[0];
                let object_key = parts[1];

                let presigned_url = s3_client
                    .get_object()
                    .bucket(bucket)
                    .key(object_key)
                    .presigned(
                        PresigningConfig::builder()
                            .expires_in(Duration::from_secs(3600))
                            .build()?,
                    )
                    .await?;

                processed_urls.insert(key, presigned_url.uri().to_string());
            } else {
                processed_urls.insert(key, url);
            }
        }

        Ok(processed_urls)
    }

    async fn generate_single_presigned_url(
        s3_client: &S3Client,
        s3_url: &str,
    ) -> Result<String> {
        if !s3_url.starts_with("s3://") {
            return Ok(s3_url.to_string());
        }

        let s3_path = s3_url
            .strip_prefix("s3://")
            .ok_or_else(|| anyhow::anyhow!("Invalid S3 URL format"))?;

        let parts: Vec<&str> = s3_path.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid S3 URL format: missing key"));
        }

        let bucket = parts[0];
        let object_key = parts[1];

        let presigned_url = s3_client
            .get_object()
            .bucket(bucket)
            .key(object_key)
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_secs(900))
                    .build()?,
            )
            .await?;

        Ok(presigned_url.uri().to_string())
    }

    pub async fn get_review_upload_url(
        &self,
        job_id: &str,
    ) -> Result<String> {
        let s3_key = format!("jobs/{}/review_required/transcription_corrected.json", job_id);

        let presigned_request = self
            .s3_client
            .put_object()
            .bucket(std::env::var("S3_BUCKET").unwrap_or("default-bucket".to_string()))
            .key(&s3_key)
            .content_type("application/json")
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_secs(3600))
                    .build()?,
            )
            .await?;

        Ok(presigned_request.uri().to_string())
    }
    
    async fn update_pipeline_status(
        db_pool: &Pool<Sqlite>,
        job_id: &str,
        status: &str,
        step_description: &str,
        progress_percentage: Option<i32>,
        result_urls: Option<HashMap<String, String>>,
        error_message: Option<&str>,
        processing_steps: Option<Vec<String>>,
        review_required_url: Option<String>,
        step: Option<i32>,
    ) {
        let now = Utc::now();
        let completed_at = if status == "completed" || status == "failed" {
            Some(now.to_rfc3339())
        } else {
            None
        };

        let result_urls_json = result_urls.and_then(|urls| {
            serde_json::to_string(&urls)
                .map_err(|e| {
                    error!("Failed to serialize result_urls: {}", e);
                    e
                })
                .ok()
        });

        let processing_steps_json = processing_steps.and_then(|steps| {
            serde_json::to_string(&steps)
                .map_err(|e| {
                    error!("Failed to serialize processing_steps: {}", e);
                    e
                })
                .ok()
        });

        let query = "
        UPDATE dubbing_pipelines SET 
            status = ?,
            step = ?,
            step_description = ?,
            progress_percentage = ?,
            result_urls = ?,
            error_message = ?,
            processing_steps = ?,
            completed_at = ?,
            updated_at = ?,
            review_required_url = ?
        WHERE job_id = ?
    ";

        if let Err(e) = sqlx::query(query)
            .bind(status)
            .bind(step)
            .bind(step_description)
            .bind(progress_percentage)
            .bind(result_urls_json)
            .bind(error_message)
            .bind(processing_steps_json)
            .bind(completed_at)
            .bind(now.to_rfc3339())
            .bind(review_required_url)
            .bind(job_id)
            .execute(db_pool)
            .await
        {
            error!("Failed to update pipeline status: {}", e);
        }
    }

    pub async fn get_pipeline_status(&self, job_id: &str) -> Result<DubbingPipelineStatus> {
        let query = "
        SELECT 
            job_id, status, step_description, progress_percentage,
            created_at, updated_at, completed_at, result_urls,
            error_message, processing_steps, system_file_name, original_file_name, step,
            review_required_url
        FROM dubbing_pipelines 
        WHERE job_id = ?
    ";

        let row = sqlx::query(query)
            .bind(job_id)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Pipeline not found"))?;

        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");
        let completed_at: Option<String> = row.get("completed_at");

        let result_urls: Option<String> = row.get("result_urls");
        let result_urls_vec = result_urls.and_then(|json| serde_json::from_str(&json).ok());

        let processing_steps: Option<String> = row.get("processing_steps");
        let processing_steps_value =
            processing_steps.and_then(|json| serde_json::from_str(&json).ok());

        let step_description: String = row.get("step_description");
        let (stage, current_step_index) =
            self.determine_stage_info(&step_description, &processing_steps_value);

        let status = DubbingPipelineStatus {
            job_id: row.get("job_id"),
            status: row.get("status"),
            step_description: row.get("step_description"),
            progress_percentage: row.get("progress_percentage"),
            created_at,
            updated_at,
            completed_at,
            result_urls: result_urls_vec,
            error_message: row.get("error_message"),
            processing_steps: processing_steps_value,
            stage: Some(stage),
            current_step_index,
            original_file_name: row.get("original_file_name"),
            step: row.get("step"),
            review_required_url: row.get("review_required_url"),
        };

        Ok(status)
    }
}
