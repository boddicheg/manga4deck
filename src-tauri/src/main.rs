// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logger;
use logger::{
    info, 
    LOGGER,
    LogResponse
};

use axum::{
    routing::{get},
    Router,
    http::StatusCode,
    Json
};
use tower_http::cors::{Any, CorsLayer};
// use serde::{Serialize};

// const APP_NAME: &str = "manga4deck";
const KAVITA_IP: &str = "127.0.0.1:11337";

#[tauri::command]
fn exit_app() {
  std::process::exit(0x0);
}

async fn get_logs() -> (StatusCode, Json<LogResponse>) {
    info("[get_logs] Getting logs...");
    let logs = LOGGER.lock().unwrap().get();
    (StatusCode::OK, Json(LogResponse { logs: logs.to_vec(), count: logs.len() }))
}

#[tokio::main]
async fn start_server() {
    // Create CORS layer (allow all origins and methods)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app = Router::new()
        .route("/api/logs", get(get_logs))
        .layer(cors);

    // Run the server
    let listener = tokio::net::TcpListener::bind(KAVITA_IP).await.unwrap();
    info(&format!("Server running at {}", KAVITA_IP));
    axum::serve(listener, app).await.unwrap();
}

fn main() {
    tauri::Builder::default()
        .setup(|_| {
            // start the server in a new thread and close the thread when the app is closed
            std::thread::spawn(|| {
                start_server();
            }); 
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![exit_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
