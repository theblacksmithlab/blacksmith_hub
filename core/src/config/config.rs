use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub tls: TlsConfig,
    pub cors: CorsConfig,
    pub qdrant: QdrantConfig,
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

#[derive(Deserialize)]
pub struct QdrantConfig {
    pub collection_name: String,
}
