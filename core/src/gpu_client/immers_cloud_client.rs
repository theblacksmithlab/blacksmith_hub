use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct ImmersCloudClient {
    client: Client,
    auth_token: String,
    tenant_id: String,
    server_id: String,
}

impl ImmersCloudClient {
    pub async fn new(
        username: &str,
        password: &str,
        project_name: &str,
        server_id: String,
    ) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        let auth_response = client
            .post("https://api.immers.cloud:5000/v3/auth/tokens")
            .json(&json!({
                "auth": {
                    "identity": {
                        "methods": ["password"],
                        "password": {
                            "user": {
                                "name": username,
                                "domain": {"name": "Default"},
                                "password": password
                            }
                        }
                    },
                    "scope": {
                        "project": {
                            "name": project_name,
                            "domain": {"name": "Default"}
                        }
                    }
                }
            }))
            .send()
            .await?;

        let auth_token = auth_response
            .headers()
            .get("X-Subject-Token")
            .ok_or_else(|| anyhow::anyhow!("No auth token received"))?
            .to_str()?
            .to_string();

        let auth_body: serde_json::Value = auth_response.json().await?;
        let tenant_id = auth_body["token"]["project"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No tenant ID found"))?
            .to_string();

        Ok(Self {
            client,
            auth_token,
            tenant_id,
            server_id,
        })
    }

    pub async fn shelve_server(&self) -> Result<()> {
        info!("Shelving AI server: {}", self.server_id);

        let response = self
            .client
            .post(&format!(
                "https://api.immers.cloud:8774/v2.1/{}/servers/{}/action",
                self.tenant_id, self.server_id
            ))
            .header("X-Auth-Token", &self.auth_token)
            .json(&json!({"shelve": null}))
            .send()
            .await?;

        if response.status().is_success() {
            info!("AI server shelved successfully");
            Ok(())
        } else {
            error!("Failed to shelve server: {}", response.status());
            Err(anyhow::anyhow!("Failed to shelve server"))
        }
    }

    pub async fn unshelve_server(&self) -> Result<()> {
        info!("Unshelving AI server: {}", self.server_id);

        let response = self
            .client
            .post(&format!(
                "https://api.immers.cloud:8774/v2.1/{}/servers/{}/action",
                self.tenant_id, self.server_id
            ))
            .header("X-Auth-Token", &self.auth_token)
            .json(&json!({"unshelve": null}))
            .send()
            .await?;

        if response.status().is_success() {
            info!("AI server unshelved successfully");
            Ok(())
        } else {
            error!("Failed to unshelve server: {}", response.status());
            Err(anyhow::anyhow!("Failed to unshelve server"))
        }
    }

    pub async fn get_service_status(&self) -> Result<String> {
        let response = self
            .client
            .get(&format!(
                "https://api.immers.cloud:8774/v2.1/{}/servers/{}",
                self.tenant_id, self.server_id
            ))
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await?;

        let server_info: serde_json::Value = response.json().await?;
        let status = server_info["server"]["status"]
            .as_str()
            .unwrap_or("UNKNOWN")
            .to_string();

        Ok(status)
    }

    pub async fn wait_for_service_active(&self, max_wait_seconds: u64) -> Result<()> {
        let start_time = std::time::Instant::now();

        loop {
            let status = self.get_service_status().await?;
            info!("Server status: {}", status);

            if status == "ACTIVE" {
                info!("Server is now active and ready");
                return Ok(());
            }

            if start_time.elapsed().as_secs() > max_wait_seconds {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for server to become active"
                ));
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}
