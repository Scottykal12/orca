use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::fs;
use orca::ApiConfig;

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
    let app_data = web::Data::new(config.clone()); // Clone config for each worker

    println!("API server listening on {}", listen_address);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_data.clone()) // Pass app_data to the App
            .service(dispatch_command)
    })
    .bind(listen_address)?
    .run()
    .await
}