// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::File;
use std::io::Read;

mod logger;
use logger::{   
    info, 
    LOGGER,
    LogResponse
};

mod kavita;
use kavita::{
    ConnectionCreds,
    get_cache_size,
    Kavita,
    Library,
    Series,
    Volume,
    ReadProgress,
};

mod storage;
mod fallback_html;

use axum::{
    routing::{get, post},
    Router,
    http::StatusCode,
    Json,
    extract::Extension,
    extract::Path,
    response::Html,
    response::Response,
    body::Body
};
use tower_http::cors::{Any, CorsLayer};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

// const APP_NAME: &str = "manga4deck";
const KAVITA_IP: &str = "0.0.0.0:11337";

#[tauri::command]
fn exit_app() {
  std::process::exit(0x0);
}

type SharedKavita = Arc<Mutex<Kavita>>;

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

async fn get_status(Extension(kavita): Extension<SharedKavita>) -> (StatusCode, Json<StatusResponse>) {
    let kavita_guard = kavita.lock().await;
    let ip = kavita_guard.get_setting("ip")
        .expect("Failed to get IP setting")
        .unwrap_or_else(|| "".to_string());
    let logged_as = kavita_guard.logged_as.clone();
    let cache = get_cache_size(1024 * 1024);

    (StatusCode::OK, Json(StatusResponse {
        status: true,
        ip,
        logged_as,
        cache,
    }))
}

async fn update_server_settings(Extension(kavita): Extension<SharedKavita>, Json(creds): Json<ConnectionCreds>) -> (StatusCode, Json<ConnectionCreds>) {
    info(&format!("Updating server settings: {:?}", creds));
    let mut kavita_guard = kavita.lock().await;
    let _ = kavita_guard.insert_setting("ip", &creds.ip);
    let _ = kavita_guard.insert_setting("username", &creds.username);
    let _ = kavita_guard.insert_setting("password", &creds.password);
    let _ = kavita_guard.insert_setting("api_key", &creds.api_key);
    let _ = kavita_guard.reconnect_with_creds().await;
    (StatusCode::OK, Json(creds))
}

async fn get_server_settings(Extension(kavita): Extension<SharedKavita>) -> (StatusCode, Json<ConnectionCreds>) {
    info("[get_server_settings] Getting server settings...");
    let kavita_guard = kavita.lock().await;
    let ip = kavita_guard.get_setting("ip")
        .expect("Failed to get IP setting")
        .unwrap_or_else(|| "".to_string());
    let username = kavita_guard.get_setting("username")
        .expect("Failed to get username setting")
        .unwrap_or_else(|| "".to_string());
    let password = kavita_guard.get_setting("password")
        .expect("Failed to get password setting")
        .unwrap_or_else(|| "".to_string());
    let api_key = kavita_guard.get_setting("api_key")
        .expect("Failed to get API key setting")
        .unwrap_or_else(|| "".to_string());
    let creds = ConnectionCreds { ip, username, password, api_key };
    (StatusCode::OK, Json(creds))
}

// #[axum::debug_handler]
async fn get_libraries(Extension(kavita): Extension<SharedKavita>) -> (StatusCode, Json<Vec<Library>>) {
    let kavita_guard = kavita.lock().await;
    let libraries = kavita_guard.get_libraries().await.unwrap();
    (StatusCode::OK, Json(libraries))
}

async fn clear_cache(Extension(kavita): Extension<SharedKavita>) -> (StatusCode, Json<()>) {
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard.clear_cache();
    (StatusCode::OK, Json(()))
}

async fn update_server_library(Extension(kavita): Extension<SharedKavita>) -> (StatusCode, Json<()>) {
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard.update_server_library().await;
    (StatusCode::OK, Json(()))
}

async fn get_series(
    Extension(kavita): Extension<SharedKavita>,
    Path(library_id): Path<i32>
) -> (StatusCode, Json<Vec<Series>>) {
    // info(&format!("Getting series for library: {}", library_id));
    let kavita_guard = kavita.lock().await;
    let series = kavita_guard.get_series(&library_id).await.unwrap();
    (StatusCode::OK, Json(series))
}

async fn get_series_cover(
    Extension(kavita): Extension<SharedKavita>,
    Path(series_id): Path<i32>
) -> (StatusCode, Response) {
    // info(&format!("Getting series cover for series: {}", series_id));
    let kavita_guard = kavita.lock().await;
    let series_cover = kavita_guard.get_series_cover(&series_id).await.unwrap();
    let mut file = File::open(series_cover.file).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap(); 
    let response = Response::builder()
        .header("Content-Type", "image/png")
        .body(axum::body::Body::from(buffer))
        .unwrap();
    (StatusCode::OK, response)
}

