// This binary is the dispatch server for the orca project.
// It sends commands to a specific client for execution and logs the events.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use clap::Parser;
use sqlx::{MySqlPool, FromRow};
use orca::{DispatchConfig, DispatchMessage, DispatchFile, DispatchFileMetadata, log_to_db};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as TokioTcpStream;
use std::path::Path;
use log::{info, error, LevelFilter};
extern crate env_logger;
use std::str::FromStr;

trait ReadWrite: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite> ReadWrite for T {}

use std::sync::Arc;
use rustls::{ClientConfig, Certificate, PrivateKey, ServerName};
use rustls_pemfile::{pkcs8_private_keys};
use tokio_rustls::TlsConnector;
use std::io::BufReader;
use std::fs::File;
use rustls_native_certs::load_native_certs;


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
    hostname: Option<String>,
}

// This function initializes the database and creates the 'events' table if it doesn't exist.
async fn init_db(pool: &MySqlPool) -> sqlx::Result<()>
{
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            id INT AUTO_INCREMENT PRIMARY KEY,
            epoch_time BIGINT,
            client_uuid VARCHAR(255),
            client_ip VARCHAR(255),
            command TEXT,
            response TEXT,
            files TEXT
        )"
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS logs (id INT AUTO_INCREMENT PRIMARY KEY, time BIGINT, service TEXT, severity TEXT, info TEXT)").execute(pool).await?;
    Ok(())
}

// This function queries the database for the client's IP address and UUID.
async fn query_client(pool: &MySqlPool, client_identifier: &str) -> sqlx::Result<ClientQueryResult> {
    let client = sqlx::query_as!(
        ClientQueryResult,
        "SELECT uuid, ip, hostname FROM clients WHERE uuid = ? OR ip = ? OR hostname = ? OR mac_address = ?",
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

    // Initialize logger
    let log_level = LevelFilter::from_str(&config.log_level).unwrap_or(LevelFilter::Info);
    env_logger::builder().filter_level(log_level).init();

    // Read the dispatch configuration.
    let config_str = fs::read_to_string("dispatch.json").expect("Failed to read dispatch.json");
    let config: DispatchConfig = serde_json::from_str(&config_str).expect("Failed to parse dispatch.json");

    // Connect to the database.
    let pool = MySqlPool::connect(&config.database_url).await.expect("Failed to open database");
    init_db(&pool).await.expect("Failed to initialize database");

    match query_client(&pool, &args.client).await {
        Ok(client) => {
            let client_address = format!("{}:{}", client.ip.as_deref().unwrap_or(""), config.client_connect_port);
            info!("Connecting to client at {}", client_address);
            log_to_db(&pool, "dispatch", "INFO", &format!("Connecting to client at {}", client_address)).await;

            let handle_connection = |mut stream: Box<dyn ReadWrite + Unpin + Send>| {
                let pool = pool.clone();
                async move {
                    info!("Connected to orca-client");
                    tokio::spawn({
                        let pool = pool.clone();
                        async move {
                            log_to_db(&pool, "dispatch", "INFO", "Connected to orca-client").await;
                        }
                    });

                    let mut files_to_send: Vec<DispatchFile> = Vec::new();
                    if let Some(files_arg) = args.files.clone() {
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
                                        error!("Error reading file {}: {}", file_path_str, e);
                                        let pool = pool.clone();
                                        let e_clone = e.to_string();
                                        let file_path_str_clone = file_path_str.to_string();
                                        tokio::spawn(async move {
                                            log_to_db(&pool, "dispatch", "ERROR", &format!("Error reading file {}: {}", file_path_str_clone, e_clone)).await;
                                        });
                                    }
                                }
                            } else {
                                error!("File not found or is not a file: {}", file_path_str);
                                let pool = pool.clone();
                                let file_path_str_clone = file_path_str.to_string();
                                tokio::spawn(async move { log_to_db(&pool, "dispatch", "ERROR", &format!("File not found or is not a file: {}", file_path_str_clone)).await });
                            }
                        }
                    }

                    let dispatch_message = DispatchMessage {
                        command: args.command.clone(),
                        files: files_to_send,
                    };

                    let serialized_message = serde_json::to_string(&dispatch_message).unwrap();
                    stream.write_all(serialized_message.as_bytes()).await.unwrap();
                    stream.write_all(b"\n").await.unwrap();

                    let mut buffer = vec![0; 1024];
                    match stream.read(&mut buffer).await {
                        Ok(bytes_read) => {
                            let response = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                            info!("Received: {}", response);
                            tokio::spawn({
                                let pool = pool.clone();
                                let response_clone = response.clone();
                                async move { log_to_db(&pool, "dispatch", "INFO", &format!("Received: {}", response_clone)).await }
                            });

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
                            error!("Failed to read from client: {}", e);
                            let pool = pool.clone();
                            let e_clone = e.to_string();
                            tokio::spawn(async move { log_to_db(&pool, "dispatch", "ERROR", &format!("Failed to read from client: {}", e_clone)).await });
                        }
                    }
                }
            };

            if config.use_tls {
                let mut root_cert_store = rustls::RootCertStore::empty();
                let native_certs = load_native_certs().expect("could not load platform certs");
                for cert in native_certs {
                    root_cert_store.add(&rustls::Certificate(cert.as_ref().to_vec())).unwrap();
                }

                let cert_file = &mut BufReader::new(File::open(&config.cert_path).unwrap());
                let key_file = &mut BufReader::new(File::open(&config.key_path).unwrap());
                let cert_chain = rustls_pemfile::certs(cert_file).unwrap().into_iter().map(Certificate).collect();
                let mut keys = pkcs8_private_keys(key_file).unwrap().into_iter().map(PrivateKey).collect::<Vec<_>>();

                let client_config = ClientConfig::builder()
                    .with_safe_defaults()
                    .with_root_certificates(root_cert_store)
                    .with_client_auth_cert(cert_chain, keys.remove(0))
                    .unwrap();

                let connector = TlsConnector::from(Arc::new(client_config));
                let stream = TokioTcpStream::connect(&client_address).await.unwrap();
                let domain = ServerName::try_from(client.hostname.as_deref().unwrap_or("localhost")).unwrap();
                let stream = connector.connect(domain, stream).await.unwrap();
                handle_connection(Box::new(stream)).await;
            } else {
                match TokioTcpStream::connect(&client_address).await {
                    Ok(stream) => {
                        handle_connection(Box::new(stream)).await;
                    }
                    Err(e) => {
                        error!("Failed to connect to client at {}: {}", client_address, e);
                        let pool = pool.clone();
                        let e_clone = e.to_string();
                        let client_address_clone = client_address.clone();
                        tokio::spawn(async move { log_to_db(&pool, "dispatch", "ERROR", &format!("Failed to connect to client at {}: {}", client_address_clone, e_clone)).await });
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to find client '{}': {}", args.client, e);
            let pool = pool.clone();
            let e_clone = e.to_string();
            let args_client_clone = args.client.clone();
            tokio::spawn(async move { log_to_db(&pool, "dispatch", "ERROR", &format!("Failed to find client '{}': {}", args_client_clone, e_clone)).await });
        }
    }
}
