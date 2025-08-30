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
#[derive(Serialize, Deserialize, Debug)]
pub struct ClientConfig {
    pub registration_server: String,
    pub listen_port: u16,
}

// Configuration for the dispatch server.
#[derive(Serialize, Deserialize, Debug)]
pub struct DispatchConfig {
    pub database_url: String,
    pub client_connect_port: u16,
}

// Configuration for the registration server.
#[derive(Serialize, Deserialize, Debug)]
pub struct RegistrationConfig {
    pub database_url: String,
    pub listen_address: String,
    pub create_db_if_not_exists: bool,
}