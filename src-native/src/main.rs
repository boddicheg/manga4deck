// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::File;
use std::io::Read;

mod logger;
use logger::{info, LogResponse, LOGGER};

mod kavita;
use kavita::{get_cache_size, ConnectionCreds, Kavita, Library, ReadProgress, Series, Volume};

mod dioxus_ui;
mod storage;

use axum::{
    body::Body,
    extract::{Extension, Path},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tower_http::cors::{Any, CorsLayer};

// const APP_NAME: &str = "manga4deck";
const KAVITA_IP: &str = "0.0.0.0:11337";

type SharedKavita = Arc<Mutex<Kavita>>;

type WebSocketSender = broadcast::Sender<serde_json::Value>;

async fn get_logs() -> (StatusCode, Json<LogResponse>) {
    // info("[get_logs] Getting logs...");
    let logs = LOGGER.lock().unwrap().get();
    (
        StatusCode::OK,
        Json(LogResponse {
            logs: logs.to_vec(),
            count: logs.len(),
        }),
    )
}

#[derive(Serialize)]
struct StatusResponse {
    status: bool,
    ip: String,
    logged_as: String,
    cache: u64,
    offline_mode: bool,
}

async fn get_status(
    Extension(kavita): Extension<SharedKavita>,
) -> (StatusCode, Json<StatusResponse>) {
    let kavita_guard = kavita.lock().await;
    let ip = kavita_guard
        .get_setting("ip")
        .ok()
        .flatten()
        .unwrap_or_else(|| "".to_string());
    let logged_as = kavita_guard.logged_as.clone();
    let cache = get_cache_size(1024 * 1024);
    let offline_mode = kavita_guard.offline_mode;

    (
        StatusCode::OK,
        Json(StatusResponse {
            status: !offline_mode,
            ip,
            logged_as,
            cache,
            offline_mode,
        }),
    )
}

async fn toggle_offline_mode(
    Extension(kavita): Extension<SharedKavita>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut kavita_guard = kavita.lock().await;
    kavita_guard.offline_mode = !kavita_guard.offline_mode;

    // Send connection status notification
    if kavita_guard.offline_mode {
        kavita_guard.send_connection_status(true, "");
        info("Manually switched to offline mode");
    } else {
        // Try to reconnect when switching back to online
        let reconnect_result = kavita_guard.reconnect_with_creds().await;
        if reconnect_result.is_err() {
            // If reconnection fails, stay in offline mode
            kavita_guard.offline_mode = true;
            kavita_guard.send_connection_status(true, "");
            info("Failed to reconnect, staying in offline mode");
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "offline_mode": kavita_guard.offline_mode,
            "message": if kavita_guard.offline_mode { "Switched to offline mode" } else { "Switched to online mode" }
        })),
    )
}

async fn update_server_settings(
    Extension(kavita): Extension<SharedKavita>,
    Json(creds): Json<ConnectionCreds>,
) -> (StatusCode, Json<ConnectionCreds>) {
    info(&format!(
        "Updating server settings: ip='{}' username='{}' password={} api_key={}",
        creds.ip,
        creds.username,
        if creds.password.is_empty() {
            "<empty>"
        } else {
            "<set>"
        },
        if creds.api_key.is_empty() {
            "<empty>"
        } else {
            "<set>"
        }
    ));
    let mut kavita_guard = kavita.lock().await;
    let _ = kavita_guard.insert_setting("ip", &creds.ip);
    let _ = kavita_guard.insert_setting("username", &creds.username);
    let _ = kavita_guard.insert_setting("password", &creds.password);
    let _ = kavita_guard.insert_setting("api_key", &creds.api_key);
    let _ = kavita_guard.reconnect_with_creds().await;
    (StatusCode::OK, Json(creds))
}

