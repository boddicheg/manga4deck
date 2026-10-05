use crate::logger::info;
use md5::{Digest, Md5};
use reqwest;
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use tokio::time::{timeout, Duration};

use crate::storage::Database;

use serde_json::json as serde_json_json;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use tokio::sync::broadcast;

/// Request rasterized pages for PDFs; Kavita uses the same endpoint for archives/images.
fn reader_image_url(
    ip: &str,
    api_key: &str,
    chapter_id: i32,
    page: i32,
) -> Result<reqwest::Url, Box<dyn std::error::Error>> {
    if chapter_id <= 0 || page < 0 {
        return Err("Invalid chapter or page index".into());
    }
    let mut url = reqwest::Url::parse(&format!("http://{ip}/api/reader/image"))?;
    url.query_pairs_mut()
        .append_pair("chapterId", &chapter_id.to_string())
        .append_pair("apiKey", api_key)
        .append_pair("page", &page.to_string())
        .append_pair("extractPdf", "true");
    Ok(url)
}

fn reader_image_extension(bytes: &[u8]) -> Result<&'static str, Box<dyn std::error::Error>> {
    let format =
        image::guess_format(bytes).map_err(|_| "Kavita did not return a rendered page image")?;
    let extension = match format {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::WebP => "webp",
        _ => return Err("Unsupported reader image format".into()),
    };
    // Check the image header without decoding a whole full-resolution page.
    image::ImageReader::with_format(std::io::Cursor::new(bytes), format).into_dimensions()?;
    Ok(extension)
}

fn cached_reader_picture(db: &Database, chapter_id: i32, page: i32) -> Option<String> {
    let file = db.get_picture(&chapter_id, &page).ok()?;
    let reader = image::ImageReader::open(&file)
        .ok()?
        .with_guessed_format()
        .ok()?;
    reader.into_dimensions().ok()?;
    Some(file)
}

fn cache_reader_picture(
    db: &Database,
    folder: &Path,
    chapter_id: i32,
    page: i32,
    bytes: &[u8],
) -> Result<String, Box<dyn std::error::Error>> {
    let extension = reader_image_extension(bytes)?;
    let filename = folder.join(format!("{}.{extension}", generate_hash_from_now()));
    fs::write(&filename, bytes)?;
    let file = filename.to_string_lossy().into_owned();
    let previous = db.get_picture(&chapter_id, &page).ok();
    if let Err(err) = db.add_picture(&MangaPicture {
        chapter_id,
        page,
        file: file.clone(),
    }) {
        let _ = fs::remove_file(filename);
        return Err(err);
    }
    if let Some(previous) = previous.filter(|previous| previous != &file) {
        let _ = fs::remove_file(previous);
    }
    Ok(file)
}

fn is_pdf_chapter(chapter: &serde_json::Value) -> bool {
    let is_pdf = |format: &serde_json::Value| {
        format.as_i64() == Some(4)
            || format
                .as_str()
                .is_some_and(|value| value.eq_ignore_ascii_case("pdf"))
    };
    is_pdf(&chapter["format"])
        || chapter["files"]
            .as_array()
            .is_some_and(|files| files.iter().any(|file| is_pdf(&file["format"])))
}

fn pdf_volume(series_id: i32, chapter: &serde_json::Value, parent_id: i32) -> Option<Volume> {
    let chapter_id = chapter["id"].as_i64()? as i32;
    let volume_id = chapter["volumeId"].as_i64().unwrap_or(parent_id as i64) as i32;
    if chapter_id <= 0 || volume_id <= 0 {
        return None;
    }
    let title = ["titleName", "title", "range"]
        .iter()
        .find_map(|key| chapter[*key].as_str().filter(|title| !title.is_empty()))
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Book {chapter_id}"));
    Some(Volume {
        // Negative local IDs distinguish individual books from real Kavita volumes.
        // Keep the real parent volume ID for progress uploads.
        id: -chapter_id,
        series_id,
        chapter_id,
        volume_id,
        title,
        read: chapter["pagesRead"].as_i64().unwrap_or(0) as i32,
        pages: chapter["pages"].as_i64().unwrap_or(0) as i32,
        is_cached: false,
    })
}

