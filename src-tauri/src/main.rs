// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logger;
use logger::{   
    info, 
    LOGGER,
    LogResponse
};

mod kavita;
use kavita::{
    KAVITA,
    ConnectionCreds,
    get_cache_size,
    Kavita
};

mod storage;

use axum::{
    routing::{get, post},
    Router,
    http::StatusCode,
    Json
};
use tower_http::cors::{Any, CorsLayer};
use serde::{Serialize};

// const APP_NAME: &str = "manga4deck";
const KAVITA_IP: &str = "127.0.0.1:11337";

#[tauri::command]
fn exit_app() {
  std::process::exit(0x0);
}

async fn initialize_global() {
    Kavita::initialize().await.expect("Failed to initialize Kavita");
}

async fn get_logs() -> (StatusCode, Json<LogResponse>) {
    // info("[get_logs] Getting logs...");
    let logs = LOGGER.lock().unwrap().get();
    (StatusCode::OK, Json(LogResponse { logs: logs.to_vec(), count: logs.len() }))
}

#[derive(Serialize)]
struct StatusResponse {
    status: bool,
    ip: String,
    logged_as: String,
    cache: u64,
}

async fn get_status() -> (StatusCode, Json<StatusResponse>) {
    // info("[get_status] Getting status...");
    // let settings = KAVITA.lock().unwrap().get_settings();
    let kavita = KAVITA.get().expect("Kavita not initialized").lock().unwrap();
    let ip = kavita.get_setting("ip")
        .expect("Failed to get IP setting")
        .unwrap_or_else(|| "".to_string());
    let logged_as = kavita.get_setting("logged_as")
        .expect("Failed to get logged_as setting")
        .unwrap_or_else(|| "".to_string());
    let cache = get_cache_size(1024 * 1024 * 1024);

    (StatusCode::OK, Json(StatusResponse {
        status: true,
        ip,
        logged_as,
        cache,
    }))
}

async fn update_server_settings(Json(creds): Json<ConnectionCreds>) -> (StatusCode, Json<ConnectionCreds>) {
    info(&format!("Updating server settings: {:?}", creds));
    let kavita = KAVITA.get().expect("Kavita not initialized").lock().unwrap();
    let _ = kavita.insert_setting("ip", &creds.ip);
    let _ = kavita.insert_setting("username", &creds.username);
    let _ = kavita.insert_setting("password", &creds.password);
    let _ = kavita.insert_setting("api_key", &creds.api_key);
    (StatusCode::OK, Json(creds))
}

async fn get_server_settings() -> (StatusCode, Json<ConnectionCreds>) {
    info("[get_server_settings] Getting server settings...");
    let kavita = KAVITA.get().expect("Kavita not initialized").lock().unwrap();
    let ip = kavita.get_setting("ip")
        .expect("Failed to get IP setting")
        .unwrap_or_else(|| "".to_string());
    let username = kavita.get_setting("username")
        .expect("Failed to get username setting")
        .unwrap_or_else(|| "".to_string());
    let password = kavita.get_setting("password")
        .expect("Failed to get password setting")
        .unwrap_or_else(|| "".to_string());
    let api_key = kavita.get_setting("api_key")
        .expect("Failed to get API key setting")
        .unwrap_or_else(|| "".to_string());
    let creds = ConnectionCreds { ip, username, password, api_key };
    (StatusCode::OK, Json(creds))
}

#[tokio::main]
async fn start_server() {
    initialize_global().await;
    // Create CORS layer (allow all origins and methods)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app = Router::new()
        .route("/api/logs", get(get_logs))
        .route("/api/status", get(get_status))
        .route("/api/server-settings", get(get_server_settings))
        .route("/api/server-settings", post(update_server_settings))
        .layer(cors);

    // Run the server
    let listener = tokio::net::TcpListener::bind(KAVITA_IP).await.unwrap();
    info(&format!("Server running at {}", KAVITA_IP));
    axum::serve(listener, app).await.unwrap();
}

fn main() {
    tauri::Builder::default()
        .setup(|_| {
            std::thread::spawn(|| {
                start_server();
            }); 
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![exit_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
