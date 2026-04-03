use crate::ai::anthropic_client::AnthropicClient;
use crate::ai::google_client::GoogleClient;
use crate::utils::uniframe_studio::dubbing_pipeline::DubbingPipelineService;
use crate::utils::uniframe_studio::gpu_instance_manager::GpuInstanceManager;
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::Client as OpenAIClient;
use aws_sdk_s3::Client as S3Client;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

pub struct UniframeStudioAppState {
    pub s3_client: Arc<S3Client>,
    pub dubbing_pipeline_service: DubbingPipelineService,
    pub local_db_pool: Pool<Sqlite>,
    pub openai_client: OpenAIClient<OpenAIConfig>,
    pub anthropic_client: AnthropicClient,
    pub google_client: GoogleClient,
    pub gpu_instance_manager: GpuInstanceManager,
}

impl UniframeStudioAppState {
    pub fn new(
        s3_client: S3Client,
        db_pool: Pool<Sqlite>,
        openai_client: OpenAIClient<OpenAIConfig>,
        anthropic_client: AnthropicClient,
        google_client: GoogleClient,
    ) -> Result<Self> {
        let gpu_instance_manager = GpuInstanceManager::new(db_pool.clone())?;

        let dubbing_pipeline_service =
            DubbingPipelineService::new(Arc::new(s3_client.clone()), db_pool.clone());

        Ok(Self {
            s3_client: Arc::new(s3_client),
            dubbing_pipeline_service,
            local_db_pool: db_pool,
            openai_client,
            anthropic_client,
            google_client,
            gpu_instance_manager,
        })
    }

    pub async fn initialize_gpu_instances(&self) -> Result<()> {
        self.gpu_instance_manager.initialize_instances().await
    }

    pub fn get_db_pool(&self) -> &Pool<Sqlite> {
        &self.local_db_pool
    }
}
