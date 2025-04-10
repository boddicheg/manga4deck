    use std::path::{PathBuf};
use std::fs;
use tokio::sync::OnceCell;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};
use serde_json::json;
use reqwest;
use std::sync::Arc;
use tokio::runtime::Handle;
use crate::logger::{
    info
};

use crate::storage::{
    Database
};

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
    pub id: String,
    pub title: String,
}

#[derive(Clone)]
pub struct Kavita {
    pub db: Database,
    pub token: String,
    pub logged_as: String,
    pub offline_mode: bool,
    pub ip: String,
}

impl Kavita {
    pub fn new() -> Self {
        let db_path = get_appdir_path(DB_PATH);
        info(&format!("Database created at {}", db_path));
        info(&format!("Cache size: {} GB", get_cache_size(1024 * 1024 * 1024)));

        let db = Database::new(&db_path).expect("Failed to create database");

        let kavita = Self {
            db,
            token: String::new(),
            logged_as: String::new(),
            offline_mode: false,
            ip: DEFAULT_IP.to_string(),
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

        // make http request to get token
        let client = reqwest::Client::new();
        let response = client.post(format!("http://{}/api/Account/login", self.ip))
            .json(&json!({
                "username": username,
                "password": password,
                "apiKey": api_key
            }))
            .send()
            .await?;

        if response.status().is_success() {
            let body = response.text().await?;
            let data: serde_json::Value = serde_json::from_str(&body)?;
            self.token = data["token"].as_str().unwrap_or("").to_string();
            self.logged_as = data["username"].as_str().unwrap_or("").to_string();
            info(&format!("Logged as: {}", self.logged_as));
        } else {
            self.offline_mode = true;
            self.logged_as = "".to_string();
            self.token = "".to_string();
            info(&format!("Failed to get token. Now in offline mode"));
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
            let response = client.post(url)
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
            let data: serde_json::Value = serde_json::from_str(&body)?;
            info(&format!("Libraries: {}", data));
            let libraries: Vec<Library> = data.as_array().unwrap().iter().map(|v| Library {
                id: v["id"].as_i64().unwrap_or(0).to_string(),
                title: v["name"].as_str().unwrap_or("").to_string(),
            }).collect();
            for library in libraries {
                self.db.add_library(&library)?;
            }
        }

        Ok(())
    }

    pub async fn get_libraries(&self) -> Result<Vec<Library>, Box<dyn std::error::Error>> {
        if !self.offline_mode {
            self.pull_libraries().await?;
        }
        let libraries = self.db.get_libraries()?;
        info(&format!("Libraries: {:?}", libraries.clone()));
        Ok(libraries)
    }
}
