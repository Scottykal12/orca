use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySqlPool};
use std::time::{SystemTime, UNIX_EPOCH};

// Information about a client.
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct ClientInfo {
    pub uuid: String,
    pub hostname: Option<String>,
    pub ip: String,
    pub mac_address: Option<String>,
}

// Configuration for the client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub registration_server: String,
    pub use_tls_for_registration: bool,
    pub listen_port: u16,
    pub use_tls_for_listen: bool,
    pub cert_path: String,
    pub key_path: String,
    pub log_file_path: String,
    pub log_level: String,
    pub workspace_dir: Option<String>,
    pub database_url: String,
}

// Configuration for the dispatch server.
#[derive(Serialize, Deserialize, Debug)]
pub struct DispatchConfig {
    pub database_url: String,
    pub client_connect_port: u16,
    pub use_tls: bool,
    pub cert_path: String,
    pub key_path: String,
}

// Configuration for the registration server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistrationConfig {
    pub database_url: String,
    pub listen_address: String,
    pub use_tls: bool,
    pub cert_path: String,
    pub key_path: String,
    pub log_level: String,
}


// Configuration for the API server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiConfig {
    pub listen_address: String,
    pub dispatch_binary_path: Option<String>,
    pub use_tls: bool,
    pub cert_path: String,
    pub key_path: String,
    pub database_url: String,
    pub log_level: String,
}

// Message sent from dispatch to client.
#[derive(Serialize, Deserialize, Debug)]
pub struct DispatchMessage {
    pub command: String,
    pub files: Vec<DispatchFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DispatchFile {
    pub name: String,
    pub content: Vec<u8>,
}

// Metadata for files, used for logging in the database
#[derive(Serialize, Deserialize, Debug)]
pub struct DispatchFileMetadata {
    pub name: String,
}

pub async fn log_to_db(pool: &MySqlPool, level: &str, message: &str) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as i64;
    let log_entry = format!("[{}] {}", level, message);
    match sqlx::query!("INSERT INTO logs (time, info) VALUES (?, ?)", now, log_entry)
        .execute(pool)
        .await
    {
        Ok(_) => {},
        Err(e) => eprintln!("Failed to log to database: {}", e),
    }
}