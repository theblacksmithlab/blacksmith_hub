use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub user_id: i64,
    pub telegram_username: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserChannel {
    pub id: i64,
    pub user_id: i64,
    pub channel_id: i64,
    pub channel_title: String,
}

/// Temporary channel data for adding channels workflow
/// Used to store channels before saving to database
#[derive(Debug, Clone)]
pub struct PendingChannel {
    pub channel_id: i64,
    pub channel_title: String,
}
