// This binary is the client for the orca project.
// It first registers with the registration server, then listens for commands from the dispatch server.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::process::{Command, Output};
use orca::{ClientInfo, ClientConfig};
use mac_address::get_mac_address;
use local_ip_address::local_ip;
use hostname;

// This function executes the command in a cross-platform way.
#[cfg(windows)]
fn execute_command(command_str: &str) -> std::io::Result<Output> {
    Command::new("cmd")
        .arg("/C")
        .arg(command_str)
        .output()
}

#[cfg(not(windows))]
fn execute_command(command_str: &str) -> std::io::Result<Output> {
    Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .output()
}

// This function handles a single dispatch server connection.
fn handle_dispatch(mut stream: TcpStream) {
    // Create a buffer to store the received data.
    let mut buffer = [0; 1024];
    // Read data from the stream into the buffer.
    match stream.read(&mut buffer) {
        // If data is successfully read, execute the command.
        Ok(bytes_read) => {
            let command_str = String::from_utf8_lossy(&buffer[..bytes_read]);
            println!("Received command: {}", command_str);

            // WARNING: Executing arbitrary commands received over the network is a huge security risk.
            // In a real application, you must validate and sanitize the command.
            let output = execute_command(&command_str).expect("failed to execute process");

            // Send the output back to the dispatch server.
            stream.write_all(&output.stdout).unwrap();
            stream.write_all(&output.stderr).unwrap();
        }
        // If there is an error reading from the server, print the error.
        Err(e) => {
            println!("Failed to read from dispatch server: {}", e);
        }
    }
}

fn register_client(config: &ClientConfig) {
    println!("Checking in with registration server...");

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
    let received_uuid = if response_str.starts_with("Client registered successfully. UUID: ") {
        response_str.trim_start_matches("Client registered successfully. UUID: ").to_string()
    } else if response_str == "UUID_IN_USE" {
        eprintln!("UUID is already in use. Deleting client.uuid and retrying registration.");
        fs::remove_file("client.uuid").expect("Failed to delete client.uuid");
        // Recursively call register_client to get a new UUID
        register_client(config);
        return; // Exit this call, the recursive call will handle the rest
    }
    else {
        // Handle unexpected response, maybe log an error or use a default
        eprintln!("Unexpected response from registration server: {}", response_str);
        "UNKNOWN_UUID".to_string() // Or handle as an error
    };

    // Save the received UUID.
    fs::write("client.uuid", &received_uuid).expect("Failed to write client.uuid");

    println!("Client registered/checked in with UUID: {}", received_uuid);
}


fn main() {
    // Read the client configuration.
    let config_str = fs::read_to_string("client.json").expect("Failed to read client.json");
    let config: ClientConfig = serde_json::from_str(&config_str).expect("Failed to parse client.json");

    // Register the client if it's not already registered.
    register_client(&config);

    // Bind a TCP listener to 0.0.0.0 on the configured port.
    let listen_address = format!("0.0.0.0:{}", config.listen_port);
    let listener = TcpListener::bind(&listen_address).unwrap();
    // Print a message to indicate that the client is running.
    println!("orca-client listening on {}", listen_address);

    // Iterate over incoming connections.
    for stream in listener.incoming() {
        match stream {
            // If the connection is successful, handle the dispatch server.
            Ok(stream) => {
                handle_dispatch(stream);
            }
            // If there is an error, print the error message.
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}