// This binary is the client for the orca project.
// It first registers with the registration server, then listens for commands from the dispatch server.

use std::env;
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write, BufRead};
use std::process::{Command, Output};
use std::path::PathBuf;
use orca::{ClientInfo, ClientConfig, DispatchMessage, DispatchFile};
use mac_address::get_mac_address;
use local_ip_address::local_ip;
use hostname;
use log::{info, warn, error, LevelFilter};
use log4rs::{append::{console::{ConsoleAppender, Target}, file::FileAppender}, config::{Appender, Config, Root}, encode::pattern::PatternEncoder};
use std::str::FromStr;
use serde_json;

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
fn handle_dispatch(mut stream: TcpStream, config: ClientConfig) {
    // Read the incoming message until a newline delimiter.
    let mut buffer = Vec::new();
    let mut reader = std::io::BufReader::new(&mut stream);
    match reader.read_until(b'\n', &mut buffer) {
        Ok(bytes_read) if bytes_read > 0 => {
            let message_str = String::from_utf8_lossy(&buffer[..bytes_read]);
            match serde_json::from_str::<DispatchMessage>(&message_str) {
                Ok(dispatch_message) => {
                    info!("Received command: {}", dispatch_message.command);

                    // Determine the workspace directory
                    let workspace_dir = match &config.workspace_dir {
                        Some(dir) => PathBuf::from(dir),
                        None => {
                            let mut path = env::current_exe().unwrap();
                            path.pop();
                            path.push("orca-workspace");
                            path
                        }
                    };

                    // Create the workspace directory if it doesn't exist
                    if let Err(e) = fs::create_dir_all(&workspace_dir) {
                        error!("Failed to create workspace directory {}: {}", workspace_dir.display(), e);
                        return;
                    }

                    // Save files to the workspace directory
                    for file in dispatch_message.files {
                        let file_path = workspace_dir.join(&file.name);
                        match fs::write(&file_path, &file.content) {
                            Ok(_) => info!("Saved file: {}", file_path.display()),
                            Err(e) => error!("Failed to save file {}: {}", file_path.display(), e),
                        }
                    }

                    // Execute the command
                    let output = execute_command(&dispatch_message.command, &config).expect("failed to execute process");

                    // Send the output back to the dispatch server.
                    stream.write_all(&output.stdout).unwrap();
                    stream.write_all(&output.stderr).unwrap();

                    // Clean up the workspace directory
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

fn register_client(config: &ClientConfig) {
    info!("Checking in with registration server...");

    // Get client information.
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let ip = local_ip().unwrap().to_string();
    let mac_address = get_mac_address().unwrap().unwrap().to_string();

    // Read existing UUID if available, otherwise generate a placeholder.
    let existing_uuid = fs::read_to_string("client.uuid").unwrap_or_else(|_| "UNREGISTERED".to_string());

    let client_info = ClientInfo {
        uuid: existing_uuid,
        hostname: Some(hostname),
        ip,
        mac_address: Some(mac_address),
    };

    // Connect to the registration server.
    let mut stream = TcpStream::connect(&config.registration_server).expect("Failed to connect to registration server");

    // Send client information.
    let client_info_json = serde_json::to_string(&client_info).unwrap();
    stream.write_all(client_info_json.as_bytes()).unwrap();

    // Receive the response from the registration server.
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer).unwrap();
    let response_str = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

    // Parse the response from the registration server.
    let received_uuid = if response_str.starts_with("Client registered successfully. UUID: ") || response_str.starts_with("Client updated successfully. UUID: ") {
        response_str.trim_start_matches("Client registered successfully. UUID: ").trim_start_matches("Client updated successfully. UUID: ").to_string()
    } else if response_str == "UUID_IN_USE" {
        warn!("UUID is already in use. Deleting client.uuid and retrying registration.");
        fs::remove_file("client.uuid").expect("Failed to delete client.uuid");
        // Recursively call register_client to get a new UUID
        register_client(config);
        return; // Exit this call, the recursive call will handle the rest
    }
    else {
        // Handle unexpected response, maybe log an error or use a default
        error!("Unexpected response from registration server: {}", response_str);
        "UNKNOWN_UUID".to_string() // Or handle as an error
    };

    // Save the received UUID.
    fs::write("client.uuid", &received_uuid).expect("Failed to write client.uuid");

    info!("Client registered/checked in with UUID: {}", received_uuid);
}


fn main() {
    // Read the client configuration.
    let config_str = fs::read_to_string("client.json").expect("Failed to read client.json");
    let config: ClientConfig = serde_json::from_str(&config_str).expect("Failed to parse client.json");

    // Initialize logger
    let log_level = LevelFilter::from_str(&config.log_level).unwrap_or(LevelFilter::Info);

    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} {l} - {m}\n")))
        .build(&config.log_file_path)
        .expect("Failed to build file appender");

    let stdout_appender = ConsoleAppender::builder().target(Target::Stdout).build();

    let log_config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .appender(Appender::builder().build("stdout", Box::new(stdout_appender)))
        .build(
            Root::builder()
                .appender("file")
                .appender("stdout")
                .build(log_level),
        )
        .expect("Failed to build log4rs config");

    log4rs::init_config(log_config).expect("Failed to initialize log4rs");

    // Register the client if it's not already registered.
    register_client(&config);

    // Bind a TCP listener to 0.0.0.0 on the configured port.
    let listen_address = format!("0.0.0.0:{}", config.listen_port);
    let listener = TcpListener::bind(&listen_address).unwrap();
    // Print a message to indicate that the client is running.
    info!("orca-client listening on {}", listen_address);

    // Iterate over incoming connections.
    for stream in listener.incoming() {
        let config_clone = config.clone();
        match stream {
            // If the connection is successful, handle the dispatch server.
            Ok(stream) => {
                handle_dispatch(stream, config_clone);
            }
            // If there is an error, print the error message.
            Err(e) => {
                error!("Error: {}", e);
            }
        }
    }
}