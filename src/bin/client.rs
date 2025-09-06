// This binary is the client for the orca project.
// It first registers with the registration server, then listens for commands from the dispatch server.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write, BufReader, BufRead};
use std::process::{Command, Output, exit};
use std::path::PathBuf;
use orca::{ClientInfo, ClientConfig, DispatchMessage};
use mac_address::get_mac_address;
use local_ip_address::local_ip;
use hostname;
use log::{info, warn, error, LevelFilter};
extern crate env_logger;
use std::str::FromStr;
use serde_json;
use std::sync::Arc;
use rustls::{ClientConfig as RustlsClientConfig, ServerConfig as RustlsServerConfig, ServerName};
use rustls_pemfile::{certs, pkcs8_private_keys};
use rustls_native_certs::load_native_certs;

struct Tee(io::Stderr, File);

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_all(buf)?;
        self.1.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()?;
        self.1.flush()
    }
}

// This function executes the command in a cross-platform way.
fn execute_command(command_str: &str, config: &ClientConfig) -> std::io::Result<Output> {
    let workspace_dir = match &config.workspace_dir {
        Some(dir) => PathBuf::from(dir),
        None => {
            let mut path = env::current_exe()?;
            path.pop();
            path.push("orca-workspace");
            path
        }
    };

    fs::create_dir_all(&workspace_dir)?;

    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(command_str);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command_str);
        c
    };

    cmd.current_dir(workspace_dir).output()
}

// This function handles a single dispatch server connection.
fn handle_dispatch<S: Read + Write>(mut stream: S, config: ClientConfig) {
    // Read the incoming message until a newline delimiter.
    let mut buffer = Vec::new();
    let mut reader = BufReader::new(&mut stream);
    match reader.read_until(b'\n', &mut buffer) {
        Ok(bytes_read) if bytes_read > 0 => {
            let message_str = String::from_utf8_lossy(&buffer[..bytes_read]);
            match serde_json::from_str::<DispatchMessage>(&message_str) {
                Ok(dispatch_message) => {
                    info!("Received command: {}", dispatch_message.command);

                    let workspace_dir = match &config.workspace_dir {
                        Some(dir) => PathBuf::from(dir),
                        None => {
                            let mut path = env::current_exe().unwrap();
                            path.pop();
                            path.push("orca-workspace");
                            path
                        }
                    };

                    if let Err(e) = fs::create_dir_all(&workspace_dir) {
                        error!("Failed to create workspace directory {}: {}", workspace_dir.display(), e);
                        return;
                    }

                    for file in dispatch_message.files {
                        let file_path = workspace_dir.join(&file.name);
                        match fs::write(&file_path, &file.content) {
                            Ok(_) => {
                                info!("Saved file: {}", file_path.display());
                            },
                            Err(e) => {
                                error!("Failed to save file {}: {}", file_path.display(), e);
                            },
                        }
                    }

                    let output = execute_command(&dispatch_message.command, &config).expect("failed to execute process");

                    stream.write_all(&output.stdout).unwrap();
                    stream.write_all(&output.stderr).unwrap();

                    if !output.status.success() {
                        error!("Command execution failed: {}", String::from_utf8_lossy(&output.stderr));
                    } else {
                        info!("Command executed successfully: {}", String::from_utf8_lossy(&output.stdout));
                    }

                    if let Err(e) = fs::remove_dir_all(&workspace_dir) {
                        error!("Failed to remove workspace directory {}: {}", workspace_dir.display(), e);
                    } else {
                        info!("Cleaned up workspace directory: {}", workspace_dir.display());
                    }
                } 
                Err(e) => {
                    error!("Failed to deserialize dispatch message: {}", e);
                }
            }
        }
        Ok(_) => {
            error!("Received empty message from dispatch server.");
        }
        Err(e) => {
            error!("Failed to read from dispatch server: {}", e);
        }
    }
}

async fn do_registration(config: &ClientConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if config.use_tls_for_registration {
        info!("Connecting to registration server with TLS.");
        let mut root_cert_store = rustls::RootCertStore::empty();
        let certs = load_native_certs().map_err(|e| format!("could not load platform certs: {}", e))?;
        for cert in certs {
            root_cert_store.add(&rustls::Certificate(cert.as_ref().to_vec()))?;
        }
        let client_config = RustlsClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        let server_name_str = config.registration_server.split(':').next().unwrap_or("");
        info!("Attempting TLS validation for server name: '{}'", server_name_str);
        let server_name = ServerName::try_from(server_name_str)
            .map_err(|e| format!("invalid server name '{}': {}", server_name_str, e))?;

        let mut conn = rustls::ClientConnection::new(Arc::new(client_config), server_name)?;
        let mut sock = TcpStream::connect(&config.registration_server)?;
        let mut stream = rustls::Stream::new(&mut conn, &mut sock);
        register_client(config, &mut stream).await?;
    } else {
        let mut stream = TcpStream::connect(&config.registration_server)?;
        register_client(config, &mut stream).await?;
    };
    Ok(())
}

