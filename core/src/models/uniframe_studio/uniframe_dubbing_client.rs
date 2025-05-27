use crate::models::uniframe_studio::uniframe_studio::{
    DubbingJobRequest, DubbingJobResult, DubbingJobStatus,
};
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct UniframeDubbingClient {
    client: Client,
    base_url: String,
}

impl UniframeDubbingClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    pub async fn process_video(&self, request: DubbingJobRequest) -> Result<DubbingJobStatus> {
        info!(
            "Sending video processing request for job_id: {}",
            request.job_id
        );
        let response = self
            .client
            .post(&format!("{}/process-video", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send process-video request to Dubbing Client")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Dubbing Client API error: {}", error_text);
        }

        let job_status = response
            .json::<DubbingJobStatus>()
            .await
            .context("Failed to parse JobStatus from Dubbing Client")?;

        Ok(job_status)
    }

    pub async fn get_job_status(&self, job_id: &str) -> Result<DubbingJobStatus> {
        let response = self
            .client
            .get(&format!("{}/job/{}", self.base_url, job_id))
            .send()
            .await
            .context("Failed to get job status from Dubbing Client")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Dubbing Client API error: {}", error_text);
        }

        let job_status = response
            .json::<DubbingJobStatus>()
            .await
            .context("Failed to parse JobStatus from Dubbing Client")?;

        Ok(job_status)
    }

    pub async fn get_job_result(&self, job_id: &str) -> Result<DubbingJobResult> {
        let response = self
            .client
            .get(&format!("{}/job/{}/result", self.base_url, job_id))
            .send()
            .await
            .context("Failed to get job result from Dubbing Client")?;

        if response.status().is_success() {
            let job_result = response
                .json::<DubbingJobResult>()
                .await
                .context("Failed to parse JobResult from Dubbing Client")?;

            Ok(job_result)
        } else if response.status() == reqwest::StatusCode::ACCEPTED {
            anyhow::bail!("Job is still processing");
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Dubbing Client API error: {}", error_text);
        }
    }
}
