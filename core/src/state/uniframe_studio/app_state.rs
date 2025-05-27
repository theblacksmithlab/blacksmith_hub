use crate::models::uniframe_studio::uniframe_dubbing_client::UniframeDubbingClient;
use crate::utils::uniframe_studio::dubbing_pipeline::DubbingPipelineService;
use aws_sdk_s3::Client as S3Client;
use std::sync::Arc;

pub struct UniframeStudioAppState {
    pub s3_client: Arc<S3Client>,
    pub dubbing_service_url: String,
    pub dubbing_client: UniframeDubbingClient,
    pub dubbing_pipeline_service: DubbingPipelineService,
}

impl UniframeStudioAppState {
    pub fn new(s3_client: S3Client, dubbing_service_url: String) -> Self {
        let dubbing_client = UniframeDubbingClient::new(dubbing_service_url.clone());

        let dubbing_pipeline_service =
            DubbingPipelineService::new(dubbing_client.clone(), Arc::new(s3_client.clone()));

        Self {
            s3_client: Arc::new(s3_client),
            dubbing_service_url,
            dubbing_client,
            dubbing_pipeline_service,
        }
    }
}
