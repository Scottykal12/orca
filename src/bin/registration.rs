// This binary is the registration server for the orca project.
// It listens for clients, registers them, and stores their information in the database.

use std::fs;
use std::sync::Arc;
use sqlx::MySqlPool;
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use orca::{ClientInfo, RegistrationConfig};
use uuid::Uuid;
use std::io::BufReader;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::{self, Certificate, PrivateKey};

// This function initializes the database and creates the 'clients' table if it doesn't exist.
async fn init_db(pool: &MySqlPool) -> sqlx::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clients (
            uuid VARCHAR(255) PRIMARY KEY,
            ip VARCHAR(255),
            hostname VARCHAR(255),
            mac_address VARCHAR(255)
        )"
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn load_certs(path: &str) -> std::io::Result<Vec<Certificate>> {
    let certfile = fs::File::open(path)?;
    let mut reader = BufReader::new(certfile);
    certs(&mut reader)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_private_key(path: &str) -> std::io::Result<PrivateKey> {
    let keyfile = fs::File::open(path)?;
    let mut reader = BufReader::new(keyfile);
    pkcs8_private_keys(&mut reader)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| PrivateKey(keys.remove(0)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the registration configuration.
    let config_str = fs::read_to_string("registration.json").expect("Failed to read registration.json");
    let config: RegistrationConfig = serde_json::from_str(&config_str).expect("Failed to parse registration.json");

    // Connect to the database.
    let pool = MySqlPool::connect(&config.database_url).await.expect("Failed to open database");
    init_db(&pool).await.expect("Failed to initialize database");

    // Test database connection with a simple query
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .expect("Failed to execute simple test query to database");

    let listener = TcpListener::bind(&config.listen_address).await?;
    let acceptor = if config.use_tls {
        println!("TLS is enabled. Loading certificate and key.");
        let certs = load_certs(&config.cert_path)?;
        let key = load_private_key(&config.key_path)?;
        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        Some(TlsAcceptor::from(Arc::new(config)))
    } else {
        None
    };

    println!("Registration server listening on {}", config.listen_address);

    loop {
        let (socket, _) = listener.accept().await?;
        
        if let Some(acceptor) = acceptor.clone() {
            let pool = pool.clone();
            tokio::spawn(async move {
                match acceptor.accept(socket).await {
                    Ok(tls_stream) => {
                        if let Err(e) = handle_client(tls_stream, pool).await {
                            eprintln!("Error handling client: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("TLS handshake error: {}. Ensure client is using TLS.", e);
                    }
                }
            });
        } else {
            let pool = pool.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, pool).await {
                    eprintln!("Error handling client: {}", e);
                }
            });
        }
    }
}

async fn handle_client<S>(mut stream: S, pool: MySqlPool) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buffer = vec![0; 1024];
    match stream.read(&mut buffer).await {
        Ok(bytes_read) => {
            if bytes_read == 0 { // Handle case where stream is closed
                return Ok(())
            }
            let received_data = String::from_utf8_lossy(&buffer[..bytes_read]);
            println!("Received from client: {}", received_data);

            match serde_json::from_str::<ClientInfo>(&received_data) {
                Ok(client_info) => {
                    println!("Parsed ClientInfo: {:?}", client_info);

                    // Check if client exists
                    let client_exists = sqlx::query_as::<_, ClientInfo>(
                        "SELECT uuid, ip, hostname, mac_address FROM clients WHERE uuid = ?"
                    )
                    .bind(&client_info.uuid)
                    .fetch_optional(&pool)
                    .await;

                    match client_exists {
                        Ok(Some(existing_client)) => {
                            if existing_client.mac_address == client_info.mac_address {
                                // Same client, update IP and hostname
                                println!("Client with UUID {} found. Updating IP and hostname.", client_info.uuid);
                                sqlx::query(
                                    "UPDATE clients SET ip = ?, hostname = ? WHERE uuid = ?"
                                )
                                .bind(&client_info.ip)
                                .bind(&client_info.hostname)
                                .bind(&client_info.uuid)
                                .execute(&pool)
                                .await
                                .map_err(|e| {
                                    eprintln!("Error updating client: {:?}", e);
                                    e
                                })?;
                                let response = format!("Client updated successfully. UUID: {}", client_info.uuid);
                                stream.write_all(response.as_bytes()).await?;
                            } else {
                                // Different client trying to use existing UUID
                                println!("Client with UUID {} already exists but MAC address does not match. Instructing client to get a new UUID.", client_info.uuid);
                                let response = "UUID_IN_USE";
                                stream.write_all(response.as_bytes()).await?;
                            }
                        }
                        Ok(None) => {
                            // Client does not exist, insert
                            println!("Client with UUID {} does not exist. Registering...", client_info.uuid);

                            let actual_uuid = if client_info.uuid.is_empty() || client_info.uuid == "UNREGISTERED" || client_info.uuid == "UNKNOWN_UUID" {
                                Uuid::new_v4().to_string()
                            } else {
                                client_info.uuid.clone()
                            };

                            println!("Attempting to insert client with UUID: {}, IP: {}, Hostname: {:?}, MAC: {:?}"
                                , actual_uuid, client_info.ip, client_info.hostname, client_info.mac_address);

                            sqlx::query(
                                "INSERT INTO clients (uuid, ip, hostname, mac_address) VALUES (?, ?, ?, ?)"
                            )
                            .bind(&actual_uuid)
                            .bind(&client_info.ip)
                            .bind(&client_info.hostname)
                            .bind(&client_info.mac_address)
                            .execute(&pool)
                            .await
                            .map_err(|e| {
                                eprintln!("Error inserting client: {:?}", e);
                                e
                            })?;
                            let response = format!("Client registered successfully. UUID: {}", actual_uuid);
                            stream.write_all(response.as_bytes()).await?;
                        }
                        Err(e) => {
                            eprintln!("Database query error: {}", e);
                            let response = format!("Database error: {}", e);
                            stream.write_all(response.as_bytes()).await?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse ClientInfo: {}", e);
                    let response = format!("Error parsing client info: {}", e);
                    stream.write_all(response.as_bytes()).await?;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read from socket: {}", e);
            return Err(e.into());
        }
    }
    Ok(())
}