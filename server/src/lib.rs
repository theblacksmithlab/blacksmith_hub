pub mod routes;

use crate::routes::request_app::handlers::handle_user_action;
use crate::routes::the_viper_room::handlers::handle_the_viper_room_user_action;
use axum::http::Method;
use axum::response::IntoResponse;
use axum::routing::{get, options, post};
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use core::state::request_app::app_state::RequestAppState;
use core::state::server::app_state::ServerAppState;
use core::state::the_viper_room::app_state::TheViperRoomAppState;
use core::utils::common::get_user_avatar;
use http::{HeaderValue, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, CorsLayer};
use crate::routes::blacksmith_web::handlers::{handle_blacksmith_web_chat_fetch, handle_blacksmith_web_tts_input, handle_blacksmith_web_user_action};
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;

pub async fn start_server(
    server_app_state: Arc<ServerAppState>,
    request_app_state: Arc<RequestAppState>,
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    blacksmith_web_app_state: Arc<BlacksmithWebAppState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let allowed_origins = server_app_state
        .config
        .cors
        .allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins.clone())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(AllowHeaders::any())
        .allow_credentials(false);

    // Request App router
    let request_app_routes = Router::new()
        .route(
            "/user_action",
            post(handle_user_action).options(|| async { StatusCode::OK }),
        )
        .route(
            "/get_user_avatar",
            get(get_user_avatar).options(|| async { StatusCode::OK }),
        )
        .with_state(request_app_state);

    // The Viper Room router
    let the_viper_room_routes = Router::new()
        .route(
            "/the_viper_room_user_action",
            post(handle_the_viper_room_user_action).options(|| async { StatusCode::OK }),
        )
        .with_state(the_viper_room_app_state);
    
    // Blacksmith Web Router
    let blacksmith_web_router = Router::new()
        .route(
            "/blacksmith_web_user_action",
            post(handle_blacksmith_web_user_action).options(|| async { StatusCode::OK }),
        )
        .route(
            "/blacksmith_web_chat_fetch",
            get(handle_blacksmith_web_chat_fetch).options(|| async { StatusCode::OK }),
        )
        .route(
            "/blacksmith_web_tts_input",
            post(handle_blacksmith_web_tts_input).options(|| async { StatusCode::OK }),
        )
        .with_state(blacksmith_web_app_state);

    let app = Router::new()
        .merge(request_app_routes)
        .merge(the_viper_room_routes)
        .merge(blacksmith_web_router)
        .route("/*path", options(|| async { StatusCode::OK }))
        .fallback(handler_404)
        .layer(cors);

    let tls_config = RustlsConfig::from_pem_file(
        &server_app_state.config.tls.cert_path,
        &server_app_state.config.tls.key_path,
    )
    .await?;

    let addr: SocketAddr = format!(
        "{}:{}",
        server_app_state.config.server.host, server_app_state.config.server.port
    )
    .parse()
    .expect("Invalid host or port configuration");
    
    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}
