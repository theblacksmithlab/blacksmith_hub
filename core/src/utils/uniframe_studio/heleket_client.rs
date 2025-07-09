use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use md5::{Digest, Md5};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

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
            api_key: std::env::var("HELEKET_API_KEY").expect("HELEKET_API_KEY must be set"),
            base_url: "https://api.heleket.com".to_string(),
            webhook_url: std::env::var("HELEKET_WEBHOOK_URL")
                .expect("HELEKET_WEBHOOK_URL must be set"),
            success_url: std::env::var("HELEKET_SUCCESS_URL")
                .unwrap_or_else(|_| "https://uniframe.studio/billing?payment=success".to_string()),
            cancel_url: std::env::var("HELEKET_CANCEL_URL").unwrap_or_else(|_| {
                "https://uniframe.studio/billing?payment=cancelled".to_string()
            }),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_percent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_currency: Option<String>,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
    pub payment_status: String,
    pub expired_at: i64,
    pub is_final: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_data: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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

    pub async fn check_invoice_status(&self, invoice_uuid: &str) -> Result<InvoiceResult> {
        let request = serde_json::json!({
            "uuid": invoice_uuid
        });

        let body = serde_json::to_string(&request)?;
        let signature = self.generate_signature(&body);

        let response = self
            .client
            .post(&format!("{}/v1/payment/info", self.config.base_url))
            .header("Content-Type", "application/json")
            .header("merchant", &self.config.merchant_id)
            .header("sign", &signature)
            .body(body.clone())
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

            error!("Heleket check status response: {:?}", response_data);
            return Err(anyhow!(error_msg));
        }

        response_data
            .result
            .ok_or_else(|| anyhow!("No result in response"))
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
            .body(body.clone())
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

        response_data
            .result
            .ok_or_else(|| anyhow!("No result in response"))
    }
}