async fn get_server_settings(
    Extension(kavita): Extension<SharedKavita>,
) -> (StatusCode, Json<ConnectionCreds>) {
    info("[get_server_settings] Getting server settings...");
    let kavita_guard = kavita.lock().await;
    let ip = kavita_guard
        .get_setting("ip")
        .ok()
        .flatten()
        .unwrap_or_else(|| "".to_string());
    let username = kavita_guard
        .get_setting("username")
        .ok()
        .flatten()
        .unwrap_or_else(|| "".to_string());
    let password = kavita_guard
        .get_setting("password")
        .ok()
        .flatten()
        .unwrap_or_else(|| "".to_string());
    let api_key = kavita_guard
        .get_setting("api_key")
        .ok()
        .flatten()
        .unwrap_or_else(|| "".to_string());
    let creds = ConnectionCreds {
        ip,
        username,
        password,
        api_key,
    };
    (StatusCode::OK, Json(creds))
}

fn content_type_from_path(path: &str) -> &'static str {
    if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    }
}

fn read_file_response(
    path: &str,
    cache_control: Option<&'static str>,
) -> Result<Response, (StatusCode, Response)> {
    let content_type = content_type_from_path(path);
    match File::open(path) {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            if file.read_to_end(&mut buffer).is_err() {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Response::new(Body::from("Failed to read file")),
                ));
            }
            let mut builder = Response::builder().header("Content-Type", content_type);
            if let Some(cc) = cache_control {
                builder = builder.header("Cache-Control", cc);
            }
            let response = builder.body(axum::body::Body::from(buffer)).map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Response::new(Body::from("Failed to build response")),
                )
            })?;
            Ok(response)
        }
        Err(_) => Err((
            StatusCode::NOT_FOUND,
            Response::new(Body::from("File not found")),
        )),
    }
}

// #[axum::debug_handler]
async fn get_libraries(
    Extension(kavita): Extension<SharedKavita>,
) -> (StatusCode, Json<Vec<Library>>) {
    let kavita_guard = kavita.lock().await;
    match kavita_guard.get_libraries().await {
        Ok(libraries) => (StatusCode::OK, Json(libraries)),
        Err(err) => {
            info(&format!("Failed to get libraries: {}", err));
            (StatusCode::SERVICE_UNAVAILABLE, Json(Vec::new()))
        }
    }
}

async fn clear_cache(Extension(kavita): Extension<SharedKavita>) -> (StatusCode, Json<()>) {
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard.clear_cache();
    (StatusCode::OK, Json(()))
}

async fn update_server_library(
    Extension(kavita): Extension<SharedKavita>,
) -> (StatusCode, Json<()>) {
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard.update_server_library().await;
    (StatusCode::OK, Json(()))
}

async fn get_series(
    Extension(kavita): Extension<SharedKavita>,
    Path(library_id): Path<i32>,
) -> (StatusCode, Json<Vec<Series>>) {
    // info(&format!("Getting series for library: {}", library_id));
    let kavita_guard = kavita.lock().await;
    match kavita_guard.get_series(&library_id).await {
        Ok(series) => (StatusCode::OK, Json(series)),
        Err(err) => {
            info(&format!(
                "Failed to get series for library {}: {}",
                library_id, err
            ));
            (StatusCode::OK, Json(Vec::new()))
        }
    }
}

async fn get_series_cover(
    Extension(kavita): Extension<SharedKavita>,
    Path(series_id): Path<i32>,
) -> (StatusCode, Response) {
    // info(&format!("Getting series cover for series: {}", series_id));
    let kavita_guard = kavita.lock().await;
    match kavita_guard.get_series_cover(&series_id).await {
        Ok(series_cover) => {
            match read_file_response(
                &series_cover.file,
                Some("public, max-age=31536000, immutable"),
            ) {
                Ok(resp) => (StatusCode::OK, resp),
                Err((status, resp)) => (status, resp),
            }
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Response::new(Body::from("Series cover not available")),
        ),
    }
}

