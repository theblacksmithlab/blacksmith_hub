use crate::uniframe_studio::local_utils::create_magic_link_html;
use axum::body::Body;
use axum::extract::State;
use axum::Json;
use axum::{http::Request, middleware::Next, response::Response};
use chrono::{DateTime, Duration, Utc};
use core::models::uniframe_studio::auth_models::{
    AuthError, AuthResponse, SendMagicLinkRequest, SessionCheckResponse, VerifyTokenRequest,
};
use core::state::uniframe_studio::app_state::UniframeStudioAppState;
use http::{HeaderMap, StatusCode};
use sqlx::{Pool, Row, Sqlite};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn handle_send_magic_link(
    State(app_state): State<Arc<UniframeStudioAppState>>,
    Json(request): Json<SendMagicLinkRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<AuthError>)> {
    let email = request.email.trim().to_lowercase();

    if !is_valid_email(&email) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthError {
                error: "Invalid email format".to_string(),
            }),
        ));
    }

    info!("Got auth-request from user with e-mail: {}", email);

    let db_pool = app_state.get_db_pool();

    if let Err(remaining_time) = check_rate_limit(db_pool, &email).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(AuthError {
                error: format!("Too many requests. Try again in {} minutes", remaining_time),
            }),
        ));
    }

    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::hours(1);

    let query = "
        INSERT INTO auth_magic_links (id, email, token, expires_at)
        VALUES (?, ?, ?, ?)
    ";

    if let Err(e) = sqlx::query(query)
        .bind(Uuid::new_v4().to_string())
        .bind(&email)
        .bind(&token)
        .bind(expires_at.to_rfc3339())
        .execute(db_pool)
        .await
    {
        error!("Failed to save magic link: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthError {
                error: "Failed to process request".to_string(),
            }),
        ));
    }

    let magic_link = format!(
        "{}?token={}",
        std::env::var("UNIFRAME_STUDIO_FRONTEND_URL")
            .unwrap_or("http://localhost:5173".to_string()),
        token
    );

    if let Err(e) = send_magic_link_email(&email, &magic_link).await {
        error!("Failed to send email: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthError {
                error: "Failed to send magic link email".to_string(),
            }),
        ));
    }

    info!("Magic link sent to: {}", email);

    Ok(Json(AuthResponse {
        success: true,
        message: "Magic link email sent successfully".to_string(),
        session_token: None,
    }))
}

async fn check_rate_limit(db_pool: &Pool<Sqlite>, email: &str) -> Result<(), i64> {
    let five_minutes_ago = (Utc::now() - Duration::minutes(5)).to_rfc3339();

    let query = "
        SELECT created_at FROM auth_magic_links
        WHERE email = ? AND created_at > ?
        ORDER BY created_at DESC LIMIT 1
    ";

    if let Ok(row) = sqlx::query(query)
        .bind(email)
        .bind(&five_minutes_ago)
        .fetch_optional(db_pool)
        .await
    {
        if let Some(row) = row {
            let last_request: String = row.get("created_at");

            let last_request_time = match DateTime::parse_from_rfc3339(&last_request) {
                Ok(dt) => dt.with_timezone(&Utc),
                Err(e) => {
                    error!("Failed to parse created_at: {}", e);
                    return Err(1);
                }
            };

            let next_allowed = last_request_time + Duration::minutes(5);
            let remaining = (next_allowed - Utc::now()).num_minutes();
            return Err(remaining.max(1));
        }
    }

    Ok(())
}

async fn send_magic_link_email(email: &str, magic_link: &str) -> anyhow::Result<()> {
    let api_key = std::env::var("BREVO_API_KEY")?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "sender": {"email": "thecableguy303808909@gmail.com"},
            "to": [{"email": email}],
            "subject": "Sign in to Uniframe Studio",
            "textContent": format!("Click this link to sign in: {}\n\nThis link expires in 1 hour.", magic_link),
            "htmlContent": create_magic_link_html(magic_link)
        }))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Log-in link sending API failed: {}",
            response.text().await?
        ))
    }
}

fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

