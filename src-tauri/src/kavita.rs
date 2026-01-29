use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use serde_json::json;
use reqwest;
use md5::{Md5, Digest};
use crate::logger::{
    info
};
use reqwest::header;

use tokio::time::{timeout, Duration};

use crate::storage::{
    Database
};

use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::collections::VecDeque;
use tokio::sync::broadcast;
use serde_json::json as serde_json_json;

fn get_datadir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    
    match std::env::consts::OS {
        "windows" => home.join("AppData/Roaming"),
        "linux" => home.join(".local/share"),
        "macos" => home.join(""),
        _ => panic!("Unsupported operating system"),
    }
}

fn get_appdir_path(relative_path: &str) -> String {
    let mut datadir = get_datadir().join("manga4deck-cache");
    
    // Create directory if it doesn't exist
    if !datadir.exists() {
        fs::create_dir_all(&datadir).expect("Failed to create directory");
    }
    
    datadir = datadir.join(relative_path);
    datadir.to_string_lossy().into_owned()
}

const COVER_THUMB_WIDTH: u32 = 300;
const COVER_THUMB_HEIGHT: u32 = 400;
const COVER_JPEG_QUALITY: u8 = 75;

fn cache_folder_path() -> PathBuf {
    let cache_folder = get_datadir().join("manga4deck-cache").join("cache");
    if !cache_folder.exists() {
        fs::create_dir_all(&cache_folder).expect("Failed to create cache directory");
    }
    cache_folder
}

fn optimize_cover_to_jpeg_bytes(original: &[u8]) -> Option<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    use image::imageops::FilterType;
    use image::ColorType;

    let img = image::load_from_memory(original).ok()?;
    // Resize similar to CSS background-size: cover (crop center, no distortion).
    let thumb = img.resize_to_fill(COVER_THUMB_WIDTH, COVER_THUMB_HEIGHT, FilterType::CatmullRom);
    let rgb = thumb.to_rgb8();
    let (w, h) = rgb.dimensions();

    let mut out: Vec<u8> = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut out, COVER_JPEG_QUALITY);
    encoder.encode(&rgb, w, h, ColorType::Rgb8.into()).ok()?;
    Some(out)
}

pub fn get_cache_size(delimiter: u64) -> u64 {
    let cache_folder = get_datadir().join("manga4deck-cache").join("cache");

    if !cache_folder.exists() {
        fs::create_dir_all(&cache_folder).expect("Failed to create directory");
    }

    let mut size = 0;
    for entry in fs::read_dir(cache_folder).unwrap() {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();
        size += metadata.len();
    }
    size / delimiter
}