async fn get_volume_cover(
    Extension(kavita): Extension<SharedKavita>,
    Path(volume_id): Path<i32>,
) -> (StatusCode, Response) {
    // info(&format!("Getting volume cover for volume: {}", volume_id));
    let kavita_guard = kavita.lock().await;
    match kavita_guard.get_volume_cover(&volume_id).await {
        Ok(volume_cover) => {
            match read_file_response(
                &volume_cover.file,
                Some("public, max-age=31536000, immutable"),
            ) {
                Ok(resp) => (StatusCode::OK, resp),
                Err((status, resp)) => (status, resp),
            }
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Response::new(Body::from("Volume cover not available")),
        ),
    }
}

async fn get_volumes(
    Extension(kavita): Extension<SharedKavita>,
    Path(series_id): Path<i32>,
) -> (StatusCode, Json<Vec<Volume>>) {
    // info(&format!("Getting volumes for series: {}", series_id));
    let kavita_guard = kavita.lock().await;
    match kavita_guard.get_volumes(&series_id).await {
        Ok(volumes) => {
            let sample: Vec<String> = volumes
                .iter()
                .take(5)
                .map(|v| format!("{}:{}", v.id, v.title))
                .collect();
            info(&format!(
                "get_volumes(series_id={}) -> {} volumes; sample: [{}]",
                series_id,
                volumes.len(),
                sample.join(", ")
            ));
            (StatusCode::OK, Json(volumes))
        }
        Err(err) => {
            info(&format!(
                "Failed to get volumes for series {}: {}",
                series_id, err
            ));
            (StatusCode::SERVICE_UNAVAILABLE, Json(Vec::new()))
        }
    }
}

async fn get_picture(
    Extension(kavita): Extension<SharedKavita>,
    Path((series_id, volume_id, chapter_id, page)): Path<(i32, i32, i32, i32)>,
) -> (StatusCode, Response) {
    // info(&format!("Getting picture for series: {}, volume: {}, chapter: {}, page: {}", series_id, volume_id, chapter_id, page));
    let kavita_guard = kavita.lock().await;
    let picture = match kavita_guard.get_picture(&chapter_id, &page).await {
        Ok(p) => p,
        Err(err) => {
            info(&format!(
                "Failed to get picture (chapter_id={}, page={}): {}",
                chapter_id, page, err
            ));
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Response::new(Body::from("Failed to load picture")),
            );
        }
    };

    let library_id = kavita_guard.get_series_library_id(series_id).unwrap_or(0);

    if let Err(err) = kavita_guard
        .save_progress(&ReadProgress {
            id: None,
            library_id,
            series_id,
            volume_id,
            chapter_id,
            page,
        })
        .await
    {
        info(&format!(
            "Progress update failed from picture route: series_id={} volume_id={} chapter_id={} page={} err={}",
            series_id, volume_id, chapter_id, page, err
        ));
    } else {
        info(&format!(
            "Progress updated from picture route: series_id={} volume_id={} chapter_id={} page={}",
            series_id, volume_id, chapter_id, page
        ));
    }

    match read_file_response(&picture, None) {
        Ok(resp) => (StatusCode::OK, resp),
        Err((status, resp)) => (status, resp),
    }
}

async fn post_progress(
    Extension(kavita): Extension<SharedKavita>,
    Path((series_id, volume_id, chapter_id, page)): Path<(i32, i32, i32, i32)>,
) -> (StatusCode, Json<serde_json::Value>) {
    info(&format!(
        "Received manual progress update request: series_id={} volume_id={} chapter_id={} page={}",
        series_id, volume_id, chapter_id, page
    ));
    let kavita_guard = kavita.lock().await;
    let library_id = kavita_guard.get_series_library_id(series_id).unwrap_or(0);
    match kavita_guard
        .save_progress(&ReadProgress {
            id: None,
            library_id,
            series_id,
            volume_id,
            chapter_id,
            page,
        })
        .await
    {
        Ok(_) => {
            info(&format!(
                "Manual progress update saved: series_id={} volume_id={} chapter_id={} page={}",
                series_id, volume_id, chapter_id, page
            ));
            (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
        }
        Err(err) => {
            info(&format!(
                "Manual progress update failed: series_id={} volume_id={} chapter_id={} page={} err={}",
                series_id, volume_id, chapter_id, page, err
            ));
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "status": "error", "message": err.to_string() })),
            )
        }
    }
}

