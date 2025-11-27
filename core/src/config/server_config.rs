use crate::models::common::app_name::AppName;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    // DEBUG: Temporary - TLS is optional during migration to nginx-only TLS termination
    // After migration: remove TlsConfig completely
    pub tls: Option<TlsConfig>,
    pub cors: CorsConfig,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Deserialize, Debug)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

pub fn get_config_path(app_name: &AppName) -> String {
    if let Ok(path) = std::env::var("CONFIG_PATH") {
        return path;
    }

    let container_path = "/app/config.yaml";
    if std::path::Path::new(container_path).exists() {
        return container_path.to_string();
    }

    match app_name {
        AppName::BlacksmithWeb => "the_forge/src/blacksmith_web/config.yaml",
        AppName::TheViperRoom => "the_forge/src/the_viper_room/config.yaml",
        AppName::UniframeStudio => "the_forge/src/uniframe_studio/config.yaml",
        _ => "config.yaml",
    }
    .to_string()
}
