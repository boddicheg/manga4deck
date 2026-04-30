use std::sync::Arc;
use std::time::Duration;

use dioxus::desktop::{tao::dpi::LogicalSize, Config, WindowBuilder};
use dioxus::prelude::*;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

use crate::kavita::{get_cache_size, Kavita, Library, Series, Volume};

pub type SharedKavita = Arc<Mutex<Kavita>>;

static KAVITA: OnceCell<SharedKavita> = OnceCell::new();
const READER_BATCH_SIZE: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Libraries,
    Series,
    Volumes,
    Reader,
    Settings,
}

#[derive(Debug, Clone)]
struct UiStatus {
    server_ip: String,
    logged_as: String,
    offline_mode: bool,
    cache_mb: u64,
    message: String,
}

impl UiStatus {
    fn line(&self) -> String {
        if self.offline_mode {
            format!("Server status: offline, ip is {}", self.server_ip)
        } else {
            format!(
                "Server status: online, logged as {}, ip is {}",
                if self.logged_as.is_empty() {
                    "(not signed in)"
                } else {
                    &self.logged_as
                },
                self.server_ip,
            )
        }
    }
}

fn kavita() -> SharedKavita {
    KAVITA
        .get()
        .expect("Dioxus UI started before Kavita state was initialized")
        .clone()
}

fn load_status() -> UiStatus {
    let cache_mb = get_cache_size(1024 * 1024);
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let kavita = kavita.lock().await;
        UiStatus {
            server_ip: kavita.ip.clone(),
            logged_as: kavita.logged_as.clone(),
            offline_mode: kavita.offline_mode,
            cache_mb,
            message: if kavita.offline_mode {
                "Offline mode is active".into()
            } else {
                "Connected to Kavita server".into()
            },
        }
    })
}

fn load_libraries() -> Result<Vec<Library>, String> {
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let kavita = kavita.lock().await;
        kavita.get_libraries().await.map_err(|err| err.to_string())
    })
}

fn load_series(library_id: i32) -> Result<Vec<Series>, String> {
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let kavita = kavita.lock().await;
        kavita
            .get_series(&library_id)
            .await
            .map_err(|err| err.to_string())
    })
}

fn load_volumes(series_id: i32) -> Result<Vec<Volume>, String> {
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let kavita = kavita.lock().await;
        kavita
            .get_volumes(&series_id)
            .await
            .map_err(|err| err.to_string())
    })
}

fn clear_cache() -> Result<(), String> {
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let kavita = kavita.lock().await;
        kavita.clear_cache().map_err(|err| err.to_string())
    })
}

fn update_server_library() -> Result<(), String> {
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let kavita = kavita.lock().await;
        kavita
            .update_server_library()
            .await
            .map_err(|err| err.to_string())
    })
}

fn toggle_offline() -> Result<bool, String> {
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let mut kavita = kavita.lock().await;
        kavita.offline_mode = !kavita.offline_mode;
        if kavita.offline_mode {
            kavita.send_connection_status(true, "");
        } else if kavita.reconnect_with_creds().await.is_err() {
            kavita.offline_mode = true;
            kavita.send_connection_status(true, "");
        }
        Ok(kavita.offline_mode)
    })
}

fn save_settings(
    server_ip: String,
    username: String,
    password: String,
    api_key: String,
) -> Result<(), String> {
    tokio::runtime::Handle::current().block_on(async {
        let kavita = kavita();
        let mut kavita = kavita.lock().await;
        kavita
            .insert_setting("ip", &server_ip)
            .map_err(|err| err.to_string())?;
        kavita
            .insert_setting("username", &username)
            .map_err(|err| err.to_string())?;
        kavita
            .insert_setting("password", &password)
            .map_err(|err| err.to_string())?;
        kavita
            .insert_setting("api_key", &api_key)
            .map_err(|err| err.to_string())?;
        kavita
            .reconnect_with_creds()
            .await
            .map_err(|err| err.to_string())
    })
}

fn exit_process() {
    std::process::exit(0);
}

fn visible_tile_count(
    page: Page,
    libraries: &[Library],
    series: &[Series],
    volumes: &[Volume],
) -> usize {
    match page {
        Page::Dashboard => 6,
        Page::Libraries => libraries.len() + 1,
        Page::Series => series.len() + 1,
        Page::Volumes => volumes.len() + 1,
        Page::Reader => 0,
        Page::Settings => 2,
    }
}

fn tile_columns(page: Page) -> usize {
    match page {
        Page::Libraries | Page::Series | Page::Volumes => 8,
        Page::Dashboard => 6,
        Page::Reader => 1,
        Page::Settings => 1,
    }
}

fn move_selection(current: usize, key: &str, count: usize, columns: usize) -> usize {
    if count == 0 {
        return 0;
    }

    match key {
        "ArrowLeft" => current.saturating_sub(1),
        "ArrowRight" => (current + 1).min(count - 1),
        "ArrowUp" => current.saturating_sub(columns),
        "ArrowDown" => (current + columns).min(count - 1),
        _ => current,
    }
}

fn selected_series(series: &[Series], selected: usize) -> Option<&Series> {
    if selected == 0 {
        return None;
    }

    series
        .iter()
        .filter(|item| item.read < 100)
        .chain(series.iter().filter(|item| item.read >= 100))
        .nth(selected - 1)
}

fn initial_reader_page(volume: &Volume) -> i32 {
    if volume.pages <= 0 {
        0
    } else {
        volume.read.clamp(0, volume.pages - 1)
    }
}

fn reader_batch_end(first_page: i32, total_pages: i32) -> i32 {
    if total_pages <= 0 {
        0
    } else {
        (first_page + READER_BATCH_SIZE - 1).min(total_pages - 1)
    }
}

fn next_reader_batch_end(current_end: i32, total_pages: i32) -> i32 {
    if total_pages <= 0 {
        0
    } else {
        (current_end + READER_BATCH_SIZE).min(total_pages - 1)
    }
}

const STYLE: &str = r#"
html, body, #main {
  width: 100%;
  height: 100%;
  margin: 0;
  background: #121316;
  color: #fff;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