async fn read_volume(
    Extension(kavita): Extension<SharedKavita>,
    Path((series_id, volume_id)): Path<(i32, i32)>,
) -> (StatusCode, Json<()>) {
    info(&format!("Reading volume: {}, {}", series_id, volume_id));
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard
        .set_volume_as_read(&series_id, &volume_id)
        .await;
    (StatusCode::OK, Json(()))
}

async fn unread_volume(
    Extension(kavita): Extension<SharedKavita>,
    Path((series_id, volume_id)): Path<(i32, i32)>,
) -> (StatusCode, Json<()>) {
    info(&format!("Unreading volume: {}, {}", series_id, volume_id));
    let kavita_guard = kavita.lock().await;
    let _ = kavita_guard
        .set_volume_as_unread(&series_id, &volume_id)
        .await;
    (StatusCode::OK, Json(()))
}

async fn cache_serie_route(
    Extension(kavita): Extension<SharedKavita>,
    Path(series_id): Path<i32>,
) -> (StatusCode, Json<serde_json::Value>) {
    let kavita_guard = kavita.lock().await;
    kavita_guard.cache_serie(series_id);
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "caching started", "series_id": series_id})),
    )
}

async fn remove_series_cache_route(
    Extension(kavita): Extension<SharedKavita>,
    Path(series_id): Path<i32>,
) -> (StatusCode, Json<serde_json::Value>) {
    let kavita_guard = kavita.lock().await;
    match kavita_guard.remove_series_cache(series_id) {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "success",
                "message": format!("Cache removed for series {}", series_id),
                "series_id": series_id
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to remove cache: {}", e),
                "series_id": series_id
            })),
        ),
    }
}

async fn start_websocket_server(sender: WebSocketSender) {
    let addr = "0.0.0.0:11338";
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(err) => {
            info(&format!(
                "Failed to bind WebSocket server {}: {}",
                addr, err
            ));
            return;
        }
    };
    info(&format!("WebSocket server running at {}", addr));

    while let Ok((stream, _)) = listener.accept().await {
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_websocket_connection(stream, sender_clone).await {
                info(&format!("WebSocket connection error: {}", e));
            }
        });
    }
}

async fn handle_websocket_connection(
    stream: TcpStream,
    sender: WebSocketSender,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await?;
    let (mut sender_ws, mut receiver_ws) = ws_stream.split();
    let mut rx = sender.subscribe();

    // Spawn a task to forward messages from the broadcast channel to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json_msg = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".to_string());
            if sender_ws.send(Message::Text(json_msg)).await.is_err() {
                break;
            }
        }
    });

    // Spawn a task to receive messages from the WebSocket (for ping/pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver_ws.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    Ok(())
}

async fn initialize_kavita() -> SharedKavita {
    // Create WebSocket broadcaster
    let (ws_sender, _) = broadcast::channel::<serde_json::Value>(100);
    let ws_sender_arc = Arc::new(ws_sender);

    let mut kavita = Kavita::new();
    // Store WebSocket sender in Kavita BEFORE reconnecting so status messages can be sent
    kavita.set_websocket_sender(ws_sender_arc.clone());
    let _ = kavita.reconnect_with_creds().await;
    // Send initial connection status
    {
        let kavita_guard = &kavita;
        if kavita_guard.offline_mode {
            kavita_guard.send_connection_status(true, "");
        } else {
            kavita_guard.send_connection_status(false, &kavita_guard.logged_as);
        }
    }

    // Start WebSocket server in a separate task
    let ws_sender_for_server = (*ws_sender_arc).clone();
    tokio::spawn(async move {
        start_websocket_server(ws_sender_for_server).await;
    });

    Arc::new(Mutex::new(kavita))
}

