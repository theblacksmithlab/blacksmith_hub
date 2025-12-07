use sqlx::FromRow;

#[derive(Debug, Clone)]
pub enum Recipient {
    Public,
    Private(i64), // user_id // make u64
}

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub user_id: i64,
    pub telegram_username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserChannel {
    pub id: i64,
    pub user_id: i64,
    pub channel_id: i64,
    pub channel_title: String,
    pub channel_username: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PendingChannel {
    pub channel_id: i64,
    pub channel_title: String,
    pub channel_username: Option<String>,
}