.app {
  min-height: 100vh;
  box-sizing: border-box;
  padding: 16px;
  background: #121316;
}
.app:focus {
  outline: none;
}
.app.series-mode {
  padding: 0;
  background: #050505;
  overflow-x: hidden;
}
.status {
  height: 33px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 12px;
  background: #22c55e;
  color: #fff;
  font-size: 17px;
  font-weight: 700;
}
.tiles {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: 8px;
}
.tile {
  width: 150px;
  height: 150px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  background: #d1d5db;
  color: #0a0a0a;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 10px;
  box-sizing: border-box;
  cursor: pointer;
}
.tile:focus,
.tile:hover,
.tile.selected {
  outline: none;
  border: 3px solid #ff4b4b;
}
.title {
  font-size: 16px;
  font-weight: 800;
  text-align: center;
}
.subtitle {
  margin-top: 8px;
  color: #4b5563;
  font-size: 13px;
  text-align: center;
}
.message {
  margin-top: 12px;
  color: #fff;
  font-size: 12px;
}
.settings {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 420px;
}
.settings input {
  height: 36px;
  padding: 0 10px;
  border: 1px solid #4b5563;
  border-radius: 4px;
}
.series-page {
  width: 100vw;
  min-height: 100vh;
  scroll-behavior: smooth;
  background:
    repeating-linear-gradient(45deg, rgba(0,0,0,0.05) 0px, rgba(0,0,0,0.05) 2px, transparent 2px, transparent 4px),
    repeating-linear-gradient(-45deg, rgba(0,0,0,0.04) 0px, rgba(0,0,0,0.04) 2px, transparent 2px, transparent 4px),
    linear-gradient(90deg, #8a552c 0%, #a66a3a 44%, #805026 100%);
}
.series-titlebar {
  height: 48px;
  display: grid;
  grid-template-columns: 120px 1fr 120px;
  align-items: center;
  background: linear-gradient(#060606, #010101);
  border-bottom: 1px solid #2a2a2a;
}
.series-titlebar h1 {
  margin: 0;
  text-align: center;
  font-size: 20px;
  line-height: 1;
  font-weight: 900;
  color: #f5f5f5;
}
.series-back {
  width: 92px;
  height: 36px;
  margin-left: 16px;
  border: 1px solid rgba(255,255,255,0.18);
  border-radius: 4px;
  background: rgba(0,0,0,0.36);
  color: #fff;
  font-size: 15px;
  font-weight: 700;
  cursor: pointer;
}
.series-back:focus,
.series-back:hover,
.series-back.selected {
  outline: 2px solid #3b82f6;
  outline-offset: 2px;
}
.series-section {
  position: relative;
  padding: 26px 16px 42px;
}
.series-section-title {
  margin: 0 0 24px;
  color: #fff;
  font-size: 20px;
  font-weight: 900;
  text-shadow: 0 2px 4px rgba(0,0,0,0.45);
}
.series-count {
  color: rgba(255,255,255,0.72);
}
.dashboard-status {
  height: 42px;
  margin: 18px 16px 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #22c55e;
  color: #fff;
  font-size: 16px;
  font-weight: 800;
  box-shadow: 0 4px 10px rgba(0,0,0,0.25);
}
.series-grid {
  position: relative;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  column-gap: 26px;
  row-gap: 68px;
  align-items: start;
}
.series-card-wrap {
  position: relative;
  display: flex;
  justify-content: center;
}
.series-card-wrap::after {
  content: "";
  position: absolute;
  left: -18px;
  right: -18px;
  bottom: -34px;
  height: 31px;
  z-index: 0;
  background: linear-gradient(to bottom, #8b4513 0%, #654321 58%, #4a3219 100%);
  border-top: 1px solid rgba(255,255,255,0.12);
  box-shadow: 0 5px 8px rgba(0,0,0,0.42), inset 0 1px 1px rgba(255,255,255,0.12);
}
.series-card-wrap::before {
  content: "";
  position: absolute;
  left: -18px;
  right: -18px;
  bottom: -38px;
  height: 6px;
  z-index: 0;
  background: linear-gradient(to bottom, rgba(0,0,0,0.35), transparent);
}
.series-card {
  position: relative;
  z-index: 1;
  width: 150px;
  height: 200px;
  padding: 0;
  border: 1px solid rgba(0,0,0,0.42);
  border-radius: 4px;
  overflow: hidden;
  background-color: #202020;
  background-size: cover;
  background-position: center;
  color: #fff;
  cursor: pointer;
  scroll-margin: 96px 32px 72px;
  box-shadow: 8px -8px 8px rgba(0,0,0,0.22), 3px -3px 4px rgba(0,0,0,0.16), inset 0 0 0 1px rgba(255,255,255,0.10);
}
.series-card:focus,
.series-card:hover,
.series-card.selected {
  outline: none;
  border: 2px solid #111;
  box-shadow: 8px -8px 12px rgba(0,0,0,0.44), 3px -3px 6px rgba(0,0,0,0.34), 0 0 0 2px rgba(59,130,246,0.65);
}
.series-progress {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 6px;
  background: rgba(0,0,0,0.7);
  box-shadow: 0 1px 2px rgba(0,0,0,0.3);
}
.series-progress-fill {
  height: 100%;
  background: #3b82f6;
  box-shadow: 0 0 4px rgba(255,255,255,0.45);
}
.series-progress-fill.complete {
  background: #22c55e;
}
.series-progress-fill.cached {
  background: #f59e0b;
}
.series-card-title {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  padding: 0 8px;
  background: rgba(0,0,0,0.86);
  border-top: 1px solid rgba(255,255,255,0.12);
  font-size: 14px;
  line-height: 1;
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.series-card-title.complete {
  background: rgba(4, 120, 87, 0.88);
}
.series-card-title.cached {
  background: rgba(180, 83, 9, 0.88);
}
.library-card {
  position: relative;
  z-index: 1;
  width: 150px;
  height: 200px;
  padding: 0;
  border: 1px solid rgba(0,0,0,0.42);
  border-radius: 4px;
  overflow: hidden;
  background:
    linear-gradient(135deg, rgba(255,255,255,0.10), transparent 36%),
    repeating-linear-gradient(90deg, rgba(255,255,255,0.08) 0px, rgba(255,255,255,0.08) 2px, transparent 2px, transparent 9px),
    linear-gradient(145deg, #34383f 0%, #20242b 52%, #111317 100%);
  color: #fff;
  cursor: pointer;
  box-shadow: 8px -8px 8px rgba(0,0,0,0.22), 3px -3px 4px rgba(0,0,0,0.16), inset 0 0 0 1px rgba(255,255,255,0.10);
}
.library-card:focus,
.library-card:hover,
.library-card.selected {
  outline: none;
  border: 2px solid #111;
  box-shadow: 8px -8px 12px rgba(0,0,0,0.44), 3px -3px 6px rgba(0,0,0,0.34), 0 0 0 2px rgba(59,130,246,0.65);
}
.library-card::before {
  content: "";
  position: absolute;
  left: 14px;
  top: 0;
  bottom: 0;
  width: 10px;
  background: rgba(0,0,0,0.24);
  border-left: 1px solid rgba(255,255,255,0.10);
  border-right: 1px solid rgba(0,0,0,0.35);
}
.library-card::after {
  content: "";
  position: absolute;
  left: 34px;
  right: 34px;
  top: 36px;
  height: 78px;
  border-radius: 4px;
  border: 1px solid rgba(255,255,255,0.18);
  background: rgba(0,0,0,0.20);
  box-shadow: inset 0 1px 10px rgba(255,255,255,0.05);
}
.library-card-title {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  min-height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  padding: 6px 8px;
  background: rgba(0,0,0,0.86);
  border-top: 1px solid rgba(255,255,255,0.12);
  font-size: 16px;
  font-weight: 800;
  line-height: 1.05;
  text-align: center;
}
.back-card {
  position: relative;
  z-index: 1;
  width: 150px;
  height: 200px;
  padding: 0;
  border: 1px solid rgba(0,0,0,0.42);
  border-radius: 4px;
  overflow: hidden;
  background:
    radial-gradient(circle at 50% 36%, rgba(255,255,255,0.16), transparent 36%),
    linear-gradient(145deg, #2e333b 0%, #20242b 56%, #111317 100%);
  color: #fff;
  cursor: pointer;
  box-shadow: 8px -8px 8px rgba(0,0,0,0.22), 3px -3px 4px rgba(0,0,0,0.16), inset 0 0 0 1px rgba(255,255,255,0.10);
}
.back-card:focus,
.back-card:hover,
.back-card.selected {
  outline: none;
  border: 2px solid #111;
  box-shadow: 8px -8px 12px rgba(0,0,0,0.44), 3px -3px 6px rgba(0,0,0,0.34), 0 0 0 2px rgba(255,75,75,0.85);
}
.back-card-arrow {
  position: absolute;
  left: 0;
  right: 0;
  top: 42px;
  text-align: center;
  font-size: 56px;
  line-height: 1;
  font-weight: 900;
}
.back-card-title {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  padding: 0 8px;
  background: rgba(0,0,0,0.86);
  border-top: 1px solid rgba(255,255,255,0.12);
  font-size: 15px;
  font-weight: 800;
  line-height: 1;
  text-align: center;
}
.dashboard-card {
  position: relative;
  z-index: 1;
  width: 150px;
  height: 200px;
  padding: 0;
  border: 1px solid rgba(0,0,0,0.42);
  border-radius: 4px;
  overflow: hidden;
  background:
    radial-gradient(circle at 50% 34%, rgba(255,255,255,0.18), transparent 34%),
    linear-gradient(145deg, #d9dee6 0%, #bfc6d0 58%, #9fa8b4 100%);
  color: #0b0f14;
  cursor: pointer;
  box-shadow: 8px -8px 8px rgba(0,0,0,0.22), 3px -3px 4px rgba(0,0,0,0.16), inset 0 0 0 1px rgba(255,255,255,0.22);
}
.dashboard-card:focus,
.dashboard-card:hover,
.dashboard-card.selected {
  outline: none;
  border: 2px solid #111;
  box-shadow: 8px -8px 12px rgba(0,0,0,0.44), 3px -3px 6px rgba(0,0,0,0.34), 0 0 0 2px rgba(255,75,75,0.85);
}
.dashboard-card-title {
  position: absolute;
  left: 0;
  right: 0;
  top: 62px;
  padding: 0 10px;
  font-size: 17px;
  font-weight: 900;
  line-height: 1.05;
  text-align: center;
}
.dashboard-card-subtitle {
  position: absolute;
  left: 0;
  right: 0;
  top: 102px;
  padding: 0 10px;
  color: #4b5563;
  font-size: 13px;
  line-height: 1.15;
  text-align: center;
}
.reader-page {
  width: 100vw;
  height: 100vh;
  overflow-y: auto;
  background: #111216;
  color: #fff;
}
.reader-bar {
  height: 38px;
  display: grid;
  grid-template-columns: 120px 1fr 120px;
  align-items: center;
  background: #050505;
  border-bottom: 1px solid #262626;
}
.reader-back {
  width: 88px;
  height: 28px;
  margin-left: 14px;
  border: 1px solid rgba(255,255,255,0.18);
  border-radius: 4px;
  background: rgba(255,255,255,0.08);
  color: #fff;
  font-size: 14px;
  font-weight: 800;
  cursor: pointer;
}
.reader-title {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  text-align: center;
  font-size: 15px;
  font-weight: 800;
}
.reader-count {
  padding-right: 14px;
  text-align: right;
  color: rgba(255,255,255,0.74);
  font-size: 13px;
  font-weight: 700;
}
.reader-stage {
  min-height: calc(100vh - 38px);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 10px 0 28px;
  box-sizing: border-box;
  background: #15161a;
}
.reader-image {
  display: block;
  width: auto;
  max-width: 100vw;
  min-height: 200px;
  object-fit: contain;
  box-shadow: 0 8px 28px rgba(0,0,0,0.45);
}
.reader-image-wrap {
  position: relative;
  width: fit-content;
  max-width: 100vw;
  margin-bottom: 14px;
}
.reader-page-label {
  position: absolute;
  top: 8px;
  right: 8px;
  padding: 4px 8px;
  border-radius: 4px;
  background: rgba(0,0,0,0.62);
  color: #fff;
  font-size: 12px;
  font-weight: 800;
}
.reader-loading {
  width: 100%;
  box-sizing: border-box;
  padding: 18px;
  color: rgba(255,255,255,0.68);
  font-size: 13px;
  font-weight: 700;
  text-align: center;
}
.reader-slider {
  position: fixed;
  left: 50%;
  bottom: 18px;
  width: min(820px, calc(100vw - 48px));
  transform: translateX(-50%);
  z-index: 20;
  box-sizing: border-box;
  padding: 12px 16px 14px;
  border: 1px solid rgba(255,255,255,0.16);
  border-radius: 8px;
  background: rgba(0,0,0,0.82);
  box-shadow: 0 10px 30px rgba(0,0,0,0.42);
}
.reader-slider-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
  color: #fff;
  font-size: 13px;
  font-weight: 800;
}
.reader-slider input {
  width: 100%;
  accent-color: #3b82f6;
}
.reader-empty {
  margin-top: 120px;
  color: rgba(255,255,255,0.72);
  font-size: 16px;
}
"#;

fn app() -> Element {
    let initial_status = load_status();
    let mut page = use_signal(|| Page::Dashboard);
    let mut selected_index = use_signal(|| 0usize);
    let mut dashboard_selection = use_signal(|| 0usize);
    let mut libraries_selection = use_signal(|| 0usize);
    let mut series_selection = use_signal(|| 0usize);
    let mut volumes_selection = use_signal(|| 0usize);
    let mut status = use_signal(|| initial_status.clone());
    let mut libraries = use_signal(Vec::<Library>::new);
    let mut series = use_signal(Vec::<Series>::new);
    let mut volumes = use_signal(Vec::<Volume>::new);
    let mut reader_volume = use_signal(|| None::<Volume>);
    let mut reader_page = use_signal(|| 0i32);
    let mut reader_first_page = use_signal(|| 0i32);
    let mut reader_loaded_until = use_signal(|| 0i32);
    let mut reader_slider_visible = use_signal(|| false);
    let mut reader_slider_generation = use_signal(|| 0u64);
    let mut server_ip = use_signal(|| initial_status.server_ip);
    let mut username = use_signal(|| initial_status.logged_as);
    let mut password = use_signal(String::new);
    let mut api_key = use_signal(String::new);

    use_effect(move || {
        let _ = selected_index();
        if matches!(page(), Page::Series | Page::Volumes) {
            document::eval(
                r#"
                setTimeout(() => {
                    const selected = document.querySelector(".series-page .selected");
                    if (selected) {
                        selected.scrollIntoView({
                            block: "nearest",
                            inline: "nearest",
                            behavior: "smooth"
                        });
                    }
                }, 0);
                "#,
            );
        }
    });

    use_effect(move || {
        let visible = reader_slider_visible();
        let generation = reader_slider_generation();
        if visible {
            spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if reader_slider_generation() == generation {
                    reader_slider_visible.set(false);
                }
            });
        }
    });

    let status_snapshot = status.read().clone();
    let page_snapshot = page();
    let libraries_snapshot = libraries.read().clone();
    let series_snapshot = series.read().clone();
    let volumes_snapshot = volumes.read().clone();
    let in_progress_series = series_snapshot
        .iter()
        .filter(|item| item.read < 100)
        .cloned()
        .collect::<Vec<_>>();
    let completed_series = series_snapshot
        .iter()
        .filter(|item| item.read >= 100)
        .cloned()
        .collect::<Vec<_>>();
    let selected_snapshot = selected_index();
    let selectable_count = visible_tile_count(
        page_snapshot,
        &libraries_snapshot,
        &series_snapshot,
        &volumes_snapshot,
    );
    let selected_snapshot = selected_snapshot.min(selectable_count.saturating_sub(1));
    let app_class = if matches!(
        page_snapshot,
        Page::Dashboard | Page::Libraries | Page::Series | Page::Volumes | Page::Reader
    ) {
        "app series-mode"
    } else {
        "app"
    };

    rsx! {
        style { "{STYLE}" }
        div {
            class: "{app_class}",
            tabindex: "0",
            autofocus: "true",
            onmounted: move |event| {
                let element = event.data();
                spawn(async move {
                    let _ = element.set_focus(true).await;
                });
            },
            onkeydown: move |event| {
                let key = event.key().to_string();
                match key.as_str() {
                    "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" => {
                        event.prevent_default();
                        if page() == Page::Reader {
                            if let Some(volume) = reader_volume.read().as_ref() {
                                match key.as_str() {
                                    "ArrowDown" => {
                                        document::eval(
                                            r#"
                                            const reader = document.querySelector(".reader-page");
                                            if (reader) {
                                                reader.scrollBy({ top: Math.floor(reader.clientHeight * 0.82), behavior: "smooth" });
                                            }
                                            "#,
                                        );
                                    }
                                    "ArrowUp" => {
                                        document::eval(
                                            r#"
                                            const reader = document.querySelector(".reader-page");
                                            if (reader) {
                                                reader.scrollBy({ top: -Math.floor(reader.clientHeight * 0.82), behavior: "smooth" });
                                            }
                                            "#,
                                        );
                                    }
                                    "ArrowLeft" | "ArrowRight" => {
                                        reader_slider_generation.set(reader_slider_generation().wrapping_add(1));
                                        if !reader_slider_visible() {
                                            reader_slider_visible.set(true);
                                        } else {
                                            let max_page = volume.pages.saturating_sub(1);
                                            let current = reader_page();
                                            let next = if key == "ArrowRight" {
                                                (current + 1).min(max_page)
                                            } else {
                                                current.saturating_sub(1)
                                            };
                                            reader_page.set(next);
                                            if next < reader_first_page() {
                                                reader_first_page.set(next);
                                            }
                                            if next > reader_loaded_until() {
                                                reader_loaded_until.set(reader_batch_end(next, volume.pages));
                                            }
                                            document::eval(&format!(
                                                r#"
                                                setTimeout(() => {{
                                                    const page = document.querySelector('[data-reader-page="{}"]');
                                                    if (page) {{
                                                        page.scrollIntoView({{ block: "start", behavior: "smooth" }});
                                                    }}
                                                }}, 0);
                                                "#,
                                                next
                                            ));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            let count = visible_tile_count(page(), &libraries.read(), &series.read(), &volumes.read());
                            let current = selected_index().min(count.saturating_sub(1));
                            let next = move_selection(current, key.as_str(), count, tile_columns(page()));
                            selected_index.set(next);
                            match page() {
                                Page::Dashboard => dashboard_selection.set(next),
                                Page::Libraries => libraries_selection.set(next),
                                Page::Series => series_selection.set(next),
                                Page::Volumes => volumes_selection.set(next),
                                Page::Reader | Page::Settings => {}
                            }
                        }
                    }
                    "Enter" | " " => {
                        event.prevent_default();
                        let selected = selected_index();
                        match page() {
                            Page::Dashboard => match selected {
                                0 => match load_libraries() {
                                    Ok(items) => {
                                        libraries.set(items);
                                        status.write().message = format!("Loaded {} libraries", libraries.read().len());
                                        dashboard_selection.set(selected);
                                        selected_index.set(0);
                                        page.set(Page::Libraries);
                                    }
                                    Err(err) => status.write().message = format!("Failed to load libraries: {err}"),
                                },
                                1 => match clear_cache() {
                                    Ok(()) => status.set(load_status()),
                                    Err(err) => status.write().message = format!("Failed to clear cache: {err}"),
                                },
                                2 => {
                                    status.write().message = "Updating server library...".into();
                                    match update_server_library() {
                                        Ok(()) => status.write().message = "Server library update requested".into(),
                                        Err(err) => status.write().message = format!("Server library update failed: {err}"),
                                    }
                                }
                                3 => {
                                    dashboard_selection.set(selected);
                                    selected_index.set(0);
                                    page.set(Page::Settings);
                                }
                                4 => match toggle_offline() {
                                    Ok(_) => status.set(load_status()),
                                    Err(err) => status.write().message = format!("Offline toggle failed: {err}"),
                                },
                                5 => exit_process(),
                                _ => {}
                            },
                            Page::Libraries => {
                                if selected == 0 {
                                    selected_index.set(dashboard_selection());
                                    page.set(Page::Dashboard);
                                } else if let Some(library) = libraries.read().get(selected - 1) {
                                    let id = library.id;
                                    match load_series(id) {
                                        Ok(items) => {
                                            series.set(items);
                                            status.write().message = format!("Loaded series for library id {id}");
                                            libraries_selection.set(selected);
                                            selected_index.set(0);
                                            page.set(Page::Series);
                                        }
                                        Err(err) => status.write().message = format!("Failed to load series: {err}"),
                                    }
                                }
                            }
                            Page::Series => {
                                if selected == 0 {
                                    selected_index.set(libraries_selection());
                                    page.set(Page::Libraries);
                                } else if let Some(item) = selected_series(&series.read(), selected) {
                                    let id = item.id;
                                    match load_volumes(id) {
                                        Ok(items) => {
                                            volumes.set(items);
                                            status.write().message = format!("Loaded volumes for series id {id}");
                                            series_selection.set(selected);
                                            selected_index.set(0);
                                            page.set(Page::Volumes);
                                        }
                                        Err(err) => status.write().message = format!("Failed to load volumes: {err}"),
                                    }
                                }
                            }
                            Page::Volumes => {
                                if selected == 0 {
                                    selected_index.set(series_selection());
                                    page.set(Page::Series);
                                } else if let Some(volume) = volumes.read().get(selected - 1) {
                                    volumes_selection.set(selected);
                                    let start_page = initial_reader_page(volume);
                                    reader_page.set(start_page);
                                    reader_first_page.set(start_page);
                                    reader_loaded_until.set(reader_batch_end(start_page, volume.pages));
                                    reader_slider_visible.set(false);
                                    reader_volume.set(Some(volume.clone()));
                                    page.set(Page::Reader);
                                }
                            }
                            Page::Reader => {}
                            Page::Settings => match selected {
                                0 => match save_settings(
                                    server_ip.read().clone(),
                                    username.read().clone(),
                                    password.read().clone(),
                                    api_key.read().clone(),
                                ) {
                                    Ok(()) => status.set(load_status()),
                                    Err(err) => status.write().message = format!("Failed to save settings: {err}"),
                                },
                                1 => {
                                    selected_index.set(dashboard_selection());
                                    page.set(Page::Dashboard);
                                }
                                _ => {}
                            },
                        }
                    }
                    "Backspace" => {
                        event.prevent_default();
                        match page() {
                            Page::Dashboard => {}
                            Page::Libraries | Page::Settings => {
                                selected_index.set(dashboard_selection());
                                page.set(Page::Dashboard);
                            }
                            Page::Series => {
                                selected_index.set(libraries_selection());
                                page.set(Page::Libraries);
                            }
                            Page::Volumes => {
                                selected_index.set(series_selection());
                                page.set(Page::Series);
                            }
                            Page::Reader => {
                                if let Some(volume) = reader_volume.read().as_ref() {
                                    let series_id = volume.series_id;
                                    match load_volumes(series_id) {
                                        Ok(items) => volumes.set(items),
                                        Err(err) => status.write().message = format!("Failed to refresh volumes: {err}"),
                                    }
                                }
                                selected_index.set(volumes_selection());
                                page.set(Page::Volumes);
                            }
                        }
                    }
                    _ => {}
                }
            },
            if !matches!(
                page_snapshot,
                Page::Dashboard | Page::Libraries | Page::Series | Page::Volumes
                    | Page::Reader
            ) {
                div { class: "status", "{status_snapshot.line()}" }
            }
            div { class: "tiles",
                {
                    match page_snapshot {
                        Page::Dashboard => rsx! {
                            div { class: "series-page",
                                div { class: "series-titlebar",
                                    div {}
                                    h1 { "Dashboard" }
                                    div {}
                                }
                                div { class: "dashboard-status", "{status_snapshot.line()}" }
                                div { class: "series-section",
                                    h2 { class: "series-section-title", "Actions" }
                                    div { class: "series-grid",
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 0 { "dashboard-card selected" } else { "dashboard-card" },
                                                onclick: move |_| {
                                                    match load_libraries() {
                                                        Ok(items) => {
                                                            libraries.set(items);
                                                            status.write().message = format!("Loaded {} libraries", libraries.read().len());
                                                            dashboard_selection.set(0);
                                                            selected_index.set(0);
                                                            page.set(Page::Libraries);
                                                        }
                                                        Err(err) => status.write().message = format!("Failed to load libraries: {err}"),
                                                    }
                                                },
                                                div { class: "dashboard-card-title", "Kavita" }
                                                div { class: "dashboard-card-subtitle", "{status_snapshot.server_ip}" }
                                            }
                                        }
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 1 { "dashboard-card selected" } else { "dashboard-card" },
                                                onclick: move |_| {
                                                    match clear_cache() {
                                                        Ok(()) => status.set(load_status()),
                                                        Err(err) => status.write().message = format!("Failed to clear cache: {err}"),
                                                    }
                                                },
                                                div { class: "dashboard-card-title", "Clean Cache" }
                                                div { class: "dashboard-card-subtitle", "{status_snapshot.cache_mb}Mb" }
                                            }
                                        }
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 2 { "dashboard-card selected" } else { "dashboard-card" },
                                                onclick: move |_| {
                                                    status.write().message = "Updating server library...".into();
                                                    match update_server_library() {
                                                        Ok(()) => status.write().message = "Server library update requested".into(),
                                                        Err(err) => status.write().message = format!("Server library update failed: {err}"),
                                                    }
                                                },
                                                div { class: "dashboard-card-title", "Update" }
                                                div { class: "dashboard-card-subtitle", "Server Kavita" }
                                            }
                                        }
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 3 { "dashboard-card selected" } else { "dashboard-card" },
                                                onclick: move |_| {
                                                    dashboard_selection.set(3);
                                                    selected_index.set(0);
                                                    page.set(Page::Settings);
                                                },
                                                div { class: "dashboard-card-title", "Settings" }
                                                div { class: "dashboard-card-subtitle", "Configure connection" }
                                            }
                                        }
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 4 { "dashboard-card selected" } else { "dashboard-card" },
                                                onclick: move |_| {
                                                    match toggle_offline() {
                                                        Ok(_) => status.set(load_status()),
                                                        Err(err) => status.write().message = format!("Offline toggle failed: {err}"),
                                                    }
                                                },
                                                div { class: "dashboard-card-title", if status_snapshot.offline_mode { "Offline" } else { "Online" } }
                                                div { class: "dashboard-card-subtitle", if status_snapshot.offline_mode { "Switch to online" } else { "Switch to offline" } }
                                            }
                                        }
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 5 { "dashboard-card selected" } else { "dashboard-card" },
                                                onclick: move |_| exit_process(),
                                                div { class: "dashboard-card-title", "Exit" }
                                                div { class: "dashboard-card-subtitle", "Close app" }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Page::Libraries => rsx! {
                            div { class: "series-page",
                                div { class: "series-titlebar",
                                    div {}
                                    h1 { "Libraries" }
                                    div {}
                                }
                                div { class: "series-section",
                                    h2 { class: "series-section-title",
                                        "Libraries "
                                        span { class: "series-count", "({libraries_snapshot.len()})" }
                                    }
                                    div { class: "series-grid",
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 0 { "back-card selected" } else { "back-card" },
                                                onclick: move |_| {
                                                    selected_index.set(dashboard_selection());
                                                    page.set(Page::Dashboard);
                                                },
                                                div { class: "back-card-arrow", "←" }
                                                div { class: "back-card-title", "Dashboard" }
                                            }
                                        }
                                        {libraries_snapshot.iter().enumerate().map(|(index, library)| {
                                            let id = library.id;
                                            let title = library.title.clone();
                                            let class = if selected_snapshot == index + 1 { "library-card selected" } else { "library-card" };
                                            rsx! {
                                                div { class: "series-card-wrap",
                                                    button {
                                                        class: "{class}",
                                                        onclick: move |_| {
                                                            match load_series(id) {
                                                                Ok(items) => {
                                                                    series.set(items);
                                                                    status.write().message = format!("Loaded series for library id {id}");
                                                                    libraries_selection.set(index + 1);
                                                                    selected_index.set(0);
                                                                    page.set(Page::Series);
                                                                }
                                                                Err(err) => status.write().message = format!("Failed to load series: {err}"),
                                                            }
                                                        },
                                                        div { class: "library-card-title", "{title}" }
                                                    }
                                                }
                                            }
                                        })}
                                    }
                                }
                            }
                        },
                        Page::Series => rsx! {
                            div { class: "series-page",
                                div { class: "series-titlebar",
                                    div {}
                                    h1 { "Series" }
                                    div {}
                                }
                                if !in_progress_series.is_empty() {
                                    div { class: "series-section",
                                        h2 { class: "series-section-title",
                                            "In progress "
                                            span { class: "series-count", "({in_progress_series.len()})" }
                                        }
                                        div { class: "series-grid",
                                            div { class: "series-card-wrap",
                                                button {
                                                    class: if selected_snapshot == 0 { "back-card selected" } else { "back-card" },
                                                    onclick: move |_| {
                                                        selected_index.set(libraries_selection());
                                                        page.set(Page::Libraries);
                                                    },
                                                    div { class: "back-card-arrow", "←" }
                                                    div { class: "back-card-title", "Libraries" }
                                                }
                                            }
                                            {in_progress_series.iter().enumerate().map(|(index, item)| {
                                                let id = item.id;
                                                let title = item.title.clone();
                                                let progress = item.read.clamp(0, 100);
                                                let cover_url = format!("http://localhost:11337/api/series-cover/{id}?thumb=1");
                                                let class = if selected_snapshot == index + 1 { "series-card selected" } else { "series-card" };
                                                rsx! {
                                                    div { class: "series-card-wrap",
                                                        button {
                                                            class: "{class}",
                                                            style: "background-image: url('{cover_url}');",
                                                            onclick: move |_| {
                                                                match load_volumes(id) {
                                                                    Ok(items) => {
                                                                        volumes.set(items);
                                                                        status.write().message = format!("Loaded volumes for series id {id}");
                                                                        series_selection.set(index + 1);
                                                                        selected_index.set(0);
                                                                        page.set(Page::Volumes);
                                                                    }
                                                                    Err(err) => status.write().message = format!("Failed to load volumes: {err}"),
                                                                }
                                                            },
                                                            div { class: "series-progress",
                                                                div {
                                                                    class: "series-progress-fill",
                                                                    style: "width: {progress}%;"
                                                                }
                                                            }
                                                            div { class: "series-card-title", "{title}" }
                                                        }
                                                    }
                                                }
                                            })}
                                        }
                                    }
                                }
                                if !completed_series.is_empty() {
                                    div { class: "series-section",
                                        h2 { class: "series-section-title",
                                            "Completed "
                                            span { class: "series-count", "({completed_series.len()})" }
                                        }
                                        div { class: "series-grid",
                                            if in_progress_series.is_empty() {
                                                div { class: "series-card-wrap",
                                                    button {
                                                        class: if selected_snapshot == 0 { "back-card selected" } else { "back-card" },
                                                        onclick: move |_| {
                                                            selected_index.set(libraries_selection());
                                                            page.set(Page::Libraries);
                                                        },
                                                        div { class: "back-card-arrow", "←" }
                                                        div { class: "back-card-title", "Libraries" }
                                                    }
                                                }
                                            }
                                            {completed_series.iter().enumerate().map(|(index, item)| {
                                                let id = item.id;
                                                let title = item.title.clone();
                                                let progress = item.read.clamp(0, 100);
                                                let cover_url = format!("http://localhost:11337/api/series-cover/{id}?thumb=1");
                                                let nav_index = in_progress_series.len() + index + 1;
                                                let class = if selected_snapshot == nav_index { "series-card selected" } else { "series-card" };
                                                rsx! {
                                                    div { class: "series-card-wrap",
                                                        button {
                                                            class: "{class}",
                                                            style: "background-image: url('{cover_url}');",
                                                            onclick: move |_| {
                                                                match load_volumes(id) {
                                                                    Ok(items) => {
                                                                        volumes.set(items);
                                                                        status.write().message = format!("Loaded volumes for series id {id}");
                                                                        series_selection.set(nav_index);
                                                                        selected_index.set(0);
                                                                        page.set(Page::Volumes);
                                                                    }
                                                                    Err(err) => status.write().message = format!("Failed to load volumes: {err}"),
                                                                }
                                                            },
                                                            div { class: "series-progress",
                                                                div {
                                                                    class: "series-progress-fill complete",
                                                                    style: "width: {progress}%;"
                                                                }
                                                            }
                                                            div { class: "series-card-title complete", "{title}" }
                                                        }
                                                    }
                                                }
                                            })}
                                        }
                                    }
                                }
                                if in_progress_series.is_empty() && completed_series.is_empty() {
                                    div { class: "series-section",
                                        h2 { class: "series-section-title", "Series" }
                                        div { class: "series-grid",
                                            div { class: "series-card-wrap",
                                                button {
                                                    class: if selected_snapshot == 0 { "back-card selected" } else { "back-card" },
                                                    onclick: move |_| {
                                                        selected_index.set(libraries_selection());
                                                        page.set(Page::Libraries);
                                                    },
                                                    div { class: "back-card-arrow", "←" }
                                                    div { class: "back-card-title", "Libraries" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Page::Volumes => rsx! {
                            div { class: "series-page",
                                div { class: "series-titlebar",
                                    div {}
                                    h1 { "Volumes" }
                                    div {}
                                }
                                div { class: "series-section",
                                    h2 { class: "series-section-title",
                                        "Volumes "
                                        span { class: "series-count", "({volumes_snapshot.len()})" }
                                    }
                                    div { class: "series-grid",
                                        div { class: "series-card-wrap",
                                            button {
                                                class: if selected_snapshot == 0 { "back-card selected" } else { "back-card" },
                                                onclick: move |_| {
                                                    selected_index.set(series_selection());
                                                    page.set(Page::Series);
                                                },
                                                div { class: "back-card-arrow", "←" }
                                                div { class: "back-card-title", "Series" }
                                            }
                                        }
                                        {volumes_snapshot.iter().enumerate().map(|(index, volume)| {
                                            let cover_id = volume.volume_id;
                                            let title = volume.title.clone();
                                            let reader_item = volume.clone();
                                            let progress = if volume.pages > 0 {
                                                ((volume.read * 100) / volume.pages).clamp(0, 100)
                                            } else {
                                                0
                                            };
                                            let complete = volume.pages > 0 && volume.read >= volume.pages;
                                            let cached = volume.is_cached;
                                            let cover_url = format!("http://localhost:11337/api/volumes-cover/{cover_id}?thumb=1");
                                            let class = if selected_snapshot == index + 1 { "series-card selected" } else { "series-card" };
                                            let progress_class = if complete {
                                                "series-progress-fill complete"
                                            } else if cached {
                                                "series-progress-fill cached"
                                            } else {
                                                "series-progress-fill"
                                            };
                                            let title_class = if complete {
                                                "series-card-title complete"
                                            } else if cached {
                                                "series-card-title cached"
                                            } else {
                                                "series-card-title"
                                            };
                                            rsx! {
                                                div { class: "series-card-wrap",
                                                    button {
                                                        class: "{class}",
                                                        style: "background-image: url('{cover_url}');",
                                                        onclick: move |_| {
                                                            volumes_selection.set(index + 1);
                                                            let start_page = initial_reader_page(&reader_item);
                                                            reader_page.set(start_page);
                                                            reader_first_page.set(start_page);
                                                            reader_loaded_until.set(reader_batch_end(start_page, reader_item.pages));
                                                            reader_slider_visible.set(false);
                                                            reader_volume.set(Some(reader_item.clone()));
                                                            page.set(Page::Reader);
                                                        },
                                                        div { class: "series-progress",
                                                            div {
                                                                class: "{progress_class}",
                                                                style: "width: {progress}%;"
                                                            }
                                                        }
                                                        div { class: "{title_class}", "{title}" }
                                                    }
                                                }
                                            }
                                        })}
                                    }
                                }
                            }
                        },
                        Page::Reader => rsx! {
                            div {
                                class: "reader-page",
                                onscroll: move |event| {
                                    if let Some(volume) = reader_volume.read().as_ref() {
                                        let data = event.data();
                                        let near_bottom = data.scroll_top()
                                            + data.client_height() as f64
                                            >= data.scroll_height() as f64 - 900.0;
                                        if near_bottom && reader_loaded_until() < volume.pages.saturating_sub(1) {
                                            reader_loaded_until.set(next_reader_batch_end(reader_loaded_until(), volume.pages));
                                        }
                                    }
                                },
                                {
                                    if let Some(volume) = reader_volume.read().clone() {
                                        let current = reader_page().clamp(0, volume.pages.saturating_sub(1));
                                        let display_page = current + 1;
                                        let total_pages = volume.pages.max(0);
                                        let title = volume.title.clone();
                                        let first_page = reader_first_page().clamp(0, volume.pages.saturating_sub(1));
                                        let loaded_until = reader_loaded_until()
                                            .clamp(first_page, volume.pages.saturating_sub(1));
                                        let loaded_count = if volume.pages > 0 {
                                            loaded_until - first_page + 1
                                        } else {
                                            0
                                        };
                                        rsx! {
                                            div { class: "reader-bar",
                                                button {
                                                    class: "reader-back",
                                                    onclick: move |_| {
                                                        let series_id = volume.series_id;
                                                        match load_volumes(series_id) {
                                                            Ok(items) => volumes.set(items),
                                                            Err(err) => status.write().message = format!("Failed to refresh volumes: {err}"),
                                                        }
                                                        selected_index.set(volumes_selection());
                                                        page.set(Page::Volumes);
                                                    },
                                                    "← Back"
                                                }
                                                div { class: "reader-title", "{title}" }
                                                div { class: "reader-count", "{display_page}/{total_pages}" }
                                            }
                                            div { class: "reader-stage",
                                                if volume.pages > 0 {
                                                    {(first_page..=loaded_until).map(|page_number| {
                                                        let image_url = format!(
                                                            "http://localhost:11337/api/picture/{}/{}/{}/{}",
                                                            volume.series_id,
                                                            volume.volume_id,
                                                            volume.chapter_id,
                                                            page_number
                                                        );
                                                        let page_label = page_number + 1;
                                                        rsx! {
                                                            div {
                                                                class: "reader-image-wrap",
                                                                "data-reader-page": "{page_number}",
                                                                img {
                                                                    class: "reader-image",
                                                                    src: "{image_url}",
                                                                    alt: "Page {page_label}",
                                                                }
                                                                div { class: "reader-page-label", "{page_label}/{total_pages}" }
                                                            }
                                                        }
                                                    })}
                                                    div { class: "reader-loading",
                                                        if loaded_until < volume.pages.saturating_sub(1) {
                                                            "Scroll down to load the next 10 pages ({loaded_count}/{total_pages} loaded)"
                                                        } else {
                                                            "All pages loaded ({loaded_count}/{total_pages})"
                                                        }
                                                    }
                                                } else {
                                                    div { class: "reader-empty", "This volume has no pages" }
                                                }
                                            }
                                            if reader_slider_visible() && volume.pages > 0 {
                                                div { class: "reader-slider",
                                                    div { class: "reader-slider-meta",
                                                        span { "0" }
                                                        span { "Page {display_page} / {total_pages}" }
                                                        span { "{volume.pages.saturating_sub(1)}" }
                                                    }
                                                    input {
                                                        r#type: "range",
                                                        min: "0",
                                                        max: "{volume.pages.saturating_sub(1)}",
                                                        value: "{current}",
                                                        readonly: true,
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div { class: "reader-bar",
                                                button {
                                                    class: "reader-back",
                                                    onclick: move |_| {
                                                        if let Some(volume) = reader_volume.read().as_ref() {
                                                            let series_id = volume.series_id;
                                                            match load_volumes(series_id) {
                                                                Ok(items) => volumes.set(items),
                                                                Err(err) => status.write().message = format!("Failed to refresh volumes: {err}"),
                                                            }
                                                        }
                                                        selected_index.set(volumes_selection());
                                                        page.set(Page::Volumes);
                                                    },
                                                    "← Back"
                                                }
                                                div { class: "reader-title", "Reader" }
                                                div {}
                                            }
                                            div { class: "reader-stage",
                                                div { class: "reader-empty", "No volume selected" }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Page::Settings => rsx! {
                            div { class: "settings",
                                input {
                                    value: "{server_ip.read()}",
                                    placeholder: "Server IP",
                                    oninput: move |event| server_ip.set(event.value()),
                                }
                                input {
                                    value: "{username.read()}",
                                    placeholder: "Username",
                                    oninput: move |event| username.set(event.value()),
                                }
                                input {
                                    value: "{password.read()}",
                                    placeholder: "Password",
                                    oninput: move |event| password.set(event.value()),
                                }
                                input {
                                    value: "{api_key.read()}",
                                    placeholder: "API Key",
                                    oninput: move |event| api_key.set(event.value()),
                                }
                                button {
                                    class: if selected_snapshot == 0 { "tile selected" } else { "tile" },
                                    onclick: move |_| {
                                        match save_settings(
                                            server_ip.read().clone(),
                                            username.read().clone(),
                                            password.read().clone(),
                                            api_key.read().clone(),
                                        ) {
                                            Ok(()) => status.set(load_status()),
                                            Err(err) => status.write().message = format!("Failed to save settings: {err}"),
                                        }
                                    },
                                    div { class: "title", "Save" }
                                    div { class: "subtitle", "Reconnect" }
                                }
                                button {
                                    class: if selected_snapshot == 1 { "tile selected" } else { "tile" },
                                    onclick: move |_| {
                                        selected_index.set(dashboard_selection());
                                        page.set(Page::Dashboard);
                                    },
                                    div { class: "title", "Dashboard" }
                                    div { class: "subtitle", "Home" }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

pub fn run_ui(kavita: SharedKavita) {
    let _ = KAVITA.set(kavita);
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("Manga4Deck")
                        .with_inner_size(LogicalSize::new(1280.0, 720.0)),
                )
                .with_menu(None),
        )
        .launch(app);
}