async fn run_http_server(kavita: SharedKavita) {
    // Create CORS layer (allow all origins and methods)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/logs", get(get_logs))
        .route("/api/status", get(get_status))
        .route("/api/toggle-offline-mode", post(toggle_offline_mode))
        .route("/api/server-settings", get(get_server_settings))
        .route("/api/server-settings", post(update_server_settings))
        .route("/api/library", get(get_libraries))
        .route("/api/clear-cache", get(clear_cache))
        .route("/api/update-lib", get(update_server_library))
        .route("/api/series/{library_id}", get(get_series))
        .route("/api/volumes/{series_id}", get(get_volumes))
        .route("/api/series-cover/{series_id}", get(get_series_cover))
        .route("/api/volumes-cover/{volume_id}", get(get_volume_cover))
        .route(
            "/api/picture/{series}/{volume}/{chapter}/{page}",
            get(get_picture),
        )
        .route(
            "/api/progress/{series_id}/{volume_id}/{chapter_id}/{page}",
            post(post_progress),
        )
        .route("/api/read-volume/{series_id}/{volume_id}", get(read_volume))
        .route(
            "/api/unread-volume/{series_id}/{volume_id}",
            get(unread_volume),
        )
        .route("/api/cache/serie/{series_id}", get(cache_serie_route))
        .route(
            "/api/cache/remove/{series_id}",
            post(remove_series_cache_route),
        )
        .layer(cors)
        .layer(Extension(kavita));

    // Run the server
    let listener = match tokio::net::TcpListener::bind(KAVITA_IP).await {
        Ok(l) => l,
        Err(err) => {
            info(&format!(
                "Failed to bind HTTP server {}: {}",
                KAVITA_IP, err
            ));
            return;
        }
    };
    info(&format!("Server running at {}", KAVITA_IP));
    if let Err(err) = axum::serve(listener, app).await {
        info(&format!("HTTP server error: {}", err));
    }
}

fn main() {
    // Force software rendering to avoid EGL issues on Steam Deck
    std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
    std::env::set_var("GALLIUM_DRIVER", "llvmpipe");
    std::env::set_var("MESA_LOADER_DRIVER_OVERRIDE", "swrast");
    std::env::set_var("MESA_GL_VERSION_OVERRIDE", "2.1");
    std::env::set_var("MESA_GLSL_VERSION_OVERRIDE", "120");

    // Disable all hardware acceleration
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    std::env::set_var("WEBKIT_USE_SINGLE_WEB_PROCESS", "1");
    std::env::set_var("WEBKIT_DISABLE_WEB_SECURITY", "1");
    std::env::set_var("WEBKIT_DISABLE_GPU_PROCESS", "1");
    std::env::set_var("WEBKIT_DISABLE_GPU", "1");
    std::env::set_var("WEBKIT_DISABLE_EGL", "1");
    std::env::set_var("WEBKIT_DISABLE_OPENGL", "1");
    std::env::set_var("WEBKIT_DISABLE_GL", "1");
    std::env::set_var("WEBKIT_DISABLE_ACCELERATED_2D_CANVAS", "1");
    std::env::set_var("WEBKIT_DISABLE_ACCELERATED_VIDEO", "1");
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING", "1");

    // Force X11 and disable Wayland
    if std::env::var("DISPLAY").is_err() {
        std::env::set_var("DISPLAY", ":0");
    }
    std::env::set_var("WAYLAND_DISPLAY", "");
    std::env::set_var("XDG_SESSION_TYPE", "x11");
    std::env::set_var("GDK_BACKEND", "x11");
    std::env::set_var("QT_QPA_PLATFORM", "xcb");

    run_frontend();
}

fn run_frontend() {
    info("🚀 Manga4Deck v0.6.0 - Starting Manga4Deck native UI...");

    let runtime = tokio::runtime::Runtime::new().expect("failed to create native UI tokio runtime");
    let _runtime_guard = runtime.enter();
    let kavita = runtime.block_on(initialize_kavita());
    let server_kavita = kavita.clone();
    runtime.spawn(async move {
        run_http_server(server_kavita).await;
    });

    dioxus_ui::run_ui(kavita);
}
