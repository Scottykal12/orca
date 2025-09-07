use actix_cors::Cors;
use actix_web::{get, post, put, web, App, HttpResponse, HttpServer, Responder};
use orca::{ApiConfig, log_to_db};
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use rustls::{ServerConfig, Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::io::BufReader;
use log::{info, error, LevelFilter};
use sqlx::mysql::{MySqlPool, MySqlRow};
use std::str::FromStr;
extern crate env_logger;
use sqlx::{Column, Row, TypeInfo, ValueRef};
use serde_json::{json, Value};

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

#[derive(Deserialize)]
struct UpdateQuery {
    pk_col: String,
    pk_val: String,
}

#[post("/dispatch")]
async fn dispatch_command(req: web::Json<DispatchRequest>, app_data: web::Data<ApiConfig>, db_pool: web::Data<MySqlPool>) -> impl Responder {
    let dispatch_bin_path = app_data.dispatch_binary_path.as_ref().cloned().unwrap_or_default();

    if dispatch_bin_path.is_empty() {
        let error_msg = "dispatch_binary_path is not set or is empty in api.json. Please set it to the path of the orca-dispatch binary.";
        error!("{}", error_msg);
        log_to_db(&db_pool, "api", "ERROR", error_msg).await;
        return HttpResponse::InternalServerError().json(DispatchResponse {
            stdout: "".to_string(),
            stderr: error_msg.to_string(),
            success: false,
        });
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

    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => {
            let error_msg = format!("Failed to execute orca-dispatch: {}", e);
            error!("{}", error_msg);
            log_to_db(&db_pool, "api", "ERROR", &error_msg).await;
            return HttpResponse::InternalServerError().json(DispatchResponse {
                stdout: "".to_string(),
                stderr: error_msg.to_string(),
                success: false,
            });
        }
    };

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

#[get("/db/{table_name}")]
async fn get_table(
    db_pool: web::Data<MySqlPool>,
    table_name: web::Path<String>,
) -> impl Responder {
    let table = table_name.into_inner();
    if !table.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return HttpResponse::BadRequest().body("Invalid table name");
    }

    let query = format!("SELECT * FROM {}", table);
    let rows: Result<Vec<MySqlRow>, sqlx::Error> = sqlx::query(&query).fetch_all(&**db_pool).await;

    match rows {
        Ok(rows) => {
            let results: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let mut map = serde_json::Map::new();
                    for col in row.columns() {
                        let key = col.name().to_string();
                        let value: Value = match row.try_get_raw(col.ordinal()) {
                            Ok(raw_value) if !raw_value.is_null() => {
                                match col.type_info().name() {
                                    "TEXT" | "VARCHAR" | "CHAR" => json!(row.get::<String, &str>(col.name())),
                                    "INT" | "BIGINT" => json!(row.get::<i64, &str>(col.name())),
                                    "BOOLEAN" => json!(row.get::<bool, &str>(col.name())),
                                    _ => json!(row.get::<String, &str>(col.name())),
                                }
                            }
                            _ => Value::Null,
                        };
                        map.insert(key, value);
                    }
                    Value::Object(map)
                })
                .collect();
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            error!("Failed to fetch from table {}: {}", table, e);
            HttpResponse::InternalServerError().body(format!("Failed to fetch from table {}: {}", table, e))
        }
    }
}

#[post("/db/{table_name}")]
async fn insert_table(
    db_pool: web::Data<MySqlPool>,
    table_name: web::Path<String>,
    body: web::Json<Vec<Value>>,
) -> impl Responder {
    let table = table_name.into_inner();
    if !table.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return HttpResponse::BadRequest().body("Invalid table name");
    }

    let mut transaction = match db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to begin transaction: {}", e);
            return HttpResponse::InternalServerError().body("Failed to begin transaction");
        }
    };

    for item in body.iter() {
        if let Value::Object(map) = item {
            let columns: Vec<String> = map.keys().map(|k| k.to_string()).collect();
            let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
            let query_str = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table,
                columns.join(", "),
                placeholders.join(", ")
            );

            let mut q = sqlx::query(&query_str);
            for key in &columns {
                q = match map.get(key).unwrap() {
                    Value::String(s) => q.bind(s),
                    Value::Number(n) => {
                        if n.is_f64() {
                            q.bind(n.as_f64().unwrap())
                        } else if n.is_i64() {
                            q.bind(n.as_i64().unwrap())
                        } else {
                            q.bind(n.as_u64().unwrap())
                        }
                    }
                    Value::Bool(b) => q.bind(b),
                    Value::Null => q.bind(None::<String>), // Assuming NULL for null values
                    _ => {
                        // Unsupported type
                        let _ = transaction.rollback().await;
                        return HttpResponse::BadRequest().body("Unsupported data type in JSON");
                    }
                };
            }

            if let Err(e) = q.execute(&mut *transaction).await {
                error!("Failed to insert into table {}: {}", table, e);
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError().body(format!("Failed to insert into table {}: {}", table, e));
            }
        } else {
            let _ = transaction.rollback().await;
            return HttpResponse::BadRequest().body("Invalid data format, expected an array of objects.");
        }
    }

    if let Err(e) = transaction.commit().await {
        error!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().body("Failed to commit transaction");
    }

    HttpResponse::Ok().body("Data inserted successfully")
}

#[put("/db/{table_name}")]
async fn update_table(
    db_pool: web::Data<MySqlPool>,
    table_name: web::Path<String>,
    query: web::Query<UpdateQuery>,
    body: web::Json<Value>,
) -> impl Responder {
    let table = table_name.into_inner();
    if !table.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return HttpResponse::BadRequest().body("Invalid table name");
    }

    if !query.pk_col.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return HttpResponse::BadRequest().body("Invalid primary key column name");
    }

    if let Value::Object(map) = body.into_inner() {
        let set_clause: Vec<String> = map
            .keys()
            .map(|k| format!("`{}` = ?", k))
            .collect();

        let query_str = format!(
            "UPDATE `{}` SET {} WHERE `{}` = ?",
            table,
            set_clause.join(", "),
            query.pk_col
        );

        let mut q = sqlx::query(&query_str);
        for key in map.keys() {
            q = match map.get(key).unwrap() {
                Value::String(s) => q.bind(s),
                Value::Number(n) => {
                    if n.is_f64() {
                        q.bind(n.as_f64().unwrap())
                    } else if n.is_i64() {
                        q.bind(n.as_i64().unwrap())
                    } else {
                        q.bind(n.as_u64().unwrap())
                    }
                }
                Value::Bool(b) => q.bind(b),
                Value::Null => q.bind(None::<String>),
                _ => {
                    return HttpResponse::BadRequest().body("Unsupported data type in JSON");
                }
            };
        }

        q = q.bind(&query.pk_val);

        match q.execute(&**db_pool).await {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    HttpResponse::Ok().body("Row updated successfully")
                } else {
                    HttpResponse::NotFound().body("Row not found")
                }
            }
            Err(e) => {
                error!("Failed to update table {}: {}", table, e);
                HttpResponse::InternalServerError()
                    .body(format!("Failed to update table {}: {}", table, e))
            }
        }
    } else {
        HttpResponse::BadRequest().body("Invalid data format, expected an object.")
    }
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
            .service(get_table)
            .service(insert_table)
            .service(update_table)
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