fn reader_volumes(series_id: i32, data: &serde_json::Value) -> Vec<Volume> {
    let array = |key: &str| data[key].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let top_level_chapter_id = array("chapters")
        .first()
        .and_then(|c| c["id"].as_i64())
        .unwrap_or(0) as i32;
    for v in array("volumes") {
        let volume_id = v["id"].as_i64().unwrap_or(0) as i32;
        let chapters = v["chapters"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        // A volume can group several PDF books. Give each its own page count and progress.
        if !chapters.is_empty() && chapters.iter().all(is_pdf_chapter) {
            for chapter in chapters {
                if let Some(book) = pdf_volume(series_id, chapter, volume_id) {
                    if seen.insert(book.chapter_id) {
                        result.push(book);
                    }
                }
            }
            continue;
        }
        let chapter_id = chapters
            .first()
            .and_then(|c| c["id"].as_i64())
            .unwrap_or(top_level_chapter_id as i64) as i32;
        if chapter_id <= 0 || volume_id <= 0 {
            continue;
        }
        seen.extend(
            chapters
                .iter()
                .filter_map(|c| c["id"].as_i64().map(|id| id as i32)),
        );
        seen.insert(chapter_id);
        result.push(Volume {
            id: volume_id,
            series_id,
            chapter_id,
            volume_id,
            title: v["name"].as_str().unwrap_or("").to_string(),
            read: v["pagesRead"].as_i64().unwrap_or(0) as i32,
            pages: v["pages"].as_i64().unwrap_or(0) as i32,
            is_cached: false,
        });
    }
    for key in ["chapters", "specials", "storylineChapters"] {
        for chapter in array(key).iter().filter(|chapter| is_pdf_chapter(chapter)) {
            if let Some(book) = pdf_volume(series_id, chapter, 0) {
                if seen.insert(book.chapter_id) {
                    result.push(book);
                }
            }
        }
    }
    result
}

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

const COVER_THUMB_WIDTH: u32 = 150;
const COVER_THUMB_HEIGHT: u32 = 200;
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
    let thumb = img.resize_to_fill(
        COVER_THUMB_WIDTH,
        COVER_THUMB_HEIGHT,
        FilterType::CatmullRom,
    );
    let rgb = thumb.to_rgb8();
    let (w, h) = rgb.dimensions();

    let mut out: Vec<u8> = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut out, COVER_JPEG_QUALITY);
    encoder.encode(&rgb, w, h, ColorType::Rgb8.into()).ok()?;
    Some(out)
}

fn is_optimized_cover_file(path: &str) -> bool {
    let is_jpeg = path.ends_with(".jpg") || path.ends_with(".jpeg");
    if !is_jpeg {
        return false;
    }

    image::image_dimensions(path)
        .map(|(width, height)| width <= COVER_THUMB_WIDTH && height <= COVER_THUMB_HEIGHT)
        .unwrap_or(false)
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
        .as_nanos();
    let mut hasher = Md5::new();
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    hasher.update(format!("{now}-{sequence}"));
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
    pub is_cached: bool,
}

#[derive(Clone)]
pub struct Kavita {
    pub db: Database,
    pub token: String,
    pub logged_as: String,
    pub kavita_version: Option<String>,
    pub offline_mode: bool,
    pub ip: String,
    pub api_key: String,
    // --- Caching fields ---
    pub caching_queue: Arc<StdMutex<VecDeque<i32>>>, // series_id queue
    pub caching_current_series: Arc<StdMutex<Option<i32>>>,
    pub caching_cancelled_series: Arc<StdMutex<HashSet<i32>>>,
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
    /// Kavita volume ID, or a negative chapter ID for an individual PDF book.
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
    /// Zero-based page index (same as `/api/Reader/image?page=`).
    pub page: i32,
}

