use actix_cors::Cors;
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use orca::{ApiConfig, log_to_db};
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use rustls::{ServerConfig, Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::io::BufReader;
use log::{info, error, LevelFilter};
use sqlx::mysql::MySqlPool;
use std::str::FromStr;
extern crate env_logger;

async fn init_db(pool: &MySqlPool) -> sqlx::Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS logs (id INT AUTO_INCREMENT PRIMARY KEY, time BIGINT, service TEXT, severity TEXT, info TEXT)").execute(pool).await?;
    Ok(())
}

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
async fn dispatch_command(req: web::Json<DispatchRequest>, app_data: web::Data<ApiConfig>, db_pool: web::Data<MySqlPool>) -> impl Responder {
    let dispatch_bin_path = app_data.dispatch_binary_path.as_ref().cloned().unwrap_or_default();

    if dispatch_bin_path.is_empty() {
        error!("dispatch_binary_path is not set or is empty in api.json. Please set it to the path of the orca-dispatch binary.");
        log_to_db(&db_pool, "api", "ERROR", "dispatch_binary_path is not set or is empty in api.json.").await;
        panic!("dispatch_binary_path is not set or is empty in api.json. Please set it to the path of the orca-dispatch binary.");
    }

    info!("Received dispatch command: {}", req.command);
    log_to_db(&db_pool, "api", "INFO", &format!("Received dispatch command: {}", req.command)).await;

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

    if !response.success {
        error!("Dispatch command failed: {}", response.stderr);
        log_to_db(&db_pool, "api", "ERROR", &format!("Dispatch command failed: {}", response.stderr)).await;
    } else {
        info!("Dispatch command successful: {}", response.stdout);
        log_to_db(&db_pool, "api", "INFO", &format!("Dispatch command successful: {}", response.stdout)).await;
    }

    HttpResponse::Ok().json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Read the API configuration.
    let config_str = fs::read_to_string("api.json").expect("Failed to read api.json");
    let config: ApiConfig = serde_json::from_str(&config_str).expect("Failed to parse api.json");

    // Initialize logger
    let log_level = LevelFilter::from_str(&config.log_level).unwrap_or(LevelFilter::Info);
    env_logger::builder().filter_level(log_level).init();

    let listen_address = config.listen_address.clone();
    let app_data = web::Data::new(config.clone());

    // Create database pool
    let pool = MySqlPool::connect(&config.database_url).await.expect("Failed to connect to database");
    init_db(&pool).await.expect("Failed to initialize database");
    let db_pool = web::Data::new(pool.clone());

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_data.clone())
            .app_data(db_pool.clone())
            .service(dispatch_command)
    });

    if config.use_tls {
        info!("TLS is enabled with rustls. Loading certificate and key.");
        log_to_db(&pool, "api", "INFO", "TLS is enabled with rustls. Loading certificate and key.").await;
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
            error!("could not find PKCS 8 private key in key file");
            log_to_db(&pool, "api", "ERROR", "could not find PKCS 8 private key in key file").await;
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

        info!("API server listening on https://{}", listen_address);
        log_to_db(&pool, "api", "INFO", &format!("API server listening on https://{}", listen_address)).await;
        server.bind_rustls_021(listen_address, tls_config)?.run().await
    } else {
        info!("API server listening on http://{}", listen_address);
        log_to_db(&pool, "api", "INFO", &format!("API server listening on http://{}", listen_address)).await;
        server.bind(listen_address)?.run().await
    }
}