async fn register_client<S: Read + Write>(config: &ClientConfig, stream: &mut S) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Checking in with registration server...");

    let hostname = hostname::get()?.into_string().map_err(|_| "Invalid hostname")?;
    let ip = local_ip()?.to_string();
    let mac_address = get_mac_address()?.ok_or("No MAC address found")?.to_string();

    let existing_uuid = fs::read_to_string("client.uuid").unwrap_or_else(|_| "UNREGISTERED".to_string());

    let client_info = ClientInfo {
        uuid: existing_uuid,
        hostname: Some(hostname.to_string()),
        ip,
        mac_address: Some(mac_address),
    };

    let client_info_json = serde_json::to_string(&client_info)?;
    stream.write_all(client_info_json.as_bytes())?;

    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let response_str = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

    let received_uuid = if response_str.starts_with("Client registered successfully. UUID: ") || response_str.starts_with("Client updated successfully. UUID: ") {
        response_str.trim_start_matches("Client registered successfully. UUID: ").trim_start_matches("Client updated successfully. UUID: ").to_string()
    } else if response_str == "UUID_IN_USE" {
        warn!("UUID is already in use. Deleting client.uuid and retrying registration.");
        fs::remove_file("client.uuid")?;
        Box::pin(do_registration(config)).await?;
        return Ok(())
    }
    else {
        error!("Unexpected response from registration server: {}", response_str);
        return Err(format!("Unexpected response from registration server: {}", response_str).into());
    };

    fs::write("client.uuid", &received_uuid)?;

    info!("Client registered/checked in with UUID: {}", received_uuid);
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_str = fs::read_to_string("client.json")?;
    let config: ClientConfig = serde_json::from_str(&config_str)?;

    let log_level = LevelFilter::from_str(&config.log_level).unwrap_or(LevelFilter::Info);
    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&config.log_file_path)?;

    let target = Box::new(Tee(io::stderr(), log_file));

    env_logger::builder()
        .filter_level(log_level)
        .target(env_logger::Target::Pipe(target))
        .init();

    if let Err(e) = do_registration(&config).await {
        error!("Failed to register with server: {}. Please check TLS configuration and server address.", e);
        return Err(e);
    }

    let listen_address = format!("0.0.0.0:{}", config.listen_port);
    let listener = TcpListener::bind(&listen_address)?;
    let acceptor = if config.use_tls_for_listen {
        info!("Listening for dispatch with TLS.");
        let cert_file = &mut BufReader::new(fs::File::open(&config.cert_path)?);
        let key_file = &mut BufReader::new(fs::File::open(&config.key_path)?);
        let cert_chain = certs(cert_file)?.into_iter().map(rustls::Certificate).collect();
        let mut keys: Vec<rustls::PrivateKey> = pkcs8_private_keys(key_file)?.into_iter().map(rustls::PrivateKey).collect();

        let mut root_cert_store = rustls::RootCertStore::empty();
        let certs = load_native_certs()?;
        for cert in certs {
            root_cert_store.add(&rustls::Certificate(cert.as_ref().to_vec()))?;
        }

        let client_cert_verifier = std::sync::Arc::new(rustls::server::AllowAnyAuthenticatedClient::new(root_cert_store));

        let server_config = RustlsServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(cert_chain, keys.remove(0))?;
        Some(Arc::new(server_config))
    } else {
        None
    };

    info!("orca-client listening on {}", listen_address);

    for stream in listener.incoming() {
        let config_clone = config.clone();
        match stream {
            Ok(mut stream) => {
                if let Some(acceptor) = acceptor.clone() {
                    match rustls::ServerConnection::new(acceptor) {
                        Ok(mut conn) => {
                            let tls_stream = rustls::Stream::new(&mut conn, &mut stream);
                            handle_dispatch(tls_stream, config_clone);
                        },
                        Err(e) => {
                            error!("mTLS handshake failed: {}", e);
                        }
                    }
                } else {
                    handle_dispatch(stream, config_clone);
                }
            }
            Err(e) => {
                error!("Error accepting incoming connection: {}", e);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("[FATAL] {}", e);
        exit(1);
    }
}

