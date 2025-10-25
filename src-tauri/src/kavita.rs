use std::path::{PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use serde_json::json;
use reqwest;
use md5::{Md5, Digest};
use crate::logger::{
    info
};

use tokio::time::{timeout, Duration};

use crate::storage::{
    Database
};

use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::collections::VecDeque;

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
        };

        kavita
    }

    pub fn insert_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.db.insert_setting(key, value)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        self.db.get_setting(key)
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

        self.ip = "192.168.1.100:5001".to_string();

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
        let response = match timeout(Duration::from_secs(5), fut).await {
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
            },
            Ok(Err(e)) => {
                self.offline_mode = true;
                self.logged_as = "".to_string();
                self.token = "".to_string();
                info(&format!("Failed to get token. Now in offline mode"));
                return Err(Box::new(e));
            }
            Err(_) => {
                self.offline_mode = true;
                self.logged_as = "".to_string();
                self.token = "".to_string();
                info(&format!("Failed to get token. Now in offline mode"));
                return Err("Timeout".into());
            }
        };
        info(&format!("reconnect_with_creds - done"));
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
                id: v["id"].as_i64().unwrap_or(0) as i32,
                library_id: library_id.clone(),
                title: v["name"].as_str().unwrap_or("").to_string(),
                read: v["pagesRead"].as_i64().unwrap_or(0) as i32 * 100 / v["pages"].as_i64().unwrap_or(0) as i32,
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
            series = series.into_iter().filter(|s| !self.is_series_cached(s.id)).collect();
        }
        // sort series by title and return sorted series
        series.sort_by_key(|s| s.title.clone());
        Ok(series)
    }

    pub async fn get_series_cover(&self, series_id: &i32) -> Result<SeriesCover, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            let url = format!("http://{}/api/image/series-cover?seriesId={}&apiKey={}", self.ip, series_id, self.api_key);
            let client = reqwest::Client::new();
            let response = client.get(url)
                // .header("Authorization", format!("Bearer {}", self.token))
                .header("Content-Type", "image/png")
                .send()
                .await?;
            let body = response.bytes().await?;
            let hash = generate_hash_from_now();
            let cache_folder = get_datadir().join("manga4deck-cache").join("cache");    
            let filename = cache_folder.join(format!("{}.png", hash));
            fs::write(&filename, body)?;
            
            let series_cover = SeriesCover {
                series_id: *series_id,
                file: filename.to_string_lossy().into_owned(),
            };
            self.db.add_series_cover(&series_cover)?;
        }
        else {
            info(&format!("Offline mode! Getting series cover from database"));
        }
        
        let series_cover = self.db.get_series_cover(series_id)?;
        Ok(series_cover)
    }

    // -------------------------------------------------------------------------
    // Volume methods
    pub async fn pull_volumes(&self, series_id: &i32) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client.get(format!("http://{}/api/series/series-detail?seriesId={}&apiKey={}", self.ip, series_id, self.api_key))
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;
        let body = response.text().await?;
        let data: serde_json::Value = serde_json::from_str(&body)?;

        if data["chapters"].is_array() && data["chapters"].as_array().map_or(0, |arr| arr.len()) > 0 {
            let volumes: Vec<Volume> = data["volumes"].as_array().unwrap_or(&Vec::new()).iter().map(|v| Volume {
                id: v["id"].as_i64().unwrap_or(0) as i32,
                series_id: series_id.clone(),
                chapter_id: v["chapters"][0]["id"].as_i64().unwrap_or(0) as i32,
                volume_id: v["id"].as_i64().unwrap_or(0) as i32,
                title: v["name"].as_str().unwrap_or("").to_string(),
                read: v["pagesRead"].as_i64().unwrap_or(0) as i32,
                pages: v["pages"].as_i64().unwrap_or(0) as i32,
                is_cached: false,
            }).collect();   
            for volume in volumes {
                self.db.add_volume(&volume)?;
            }
        }
        Ok(())
    }   

    pub async fn get_volumes(&self, series_id: &i32) -> Result<Vec<Volume>, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            self.pull_volumes(series_id).await?;
        }
        let mut volumes = self.db.get_volumes(series_id)?;
        if self.offline_mode {
            volumes = volumes.into_iter().filter(|v| self.is_volume_cached(v.id)).collect();
        }
        // sort volumes by title and converted to int and return sorted volumes
        volumes.sort_by_key(|v| v.title.clone().replace(|c: char| !c.is_digit(10), "").parse::<i32>().unwrap_or(0));
        // Update is_cached for each volume
        for v in &mut volumes {
            v.is_cached = self.is_volume_cached(v.id);
        }
        Ok(volumes)
    }

    pub async fn get_volume_cover(&self, volume_id: &i32) -> Result<VolumeCover, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            let url = format!("http://{}/api/image/volume-cover?volumeId={}&apiKey={}", self.ip, volume_id, self.api_key);
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
            
            let volume_cover = VolumeCover {
                volume_id: *volume_id,
                file: filename.to_string_lossy().into_owned(),
            };
            self.db.add_volume_cover(&volume_cover)?;
        }
        
        let volume_cover = self.db.get_volume_cover(volume_id)?;
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
        let url = format!("http://{}/api/reader/progress", self.ip);
        if !self.offline_mode {
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
            *handle_guard = Some(thread::spawn(move || {
                cache_serie_threaded(db, queue, ip, api_key, token);
            }));
        }
    }
}

// Background thread for caching series
fn cache_serie_threaded(db: Database, queue: Arc<StdMutex<VecDeque<i32>>>, ip: String, api_key: String, _token: String) {
    use ureq;
    use std::io::Read;
    loop {
        let series_id = {
            let mut q = queue.lock().unwrap();
            q.pop_front()
        };
        if let Some(series_id) = series_id {
            let mut volumes = db.get_volumes(&series_id).unwrap_or_default();
            // Sort volumes by the number in their title (e.g., 'Volume 20' < 'Volume 21')
            volumes.sort_by_key(|v| {
                let digits: String = v.title.chars().filter(|c| c.is_digit(10)).collect();
                digits.parse::<i32>().unwrap_or(0)
            });
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

            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}
