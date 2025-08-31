use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
    pub listen_port: u16,
    pub log_file_path: String,
    pub log_level: String,
    pub workspace_dir: Option<String>,
}

// Configuration for the dispatch server.
#[derive(Serialize, Deserialize, Debug)]
pub struct DispatchConfig {
    pub database_url: String,
    pub client_connect_port: u16,
}

// Configuration for the registration server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistrationConfig {
    pub database_url: String,
    pub listen_address: String,
}


// Configuration for the API server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiConfig {
    pub listen_address: String,
    pub dispatch_binary_path: Option<String>,
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
