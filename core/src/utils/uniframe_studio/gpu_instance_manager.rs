use crate::gpu_instance_client::immers_cloud_client::ImmersCloudClient;
use anyhow::{Context, Result};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct GpuInstance {
    pub server_id: String,
    pub service_url: String,
    pub status: GpuInstanceStatus,
    pub current_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuInstanceStatus {
    Available,
    WakingUp,
    Busy,
    Error,
}

impl GpuInstanceStatus {
    fn to_string(&self) -> &'static str {
        match self {
            GpuInstanceStatus::Available => "available",
            GpuInstanceStatus::WakingUp => "waking_up",
            GpuInstanceStatus::Busy => "busy",
            GpuInstanceStatus::Error => "error",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "available" => GpuInstanceStatus::Available,
            "waking_up" => GpuInstanceStatus::WakingUp,
            "busy" => GpuInstanceStatus::Busy,
            "error" => GpuInstanceStatus::Error,
            _ => GpuInstanceStatus::Error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuInstanceManager {
    db_pool: Pool<Sqlite>,
    instances_config: HashMap<String, String>, // server_id -> service_url
}

impl GpuInstanceManager {
    pub fn new(db_pool: Pool<Sqlite>) -> Result<Self> {
        // Parse GPU_INSTANCES from environment variable
        // Format: "server1:https://gpu1.example.com,server2:https://gpu2.example.com"
        let gpu_instances_env = std::env::var("GPU_INSTANCES")
            .context("GPU_INSTANCES environment variable must be set")?;

        let mut instances_config = HashMap::new();

        for instance_config in gpu_instances_env.split(',') {
            let parts: Vec<&str> = instance_config.trim().split(':').collect();
            if parts.len() < 2 {
                return Err(anyhow::anyhow!(
                    "Invalid GPU_INSTANCES format. Expected 'server_id:url,server_id:url...'"
                ));
            }

            let server_id = parts[0].to_string();
            let service_url = parts[1..].join(":");

            instances_config.insert(server_id, service_url);
        }

        if instances_config.is_empty() {
            return Err(anyhow::anyhow!("No GPU instances configured"));
        }

        info!("Configured {} GPU instances", instances_config.len());

        Ok(Self {
            db_pool,
            instances_config,
        })
    }

    /// Initialize GPU instances in database on startup
    pub async fn initialize_instances(&self) -> Result<()> {
        info!("Initializing GPU instances in database...");

        for (server_id, service_url) in &self.instances_config {
            sqlx::query(
                "INSERT OR REPLACE INTO gpu_instances (server_id, service_url, status, current_job_id, last_updated) 
                 VALUES (?, ?, 'available', NULL, datetime('now'))"
            )
                .bind(server_id)
                .bind(service_url)
                .execute(&self.db_pool)
                .await
                .context("Failed to initialize GPU instance in database")?;
        }

        self.sync_instances_status().await?;

        info!("GPU instances initialization completed");
        Ok(())
    }

    /// Sync actual status of all instances from Immers Cloud
    async fn sync_instances_status(&self) -> Result<()> {
        info!("Syncing GPU instances status from Immers Cloud...");

        for server_id in self.instances_config.keys() {
            match self.get_actual_instance_status(server_id).await {
                Ok(immers_status) => {
                    let our_status = match immers_status.as_str() {
                        "SHELVED_OFFLOADED" => GpuInstanceStatus::Available,
                        "ACTIVE" => GpuInstanceStatus::Busy,
                        _ => GpuInstanceStatus::Error,
                    };

                    self.update_instance_status(server_id, our_status.clone(), None)
                        .await?;
                    info!(
                        "Instance {} status synced: {} -> {}",
                        server_id,
                        immers_status,
                        our_status.to_string()
                    );
                }
                Err(e) => {
                    warn!("Failed to get status for instance {}: {}", server_id, e);
                    self.update_instance_status(server_id, GpuInstanceStatus::Error, None)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Get actual status from Immers Cloud for specific instance
    async fn get_actual_instance_status(&self, server_id: &str) -> Result<String> {
        let immers_client = ImmersCloudClient::new(
            &std::env::var("IMMERS_USERNAME").context("IMMERS_USERNAME not set")?,
            &std::env::var("IMMERS_PASSWORD").context("IMMERS_PASSWORD not set")?,
            &std::env::var("IMMERS_PROJECT").context("IMMERS_PROJECT not set")?,
            server_id.to_string(),
        )
        .await?;

        immers_client.get_service_status().await
    }

    /// Atomically acquire a free GPU instance for a job
    pub async fn acquire_instance(&self, job_id: &str) -> Result<Option<GpuInstance>> {
        for attempt in 1..=3 {
            info!(
                "Attempt {}/3 to acquire GPU instance for job {}",
                attempt, job_id
            );

            let mut tx = self.db_pool.begin().await?;

            let row = sqlx::query(
                "SELECT server_id, service_url, status 
                 FROM gpu_instances 
                 WHERE status = 'available' 
                 LIMIT 1",
            )
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(row) = row {
                let server_id: String = row.get("server_id");
                let service_url: String = row.get("service_url");

                let updated_rows = sqlx::query(
                    "UPDATE gpu_instances 
                     SET status = 'waking_up', current_job_id = ?, last_updated = datetime('now')
                     WHERE server_id = ? AND status = 'available'",
                )
                .bind(job_id)
                .bind(&server_id)
                .execute(&mut *tx)
                .await?
                .rows_affected();

                if updated_rows > 0 {
                    tx.commit().await?;

                    info!(
                        "Successfully acquired GPU instance {} for job {}",
                        server_id, job_id
                    );

                    return Ok(Some(GpuInstance {
                        server_id,
                        service_url,
                        status: GpuInstanceStatus::WakingUp,
                        current_job_id: Some(job_id.to_string()),
                    }));
                } else {
                    // Someone else took this instance, try again
                    tx.rollback().await?;
                    continue;
                }
            } else {
                tx.rollback().await?;
                info!("No available GPU instances found, attempt {}/3", attempt);
            }

            // Wait a bit before retrying
            if attempt < 3 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        info!("Failed to acquire any GPU instance after 3 attempts");
        Ok(None)
    }

    /// Release a GPU instance after job completion
    pub async fn release_instance(&self, server_id: &str, job_id: &str) -> Result<()> {
        info!("Releasing GPU instance {} for job {}", server_id, job_id);

        if let Err(e) = self.shelve_instance(server_id).await {
            error!("Failed to shelve instance {}: {}", server_id, e);
        }

        sqlx::query(
            "UPDATE gpu_instances 
             SET status = 'available', current_job_id = NULL, last_updated = datetime('now')
             WHERE server_id = ? AND current_job_id = ?",
        )
        .bind(server_id)
        .bind(job_id)
        .execute(&self.db_pool)
        .await?;

        info!("Successfully released GPU instance {}", server_id);
        Ok(())
    }

    /// Shelve a specific GPU instance
    async fn shelve_instance(&self, server_id: &str) -> Result<()> {
        let immers_client = ImmersCloudClient::new(
            &std::env::var("IMMERS_USERNAME").context("IMMERS_USERNAME not set")?,
            &std::env::var("IMMERS_PASSWORD").context("IMMERS_PASSWORD not set")?,
            &std::env::var("IMMERS_PROJECT").context("IMMERS_PROJECT not set")?,
            server_id.to_string(),
        )
        .await?;

        immers_client.shelve_server().await
    }

    /// Update instance status in database
    async fn update_instance_status(
        &self,
        server_id: &str,
        status: GpuInstanceStatus,
        job_id: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE gpu_instances 
             SET status = ?, current_job_id = ?, last_updated = datetime('now')
             WHERE server_id = ?",
        )
        .bind(status.to_string())
        .bind(job_id)
        .bind(server_id)
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    /// Mark instance as busy when job starts processing
    pub async fn mark_instance_busy(&self, server_id: &str, job_id: &str) -> Result<()> {
        self.update_instance_status(server_id, GpuInstanceStatus::Busy, Some(job_id.to_string()))
            .await
    }

    /// Mark instance as error state
    pub async fn mark_instance_error(&self, server_id: &str) -> Result<()> {
        self.update_instance_status(server_id, GpuInstanceStatus::Error, None)
            .await
    }

    /// Get all instances status for monitoring
    pub async fn get_instances_status(&self) -> Result<Vec<GpuInstance>> {
        let rows = sqlx::query(
            "SELECT server_id, service_url, status, current_job_id 
             FROM gpu_instances 
             ORDER BY server_id",
        )
        .fetch_all(&self.db_pool)
        .await?;

        let instances = rows
            .into_iter()
            .map(|row| GpuInstance {
                server_id: row.get("server_id"),
                service_url: row.get("service_url"),
                status: GpuInstanceStatus::from_string(&row.get::<String, _>("status")),
                current_job_id: row.get("current_job_id"),
            })
            .collect();

        Ok(instances)
    }
}