pub fn generate_hash_from_now() -> String {
    let now = std::time::SystemTime::now() 
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut hasher = Md5::new();
    hasher.update(format!("{}", now));
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

const DB_PATH: &str = "cache.sqlite";

const DEFAULT_IP: &str = "localhost:5000";
const DEFAULT_USERNAME: &str = "";
const DEFAULT_PASSWORD: &str = "";
const DEFAULT_API_KEY: &str = "";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionCreds {
    pub ip: String,
    pub username: String,
    pub password: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Library {
    pub id: i32,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Series {
    pub id: i32,      
    pub library_id: i32,
    pub title: String,
    pub read: i32,
    pub pages: i32,
}

#[derive(Clone)]
pub struct Kavita {
    pub db: Database,
    pub token: String,
    pub logged_as: String,
    pub offline_mode: bool,
    pub ip: String,
    pub api_key: String,
    // --- Caching fields ---
    pub caching_queue: Arc<StdMutex<VecDeque<i32>>>, // series_id queue
    pub caching_thread_handle: Arc<StdMutex<Option<thread::JoinHandle<()>>>>,
    // --- WebSocket fields ---
    pub ws_sender: Option<Arc<broadcast::Sender<serde_json::Value>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeriesCover {
    pub series_id: i32,
    pub file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Volume {
    pub id: i32,
    pub series_id: i32,
    pub chapter_id: i32,
    pub volume_id: i32,
    pub title: String,
    pub read: i32,
    pub pages: i32,
    pub is_cached: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VolumeCover {
    pub volume_id: i32,
    pub file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MangaPicture {
    pub chapter_id: i32,
    pub page: i32,
    pub file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadProgress {
    pub id: Option<i32>,
    pub library_id: i32,
    pub series_id: i32,
    pub volume_id: i32,
    pub chapter_id: i32,
    pub page: i32,
}

impl Kavita {
    pub fn new() -> Self {
        let db_path = get_appdir_path(DB_PATH);
        info(&format!("Database created at {}", db_path));
        info(&format!("Cache size: {} MB", get_cache_size(1024 * 1024)));

        let db = Database::new(&db_path).expect("Failed to create database");

        let kavita = Self {
            db,
            token: String::new(),
            logged_as: String::new(),
            offline_mode: true,
            ip: DEFAULT_IP.to_string(),
            api_key: DEFAULT_API_KEY.to_string(),
            caching_queue: Arc::new(StdMutex::new(VecDeque::new())),
            caching_thread_handle: Arc::new(StdMutex::new(None)),
            ws_sender: None,
        };

        kavita
    }

    pub fn insert_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.db.insert_setting(key, value)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        self.db.get_setting(key)
    }

    pub fn set_websocket_sender(&mut self, sender: Arc<broadcast::Sender<serde_json::Value>>) {
        self.ws_sender = Some(sender);
    }

    fn send_websocket_message(&self, event: &str, message: &str, data: Option<serde_json::Value>) {
        if let Some(sender) = &self.ws_sender {
            let ws_msg = serde_json_json!({
                "event": event,
                "message": message,
                "data": data
            });
            let _ = sender.send(ws_msg);
        }
    }

    pub fn send_connection_status(&self, is_offline: bool, logged_as: &str) {
        if is_offline {
            self.send_websocket_message(
                "connection_status",
                "Disconnected from Kavita server - Offline mode",
                Some(serde_json_json!({
                    "mode": "offline",
                    "connected": false
                }))
            );
        } else {
            self.send_websocket_message(
                "connection_status",
                &format!("Connected to Kavita server as {}", logged_as),
                Some(serde_json_json!({
                    "mode": "online",
                    "connected": true,
                    "username": logged_as
                }))
            );
        }
    }

    pub async fn reconnect_with_creds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info(&format!("reconnect_with_creds + "));
        // Get settings with proper error handling
        self.ip = self.db.get_setting("ip")
            .expect("Failed to get IP setting")
            .unwrap_or_else(|| DEFAULT_IP.to_string());
        let username = self.db.get_setting("username")
            .expect("Failed to get username setting")
            .unwrap_or_else(|| DEFAULT_USERNAME.to_string());
        let password = self.db.get_setting("password")
            .expect("Failed to get password setting")
            .unwrap_or_else(|| DEFAULT_PASSWORD.to_string());
        let api_key = self.db.get_setting("api_key")
            .expect("Failed to get API key setting")
            .unwrap_or_else(|| DEFAULT_API_KEY.to_string());
    
        info(&format!("IP: {}", self.ip));
        info(&format!("Username: {}", username));
        info(&format!("Password: {}", password));
        info(&format!("API Key: {}", api_key));

        // self.ip = "192.168.1.100:5001".to_string();

        // make http request to get token
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        let fut = client.post(format!("http://{}/api/Account/login", self.ip))
            .json(&json!({
                "username": username,
                "password": password,
                "apiKey": api_key
            }))
            .send();
        match timeout(Duration::from_secs(5), fut).await {
            Ok(Ok(resp)) => 
            {
                let body = resp.text().await.unwrap();
                info(&format!("Body: {}", body));
                let data: serde_json::Value = serde_json::from_str(&body)?;
                self.token = data["token"].as_str().unwrap_or("").to_string();
                self.logged_as = data["username"].as_str().unwrap_or("").to_string();
                self.api_key = data["apiKey"].as_str().unwrap_or("").to_string();
                self.offline_mode = false;
                info(&format!("Logged as: {}", self.logged_as));
                // Send connection status notification
                self.send_connection_status(false, &self.logged_as);
            },
            Ok(Err(e)) => {
                self.offline_mode = true;
                self.logged_as = "".to_string();
                self.token = "".to_string();
                info(&format!("Failed to get token. Now in offline mode"));
                // Send connection status notification
                self.send_connection_status(true, "");
                return Err(Box::new(e));
            }
            Err(_) => {
                self.offline_mode = true;
                self.logged_as = "".to_string();
                self.token = "".to_string();
                info(&format!("Failed to get token. Now in offline mode"));
                // Send connection status notification
                self.send_connection_status(true, "");
                return Err("Timeout".into());
            }
        };
        info(&format!("reconnect_with_creds - done"));
        
        // Upload any offline progress now that we're online (in background thread)
        if !self.offline_mode {
            info("Spawning background task to upload offline progress...");
            let db = self.db.clone();
            let ip = self.ip.clone();
            let token = self.token.clone();
            let ws_sender = self.ws_sender.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::upload_progress_background(db, ip, token, ws_sender).await {
                    info(&format!("Failed to upload offline progress: {}", e));
                }
            });
        }
        
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Cache methods
    pub fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cache_folder = get_datadir().join("manga4deck-cache").join("cache");
        // remove all files in cache folder
        for entry in fs::read_dir(cache_folder)? {
            fs::remove_file(entry.unwrap().path())?;
        }
        self.db.clean()?;
        Ok(())
    }

    pub async fn update_server_library(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("http://{}/api/library/scan-all", self.ip);
        if !self.offline_mode {
            let client = reqwest::Client::new();
            let _ = client.post(url)
                .header("Authorization", format!("Bearer {}", self.token))
                .json(&json!({
                    "force": true
                }))
                .send()
                .await?;
        }
        Ok(())
    }
    // -------------------------------------------------------------------------
    // Library methods
    pub async fn pull_libraries(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client.get(format!("http://{}/api/library/libraries", self.ip))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;    

        if response.status().is_success() {
            let body = response.text().await?;
            // info(&format!("Libraries: {}", body));
            let data: serde_json::Value = serde_json::from_str(&body)?;
            let libraries: Vec<Library> = data.as_array().unwrap().iter().map(|v| Library {
                id: v["id"].as_i64().unwrap_or(0) as i32,
                title: v["name"].as_str().unwrap_or("").to_string(),
            }).collect();
            for library in libraries {
                self.db.add_library(&library)?;
            }
        }
        else {
            info(&format!("Failed to get libraries: response status {}", response.status()));
        }   


        Ok(())
    }

    pub async fn get_libraries(&self) -> Result<Vec<Library>, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            self.pull_libraries().await?;
        }
        let libraries = self.db.get_libraries()?;
        Ok(libraries)
    }

    // -------------------------------------------------------------------------
    // Series methods
    pub async fn pull_series(&self, library_id: &i32) -> Result<(), Box<dyn std::error::Error>> {
        // let mut result = Vec::new();
        if !self.offline_mode {
            let client = reqwest::Client::new();
            let response = client.post(format!("http://{}/api/series/v2", self.ip))
                .header("Authorization", format!("Bearer {}", self.token))
                .json(&json!({
                    "statements": [
                        {
                            "comparison": 0,
                            "field": 19,
                            "value": library_id.to_string()
                        }
                    ],
                    "combination": 1,
                    "limitTo": 0
                }))
                .send()
                .await?;
            let body = response.text().await?;
            let data: serde_json::Value = serde_json::from_str(&body)?;
            let series: Vec<Series> = data.as_array().unwrap().iter().map(|v| Series {
                // Avoid panics if Kavita returns 0 pages
                // (some series can legitimately have 0 pages while metadata is still processing).
                id: v["id"].as_i64().unwrap_or(0) as i32,
                library_id: library_id.clone(),
                title: v["name"].as_str().unwrap_or("").to_string(),
                read: {
                    let pages_read = v["pagesRead"].as_i64().unwrap_or(0) as i32;
                    let pages = v["pages"].as_i64().unwrap_or(0) as i32;
                    if pages <= 0 {
                        0
                    } else {
                        (pages_read * 100) / pages
                    }
                },
                pages: v["pages"].as_i64().unwrap_or(0) as i32,
            }).collect();
            for series in series {
                self.db.add_series(&series)?;
            }
        }

        Ok(())
    }

    pub async fn get_series(&self, library_id: &i32) -> Result<Vec<Series>, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            self.pull_series(library_id).await?;
        }
        let mut series = self.db.get_series(library_id)?;
        // return only cached series
        if self.offline_mode {
            series = series.into_iter().filter(|s| self.is_series_cached(s.id)).collect();
        }
        // sort series by title and return sorted series
        series.sort_by_key(|s| s.title.clone());
        Ok(series)
    }

    pub async fn get_series_cover(&self, series_id: &i32) -> Result<SeriesCover, Box<dyn std::error::Error>> {
        // Prefer existing cached cover (and migrate old large PNGs to WebP).
        if let Ok(existing) = self.db.get_series_cover(series_id) {
            if Path::new(&existing.file).exists() {
                if existing.file.ends_with(".jpg") || existing.file.ends_with(".jpeg") {
                    return Ok(existing);
                }
                // Migrate/optimize old cached cover file to JPEG thumbnail.
                if let Ok(bytes) = fs::read(&existing.file) {
                    if let Some(jpg) = optimize_cover_to_jpeg_bytes(&bytes) {
                        let hash = generate_hash_from_now();
                        let filename = cache_folder_path().join(format!("{}.jpg", hash));
                        fs::write(&filename, jpg)?;
                        let updated = SeriesCover {
                            series_id: *series_id,
                            file: filename.to_string_lossy().into_owned(),
                        };
                        self.db.add_series_cover(&updated)?;
                        let _ = fs::remove_file(&existing.file);
                        return Ok(updated);
                    }
                }
                // If optimization fails, serve the existing file as-is.
                return Ok(existing);
            }
        }

        if self.offline_mode {
            info("Offline mode! Series cover not available in cache.");
            return Err("Series cover not available offline".into());
        }

        let url = format!(
            "http://{}/api/image/series-cover?seriesId={}&apiKey={}",
            self.ip, series_id, self.api_key
        );
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Accept", "image/*")
            .send()
            .await?;

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let body = response.bytes().await?;

        let (out_bytes, ext) = if let Some(jpg) = optimize_cover_to_jpeg_bytes(&body) {
            (jpg, "jpg")
        } else if content_type.contains("jpeg") {
            (body.to_vec(), "jpg")
        } else if content_type.contains("png") {
            (body.to_vec(), "png")
        } else {
            (body.to_vec(), "bin")
        };

        let hash = generate_hash_from_now();
        let filename = cache_folder_path().join(format!("{}.{}", hash, ext));
        fs::write(&filename, out_bytes)?;

        let series_cover = SeriesCover {
            series_id: *series_id,
            file: filename.to_string_lossy().into_owned(),
        };
        self.db.add_series_cover(&series_cover)?;

        Ok(series_cover)
    }

    // -------------------------------------------------------------------------
    // Volume methods
    pub async fn pull_volumes(&self, series_id: &i32) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!(
                "http://{}/api/series/series-detail?seriesId={}&apiKey={}",
                self.ip, series_id, self.api_key
            ))
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            info(&format!(
                "pull_volumes(series_id={}) failed: http_status={} body_snippet={}",
                series_id,
                status,
                body.chars().take(200).collect::<String>()
            ));
            return Err(format!("series-detail request failed with status {}", status).into());
        }

        let data: serde_json::Value = serde_json::from_str(&body)?;

        let top_level_chapter_id = data["chapters"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["id"].as_i64())
            .unwrap_or(0) as i32;

        let mut inserted = 0usize;
        let volumes_arr = data["volumes"]
            .as_array()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        for v in volumes_arr {
            let chapter_id = v["chapters"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["id"].as_i64())
                .unwrap_or(top_level_chapter_id as i64) as i32;

            // If we can't resolve a chapter id, this volume isn't readable anyway.
            if chapter_id <= 0 {
                continue;
            }

            let volume = Volume {
                id: v["id"].as_i64().unwrap_or(0) as i32,
                series_id: series_id.clone(),
                chapter_id,
                volume_id: v["id"].as_i64().unwrap_or(0) as i32,
                title: v["name"].as_str().unwrap_or("").to_string(),
                read: v["pagesRead"].as_i64().unwrap_or(0) as i32,
                pages: v["pages"].as_i64().unwrap_or(0) as i32,
                is_cached: false,
            };
            self.db.add_volume(&volume)?;
            inserted += 1;
        }

        if inserted == 0 {
            info(&format!(
                "pull_volumes(series_id={}) inserted 0 volumes (volumes_json_len={}, top_level_chapter_id={})",
                series_id,
                volumes_arr.len(),
                top_level_chapter_id
            ));
        }
        Ok(())
    }   

    pub async fn get_volumes(&self, series_id: &i32) -> Result<Vec<Volume>, Box<dyn std::error::Error>> {
        let mut last_pull_err: Option<String> = None;

        if !self.offline_mode {
            if let Err(e) = self.pull_volumes(series_id).await {
                last_pull_err = Some(e.to_string());
                info(&format!("Failed to pull volumes for series {}: {}", series_id, e));
            }
        }

        let mut volumes = self.db.get_volumes(series_id)?;

        // If the remote fetch failed AND we have no local data, retry once before returning an error.
        if !self.offline_mode && volumes.is_empty() && last_pull_err.is_some() {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if let Err(e) = self.pull_volumes(series_id).await {
                last_pull_err = Some(e.to_string());
                info(&format!("Retry pull_volumes failed for series {}: {}", series_id, e));
            } else {
                last_pull_err = None;
            }
            volumes = self.db.get_volumes(series_id)?;
            if volumes.is_empty() && last_pull_err.is_some() {
                return Err(format!(
                    "Failed to fetch volumes for series {} (db empty): {}",
                    series_id,
                    last_pull_err.unwrap_or_else(|| "unknown".to_string())
                )
                .into());
            }
        }

        // In offline mode we still return the full volume list so it doesn't look like
        // the series is "not loading". The UI already has `is_cached` to indicate if
        // a volume is available for offline reading.
        // sort volumes by title and converted to int and return sorted volumes
        volumes.sort_by_key(|v| v.title.clone().replace(|c: char| !c.is_digit(10), "").parse::<i32>().unwrap_or(0));
        // Update is_cached for each volume
        for v in &mut volumes {
            v.is_cached = self.is_volume_cached(v.id);
        }
        info(&format!(
            "kavita.get_volumes(series_id={}, offline_mode={}) -> {} volumes",
            series_id,
            self.offline_mode,
            volumes.len()
        ));
        Ok(volumes)
    }

    pub async fn get_volume_cover(&self, volume_id: &i32) -> Result<VolumeCover, Box<dyn std::error::Error>> {
        // Prefer existing cached cover (and migrate old large PNGs to WebP).
        if let Ok(existing) = self.db.get_volume_cover(volume_id) {
            if Path::new(&existing.file).exists() {
                if existing.file.ends_with(".jpg") || existing.file.ends_with(".jpeg") {
                    return Ok(existing);
                }
                // Migrate/optimize old cached cover file to JPEG thumbnail.
                if let Ok(bytes) = fs::read(&existing.file) {
                    if let Some(jpg) = optimize_cover_to_jpeg_bytes(&bytes) {
                        let hash = generate_hash_from_now();
                        let filename = cache_folder_path().join(format!("{}.jpg", hash));
                        fs::write(&filename, jpg)?;
                        let updated = VolumeCover {
                            volume_id: *volume_id,
                            file: filename.to_string_lossy().into_owned(),
                        };
                        self.db.add_volume_cover(&updated)?;
                        let _ = fs::remove_file(&existing.file);
                        return Ok(updated);
                    }
                }
                // If optimization fails, serve the existing file as-is.
                return Ok(existing);
            }
        }

        if self.offline_mode {
            info("Offline mode! Volume cover not available in cache.");
            return Err("Volume cover not available offline".into());
        }

        let url = format!(
            "http://{}/api/image/volume-cover?volumeId={}&apiKey={}",
            self.ip, volume_id, self.api_key
        );
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Accept", "image/*")
            .send()
            .await?;

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let body = response.bytes().await?;

        let (out_bytes, ext) = if let Some(jpg) = optimize_cover_to_jpeg_bytes(&body) {
            (jpg, "jpg")
        } else if content_type.contains("jpeg") {
            (body.to_vec(), "jpg")
        } else if content_type.contains("png") {
            (body.to_vec(), "png")
        } else {
            (body.to_vec(), "bin")
        };

        let hash = generate_hash_from_now();
        let filename = cache_folder_path().join(format!("{}.{}", hash, ext));
        fs::write(&filename, out_bytes)?;

        let volume_cover = VolumeCover {
            volume_id: *volume_id,
            file: filename.to_string_lossy().into_owned(),
        };
        self.db.add_volume_cover(&volume_cover)?;

        Ok(volume_cover)
    }
    // -------------------------------------------------------------------------    
    // Picture methods
    pub async fn get_picture(&self, chapter_id: &i32, page: &i32) -> Result<String, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            let url = format!("http://{}/api/reader/image?chapterId={}&apiKey={}&page={}", self.ip, chapter_id, self.api_key, page);
            let client = reqwest::Client::new();
            let response = client.get(url)
                .header("Content-Type", "image/png")
                .send()
                .await?;
            let body = response.bytes().await?;
            let hash = generate_hash_from_now();
            let cache_folder = get_datadir().join("manga4deck-cache").join("cache");    
            let filename = cache_folder.join(format!("{}.png", hash));
            fs::write(&filename, body)?;

            let picture = MangaPicture {
                chapter_id: *chapter_id,
                page: *page,
                file: filename.to_string_lossy().into_owned(),
            };
            self.db.add_picture(&picture)?;
        }

        let picture = self.db.get_picture(chapter_id, page)?;
        Ok(picture)
    }
    // -------------------------------------------------------------------------
    // Read Progress methods
    pub async fn save_progress(&self, progress: &ReadProgress) -> Result<(), Box<dyn std::error::Error>> {
        if !self.offline_mode {
            // Online mode: send to remote server
            let url = format!("http://{}/api/reader/progress", self.ip);
            let client = reqwest::Client::new();
            let _ = client.post(url)
                .header("Authorization", format!("Bearer {}", self.token))
                .json(&json!({
                    "seriesId": progress.series_id,
                    "volumeId": progress.volume_id,
                    "chapterId": progress.chapter_id,
                    "pageNum": progress.page,
                }))
                .send()
                .await?;
        } else {
            // Offline mode: save to local database
            info(&format!("Saving progress offline: series_id={}, volume_id={}, chapter_id={}, page={}", 
                progress.series_id, progress.volume_id, progress.chapter_id, progress.page));
            self.db.add_read_progress(progress)?;            
            // Update volume read pages in offline mode
            if let Ok(Some(mut volume)) = self.db.get_volume_by_id(progress.volume_id) {
                // Update the read pages count for this volume
                volume.read = progress.page;
                self.db.add_volume(&volume)?;
                info(&format!("Updated volume {} read pages to {}", volume.id, volume.read));
            }
        }
        Ok(())
    }

    pub async fn upload_progress(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.offline_mode {
            info("Cannot upload progress: currently in offline mode");
            return Ok(());
        }

        info("Uploading offline progress to server...");
        let all_progress = self.db.get_all_read_progress()?;
        let progress_count = all_progress.len();
        
        for progress in &all_progress {
            let url = format!("http://{}/api/reader/progress", self.ip);
            let client = reqwest::Client::new();
            let response = client.post(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .json(&json!({
                    "seriesId": progress.series_id,
                    "volumeId": progress.volume_id,
                    "chapterId": progress.chapter_id,
                    "pageNum": progress.page,
                }))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        info(&format!("Successfully uploaded progress for series_id={}, page={}", 
                            progress.series_id, progress.page));
                    } else {
                        info(&format!("Failed to upload progress for series_id={}, page={}: status {}", 
                            progress.series_id, progress.page, resp.status()));
                    }
                }
                Err(e) => {
                    info(&format!("Error uploading progress for series_id={}, page={}: {}", 
                        progress.series_id, progress.page, e));
                }
            }
        }

        // Clear local progress after successful upload
        if progress_count > 0 {
            info(&format!("Clearing {} offline progress entries", progress_count));
            self.db.clear_read_progress()?;
        }

        Ok(())
    }

    // Background version that doesn't require &self, used for spawning tasks
    async fn upload_progress_background(
        db: Database,
        ip: String,
        token: String,
        ws_sender: Option<Arc<broadcast::Sender<serde_json::Value>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Send start message via WebSocket
        if let Some(sender) = &ws_sender {
            let start_msg = serde_json_json!({
                "event": "progress_upload_start",
                "message": "Starting to upload offline progress...",
                "data": null
            });
            let _ = sender.send(start_msg);
        }

        info("Uploading offline progress to server in background thread...");
        let all_progress = db.get_all_read_progress()?;
        let progress_count = all_progress.len();
        
        if progress_count == 0 {
            info("No offline progress to upload");
            // Send end message even if no progress
            if let Some(sender) = &ws_sender {
                let end_msg = serde_json_json!({
                    "event": "progress_upload_end",
                    "message": "No offline progress to upload",
                    "data": {
                        "total": 0,
                        "succeeded": 0,
                        "failed": 0
                    }
                });
                let _ = sender.send(end_msg);
            }
            return Ok(());
        }

        info(&format!("Found {} progress entries to upload", progress_count));
        
        let mut success_count = 0;
        let mut fail_count = 0;
        
        for progress in &all_progress {
            let url = format!("http://{}/api/reader/progress", ip);
            let client = reqwest::Client::new();
            let response = client.post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .json(&json!({
                    "seriesId": progress.series_id,
                    "volumeId": progress.volume_id,
                    "chapterId": progress.chapter_id,
                    "pageNum": progress.page,
                }))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        success_count += 1;
                        if success_count % 10 == 0 {
                            info(&format!("Uploaded {}/{} progress entries...", success_count, progress_count));
                        }
                    } else {
                        fail_count += 1;
                        info(&format!("Failed to upload progress for series_id={}, page={}: status {}", 
                            progress.series_id, progress.page, resp.status()));
                    }
                }
                Err(e) => {
                    fail_count += 1;
                    info(&format!("Error uploading progress for series_id={}, page={}: {}", 
                        progress.series_id, progress.page, e));
                }
            }
        }

        info(&format!("Progress upload complete: {} succeeded, {} failed", success_count, fail_count));

        // Send end message via WebSocket
        if let Some(sender) = &ws_sender {
            let end_msg = serde_json_json!({
                "event": "progress_upload_end",
                "message": format!("Progress upload complete: {} succeeded, {} failed", success_count, fail_count),
                "data": {
                    "total": progress_count,
                    "succeeded": success_count,
                    "failed": fail_count
                }
            });
            let _ = sender.send(end_msg);
        }

        // Clear local progress after upload attempt (even if some failed)
        // This prevents re-uploading the same entries on next connection
        if success_count > 0 {
            info(&format!("Clearing {} successfully uploaded progress entries", success_count));
            // Note: We clear all entries, not just successful ones, to avoid infinite retry loops
            // Failed entries will be lost, but new progress will continue to be saved
            db.clear_read_progress()?;
        }

        Ok(())
    } 

    pub async fn set_volume_as_read(&self, series_id: &i32, volume_id: &i32) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("http://{}/api/reader/mark-volume-read", self.ip);
        let client = reqwest::Client::new();
        let _ = client.post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&json!({
                "seriesId": series_id,
                "volumeId": volume_id,
            }))
            .send()
            .await?;
        Ok(())
    }

    pub async fn set_volume_as_unread(&self, series_id: &i32, volume_id: &i32) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("http://{}/api/reader/mark-volume-unread", self.ip);
        let client = reqwest::Client::new();
        let _ = client.post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&json!({
                "seriesId": series_id,
                "volumeId": volume_id,
            }))
            .send()
            .await?;    
        Ok(())
    }

    // Check if all volumes in a series are cached
    pub fn is_series_cached(&self, series_id: i32) -> bool {
        let volumes = self.db.get_volumes(&series_id).unwrap_or_default();
        for volume in volumes {
            if !self.is_volume_cached(volume.id) {
                return false;
            }
        }
        true
    }

    // Check if all pages in a volume are cached
    pub fn is_volume_cached(&self, volume_id: i32) -> bool {
        if let Some((chapter_id, pages)) = self.db.get_volume_chapter_and_pages(volume_id) {
            for page in 1..=pages {
                if !self.db.is_picture_cached(chapter_id, page) {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    // Add a series to the caching queue and start the thread if not running
    pub fn cache_serie(&self, series_id: i32) {
        {
            let mut queue = self.caching_queue.lock().unwrap();
            if !queue.contains(&series_id) {
                queue.push_back(series_id);
            }
        }
        let mut handle_guard = self.caching_thread_handle.lock().unwrap();
        if handle_guard.is_none() {
            let db = self.db.clone();
            let queue = self.caching_queue.clone();
            let ip = self.ip.clone();
            let api_key = self.api_key.clone();
            let token = self.token.clone();
            let ws_sender = self.ws_sender.clone();
            *handle_guard = Some(thread::spawn(move || {
                cache_serie_threaded(db, queue, ip, api_key, token, ws_sender);
            }));
        }
    }

    // Remove cached volumes for a series and remove from caching queue
    pub fn remove_series_cache(&self, series_id: i32) -> Result<(), Box<dyn std::error::Error>> {
        // Check if series has cached volumes
        if !self.db.has_cached_volumes(series_id) {
            return Ok(()); // Nothing to remove
        }

        // Get all picture files for this series
        let picture_files = self.db.get_series_picture_files(series_id)?;
        
        // Delete files from disk
        for file_path in &picture_files {
            if let Err(e) = fs::remove_file(file_path) {
                info(&format!("Failed to delete cache file {}: {}", file_path, e));
            }
        }

        // Delete from database
        self.db.delete_series_cache(series_id)?;

        // Remove from caching queue if present
        {
            let mut queue = self.caching_queue.lock().unwrap();
            queue.retain(|&id| id != series_id);
        }

        info(&format!("Removed cache for series {} ({} files)", series_id, picture_files.len()));
        Ok(())
    }
}

// Background thread for caching series
fn cache_serie_threaded(
    db: Database, 
    queue: Arc<StdMutex<VecDeque<i32>>>, 
    ip: String, 
    api_key: String, 
    _token: String,
    ws_sender: Option<Arc<broadcast::Sender<serde_json::Value>>>,
) {
    use ureq;
    use std::io::Read;
    loop {
        let series_id = {
            let mut q = queue.lock().unwrap();
            q.pop_front()
        };
        if let Some(series_id) = series_id {
            // Send caching start notification
            if let Some(sender) = &ws_sender {
                let start_msg = serde_json_json!({
                    "event": "caching_start",
                    "message": format!("Starting to cache series {}", series_id),
                    "data": {
                        "series_id": series_id
                    }
                });
                let _ = sender.send(start_msg);
            }

            let mut volumes = db.get_volumes(&series_id).unwrap_or_default();
            // Sort volumes by the number in their title (e.g., 'Volume 20' < 'Volume 21')
            volumes.sort_by_key(|v| {
                let digits: String = v.title.chars().filter(|c| c.is_digit(10)).collect();
                digits.parse::<i32>().unwrap_or(0)
            });
            
            let total_volumes = volumes.len();
            let mut cached_volumes = 0;
            
            for volume in volumes {
                if volume.pages > 0 && volume.read >= volume.pages {
                    continue; // Skip fully read volumes
                }
                info(&format!("Start caching volume {} (title: {}) in series {}", volume.id, volume.title, series_id));
                if let Some((chapter_id, pages)) = db.get_volume_chapter_and_pages(volume.id) {
                    for page in 1..=pages {
                        if !db.is_picture_cached(chapter_id, page) {
                            let url = format!("http://{}/api/reader/image?chapterId={}&apiKey={}&page={}", ip, chapter_id, api_key, page);
                            let resp = ureq::get(&url).call();
                            if let Ok(response) = resp {
                                let mut bytes = Vec::new();
                                response.into_reader().read_to_end(&mut bytes).unwrap();
                                let hash = generate_hash_from_now();
                                let cache_folder = get_datadir().join("manga4deck-cache").join("cache");    
                                let filename = cache_folder.join(format!("{}.png", hash));
                                fs::write(&filename, &bytes).unwrap();
                                let picture = crate::kavita::MangaPicture {
                                    chapter_id,
                                    page,
                                    file: filename.to_string_lossy().into_owned(),
                                };
                                db.add_picture(&picture).unwrap();
                            }
                        }
                    }
                }
                info(&format!("Finished caching volume {} (title: {}) in series {}", volume.id, volume.title, series_id));
                
                // Send volume cached notification
                cached_volumes += 1;
                if let Some(sender) = &ws_sender {
                    let volume_msg = serde_json_json!({
                        "event": "volume_cached",
                        "message": format!("Cached volume: {}", volume.title),
                        "data": {
                            "series_id": series_id,
                            "volume_id": volume.id,
                            "volume_title": volume.title,
                            "progress": {
                                "current": cached_volumes,
                                "total": total_volumes
                            }
                        }
                    });
                    let _ = sender.send(volume_msg);
                }
            }
            
            // Send caching end notification
            if let Some(sender) = &ws_sender {
                let end_msg = serde_json_json!({
                    "event": "caching_end",
                    "message": format!("Finished caching series {}", series_id),
                    "data": {
                        "series_id": series_id,
                        "volumes_cached": cached_volumes,
                        "total_volumes": total_volumes
                    }
                });
                let _ = sender.send(end_msg);
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}
