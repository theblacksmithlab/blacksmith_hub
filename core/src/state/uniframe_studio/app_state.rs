use crate::models::uniframe_studio::dubbing_client::DubbingClient;
use crate::utils::uniframe_studio::dubbing_pipeline::DubbingPipelineService;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use aws_sdk_s3::Client as S3Client;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

pub struct UniframeStudioAppState {
    pub s3_client: Arc<S3Client>,
    pub dubbing_service_url: String,
    pub dubbing_client: DubbingClient,
    pub dubbing_pipeline_service: DubbingPipelineService,
    pub local_db_pool: Pool<Sqlite>,
    pub llm_client: LLM_Client<OpenAIConfig>,
}

impl UniframeStudioAppState {
    pub fn new(
        s3_client: S3Client,
        dubbing_service_url: String,
        db_pool: Pool<Sqlite>,
        llm_client: LLM_Client<OpenAIConfig>,
    ) -> Self {
        let dubbing_client = DubbingClient::new(dubbing_service_url.clone());

        let dubbing_pipeline_service = DubbingPipelineService::new(
            dubbing_client.clone(),
            Arc::new(s3_client.clone()),
            db_pool.clone(),
        );

        Self {
            s3_client: Arc::new(s3_client),
            dubbing_service_url,
            dubbing_client,
            dubbing_pipeline_service,
            local_db_pool: db_pool,
            llm_client,
        }
    }

    pub fn get_db_pool(&self) -> &Pool<Sqlite> {
        &self.local_db_pool
    }
}