async fn get_volume_cover(
    Extension(kavita): Extension<SharedKavita>,
    Path(volume_id): Path<i32>
) -> (StatusCode, Response) {
    // info(&format!("Getting volume cover for volume: {}", volume_id));
    let kavita_guard = kavita.lock().await;
    let volume_cover = kavita_guard.get_volume_cover(&volume_id).await.unwrap();
    let mut file = File::open(volume_cover.file).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap(); 
    let response = Response::builder()
        .header("Content-Type", "image/png")
        .body(axum::body::Body::from(buffer))
        .unwrap();
    (StatusCode::OK, response)
}

async fn get_volumes(
    Extension(kavita): Extension<SharedKavita>,
    Path(series_id): Path<i32>
) -> (StatusCode, Json<Vec<Volume>>) {
    // info(&format!("Getting volumes for series: {}", series_id));
    let kavita_guard = kavita.lock().await;
    let volumes = kavita_guard.get_volumes(&series_id).await.unwrap();
    (StatusCode::OK, Json(volumes))
}

async fn get_picture(
    Extension(kavita): Extension<SharedKavita>,
    Path((series_id, volume_id, chapter_id, page)): Path<(i32, i32, i32, i32)>
) -> (StatusCode, Response) {
    // info(&format!("Getting picture for series: {}, volume: {}, chapter: {}, page: {}", series_id, volume_id, chapter_id, page));
    let kavita_guard = kavita.lock().await;
    let picture = kavita_guard.get_picture(&chapter_id, &page).await.unwrap();
    kavita_guard.save_progress(&ReadProgress { series_id, volume_id, chapter_id, page }).await.unwrap();

    let mut file = File::open(picture).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap(); 
    (StatusCode::OK, Response::builder().body(axum::body::Body::from(buffer)).unwrap())
}

async fn read_volume(
    Extension(kavita): Extension<SharedKavita>,
    Path((series_id, volume_id)): Path<(i32, i32)>
) -> (StatusCode, Json<()>) {
    info(&format!("Reading volume: {}, {}", series_id, volume_id));
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard.set_volume_as_read(&series_id, &volume_id).await;
    (StatusCode::OK, Json(()))
}

async fn unread_volume(
    Extension(kavita): Extension<SharedKavita>, 
    Path((series_id, volume_id)): Path<(i32, i32)>
) -> (StatusCode, Json<()>) {
    info(&format!("Unreading volume: {}, {}", series_id, volume_id));
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard.set_volume_as_unread(&series_id, &volume_id).await;
    (StatusCode::OK, Json(()))
}

async fn cache_serie_route(
    Extension(kavita): Extension<SharedKavita>,
    Path(series_id): Path<i32>
) -> (StatusCode, Json<serde_json::Value>) {
    let kavita_guard = kavita.lock().await;
    kavita_guard.cache_serie(series_id);
    (StatusCode::OK, Json(serde_json::json!({"status": "caching started", "series_id": series_id})))
}

async fn serve_frontend() -> Result<Html<String>, StatusCode> {
    // Always serve the fallback HTML form
    info("Serving fallback HTML form");
    Ok(Html(fallback_html::get_fallback_html().to_string()))
}

#[tokio::main]
async fn start_server() {
    // Create CORS layer (allow all origins and methods)
    let mut kavita = Kavita::new();
    let _ = kavita.reconnect_with_creds().await;
    let kavita = Arc::new(Mutex::new(kavita));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(serve_frontend))
        .route("/api/logs", get(get_logs))
        .route("/api/status", get(get_status))
        .route("/api/server-settings", get(get_server_settings))
        .route("/api/server-settings", post(update_server_settings))
        .route("/api/library", get(get_libraries))
        .route("/api/clear-cache", get(clear_cache))
        .route("/api/update-lib", get(update_server_library))
        .route("/api/series/{library_id}", get(get_series))
        .route("/api/volumes/{series_id}", get(get_volumes))
        .route("/api/series-cover/{series_id}", get(get_series_cover))
        .route("/api/volumes-cover/{volume_id}", get(get_volume_cover))
        .route("/api/picture/{series}/{volume}/{chapter}/{page}", get(get_picture))
        .route("/api/read-volume/{series_id}/{volume_id}", get(read_volume))
        .route("/api/unread-volume/{series_id}/{volume_id}", get(unread_volume))
        .route("/api/cache/serie/{series_id}", get(cache_serie_route))
        .layer(cors)
        .layer(Extension(kavita));

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
