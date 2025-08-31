// This binary is the dispatch server for the orca project.
// It sends commands to a specific client for execution and logs the events.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use clap::Parser;
use sqlx::{MySqlPool, FromRow};
use orca::{DispatchConfig, DispatchMessage, DispatchFile, DispatchFileMetadata};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream as TokioTcpStream;
use std::path::Path;
use std::io::prelude::*;

// Added a comment to force re-compilation and re-preparation of SQLx queries.

/// Command-line arguments for the dispatch server.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The command to execute on the client.
    #[arg(short, long)]
    command: String,

    /// The client to send the command to (uuid, ip, hostname, or mac_address).
    #[arg(short = 'i', long)]
    client: String,

    /// Comma-separated list of files to send to the client.
    #[arg(short, long)]
    files: Option<String>,
}

#[derive(FromRow)]
struct ClientQueryResult {
    uuid: String,
    ip: Option<String>,
}

// This function initializes the database and creates the 'events' table if it doesn't exist.
async fn init_db(pool: &MySqlPool) -> sqlx::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            id INT AUTO_INCREMENT PRIMARY KEY,
            epoch_time BIGINT,
            client_uuid VARCHAR(255),
            client_ip VARCHAR(255),
            command TEXT,
            response TEXT,
            files TEXT
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

// This function queries the database for the client's IP address and UUID.
async fn query_client(pool: &MySqlPool, client_identifier: &str) -> sqlx::Result<ClientQueryResult> {
    let client = sqlx::query_as!(
        ClientQueryResult,
        "SELECT uuid, ip FROM clients WHERE uuid = ? OR ip = ? OR hostname = ? OR mac_address = ?",
        client_identifier,
        client_identifier,
        client_identifier,
        client_identifier
    )
    .fetch_one(pool)
    .await?;

    Ok(client)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Read the dispatch configuration.
    let config_str = fs::read_to_string("dispatch.json").expect("Failed to read dispatch.json");
    let config: DispatchConfig = serde_json::from_str(&config_str).expect("Failed to parse dispatch.json");

    // Connect to the database.
    let pool = MySqlPool::connect(&config.database_url).await.expect("Failed to open database");
    init_db(&pool).await.expect("Failed to initialize database");

    match query_client(&pool, &args.client).await {
        Ok(client) => {
            let client_address = format!("{}:{}", client.ip.as_deref().unwrap_or(""), config.client_connect_port);
            println!("Connecting to client at {}", client_address);

            match TokioTcpStream::connect(&client_address).await {
                Ok(mut stream) => {
                    println!("Connected to orca-client");

                    let mut files_to_send: Vec<DispatchFile> = Vec::new();
                    if let Some(files_arg) = args.files {
                        for file_path_str in files_arg.split(',') {
                            let file_path = Path::new(file_path_str.trim());
                            if file_path.exists() && file_path.is_file() {
                                match fs::read(file_path) {
                                    Ok(content) => {
                                        files_to_send.push(DispatchFile {
                                            name: file_path.file_name().unwrap().to_string_lossy().into_owned(),
                                            content,
                                        });
                                    },
                                    Err(e) => {
                                        eprintln!("Error reading file {}: {}", file_path_str, e);
                                    }
                                }
                            } else {
                                eprintln!("File not found or is not a file: {}", file_path_str);
                            }
                        }
                    }

                    let dispatch_message = DispatchMessage {
                        command: args.command,
                        files: files_to_send,
                    };

                    let serialized_message = serde_json::to_string(&dispatch_message).unwrap();
                    stream.write_all(serialized_message.as_bytes()).await.unwrap();

                    // Add a delimiter to indicate the end of the JSON message
                    stream.write_all(b"\n").await.unwrap();

                    let mut buffer = vec![0; 1024];
                    match stream.read(&mut buffer).await {
                        Ok(bytes_read) => {
                            let response = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                            println!("Received: {}", response);

                            let epoch_time = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs();

                            let files_metadata: Vec<DispatchFileMetadata> = dispatch_message.files.iter().map(|f| DispatchFileMetadata { name: f.name.clone() }).collect();
                            let files_json = serde_json::to_string(&files_metadata).unwrap_or_else(|_| "[]".to_string());

                            sqlx::query(
                                "INSERT INTO events (epoch_time, client_uuid, client_ip, command, response, files) VALUES (?, ?, ?, ?, ?, ?)",
                            )
                            .bind(&(epoch_time as i64))
                            .bind(&client.uuid)
                            .bind(&client.ip)
                            .bind(&dispatch_message.command)
                            .bind(&response)
                            .bind(&files_json)
                            .execute(&pool)
                            .await
                            .unwrap();
                        }
                        Err(e) => {
                            println!("Failed to read from client: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to connect to client at {}: {}", client_address, e);
                }
            }
        }
        Err(e) => {
            println!("Failed to find client '{}': {}", args.client, e);
        }
    }
}