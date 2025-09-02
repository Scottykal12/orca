use actix_cors::Cors;
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use orca::ApiConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use rustls::{ServerConfig, Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::io::BufReader;

#[derive(Deserialize)]
struct DispatchRequest {
    command: String,
    client: String,
    files: Option<String>,
}

#[derive(Serialize)]
struct DispatchResponse {
    stdout: String,
    stderr: String,
    success: bool,
}

#[post("/dispatch")]
async fn dispatch_command(req: web::Json<DispatchRequest>, app_data: web::Data<ApiConfig>) -> impl Responder {
    let dispatch_bin_path = app_data.dispatch_binary_path.as_ref().cloned().unwrap_or_default();

    if dispatch_bin_path.is_empty() {
        panic!("dispatch_binary_path is not set or is empty in api.json. Please set it to the path of the orca-dispatch binary.");
    }

    let mut cmd = Command::new(dispatch_bin_path);
    cmd.arg("-c")
        .arg(&req.command)
        .arg("-i")
        .arg(&req.client);

    if let Some(files) = &req.files {
        cmd.arg("--files").arg(files.clone());
    }

    let output = cmd.output().expect("Failed to execute orca-dispatch");

    let response = DispatchResponse {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
    };

    HttpResponse::Ok().json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Read the API configuration.
    let config_str = fs::read_to_string("api.json").expect("Failed to read api.json");
    let config: ApiConfig = serde_json::from_str(&config_str).expect("Failed to parse api.json");

    let listen_address = config.listen_address.clone();
    let app_data = web::Data::new(config.clone());

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_data.clone())
            .service(dispatch_command)
    });

    if config.use_tls {
        println!("TLS is enabled with rustls. Loading certificate and key.");
        // load TLS cert/key
        let cert_file = &mut BufReader::new(fs::File::open(&config.cert_path)?);
        let key_file = &mut BufReader::new(fs::File::open(&config.key_path)?);

        let cert_chain = certs(cert_file)
            .unwrap()
            .into_iter()
            .map(Certificate)
            .collect();
        let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
            .unwrap()
            .into_iter()
            .map(PrivateKey)
            .collect();

        if keys.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "could not find PKCS 8 private key in key file",
            ));
        }

        let tls_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, keys.remove(0))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        println!("API server listening on https://{}", listen_address);
        server.bind_rustls_021(listen_address, tls_config)?.run().await
    } else {
        println!("API server listening on http://{}", listen_address);
        server.bind(listen_address)?.run().await
    }
}