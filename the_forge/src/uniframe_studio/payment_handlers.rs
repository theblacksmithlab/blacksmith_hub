use axum::body::Bytes;
use axum::extract::State;
use base64::engine::general_purpose;
use base64::Engine;
use core::models::uniframe_studio::accounting_models::UserBalance;
use core::state::uniframe_studio::app_state::UniframeStudioAppState;
use http::StatusCode;
use md5::{Digest, Md5};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info, warn};

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct PaymentWebhook {
    #[serde(rename = "type")]
    pub webhook_type: String,
    pub uuid: String,
    pub order_id: String,
    pub amount: String,
    pub payment_amount: String,
    pub payment_amount_usd: String,
    pub merchant_amount: String,
    pub commission: String,
    pub is_final: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address_uuid: Option<String>,
    pub network: String,
    pub currency: String,
    pub payer_currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convert: Option<ConvertInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
    pub sign: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ConvertInfo {
    pub to_currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commission: Option<String>,
    pub rate: String,
    pub amount: String,
}

pub async fn handle_payment_webhook(
    State(app_state): State<Arc<UniframeStudioAppState>>,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    let raw_body = String::from_utf8(body.to_vec()).map_err(|_| {
        eprintln!("Invalid UTF-8 in webhook body");
        StatusCode::BAD_REQUEST
    })?;

    println!("Received webhook: {}", raw_body);

    let webhook_data: PaymentWebhook = serde_json::from_str(&raw_body).map_err(|e| {
        eprintln!("Failed to parse webhook JSON: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    if !verify_webhook_signature(&webhook_data, &raw_body) {
        eprintln!(
            "Invalid webhook signature for order_id: {}",
            webhook_data.order_id
        );
        return Err(StatusCode::UNAUTHORIZED);
    }

    if let Err(e) = process_payment_webhook(&app_state, webhook_data).await {
        eprintln!("Failed to process webhook: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(StatusCode::OK)
}

// fn verify_webhook_signature(webhook_data: &PaymentWebhook, raw_body: &str) -> bool {
//     let api_key = std::env::var("HELEKET_API_KEY").unwrap_or_default();
// 
//     let mut data: serde_json::Value = match serde_json::from_str(raw_body) {
//         Ok(value) => value,
//         Err(_) => return false,
//     };
// 
//     if let Some(obj) = data.as_object_mut() {
//         obj.remove("sign");
//     }
// 
//     let json_string = serde_json::to_string(&data).unwrap_or_default();
//     let data_base64 = general_purpose::STANDARD.encode(json_string);
//     let data_with_key = format!("{}{}", data_base64, api_key);
// 
//     let mut hasher = Md5::new();
//     hasher.update(data_with_key.as_bytes());
//     let result = hasher.finalize();
//     let calculated_signature = format!("{:x}", result);
// 
//     calculated_signature == webhook_data.sign
// }

fn verify_webhook_signature(webhook_data: &PaymentWebhook, raw_body: &str) -> bool {
    let api_key = std::env::var("HELEKET_API_KEY").unwrap_or_default();
    
    let data: serde_json::Value = match serde_json::from_str(raw_body) {
        Ok(value) => value,
        Err(_) => return false,
    };
    
    let json_without_sign = recreate_original_order(&data);

    let data_base64 = general_purpose::STANDARD.encode(&json_without_sign);
    let data_with_key = format!("{}{}", data_base64, api_key);

    let mut hasher = Md5::new();
    hasher.update(data_with_key.as_bytes());
    let result = hasher.finalize();
    let calculated_signature = format!("{:x}", result);

    calculated_signature == webhook_data.sign
}

fn recreate_original_order(value: &serde_json::Value) -> String {
    let field_order = [
        "type", "uuid", "order_id", "amount", "payment_amount", "payment_amount_usd",
        "merchant_amount", "commission", "is_final", "status", "from", "wallet_address_uuid",
        "network", "currency", "payer_currency", "payer_amount", "payer_amount_exchange_rate",
        "additional_data", "transfer_id", "convert", "txid"
    ];

    if let Some(obj) = value.as_object() {
        let mut result = String::from("{");
        let mut first = true;

        for field in &field_order {
            if let Some(val) = obj.get(*field) {
                if !first {
                    result.push(',');
                }
                first = false;
                result.push_str(&format!("\"{}\":", field));
                result.push_str(&serde_json::to_string(val).unwrap());
            }
        }
        result.push('}');
        result
    } else {
        serde_json::to_string(value).unwrap()
    }
}

async fn process_payment_webhook(
    app_state: &Arc<UniframeStudioAppState>,
    webhook_data: PaymentWebhook,
) -> Result<(), Box<dyn std::error::Error>> {
    let webhook_id = &webhook_data.uuid;
    let user_id = extract_user_id_from_order(&webhook_data.order_id)?;
    let db_pool = app_state.get_db_pool();

    let already_processed = sqlx::query("SELECT 1 FROM processed_webhooks WHERE webhook_id = ?")
        .bind(webhook_id)
        .fetch_optional(db_pool)
        .await?
        .is_some();

    if already_processed {
        println!("Webhook {} already processed, skipping", webhook_id);
        return Ok(());
    }

    match webhook_data.status.as_str() {
        "paid" | "paid_over" => {
            info!(
                "Processing successful payment for order_id: {}",
                webhook_data.order_id
            );

            let amount_usd: f64 = webhook_data.payment_amount_usd.parse()?;

            let mut user_balance = UserBalance::get_or_create(&db_pool, &user_id).await?;
            user_balance
                .add_funds(
                    &db_pool,
                    amount_usd,
                    &format!("Top-up via Heleket: {}", webhook_data.order_id),
                )
                .await?;

            info!(
                "Successfully added ${} to user {} balance",
                amount_usd, user_id
            );
        }
        "fail" | "wrong_amount" | "cancel" | "system_fail" => {
            error!(
                "Payment failed for order_id: {}, status: {}",
                webhook_data.order_id, webhook_data.status
            );
        }
        _ => {
            warn!("Received intermediate status: {}", webhook_data.status);
        }
    }

    sqlx::query("INSERT INTO processed_webhooks (webhook_id, order_id) VALUES (?, ?)")
        .bind(webhook_id)
        .bind(&webhook_data.order_id)
        .execute(db_pool)
        .await?;

    Ok(())
}

fn extract_user_id_from_order(order_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(captures) = order_id.strip_prefix("topup_") {
        if let Some(underscore_pos) = captures.rfind('_') {
            return Ok(captures[..underscore_pos].to_string());
        }
    }
    Err("Invalid order_id format".into())
}