pub async fn handle_verify_token(
    State(app_state): State<Arc<UniframeStudioAppState>>,
    Json(request): Json<VerifyTokenRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<AuthError>)> {
    info!("Authorizing user...");

    let token = request.token.trim();
    let db_pool = app_state.get_db_pool();

    let query = "
        SELECT email, expires_at, used FROM auth_magic_links
        WHERE token = ? AND used = FALSE
    ";

    let magic_link_row = match sqlx::query(query).bind(token).fetch_optional(db_pool).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "Invalid or expired token".to_string(),
                }),
            ));
        }
        Err(e) => {
            error!("Database error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError {
                    error: "Internal server error".to_string(),
                }),
            ));
        }
    };

    let email: String = magic_link_row.get("email");
    let expires_at: String = magic_link_row.get("expires_at");

    let expires_time = match DateTime::parse_from_rfc3339(&expires_at) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(e) => {
            error!("Failed to parse expires_at: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError {
                    error: "Invalid date format in database".to_string(),
                }),
            ));
        }
    };

    if Utc::now() > expires_time {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: "Token has expired".to_string(),
            }),
        ));
    }

    let update_query = "UPDATE auth_magic_links SET used = TRUE WHERE token = ?";
    sqlx::query(update_query)
        .bind(token)
        .execute(db_pool)
        .await
        .map_err(|e| {
            error!("Failed to mark token as used: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let user_id = create_or_get_user(db_pool, &email).await?;

    let session_token = Uuid::new_v4().to_string();
    let session_expires = Utc::now() + Duration::days(30);

    let session_query = "
        INSERT INTO auth_sessions (id, user_id, token, expires_at)
        VALUES (?, ?, ?, ?)
    ";

    sqlx::query(session_query)
        .bind(Uuid::new_v4().to_string())
        .bind(&user_id)
        .bind(&session_token)
        .bind(session_expires.to_rfc3339())
        .execute(db_pool)
        .await
        .map_err(|e| {
            error!("Failed to create session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError {
                    error: "Failed to create session".to_string(),
                }),
            )
        })?;

    info!("User {} successfully authenticated", email);

    Ok(Json(AuthResponse {
        success: true,
        message: "Authentication successful".to_string(),
        session_token: Some(session_token),
    }))
}

async fn create_or_get_user(
    db_pool: &Pool<Sqlite>,
    email: &str,
) -> Result<String, (StatusCode, Json<AuthError>)> {
    let query = "SELECT id FROM auth_users WHERE email = ?";

    match sqlx::query(query).bind(email).fetch_optional(db_pool).await {
        Ok(Some(row)) => {
            return Ok(row.get("id"));
        }
        Ok(None) => {}
        Err(e) => {
            error!("Database error while looking up user: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError {
                    error: "Database error occurred".to_string(),
                }),
            ));
        }
    }

    let user_id = Uuid::new_v4().to_string();
    let insert_query = "INSERT INTO auth_users (id, email) VALUES (?, ?)";

    match sqlx::query(insert_query)
        .bind(&user_id)
        .bind(email)
        .execute(db_pool)
        .await
    {
        Ok(_) => {
            info!("Created new user: {}", email);
            Ok(user_id)
        }
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint") {
                match sqlx::query(query).bind(email).fetch_optional(db_pool).await {
                    Ok(Some(row)) => Ok(row.get("id")),
                    _ => {
                        error!("Failed to create user due to race condition: {}", e);
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(AuthError {
                                error: "Failed to create user".to_string(),
                            }),
                        ))
                    }
                }
            } else {
                error!("Failed to create user: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AuthError {
                        error: "Failed to create user".to_string(),
                    }),
                ))
            }
        }
    }
}

pub async fn handle_check_session(
    State(app_state): State<Arc<UniframeStudioAppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionCheckResponse>, (StatusCode, Json<AuthError>)> {
    let db_pool = app_state.get_db_pool();

    let session_token = match extract_session_token(&headers) {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "Missing authorization token".to_string(),
                }),
            ));
        }
    };

    match verify_session_token(db_pool, &session_token).await {
        Ok((user_email, _user_id)) => Ok(Json(SessionCheckResponse {
            valid: true,
            user_email,
        })),
        Err(error_msg) => Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError { error: error_msg }),
        )),
    }
}

fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("authorization")?.to_str().ok()?;

    if auth_header.starts_with("Bearer ") {
        Some(auth_header[7..].to_string())
    } else {
        None
    }
}

async fn verify_session_token(
    db_pool: &Pool<Sqlite>,
    session_token: &str,
) -> Result<(String, String), String> {
    let query = "
        SELECT u.email, s.user_id, s.expires_at
        FROM auth_sessions s
        JOIN auth_users u ON s.user_id = u.id
        WHERE s.token = ?
    ";

    let row = match sqlx::query(query)
        .bind(session_token)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return Err("Invalid session token".to_string()),
        Err(e) => {
            error!("Database error during session verification: {}", e);
            return Err("Internal server error".to_string());
        }
    };

    let email: String = row.get("email");
    let user_id: String = row.get("user_id");
    let expires_at: String = row.get("expires_at");

    let expires_time = match DateTime::parse_from_rfc3339(&expires_at) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(e) => {
            error!("Failed to parse expires_at: {}", e);
            return Err("Invalid date format in database".to_string());
        }
    };

    if Utc::now() > expires_time {
        let delete_query = "DELETE FROM auth_sessions WHERE token = ?";
        let _ = sqlx::query(delete_query)
            .bind(session_token)
            .execute(db_pool)
            .await;

        return Err("Session has expired".to_string());
    }

    Ok((email, user_id))
}

pub async fn auth_middleware(
    State(app_state): State<Arc<UniframeStudioAppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = req.headers();
    let db_pool = app_state.get_db_pool();

    let session_token = match extract_session_token(headers) {
        Some(token) => token,
        None => {
            warn!("Missing authorization token in protected route");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    match verify_session_token(db_pool, &session_token).await {
        Ok((_user_email, user_id)) => {
            req.extensions_mut().insert(user_id);

            let response = next.run(req).await;
            Ok(response)
        }
        Err(error_msg) => {
            warn!("Session verification failed: {}", error_msg);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
