// This binary is the registration server for the orca project.
// It listens for clients, registers them, and stores their information in the database.

use std::fs;
use sqlx::MySqlPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use orca::{ClientInfo, RegistrationConfig};
use uuid::Uuid;

// This function initializes the database and creates the 'clients' table if it doesn't exist.
async fn init_db(pool: &MySqlPool) -> sqlx::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clients (\n            uuid VARCHAR(255) PRIMARY KEY,\n            ip VARCHAR(255),\n            hostname VARCHAR(255),\n            mac_address VARCHAR(255)\n        )"
    )
    .execute(pool)
    .await?;
    Ok(())
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
    println!("Registration server listening on {}", config.listen_address);

    loop {
        let (socket, _) = listener.accept().await?;
        let pool = pool.clone(); // Clone the pool for each task
        tokio::spawn(async move {
            handle_client(socket, pool).await;
        });
    }
}

async fn handle_client(mut stream: TcpStream, pool: MySqlPool) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0; 1024];
    match stream.read(&mut buffer).await {
        Ok(bytes_read) => {
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
                        Ok(Some(_)) => {
                            // Client with this UUID already exists. Tell client to get a new UUID.
                            println!("Client with UUID {} already exists. Instructing client to get a new UUID.", client_info.uuid);
                            let response = "UUID_IN_USE";
                            stream.write_all(response.as_bytes()).await?;
                        }
                        Ok(None) => {
                            // Client does not exist, insert
                            println!("Client with UUID {} does not exist. Registering...", client_info.uuid);

                            let actual_uuid = if client_info.uuid.is_empty() || client_info.uuid == "UNREGISTERED" || client_info.uuid == "UNKNOWN_UUID" {
                                Uuid::new_v4().to_string()
                            } else {
                                client_info.uuid.clone()
                            };

                            println!("Attempting to insert client with UUID: {}, IP: {}, Hostname: {:?}, MAC: {:?}",
                                actual_uuid, client_info.ip, client_info.hostname, client_info.mac_address);

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