/// Kavita stores `pageNum` as `PagesRead`: 0 = none, and a chapter is complete when `pageNum >= chapter.Pages`.
/// Reader images use zero-based indices, so we add one when posting progress.
#[inline]
fn kavita_progress_page_num(zero_based_page: i32) -> i32 {
    zero_based_page.saturating_add(1)
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
            kavita_version: None,
            offline_mode: true,
            ip: DEFAULT_IP.to_string(),
            api_key: DEFAULT_API_KEY.to_string(),
            caching_queue: Arc::new(StdMutex::new(VecDeque::new())),
            caching_current_series: Arc::new(StdMutex::new(None)),
            caching_cancelled_series: Arc::new(StdMutex::new(HashSet::new())),
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

    pub fn get_series_library_id(&self, series_id: i32) -> Option<i32> {
        self.db.get_series_library_id(series_id).ok().flatten()
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
                })),
            );
        } else {
            self.send_websocket_message(
                "connection_status",
                &format!("Connected to Kavita server as {}", logged_as),
                Some(serde_json_json!({
                    "mode": "online",
                    "connected": true,
                    "username": logged_as
                })),
            );
        }
    }

    pub async fn reconnect_with_creds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info(&format!("reconnect_with_creds + "));
        // Get settings with proper error handling
        self.ip = self
            .db
            .get_setting("ip")
            .expect("Failed to get IP setting")
            .unwrap_or_else(|| DEFAULT_IP.to_string());
        let username = self
            .db
            .get_setting("username")
            .expect("Failed to get username setting")
            .unwrap_or_else(|| DEFAULT_USERNAME.to_string());
        let password = self
            .db
            .get_setting("password")
            .expect("Failed to get password setting")
            .unwrap_or_else(|| DEFAULT_PASSWORD.to_string());
        let api_key = self
            .db
            .get_setting("api_key")
            .expect("Failed to get API key setting")
            .unwrap_or_else(|| DEFAULT_API_KEY.to_string());

        info(&format!("IP: {}", self.ip));
        info(&format!("Username: {}", username));
        info(&format!(
            "Password: {}",
            if password.is_empty() {
                "<empty>"
            } else {
                "<set>"
            }
        ));
        info(&format!(
            "API Key: {}",
            if api_key.is_empty() {
                "<empty>"
            } else {
                "<set>"
            }
        ));

        // self.ip = "192.168.1.100:5001".to_string();

        // make http request to get token
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        let fut = client
            .post(format!("http://{}/api/Account/login", self.ip))
            .json(&json!({
                "username": username,
                "password": password,
                "apiKey": api_key
            }))
            .send();
        match timeout(Duration::from_secs(5), fut).await {
            Ok(Ok(resp)) => {
                let body = resp.text().await.unwrap();
                let data: serde_json::Value = serde_json::from_str(&body)?;
                self.token = data["token"].as_str().unwrap_or("").to_string();
                self.logged_as = data["username"].as_str().unwrap_or("").to_string();
                self.api_key = data["apiKey"].as_str().unwrap_or("").to_string();
                self.kavita_version = data["kavitaVersion"].as_str().map(|s| s.to_string());
                self.offline_mode = false;
                info(&format!("Logged as: {}", self.logged_as));
                if let Some(v) = &self.kavita_version {
                    info(&format!("Kavita version: {}", v));
                }
                // Send connection status notification
                self.send_connection_status(false, &self.logged_as);
            }
            Ok(Err(e)) => {
                self.offline_mode = true;
                self.logged_as = "".to_string();
                self.token = "".to_string();
                self.kavita_version = None;
                info(&format!("Failed to get token. Now in offline mode"));
                // Send connection status notification
                self.send_connection_status(true, "");
                return Err(Box::new(e));
            }
            Err(_) => {
                self.offline_mode = true;
                self.logged_as = "".to_string();
                self.token = "".to_string();
                self.kavita_version = None;
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
            let _ = client
                .post(url)
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
        let response = client
            .get(format!("http://{}/api/library/libraries", self.ip))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if response.status().is_success() {
            let body = response.text().await?;
            // info(&format!("Libraries: {}", body));
            let data: serde_json::Value = serde_json::from_str(&body)?;
            let libraries: Vec<Library> = data
                .as_array()
                .unwrap()
                .iter()
                .map(|v| Library {
                    id: v["id"].as_i64().unwrap_or(0) as i32,
                    title: v["name"].as_str().unwrap_or("").to_string(),
                })
                .collect();
            for library in libraries {
                self.db.add_library(&library)?;
            }
        } else {
            info(&format!(
                "Failed to get libraries: response status {}",
                response.status()
            ));
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
            let response = client
                .post(format!("http://{}/api/series/v2", self.ip))
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
            let series: Vec<Series> = data
                .as_array()
                .unwrap()
                .iter()
                .map(|v| Series {
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
                    is_cached: false,
                })
                .collect();
            for series in series {
                self.db.add_series(&series)?;
            }
        }

        Ok(())
    }

    pub async fn get_series(
        &self,
        library_id: &i32,
    ) -> Result<Vec<Series>, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            self.pull_series(library_id).await?;
        }
        let mut series = self.db.get_series(library_id)?;
        for item in &mut series {
            item.is_cached = self.is_series_cached(item.id);
        }
        // return only cached series
        if self.offline_mode {
            series = series
                .into_iter()
                .filter(|s| self.is_series_cached(s.id))
                .collect();
        }
        // sort series by title and return sorted series
        series.sort_by_key(|s| s.title.clone());
        Ok(series)
    }

    pub async fn get_series_cover(
        &self,
        series_id: &i32,
    ) -> Result<SeriesCover, Box<dyn std::error::Error>> {
        // Prefer existing cached cover, but migrate old large files to JPEG thumbnails.
        if let Ok(existing) = self.db.get_series_cover(series_id) {
            if Path::new(&existing.file).exists() {
                if is_optimized_cover_file(&existing.file) {
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
        let response = client.get(url).header("Accept", "image/*").send().await?;

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

        let volumes = reader_volumes(*series_id, &data);
        if volumes.is_empty() {
            info(&format!(
                "pull_volumes(series_id={series_id}) returned no readable volumes"
            ));
        }
        self.db.replace_volumes(*series_id, &volumes)?;
        Ok(())
    }

    pub async fn get_volumes(
        &self,
        series_id: &i32,
    ) -> Result<Vec<Volume>, Box<dyn std::error::Error>> {
        let mut last_pull_err: Option<String> = None;

        if !self.offline_mode {
            if let Err(e) = self.pull_volumes(series_id).await {
                last_pull_err = Some(e.to_string());
                info(&format!(
                    "Failed to pull volumes for series {}: {}",
                    series_id, e
                ));
            }
        }

        let mut volumes = self.db.get_volumes(series_id)?;

        // If the remote fetch failed AND we have no local data, retry once before returning an error.
        if !self.offline_mode && volumes.is_empty() && last_pull_err.is_some() {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if let Err(e) = self.pull_volumes(series_id).await {
                last_pull_err = Some(e.to_string());
                info(&format!(
                    "Retry pull_volumes failed for series {}: {}",
                    series_id, e
                ));
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
        volumes.sort_by_key(|v| {
            v.title
                .clone()
                .replace(|c: char| !c.is_digit(10), "")
                .parse::<i32>()
                .unwrap_or(0)
        });
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

    pub fn get_cached_volumes(
        &self,
        series_id: &i32,
    ) -> Result<Vec<Volume>, Box<dyn std::error::Error>> {
        let mut volumes = self.db.get_volumes(series_id)?;
        volumes.sort_by_key(|v| {
            v.title
                .clone()
                .replace(|c: char| !c.is_digit(10), "")
                .parse::<i32>()
                .unwrap_or(0)
        });
        for v in &mut volumes {
            v.is_cached = self.is_volume_cached(v.id);
        }
        Ok(volumes)
    }

    pub async fn get_volume_cover(
        &self,
        volume_id: &i32,
    ) -> Result<VolumeCover, Box<dyn std::error::Error>> {
        // Prefer existing cached cover, but migrate old large files to JPEG thumbnails.
        if let Ok(existing) = self.db.get_volume_cover(volume_id) {
            if Path::new(&existing.file).exists() {
                if is_optimized_cover_file(&existing.file) {
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

        let url = if *volume_id < 0 {
            let book = self
                .db
                .get_volume_by_id(*volume_id)?
                .ok_or("Unknown PDF book")?;
            format!(
                "http://{}/api/image/chapter-cover?chapterId={}&apiKey={}",
                self.ip, book.chapter_id, self.api_key
            )
        } else {
            format!(
                "http://{}/api/image/volume-cover?volumeId={}&apiKey={}",
                self.ip, volume_id, self.api_key
            )
        };
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "image/*")
            .send()
            .await?
            .error_for_status()
            .map_err(reqwest::Error::without_url)?;

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
    pub async fn get_picture(
        &self,
        chapter_id: &i32,
        page: &i32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.get_picture_in_folder(*chapter_id, *page, &cache_folder_path())
            .await
    }

    async fn get_picture_in_folder(
        &self,
        chapter_id: i32,
        page: i32,
        folder: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if chapter_id <= 0 || page < 0 {
            return Err("Invalid chapter or page index".into());
        }
        if let Some(file) = cached_reader_picture(&self.db, chapter_id, page) {
            return Ok(file);
        }
        if self.offline_mode {
            return Err("Page is not available in the offline cache".into());
        }
        let url = reader_image_url(&self.ip, &self.api_key, chapter_id, page)?;
        let response = reqwest::Client::new()
            .get(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "image/*")
            .send()
            .await
            .map_err(reqwest::Error::without_url)?
            .error_for_status()
            .map_err(reqwest::Error::without_url)?;
        let body = response.bytes().await?;
        cache_reader_picture(&self.db, folder, chapter_id, page, &body)
    }
    // -------------------------------------------------------------------------
    // Read Progress methods
    pub async fn save_progress(
        &self,
        progress: &ReadProgress,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let page_num = kavita_progress_page_num(progress.page);
        if !self.offline_mode {
            // Online mode: send to remote server
            let url = format!("http://{}/api/reader/progress", self.ip);
            let client = reqwest::Client::new();
            let resp = client
                .post(url)
                .header("Authorization", format!("Bearer {}", self.token))
                .json(&json!({
                    "libraryId": progress.library_id,
                    "seriesId": progress.series_id,
                    "volumeId": progress.volume_id,
                    "chapterId": progress.chapter_id,
                    "pageNum": page_num
                }))
                .send()
                .await?;
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            if status.as_u16() != 200 {
                info(&format!(
                    "api/reader/progress response (save_progress): series_id={} volume_id={} chapter_id={} page_idx={} pageNum={} status={} body={}",
                    progress.series_id, progress.volume_id, progress.chapter_id, progress.page, page_num, status, body
                ));
            }
        } else {
            // Offline mode: save to local database
            info(&format!("Saving progress offline: series_id={}, volume_id={}, chapter_id={}, page_idx={}, pagesRead={}", 
                progress.series_id, progress.volume_id, progress.chapter_id, progress.page, page_num));
            self.db.add_read_progress(progress)?;
            // Update volume read pages in offline mode
            if let Some(mut volume) =
                self.db
                    .get_volumes(&progress.series_id)?
                    .into_iter()
                    .find(|volume| {
                        volume.chapter_id == progress.chapter_id
                            && volume.volume_id == progress.volume_id
                    })
            {
                // Align with Kavita `pagesRead` (same as online `pageNum`)
                volume.read = if volume.pages > 0 {
                    page_num.min(volume.pages)
                } else {
                    page_num
                };
                self.db.add_volume(&volume)?;
                info(&format!(
                    "Updated volume {} read pages to {}",
                    volume.id, volume.read
                ));
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
            let page_num = kavita_progress_page_num(progress.page);
            let url = format!("http://{}/api/reader/progress", self.ip);
            let client = reqwest::Client::new();
            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .json(&json!({
                    "libraryId": progress.library_id,
                    "seriesId": progress.series_id,
                    "volumeId": progress.volume_id,
                    "chapterId": progress.chapter_id,
                    "pageNum": page_num
                }))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    if status.as_u16() != 200 {
                        info(&format!(
                            "api/reader/progress response (upload_progress): series_id={} page_idx={} pageNum={} status={} body={}",
                            progress.series_id, progress.page, page_num, status, body
                        ));
                    }
                    if status.is_success() {
                        info(&format!(
                            "Successfully uploaded progress for series_id={}, page={}",
                            progress.series_id, progress.page
                        ));
                    } else {
                        info(&format!(
                            "Failed to upload progress for series_id={}, page={}: status {}",
                            progress.series_id, progress.page, status
                        ));
                    }
                }
                Err(e) => {
                    info(&format!(
                        "Error uploading progress for series_id={}, page={}: {}",
                        progress.series_id, progress.page, e
                    ));
                }
            }
        }

        // Clear local progress after successful upload
        if progress_count > 0 {
            info(&format!(
                "Clearing {} offline progress entries",
                progress_count
            ));
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

        info(&format!(
            "Found {} progress entries to upload",
            progress_count
        ));

        let mut success_count = 0;
        let mut fail_count = 0;

        for progress in &all_progress {
            let page_num = kavita_progress_page_num(progress.page);
            let url = format!("http://{}/api/reader/progress", ip);
            let client = reqwest::Client::new();
            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .json(&json!({
                    "libraryId": progress.library_id,
                    "seriesId": progress.series_id,
                    "volumeId": progress.volume_id,
                    "chapterId": progress.chapter_id,
                    "pageNum": page_num
                }))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    if status.as_u16() != 200 {
                        info(&format!(
                            "api/reader/progress response (upload_progress_background): series_id={} page_idx={} pageNum={} status={} body={}",
                            progress.series_id, progress.page, page_num, status, body
                        ));
                    }
                    if status.is_success() {
                        success_count += 1;
                        if success_count % 10 == 0 {
                            info(&format!(
                                "Uploaded {}/{} progress entries...",
                                success_count, progress_count
                            ));
                        }
                    } else {
                        fail_count += 1;
                        info(&format!(
                            "Failed to upload progress for series_id={}, page={}: status {}",
                            progress.series_id, progress.page, status
                        ));
                    }
                }
                Err(e) => {
                    fail_count += 1;
                    info(&format!(
                        "Error uploading progress for series_id={}, page={}: {}",
                        progress.series_id, progress.page, e
                    ));
                }
            }
        }

        info(&format!(
            "Progress upload complete: {} succeeded, {} failed",
            success_count, fail_count
        ));

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
            info(&format!(
                "Clearing {} successfully uploaded progress entries",
                success_count
            ));
            // Note: We clear all entries, not just successful ones, to avoid infinite retry loops
            // Failed entries will be lost, but new progress will continue to be saved
            db.clear_read_progress()?;
        }

        Ok(())
    }

    pub async fn set_volume_as_read(
        &self,
        series_id: &i32,
        volume_id: &i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_reader_item_read(*series_id, *volume_id, true)
            .await
    }

    pub async fn set_volume_as_unread(
        &self,
        series_id: &i32,
        volume_id: &i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_reader_item_read(*series_id, *volume_id, false)
            .await
    }

    async fn set_reader_item_read(
        &self,
        series_id: i32,
        local_id: i32,
        read: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let action = if read { "read" } else { "unread" };
        let (endpoint, payload) = if local_id < 0 {
            let book = self
                .db
                .get_volume_by_id(local_id)?
                .ok_or("Unknown PDF book")?;
            if book.series_id != series_id {
                return Err("Book does not belong to this series".into());
            }
            (
                format!("mark-multiple-{action}"),
                json!({
                    "seriesId": series_id, "volumeIds": [], "chapterIds": [book.chapter_id]
                }),
            )
        } else {
            (
                format!("mark-volume-{action}"),
                json!({"seriesId": series_id, "volumeId": local_id}),
            )
        };
        reqwest::Client::new()
            .post(format!("http://{}/api/reader/{endpoint}", self.ip))
            .bearer_auth(&self.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(reqwest::Error::without_url)?;
        Ok(())
    }

    // Check if every unread volume in a series is cached. The cache worker
    // intentionally skips completed volumes.
    pub fn is_series_cached(&self, series_id: i32) -> bool {
        self.db.is_series_fully_cached(series_id)
    }

    // Check if all pages in a volume are cached
    pub fn is_volume_cached(&self, volume_id: i32) -> bool {
        self.db.is_volume_fully_cached(volume_id)
    }

    // Add a series to the caching queue and start the thread if not running
    pub fn cache_serie(&self, series_id: i32) {
        {
            let mut cancelled = self.caching_cancelled_series.lock().unwrap();
            cancelled.remove(&series_id);
        }
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
            let current_series = self.caching_current_series.clone();
            let cancelled_series = self.caching_cancelled_series.clone();
            let ip = self.ip.clone();
            let api_key = self.api_key.clone();
            let token = self.token.clone();
            let ws_sender = self.ws_sender.clone();
            *handle_guard = Some(thread::spawn(move || {
                cache_serie_threaded(
                    db,
                    queue,
                    current_series,
                    cancelled_series,
                    ip,
                    api_key,
                    token,
                    ws_sender,
                );
            }));
        }
    }

    pub fn is_series_caching(&self, series_id: i32) -> bool {
        let is_current = self
            .caching_current_series
            .lock()
            .unwrap()
            .map(|current| current == series_id)
            .unwrap_or(false);
        if is_current {
            return true;
        }

        self.caching_queue.lock().unwrap().contains(&series_id)
    }

    pub fn stop_cache_serie(&self, series_id: i32) {
        {
            let mut queue = self.caching_queue.lock().unwrap();
            queue.retain(|&id| id != series_id);
        }
        {
            let mut cancelled = self.caching_cancelled_series.lock().unwrap();
            cancelled.insert(series_id);
        }
        info(&format!("Requested cache stop for series {}", series_id));
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
        {
            let mut cancelled = self.caching_cancelled_series.lock().unwrap();
            cancelled.insert(series_id);
        }

        info(&format!(
            "Removed cache for series {} ({} files)",
            series_id,
            picture_files.len()
        ));
        Ok(())
    }
}

// Background thread for caching series
fn cache_serie_threaded(
    db: Database,
    queue: Arc<StdMutex<VecDeque<i32>>>,
    current_series: Arc<StdMutex<Option<i32>>>,
    cancelled_series: Arc<StdMutex<HashSet<i32>>>,
    ip: String,
    api_key: String,
    token: String,
    ws_sender: Option<Arc<broadcast::Sender<serde_json::Value>>>,
) {
    use std::io::Read;
    use ureq;
    loop {
        let series_id = {
            let mut q = queue.lock().unwrap();
            q.pop_front()
        };
        if let Some(series_id) = series_id {
            {
                let mut current = current_series.lock().unwrap();
                *current = Some(series_id);
            }
            {
                let mut cancelled = cancelled_series.lock().unwrap();
                cancelled.remove(&series_id);
            }

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
            let mut cancelled = false;

            for volume in volumes {
                if cancelled_series.lock().unwrap().contains(&series_id) {
                    cancelled = true;
                    break;
                }
                if volume.pages > 0 && volume.read >= volume.pages {
                    continue; // Skip fully read volumes
                }
                info(&format!(
                    "Start caching volume {} (title: {}) in series {}",
                    volume.id, volume.title, series_id
                ));
                if let Some(sender) = &ws_sender {
                    let volume_start_msg = serde_json_json!({
                        "event": "volume_caching_start",
                        "message": format!(
                            "Start caching volume {} (title: {}) in series {}",
                            volume.id, volume.title, series_id
                        ),
                        "data": {
                            "series_id": series_id,
                            "volume_id": volume.id,
                            "volume_title": volume.title
                        }
                    });
                    let _ = sender.send(volume_start_msg);
                }
                if let Some((chapter_id, pages)) = db.get_volume_chapter_and_pages(volume.id) {
                    for page in 0..pages {
                        if cancelled_series.lock().unwrap().contains(&series_id) {
                            cancelled = true;
                            break;
                        }
                        if cached_reader_picture(&db, chapter_id, page).is_none() {
                            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                                let url = reader_image_url(&ip, &api_key, chapter_id, page)?;
                                let response = ureq::get(url.as_str())
                                    .set("Authorization", &format!("Bearer {token}"))
                                    .set("Accept", "image/*")
                                    .call()
                                    .map_err(|_| "Kavita page request failed")?;
                                let mut bytes = Vec::new();
                                response.into_reader().read_to_end(&mut bytes)?;
                                cache_reader_picture(
                                    &db,
                                    &cache_folder_path(),
                                    chapter_id,
                                    page,
                                    &bytes,
                                )?;
                                Ok(())
                            })();
                            if let Err(err) = result {
                                info(&format!(
                                    "Failed to cache chapter {chapter_id}, page {page}: {err}"
                                ));
                            }
                        }
                    }
                }
                if cancelled {
                    break;
                }
                if !db.is_volume_fully_cached(volume.id) {
                    info(&format!("Volume {} is only partially cached", volume.id));
                    continue;
                }
                info(&format!(
                    "Finished caching volume {} (title: {}) in series {}",
                    volume.id, volume.title, series_id
                ));

                // Send volume cached notification
                cached_volumes += 1;
                if let Some(sender) = &ws_sender {
                    let volume_msg = serde_json_json!({
                        "event": "volume_cached",
                        "message": format!(
                            "Finished caching volume {} (title: {}) in series {}",
                            volume.id, volume.title, series_id
                        ),
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

            {
                let mut current = current_series.lock().unwrap();
                if *current == Some(series_id) {
                    *current = None;
                }
            }
            {
                let mut cancelled_set = cancelled_series.lock().unwrap();
                cancelled_set.remove(&series_id);
            }

            // Send caching end notification
            if let Some(sender) = &ws_sender {
                let end_msg = serde_json_json!({
                    "event": if cancelled { "caching_cancelled" } else { "caching_end" },
                    "message": if cancelled {
                        format!("Stopped caching series {}", series_id)
                    } else {
                        format!("Finished caching whole series {}", series_id)
                    },
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCache(PathBuf);
    impl TestCache {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("manga4deck-test-{}", generate_hash_from_now()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestCache {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_kavita() -> Kavita {
        Kavita {
            db: Database::new(&":memory:".to_string()).unwrap(),
            token: "test-token".into(),
            logged_as: String::new(),
            kavita_version: None,
            offline_mode: false,
            ip: "127.0.0.1:1".into(),
            api_key: "key+&=".into(),
            caching_queue: Arc::new(StdMutex::new(VecDeque::new())),
            caching_current_series: Arc::new(StdMutex::new(None)),
            caching_cancelled_series: Arc::new(StdMutex::new(HashSet::new())),
            caching_thread_handle: Arc::new(StdMutex::new(None)),
            ws_sender: None,
        }
    }

    fn page_image(format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut bytes, format)
            .unwrap();
        bytes.into_inner()
    }

    async fn mock_page(status: u16, body: Vec<u8>) -> (String, tokio::task::JoinHandle<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let mut buffer = [0; 1024];
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
            }
            let header_end = request
                .windows(4)
                .position(|bytes| bytes == b"\r\n\r\n")
                .unwrap()
                + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let body_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            while request.len() < header_end + body_length {
                let mut buffer = [0; 1024];
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
            }
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(&body).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        (address, task)
    }

    #[tokio::test]
    async fn rendered_pdf_page_is_cached_with_its_format_and_reads_offline() {
        let cache = TestCache::new();
        let mut kavita = test_kavita();
        let bytes = page_image(image::ImageFormat::Jpeg);
        let (address, request) = mock_page(200, bytes.clone()).await;
        kavita.ip = address;
        let file = kavita
            .get_picture_in_folder(101, 2, &cache.0)
            .await
            .unwrap();
        let request = request.await.unwrap();
        assert!(request.starts_with("GET /api/reader/image?"));
        assert!(request.contains("chapterId=101"));
        assert!(request.contains("page=2"));
        assert!(request.contains("extractPdf=true"));
        assert!(request.contains("apiKey=key%2B%26%3D"));
        assert!(request.contains("authorization: Bearer test-token"));
        assert!(file.ends_with(".jpg"));
        assert_eq!(fs::read(&file).unwrap(), bytes);
        // No server is listening any more: a cached page also works online.
        assert_eq!(
            kavita
                .get_picture_in_folder(101, 2, &cache.0)
                .await
                .unwrap(),
            file
        );
        kavita.offline_mode = true;
        assert_eq!(
            kavita
                .get_picture_in_folder(101, 2, &cache.0)
                .await
                .unwrap(),
            file
        );
        assert!(kavita
            .get_picture_in_folder(101, 0, &cache.0)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn failed_or_unrendered_responses_do_not_enter_the_cache() {
        let cache = TestCache::new();
        let mut kavita = test_kavita();
        for (status, body) in [
            (500, page_image(image::ImageFormat::Png)),
            (200, b"%PDF-1.7".to_vec()),
            (200, b"<html>Error</html>".to_vec()),
            (204, Vec::new()),
        ] {
            let (address, request) = mock_page(status, body).await;
            kavita.ip = address;
            assert!(kavita
                .get_picture_in_folder(101, 0, &cache.0)
                .await
                .is_err());
            request.await.unwrap();
            assert!(kavita.db.get_picture(&101, &0).is_err());
            assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 0);
        }
    }

    #[test]
    fn bad_cached_pages_can_be_replaced_and_missing_files_are_not_reused() {
        let cache = TestCache::new();
        let db = test_kavita().db;
        let invalid = cache.0.join("bad.png");
        fs::write(&invalid, b"%PDF-1.7").unwrap();
        db.add_picture(&MangaPicture {
            chapter_id: 101,
            page: 0,
            file: invalid.to_string_lossy().into_owned(),
        })
        .unwrap();
        assert!(cached_reader_picture(&db, 101, 0).is_none());
        let file =
            cache_reader_picture(&db, &cache.0, 101, 0, &page_image(image::ImageFormat::Png))
                .unwrap();
        assert_eq!(cached_reader_picture(&db, 101, 0), Some(file.clone()));
        assert!(!invalid.exists());
        fs::remove_file(file).unwrap();
        assert!(cached_reader_picture(&db, 101, 0).is_none());
        assert!(reader_image_url("localhost:5000", "key", 101, -1).is_err());
        assert!(reader_image_url("localhost:5000", "key", 0, 0).is_err());
    }

    #[test]
    fn pdf_books_are_individual_entries_and_duplicate_chapters_are_skipped() {
        let first = json!({"id": 101, "volumeId": 11, "format": 4, "titleName": "Book One", "pages": 3, "pagesRead": 1});
        let second = json!({"id": 102, "volumeId": 11, "files": [{"format": 4}], "title": "Book Two", "pages": 2});
        let special = json!({"id": 103, "volumeId": 12, "format": "Pdf", "title": "Standalone PDF", "pages": 4});
        let volumes = reader_volumes(
            1,
            &json!({
                "volumes": [{"id": 11, "name": "Grouped books", "pages": 5, "chapters": [first, second]}],
                "chapters": [first], "specials": [special], "storylineChapters": [special],
            }),
        );
        assert_eq!(volumes.len(), 3);
        assert_eq!(
            (
                volumes[0].id,
                volumes[0].volume_id,
                volumes[0].pages,
                volumes[0].read
            ),
            (-101, 11, 3, 1)
        );
        assert_eq!(volumes[0].title, "Book One");
        assert_eq!((volumes[1].id, volumes[1].pages), (-102, 2));
        assert_eq!((volumes[2].chapter_id, volumes[2].volume_id), (103, 12));
        let manga = reader_volumes(
            1,
            &json!({"volumes": [{"id": 20, "name": "Volume 1", "pages": 10, "chapters": [{"id": 200, "format": 1}]}]}),
        );
        assert_eq!(
            (manga[0].id, manga[0].chapter_id, manga[0].pages),
            (20, 200, 10)
        );
    }

    #[tokio::test]
    async fn offline_pdf_progress_updates_only_the_selected_book_and_keeps_server_ids() {
        let mut kavita = test_kavita();
        kavita.offline_mode = true;
        let volumes = reader_volumes(
            1,
            &json!({"specials": [
                {"id": 101, "volumeId": 11, "format": 4, "pages": 3},
                {"id": 102, "volumeId": 11, "format": 4, "pages": 2},
            ]}),
        );
        kavita.db.replace_volumes(1, &volumes).unwrap();
        kavita
            .save_progress(&ReadProgress {
                id: None,
                library_id: 1,
                series_id: 1,
                volume_id: 11,
                chapter_id: 101,
                page: 2,
            })
            .await
            .unwrap();
        assert_eq!(kavita.db.get_volume_by_id(-101).unwrap().unwrap().read, 3);
        assert_eq!(kavita.db.get_volume_by_id(-102).unwrap().unwrap().read, 0);
        let progress = kavita.db.get_all_read_progress().unwrap();
        assert_eq!(
            (
                progress[0].volume_id,
                progress[0].chapter_id,
                progress[0].page
            ),
            (11, 101, 2)
        );
    }
    #[tokio::test]
    async fn pdf_read_actions_target_the_book_chapter() {
        let mut kavita = test_kavita();
        let books = reader_volumes(
            1,
            &json!({"specials": [
                {"id": 101, "volumeId": 11, "format": 4, "pages": 3},
                {"id": 102, "volumeId": 11, "format": 4, "pages": 2},
            ]}),
        );
        kavita.db.replace_volumes(1, &books).unwrap();
        for read in [true, false] {
            let (address, request) = mock_page(200, Vec::new()).await;
            kavita.ip = address;
            kavita.set_reader_item_read(1, -101, read).await.unwrap();
            let request = request.await.unwrap();
            let action = if read { "read" } else { "unread" };
            assert!(request.starts_with(&format!("POST /api/reader/mark-multiple-{action} ")));
            let payload: serde_json::Value =
                serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(
                payload,
                json!({"seriesId": 1, "volumeIds": [], "chapterIds": [101]})
            );
        }
    }
}
