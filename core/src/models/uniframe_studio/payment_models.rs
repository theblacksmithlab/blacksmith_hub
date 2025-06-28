use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TopUpRequest {
    pub amount_usd: f64,
}

#[derive(Serialize)]
pub struct TopUpResponse {
    pub payment_url: String,
    pub order_id: String,
    pub amount_usd: f64,
}