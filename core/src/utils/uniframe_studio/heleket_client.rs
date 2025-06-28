use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::{anyhow, Result};
use uuid::Uuid;
use md5::{Md5, Digest};
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone)]
pub struct HeleketConfig {
    pub merchant_id: String,
    pub api_key: String,
    pub base_url: String,
    pub webhook_url: String,
    pub success_url: String,
    pub cancel_url: String,
}

impl Default for HeleketConfig {
    fn default() -> Self {
        Self {
            merchant_id: std::env::var("HELEKET_MERCHANT_ID")
                .expect("HELEKET_MERCHANT_ID must be set"),
            api_key: std::env::var("HELEKET_API_KEY")
                .expect("HELEKET_API_KEY must be set"),
            base_url: "https://api.heleket.com".to_string(),
            webhook_url: std::env::var("HELEKET_WEBHOOK_URL")
                .expect("HELEKET_WEBHOOK_URL must be set"),
            success_url: std::env::var("HELEKET_SUCCESS_URL")
                .unwrap_or_else(|_| "https://uniframe.studio/billing?payment=success".to_string()),
            cancel_url: std::env::var("HELEKET_CANCEL_URL")
                .unwrap_or_else(|_| "https://uniframe.studio/billing?payment=cancelled".to_string()),
        }
    }
}

#[derive(Serialize)]
pub struct CreateInvoiceRequest {
    pub amount: String,
    pub currency: String,
    pub order_id: String,
    pub url_callback: String,
    pub url_success: String,
    pub url_return: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CreateInvoiceResponse {
    pub state: i32,
    pub result: Option<InvoiceResult>,
    pub message: Option<String>,
    pub errors: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct InvoiceResult {
    pub uuid: String,
    pub order_id: String,
    pub amount: String,
    pub url: String,
}

pub struct HeleketClient {
    client: Client,
    config: HeleketConfig,
}

impl HeleketClient {
    pub fn new(config: HeleketConfig) -> Self {
        let client = Client::new();
        Self { client, config }
    }

    fn generate_signature(&self, body: &str) -> String {
        let data_base64 = general_purpose::STANDARD.encode(body);
        let data_with_key = format!("{}{}", data_base64, self.config.api_key);

        let mut hasher = Md5::new();
        hasher.update(data_with_key.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub async fn create_invoice(&self, amount_usd: f64, user_id: &str) -> Result<InvoiceResult> {
        let order_id = format!("topup_{}_{}", user_id, Uuid::new_v4());

        let request = CreateInvoiceRequest {
            amount: amount_usd.to_string(),
            currency: "USD".to_string(),
            order_id: order_id.clone(),
            url_callback: self.config.webhook_url.clone(),
            url_success: self.config.success_url.clone(),
            url_return: self.config.cancel_url.clone(),
        };

        let body = serde_json::to_string(&request)?;
        let signature = self.generate_signature(&body);

        let response = self
            .client
            .post(&format!("{}/v1/payment", self.config.base_url))
            .header("Content-Type", "application/json")
            .header("merchant", &self.config.merchant_id)
            .header("sign", &signature)
            .body(body)
            .send()
            .await?;

        let response_data: CreateInvoiceResponse = response.json().await?;

        if response_data.state != 0 {
            let error_msg = if let Some(message) = response_data.clone().message {
                format!("Heleket API error: {}", message)
            } else if let Some(errors) = response_data.clone().errors {
                format!("Heleket API validation errors: {}", errors)
            } else {
                format!("Heleket API error: state = {}", response_data.state)
            };

            eprintln!("Heleket response: {:?}", response_data);
            return Err(anyhow!(error_msg));
        }

        response_data.result.ok_or_else(|| anyhow!("No result in response"))
    }
}