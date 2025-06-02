use crate::models::uniframe_studio::uniframe_dubbing_client::UniframeDubbingClient;
use crate::models::uniframe_studio::uniframe_studio::{
    DubbingJobRequest, DubbingJobResult, DubbingPipelinePrepareRequest,
    DubbingPipelinePrepareResponse, DubbingPipelineRequest, DubbingPipelineResponse,
    DubbingPipelineStatus,
};
use anyhow::{Context, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{error, info, instrument};
use uuid::Uuid;
use crate::gpu_client::immers_cloud_client::ImmersCloudClient;

#[derive(Debug, Clone)]
struct PipelineState {
    pipeline_id: String,
    job_id: String,
    status: String,
    step_description: String,
    progress_percentage: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    result_urls: Option<HashMap<String, String>>,
    error_message: Option<String>,
    processing_steps: Option<Vec<String>>,
}

pub struct DubbingPipelineService {
    dubbing_client: UniframeDubbingClient,
    s3_client: Arc<S3Client>,
    pipelines: Arc<Mutex<HashMap<String, PipelineState>>>,
}

impl DubbingPipelineService {
    pub fn new(dubbing_client: UniframeDubbingClient, s3_client: Arc<S3Client>) -> Self {
        Self {
            dubbing_client,
            s3_client,
            pipelines: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[instrument(skip(self, request), fields(pipeline_id))]
    pub async fn prepare_pipeline(
        &self,
        request: DubbingPipelinePrepareRequest,
    ) -> Result<DubbingPipelinePrepareResponse> {
        let pipeline_id = Uuid::new_v4().to_string();
        tracing::Span::current().record("pipeline_id", &pipeline_id);
        let job_id = Uuid::new_v4().to_string();

        info!("Preparing pipeline {} | job_id: {}", pipeline_id, job_id);
        
        let s3_key = format!("uploads/{}/input/{}", pipeline_id, request.filename);
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

        let response = DubbingPipelinePrepareResponse {
            pipeline_id,
            job_id,
            upload_url: upload_url.uri().to_string(),
            video_s3_url,
            expires_in: 3600,
        };
        
        info!("Prepare fn server response: {:?}", response);
        
        Ok(response)
    }

    #[instrument(skip(self, request), fields(pipeline_id))]
    pub async fn start_pipeline(
        &self,
        request: DubbingPipelineRequest,
        is_premium: bool,
    ) -> Result<DubbingPipelineResponse> {
        let pipeline_id = request.pipeline_id.clone();
        let job_id = request.job_id.clone();
        let now = Utc::now();

        tracing::Span::current().record("pipeline_id", &pipeline_id);

        let initial_pipeline_state = PipelineState {
            pipeline_id: pipeline_id.clone(),
            job_id: job_id.clone(),
            status: "initializing".to_string(),
            step_description: "Setting up technical environment...".to_string(),
            progress_percentage: Some(0),
            created_at: now,
            updated_at: now,
            completed_at: None,
            result_urls: None,
            error_message: None,
            processing_steps: None,
        };

        {
            let mut pipelines = self.pipelines.lock().await;
            pipelines.insert(pipeline_id.clone(), initial_pipeline_state);
        }

        let response = DubbingPipelineResponse {
            pipeline_id: pipeline_id.clone(),
            job_id: job_id.clone(),
            status: "initializing".to_string(),
            created_at: now.to_rfc3339(),
        };

        let request_clone = request;
        let dubbing_client = self.dubbing_client.clone();
        let s3_client = self.s3_client.clone();
        let pipelines = self.pipelines.clone();

        tokio::spawn(async move {
            Self::pipeline_processor(
                pipeline_id,
                job_id,
                request_clone,
                is_premium,
                dubbing_client,
                s3_client,
                pipelines,
            ).await;
        });

        Ok(response)
    }

    async fn pipeline_processor(
        pipeline_id: String,
        job_id: String,
        request: DubbingPipelineRequest,
        is_premium: bool,
        dubbing_client: UniframeDubbingClient,
        s3_client: Arc<S3Client>,
        pipelines: Arc<Mutex<HashMap<String, PipelineState>>>,
    ) {
        info!("Starting pipeline processor for pipeline_id={}", pipeline_id);
        
        let gpu_result = async {
            info!("Checking GPU processing service status...");

            let immers_cloud_client = ImmersCloudClient::new(
                &std::env::var("IMMERS_USERNAME").context("IMMERS_USERNAME not set")?,
                &std::env::var("IMMERS_PASSWORD").context("IMMERS_PASSWORD not set")?,
                &std::env::var("IMMERS_PROJECT").context("IMMERS_PROJECT not set")?,
                std::env::var("IMMERS_AI_SERVER_ID").context("IMMERS_AI_SERVER_ID not set")?
            ).await.context("Failed to initialize Immers.Cloud client")?;

            let gpu_service_status = immers_cloud_client.get_service_status().await?;

            info!("GPU processing service status: {}", gpu_service_status);

            if gpu_service_status == "SHELVED_OFFLOADED" || gpu_service_status == "SHELVED" {
                info!("GPU processing service is sleeping, initiating wake-up process...");

                Self::update_pipeline_status(
                    &pipelines,
                    &pipeline_id,
                    "initializing",
                    "Resurrecting system components...",
                    Some(0),
                    None,
                    None,
                ).await;

                immers_cloud_client.unshelve_server().await?;

                Self::update_pipeline_status(
                    &pipelines,
                    &pipeline_id,
                    "initializing",
                    "Warming up GPUs...",
                    Some(0),
                    None,
                    None,
                ).await;

                immers_cloud_client.wait_for_service_active(600).await?;

                info!("GPU processing service is now active, waiting for services to start...");

                Self::update_pipeline_status(
                    &pipelines,
                    &pipeline_id,
                    "initializing",
                    "Preparing technical environment...",
                    Some(0),
                    None,
                    None,
                ).await;

                tokio::time::sleep(Duration::from_secs(90)).await;
                
                let max_attempts = 30;
                for attempt in 1..=max_attempts {
                    info!("Checking GPU processing service readiness, attempt {}/{}", attempt, max_attempts);

                    match dubbing_client.health_check().await {
                        Ok(_) => {
                            info!("GPU processing service is ready");
                            break;
                        }
                        Err(e) => {
                            if attempt == max_attempts {
                                return Err(anyhow::anyhow!("GPU processing service failed to become ready: {}", e));
                            }
                            tokio::time::sleep(Duration::from_secs(10)).await;
                        }
                    }
                }

                info!("GPU processing service and services are ready for processing");
            } else if gpu_service_status == "ACTIVE" {
                info!("GPU processing service is already active");
                
                if let Err(e) = dubbing_client.health_check().await {
                    return Err(anyhow::anyhow!("GPU processing service is not responding: {}", e));
                }
            } else {
                return Err(anyhow::anyhow!("GPU processing service is in unexpected state: {}", gpu_service_status));
            }

            Ok(())
        }.await;
        
        if let Err(e) = gpu_result {
            error!("Failed to prepare GPU service: {}", e);
            Self::update_pipeline_status(
                &pipelines,
                &pipeline_id,
                "failed",
                "Technical environment initialization failed",
                Some(0),
                None,
                Some(&format!("Failed to prepare GPU service: {}", e)),
            ).await;
            return;
        }
        
        
        info!("GPU service ready, submitting job to processing service...");

        Self::update_pipeline_status(
            &pipelines,
            &pipeline_id,
            "initializing",
            "Launching processing pipeline...",
            Some(0),
            None,
            None,
        ).await;

        let dubbing_job_request = DubbingJobRequest {
            job_id: job_id.clone(),
            video_url: request.video_url,
            target_language: request.target_language,
            tts_provider: request.tts_provider,
            tts_voice: request.tts_voice,
            source_language: request.source_language,
            is_premium,
            api_keys: request.api_keys,
        };

        info!("DubbingJobRequest: {:?}", dubbing_job_request);

        let job_submission_result = dubbing_client.process_video(dubbing_job_request).await;

        match job_submission_result {
            Ok(job_status) => {
                info!("Successfully submitted job to processing service");
                
                Self::update_pipeline_status(
                    &pipelines,
                    &pipeline_id,
                    &job_status.status,
                    &job_status.step_description.unwrap_or_else(|| "Processing started".to_string()),
                    job_status.progress_percentage,
                    None,
                    job_status.error_message.as_deref(),
                ).await;
                
                
                info!("Starting pipeline monitoring process...");
                
                Self::run_dubbing_pipeline_process(
                    pipeline_id,
                    job_id,
                    dubbing_client,
                    s3_client,
                    pipelines,
                ).await;
            }
            Err(e) => {
                error!("Failed to submit job to processing service: {}", e);
                Self::update_pipeline_status(
                    &pipelines,
                    &pipeline_id,
                    "failed",
                    "Processing pipeline launch failed",
                    Some(0),
                    None,
                    Some(&format!("Failed to submit job: {}", e)),
                ).await;
            }
        }
    }
    
    async fn run_dubbing_pipeline_process(
        pipeline_id: String,
        job_id: String,
        dubbing_client: UniframeDubbingClient,
        s3_client: Arc<S3Client>,
        pipelines: Arc<Mutex<HashMap<String, PipelineState>>>,
    ) {
        info!(
            "Starting pipeline process for pipeline_id={}, job_id={}",
            pipeline_id, job_id
        );

        let max_attempts = 100;
        let interval = Duration::from_secs(30);
        let mut result: Option<Result<DubbingJobResult>> = None;

        for attempt in 1..=max_attempts {
            info!("Checking job status, attempt {}/{}", attempt, max_attempts);

            match dubbing_client.get_job_status(&job_id).await {
                Ok(status) => {
                    let step_description = status
                        .step_description
                        .clone()
                        .unwrap_or_else(|| format!("Processing step {}", status.step.unwrap_or(0)));

                    Self::update_pipeline_status(
                        &pipelines,
                        &pipeline_id,
                        &status.status,
                        &step_description,
                        status.progress_percentage,
                        None,
                        status.error_message.as_deref(),
                    )
                    .await;

                    if status.status == "completed" || status.status == "failed" {
                        if status.status == "completed" {
                            info!("Job completed successfully, retrieving results");
                            Self::update_pipeline_status(
                                &pipelines,
                                &pipeline_id,
                                "generating_results",
                                "retrieving_result_urls",
                                Some(100),
                                None,
                                None,
                            )
                            .await;

                            result = Some(dubbing_client.get_job_result(&job_id).await);
                        } else {
                            info!("Job failed with error: {:?}", status.error_message);
                            Self::update_pipeline_status(
                                &pipelines,
                                &pipeline_id,
                                "failed",
                                "job_processing_failed",
                                Some(100),
                                None,
                                status.error_message.as_deref(),
                            )
                            .await;
                            break;
                        }
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to get job status: {}", e);
                    if attempt >= 3 {
                        Self::update_pipeline_status(
                            &pipelines,
                            &pipeline_id,
                            "failed",
                            "failed_to_get_job_status",
                            Some(100),
                            None,
                            Some(&format!("Failed to get job status: {}", e)),
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
                info!("Processing successful job result");

                if let Some(result_urls) = job_result.result_urls {
                    let processed_urls = Self::process_result_urls(s3_client, result_urls).await;

                    match processed_urls {
                        Ok(urls) => {
                            info!(
                                "Pipeline completed successfully with {} result URLs",
                                urls.len()
                            );
                            Self::update_pipeline_status(
                                &pipelines,
                                &pipeline_id,
                                "completed",
                                "pipeline_completed",
                                Some(100),
                                Some(urls),
                                None,
                            )
                            .await;
                        }
                        Err(e) => {
                            error!("Failed to process result URLs: {}", e);
                            Self::update_pipeline_status(
                                &pipelines,
                                &pipeline_id,
                                "failed",
                                "failed_to_process_result_urls",
                                Some(100),
                                None,
                                Some(&format!("Failed to process result URLs: {}", e)),
                            )
                            .await;
                        }
                    }
                } else {
                    error!("Job completed but no result URLs provided");
                    Self::update_pipeline_status(
                        &pipelines,
                        &pipeline_id,
                        "failed",
                        "no_result_urls",
                        Some(100),
                        None,
                        Some("Job completed but no result URLs provided"),
                    )
                    .await;
                }
            }
            Some(Err(e)) => {
                error!("Failed to get job results: {}", e);
                Self::update_pipeline_status(
                    &pipelines,
                    &pipeline_id,
                    "failed",
                    "failed_to_get_job_results",
                    Some(100),
                    None,
                    Some(&format!("Failed to get job results: {}", e)),
                )
                .await;
            }
            None => {
                error!("Maximum waiting time exceeded");
                Self::update_pipeline_status(
                    &pipelines,
                    &pipeline_id,
                    "failed",
                    "timeout",
                    Some(100),
                    None,
                    Some("Maximum waiting time exceeded"),
                )
                .await;
            }
        }

        info!("Pipeline process completed for pipeline_id={}", pipeline_id);
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

    async fn update_pipeline_status(
        pipelines: &Arc<Mutex<HashMap<String, PipelineState>>>,
        pipeline_id: &str,
        status: &str,
        step_description: &str,
        progress_percentage: Option<i32>,
        result_urls: Option<HashMap<String, String>>,
        error_message: Option<&str>,
    ) {
        let now = Utc::now();
        let completed_at = if status == "completed" || status == "failed" {
            Some(now)
        } else {
            None
        };

        let mut pipelines = pipelines.lock().await;

        if let Some(pipeline) = pipelines.get_mut(pipeline_id) {
            pipeline.status = status.to_string();
            pipeline.step_description = step_description.to_string();
            pipeline.progress_percentage = progress_percentage;
            pipeline.updated_at = now;
            pipeline.completed_at = completed_at;

            if let Some(urls) = result_urls {
                pipeline.result_urls = Some(urls);
            }

            if let Some(error) = error_message {
                pipeline.error_message = Some(error.to_string());
            }
        }
    }

    pub async fn get_pipeline_status(&self, pipeline_id: &str) -> Result<DubbingPipelineStatus> {
        let pipelines = self.pipelines.lock().await;

        let pipeline = pipelines
            .get(pipeline_id)
            .ok_or_else(|| anyhow::anyhow!("Pipeline not found"))?;

        let status = DubbingPipelineStatus {
            pipeline_id: pipeline.pipeline_id.clone(),
            job_id: pipeline.job_id.clone(),
            status: pipeline.status.clone(),
            step_description: pipeline.step_description.clone(),
            progress_percentage: pipeline.progress_percentage,
            created_at: pipeline.created_at.to_rfc3339(),
            updated_at: pipeline.updated_at.to_rfc3339(),
            completed_at: pipeline.completed_at.map(|dt| dt.to_rfc3339()),
            result_urls: pipeline.result_urls.clone(),
            error_message: pipeline.error_message.clone(),
            processing_steps: pipeline.processing_steps.clone(),
        };

        Ok(status)
    }
}
