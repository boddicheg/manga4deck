use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use dioxus::desktop::{tao::dpi::LogicalSize, Config, WindowBuilder};
use dioxus::prelude::*;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

use crate::kavita::{
    get_cache_size, ConnectionCreds, Kavita, Library, ReadProgress, Series, Volume,
};

#[path = "ember_ui.rs"]
mod ember_ui;
use ember_ui::{EmberAction, EmberScreen, HomeRead};

pub type SharedKavita = Arc<Mutex<Kavita>>;

static KAVITA: OnceCell<SharedKavita> = OnceCell::new();
const READER_BATCH_SIZE: i32 = 10;
const SHELF_COLUMNS: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Libraries,
    Series,
    Volumes,
    Reader,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
struct UiStatus {
    server_ip: String,
    logged_as: String,
    saved_username: String,
    offline_mode: bool,
    cache_mb: u64,
    message: String,
}

fn kavita() -> SharedKavita {
    KAVITA
        .get()
        .expect("Dioxus UI started before Kavita state was initialized")
        .clone()
}

async fn snapshot() -> Kavita {
    let shared = kavita();
    let guard = shared.lock().await;
    guard.clone()
}

async fn load_status() -> UiStatus {
    let kavita = snapshot().await;
    UiStatus {
        server_ip: kavita.ip.clone(),
        logged_as: kavita.logged_as.clone(),
        saved_username: kavita
            .get_setting("username")
            .ok()
            .flatten()
            .unwrap_or_default(),
        offline_mode: kavita.offline_mode,
        cache_mb: get_cache_size(1024 * 1024),
        message: if kavita.offline_mode {
            "Offline mode is active".into()
        } else {
            "Connected to Kavita server".into()
        },
    }
}

async fn load_libraries() -> Result<Vec<Library>, String> {
    snapshot()
        .await
        .get_libraries()
        .await
        .map_err(|err| err.to_string())
}

async fn load_home_read() -> Result<Option<HomeRead>, String> {
    let state = snapshot().await;
    tokio::task::spawn_blocking(move || {
        let candidates = state.db.continue_reading().map_err(|err| err.to_string())?;
        Ok(candidates
            .into_iter()
            .find_map(|(volume, title, library_id)| {
                if state.offline_mode && !state.db.is_volume_fully_cached(volume.id) {
                    return None;
                }
                Some(HomeRead {
                    volume,
                    title,
                    library_id,
                })
            }))
    })
    .await
    .map_err(|err| err.to_string())?
}

async fn load_downloaded_series() -> Result<Vec<Series>, String> {
    let state = snapshot().await;
    tokio::task::spawn_blocking(move || {
        let mut items = Vec::new();
        for library in state.db.get_libraries().map_err(|err| err.to_string())? {
            for item in state
                .db
                .get_series(&library.id)
                .map_err(|err| err.to_string())?
            {
                let ready = state
                    .db
                    .get_volumes(&item.id)
                    .map_err(|err| err.to_string())?
                    .iter()
                    .any(|v| state.db.is_volume_fully_cached(v.id));
                if ready || state.is_series_caching(item.id) {
                    items.push(item);
                }
            }
        }
        Ok(items)
    })
    .await
    .map_err(|err| err.to_string())?
}

async fn load_series(library_id: i32) -> Result<Vec<Series>, String> {
    snapshot()
        .await
        .get_series(&library_id)
        .await
        .map_err(|err| err.to_string())
}

async fn load_volumes(series_id: i32) -> Result<Vec<Volume>, String> {
    snapshot()
        .await
        .get_volumes(&series_id)
        .await
        .map_err(|err| err.to_string())
}

async fn load_cached_volumes(series_id: i32) -> Result<Vec<Volume>, String> {
    snapshot()
        .await
        .get_cached_volumes(&series_id)
        .map_err(|err| err.to_string())
}

async fn toggle_volume_read(volume: &Volume) -> Result<bool, String> {
    let kavita = snapshot().await;
    toggle_reader_item(&kavita, volume).await
}

async fn toggle_reader_item(kavita: &Kavita, volume: &Volume) -> Result<bool, String> {
    if kavita.offline_mode {
        return Err("Go online to change a chapter's read status".into());
    }
    let is_complete = volume.pages > 0 && volume.read >= volume.pages;
    if is_complete {
        kavita
            .set_volume_as_unread(&volume.series_id, &volume.id)
            .await
    } else {
        kavita
            .set_volume_as_read(&volume.series_id, &volume.id)
            .await
    }
    .map_err(|err| err.to_string())?;
    Ok(!is_complete)
}

async fn toggle_series_caching(series_id: i32) -> Result<bool, String> {
    let kavita = snapshot().await;
    if kavita.is_series_caching(series_id) {
        kavita.stop_cache_serie(series_id);
        Ok(false)
    } else {
        if kavita.offline_mode {
            return Err("Go online before downloading a series".into());
        }
        kavita.cache_serie(series_id);
        Ok(true)
    }
}

async fn is_series_caching(series_id: i32) -> bool {
    snapshot().await.is_series_caching(series_id)
}

async fn is_series_fully_cached(series_id: i32) -> bool {
    snapshot().await.is_series_cached(series_id)
}

async fn remove_series_cache(series_id: i32) -> Result<(), String> {
    snapshot()
        .await
        .remove_series_cache(series_id)
        .map_err(|err| err.to_string())
}

async fn clear_cache() -> Result<(), String> {
    snapshot()
        .await
        .clear_cache()
        .map_err(|err| err.to_string())
}

async fn update_server_library() -> Result<(), String> {
    let state = snapshot().await;
    if state.offline_mode {
        return Err("Go online to update the server library".into());
    }
    state
        .update_server_library()
        .await
        .map_err(|err| err.to_string())
}

async fn toggle_offline() -> Result<bool, String> {
    let mut state = snapshot().await;
    state.offline_mode = !state.offline_mode;
    let result = if state.offline_mode {
        state.send_connection_status(true, "");
        Ok(())
    } else {
        state
            .reconnect_with_creds()
            .await
            .map_err(|err| err.to_string())
    };
    let offline = state.offline_mode;
    *kavita().lock().await = state;
    result.map(|()| offline)
}

fn store_settings(
    state: &Kavita,
    server_ip: &str,
    username: &str,
    password: &str,
    api_key: &str,
) -> Result<(), String> {
    state
        .insert_setting("ip", server_ip.trim())
        .map_err(|err| err.to_string())?;
    state
        .insert_setting("username", username.trim())
        .map_err(|err| err.to_string())?;
    // Secrets are deliberately empty in the form after startup. An empty field
    // means keep the saved credential, so editing the address cannot erase it.
    if !password.is_empty() {
        state
            .insert_setting("password", password)
            .map_err(|err| err.to_string())?;
    }
    if !api_key.trim().is_empty() {
        state
            .insert_setting("api_key", api_key.trim())
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn save_settings(
    server_ip: String,
    username: String,
    password: String,
    api_key: String,
) -> Result<(), String> {
    let mut state = snapshot().await;
    store_settings(&state, &server_ip, &username, &password, &api_key)?;
    let result = state
        .reconnect_with_creds()
        .await
        .map_err(|err| err.to_string());
    *kavita().lock().await = state;
    result
}

async fn connect_settings(
    creds: ConnectionCreds,
    mut status: Signal<UiStatus>,
    mut toast: Signal<Option<String>>,
    mut toast_generation: Signal<u64>,
) {
    let result = save_settings(creds.ip, creds.username, creds.password, creds.api_key).await;
    status.set(load_status().await);
    let message = match result {
        Ok(()) => "Connected to Kavita. Settings saved.".to_string(),
        Err(err) => format!("Could not connect: {err}"),
    };
    show_toast(&mut status, &mut toast, &mut toast_generation, message);
    document::eval("document.querySelector('.settings-save')?.focus({ preventScroll: true });");
}

fn run_ui_action(mut busy: Signal<bool>, action: impl std::future::Future<Output = ()> + 'static) {
    if busy() {
        return;
    }
    busy.set(true);
    spawn(async move {
        if tokio::time::timeout(Duration::from_secs(20), action)
            .await
            .is_err()
        {
            push_toast("The request timed out. Please try again.".into());
        }
        busy.set(false);
    });
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
        Page::Dashboard => 1,
        Page::Libraries => libraries.len() + 1,
        Page::Series => series.len() + 1,
        Page::Volumes => volumes.len() + 1,
        Page::Reader => 0,
        Page::Settings => 6,
    }
}

fn tile_columns(page: Page, shelf_columns: usize) -> usize {
    match page {
        Page::Libraries | Page::Series | Page::Volumes => shelf_columns.max(1),
        Page::Dashboard => 1,
        Page::Reader => 1,
        Page::Settings => 3,
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

fn move_series_selection(
    current: usize,
    key: &str,
    in_progress_count: usize,
    completed_count: usize,
    columns: usize,
) -> usize {
    let columns = columns.max(1);
    if in_progress_count == 0 || completed_count == 0 {
        return move_selection(
            current,
            key,
            in_progress_count + completed_count + 1,
            columns,
        );
    }

    match key {
        "ArrowDown" if current <= in_progress_count => {
            let target = current + columns;
            if current / columns < in_progress_count / columns {
                target.min(in_progress_count)
            } else {
                let column = current % columns;
                in_progress_count + 1 + column.min(completed_count - 1)
            }
        }
        "ArrowUp" if current > in_progress_count => {
            let completed_index = current - in_progress_count - 1;
            if completed_index >= columns {
                current - columns
            } else {
                let column = completed_index % columns;
                let last_in_progress_row = (in_progress_count / columns) * columns;
                (last_in_progress_row + column).min(in_progress_count)
            }
        }
        _ => move_selection(
            current,
            key,
            in_progress_count + completed_count + 1,
            columns,
        ),
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

fn series_selection_after_refresh(
    previous: &[Series],
    selected: usize,
    updated: &[Series],
) -> usize {
    let Some(previous) = selected_series(previous, selected) else {
        return 0;
    };
    updated
        .iter()
        .filter(|s| s.read < 100)
        .chain(updated.iter().filter(|s| s.read >= 100))
        .position(|series| series.id == previous.id)
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn replace_series(
    items: Vec<Series>,
    mut series: Signal<Vec<Series>>,
    mut remembered: Signal<usize>,
    mut selected: Signal<usize>,
    page: Signal<Page>,
) {
    let current = if page() == Page::Series {
        selected()
    } else {
        remembered()
    };
    let next = series_selection_after_refresh(&series.peek(), current, &items);
    series.set(items);
    remembered.set(next);
    if page() == Page::Series {
        selected.set(next);
    }
}

fn current_series_library_id(
    series: &[Series],
    libraries: &[Library],
    libraries_selection: usize,
) -> Option<i32> {
    series.first().map(|item| item.library_id).or_else(|| {
        libraries_selection
            .checked_sub(1)
            .and_then(|index| libraries.get(index))
            .map(|library| library.id)
    })
}

fn first_non_complete_volume_selection(volumes: &[Volume]) -> usize {
    volumes
        .iter()
        .position(|volume| volume.pages <= 0 || volume.read < volume.pages)
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn initial_reader_page(volume: &Volume) -> i32 {
    if volume.pages <= 0 {
        0
    } else {
        // `read` is a count; resume at the last viewed page, not the next image.
        volume.read.saturating_sub(1).clamp(0, volume.pages - 1)
    }
}

struct ReaderPages {
    current: i32,
    display_page: i32,
    total_pages: i32,
    first_page: i32,
    loaded_until: i32,
    loaded_count: i32,
}

impl ReaderPages {
    fn new(pages: i32, current: i32, first: i32, loaded: i32) -> Self {
        let total_pages = pages.max(0);
        let max_page = total_pages.saturating_sub(1).max(0);
        let current = current.clamp(0, max_page);
        let first_page = first.clamp(0, max_page);
        let loaded_until = loaded.clamp(first_page, max_page);
        Self {
            current,
            display_page: if total_pages > 0 { current + 1 } else { 0 },
            total_pages,
            first_page,
            loaded_until,
            loaded_count: if total_pages > 0 {
                loaded_until - first_page + 1
            } else {
                0
            },
        }
    }
}

fn track_reader(
    volume: Volume,
    page: Signal<Page>,
    reader_volume: Signal<Option<Volume>>,
    mut reader_page: Signal<i32>,
) {
    spawn(async move {
        let mut observer = document::eval(include_str!("reader.js"));
        while let Ok(visible_page) = observer.recv::<i32>().await {
            if page() != Page::Reader
                || reader_volume.read().as_ref().map(|v| v.chapter_id) != Some(volume.chapter_id)
            {
                break;
            }
            if !(0..volume.pages).contains(&visible_page) {
                continue;
            }
            reader_page.set(visible_page);
            let snapshot = {
                let shared = kavita();
                let guard = shared.lock().await;
                guard.clone()
            };
            let progress = ReadProgress {
                id: None,
                library_id: snapshot
                    .get_series_library_id(volume.series_id)
                    .unwrap_or(0),
                series_id: volume.series_id,
                volume_id: volume.volume_id,
                chapter_id: volume.chapter_id,
                page: visible_page,
            };
            if let Err(err) = snapshot.insert_setting("last_read_volume", &volume.id.to_string()) {
                push_toast(format!("Failed to remember the current volume: {err}"));
            }
            if let Err(err) = snapshot.save_progress(&progress).await {
                push_toast(format!("Failed to save reading progress: {err}"));
            }
        }
    });
}

fn seek_reader(
    target: i32,
    pages: i32,
    mut current: Signal<i32>,
    mut first: Signal<i32>,
    mut loaded: Signal<i32>,
    mut slider_visible: Signal<bool>,
    mut slider_generation: Signal<u64>,
) {
    if pages <= 0 {
        return;
    }
    let target = target.clamp(0, pages - 1);
    current.set(target);
    if target < first() {
        first.set(target);
    }
    if target > loaded() {
        loaded.set(reader_batch_end(target, pages));
    }
    slider_visible.set(true);
    slider_generation.set(slider_generation().wrapping_add(1));
    let seek = document::eval(include_str!("seek.js"));
    let _ = seek.send(target);
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

fn show_toast(
    status: &mut Signal<UiStatus>,
    toast: &mut Signal<Option<String>>,
    toast_generation: &mut Signal<u64>,
    message: String,
) {
    status.write().message = message.clone();
    toast.set(Some(message));
    toast_generation.set(toast_generation().wrapping_add(1));
}

fn push_toast(message: String) {
    let message = serde_json::to_string(&message).unwrap_or_else(|_| "\"\"".to_string());
    document::eval(&format!(
        r#"
        (() => {{
            const existing = document.querySelector(".toast.cache-toast");
            if (existing) existing.remove();
            const toast = document.createElement("div");
            toast.className = "toast cache-toast";
            toast.textContent = {};
            document.body.appendChild(toast);
            setTimeout(() => toast.remove(), 3000);
        }})();
        "#,
        message
    ));
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
.busy-indicator {
  position: fixed;
  top: 8px;
  right: 12px;
  z-index: 110;
  padding: 8px 14px;
  background: #111827;
  border-radius: 4px;
  color: #fff;
}
.app[aria-busy="true"] button {
  pointer-events: none;
}
.app:focus {
  outline: none;
}
.app.series-mode {
  padding: 0;
  background: #050505;
  overflow-x: hidden;
}
.tiles {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: 8px;
}
.toast {
  position: fixed;
  left: 50%;
  bottom: 24px;
  z-index: 100;
  max-width: min(680px, calc(100vw - 40px));
  box-sizing: border-box;
  padding: 12px 18px;
  border: 1px solid rgba(96,165,250,0.55);
  border-radius: 6px;
  background: rgba(8,12,20,0.92);
  color: #eef6ff;
  box-shadow: 0 10px 28px rgba(0,0,0,0.45), 0 0 20px rgba(56,189,248,0.18);
  transform: translateX(-50%);
  font-size: 15px;
  font-weight: 800;
  line-height: 1.2;
  text-align: center;
  pointer-events: none;
  animation: toast-in 140ms ease-out;
}
@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateX(-50%) translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }
}
.series-page {
  width: 100%;
  min-height: 100vh;
  scroll-behavior: smooth;
  background:
    repeating-linear-gradient(45deg, rgba(0,0,0,0.05) 0px, rgba(0,0,0,0.05) 2px, transparent 2px, transparent 4px),
    repeating-linear-gradient(-45deg, rgba(0,0,0,0.04) 0px, rgba(0,0,0,0.04) 2px, transparent 2px, transparent 4px),
    linear-gradient(90deg, #8a552c 0%, #a66a3a 44%, #805026 100%);
}
.series-titlebar {
  height: 34px;
  display: grid;
  grid-template-columns: 120px 1fr 120px;
  align-items: center;
  background: linear-gradient(#060606, #010101);
  border-bottom: 1px solid #2a2a2a;
}
.series-titlebar h1 {
  margin: 0;
  text-align: center;
  font-size: 18px;
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
  padding: 16px 16px 42px;
}
.series-section-title {
  margin: 0 0 18px;
  color: #fff;
  font-size: 18px;
  font-weight: 900;
  text-shadow: 0 2px 4px rgba(0,0,0,0.45);
}
.series-count {
  color: rgba(255,255,255,0.72);
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
  scroll-margin: 96px 0 72px;
  box-shadow: 8px -8px 8px rgba(0,0,0,0.22), 3px -3px 4px rgba(0,0,0,0.16), inset 0 0 0 1px rgba(255,255,255,0.10);
  transform: translateY(0) scale(1);
  transition: transform 120ms ease, box-shadow 120ms ease, border-color 120ms ease;
}
.series-card:focus,
.series-card:hover {
  outline: none;
  border: 2px solid rgba(248,250,252,0.8);
  box-shadow:
    8px -8px 12px rgba(0,0,0,0.44),
    3px -3px 6px rgba(0,0,0,0.34),
    0 0 0 2px rgba(59,130,246,0.56);
}
.series-card.selected {
  outline: none;
  border: 2px solid #60a5fa;
  transform: translateY(-6px) scale(1.04);
  box-shadow:
    10px -10px 18px rgba(0,0,0,0.54),
    4px -4px 8px rgba(0,0,0,0.38),
    0 0 0 3px rgba(96,165,250,0.95),
    0 0 0 6px rgba(17,24,39,0.88),
    0 0 24px rgba(56,189,248,0.64);
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
.series-card-title span {
  display: block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
}
.series-card.selected .series-card-title {
  justify-content: flex-start;
}
.series-card.selected .series-card-title span {
  max-width: none;
  min-width: 100%;
  overflow: visible;
  text-overflow: clip;
  animation: title-marquee 7s linear infinite;
}
@keyframes title-marquee {
  0%, 18% {
    transform: translateX(0);
  }
  82%, 100% {
    transform: translateX(calc(-100% + 134px));
  }
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
.reader-page {
  width: 100vw;
  height: 100vh;
  overflow-y: auto;
  background: #111216;
  color: #fff;
}
.reader-bar {
  height: 34px;
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
  background: transparent;
  border: none;
  cursor: pointer;
  padding-right: 14px;
  text-align: right;
  color: rgba(255,255,255,0.74);
  font-size: 13px;
  font-weight: 700;
}
.reader-stage {
  min-height: calc(100vh - 34px);
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
    let mut status = use_signal(|| tokio::runtime::Handle::current().block_on(load_status()));
    let busy = use_signal(|| false);
    let mut shelf_columns = use_signal(|| SHELF_COLUMNS);
    let mut page = use_signal(|| Page::Dashboard);
    let mut selected_index = use_signal(|| 0usize);
    let mut dashboard_selection = use_signal(|| 0usize);
    let mut libraries_selection = use_signal(|| 0usize);
    let mut series_selection = use_signal(|| 0usize);
    let mut volumes_selection = use_signal(|| 0usize);
    let mut toast = use_signal(|| None::<String>);
    let mut toast_generation = use_signal(|| 0u64);
    let mut libraries = use_signal(Vec::<Library>::new);
    let mut series = use_signal(Vec::<Series>::new);
    let mut volumes = use_signal(Vec::<Volume>::new);
    let mut reader_volume = use_signal(|| None::<Volume>);
    let mut reader_page = use_signal(|| 0i32);
    let mut reader_first_page = use_signal(|| 0i32);
    let mut reader_loaded_until = use_signal(|| 0i32);
    let mut reader_slider_visible = use_signal(|| false);
    let mut reader_slider_generation = use_signal(|| 0u64);
    let mut cache_events_started = use_signal(|| false);

    let mut home_read = use_signal(|| None::<HomeRead>);
    let mut home_loading = use_signal(|| true);
    let mut downloaded_series = use_signal(Vec::<Series>::new);
    let mut home_generation = use_signal(|| 0u64);
    use_effect(move || {
        let current = page();
        let _offline = status.read().offline_mode;
        let generation = (*home_generation.peek()).wrapping_add(1);
        home_generation.set(generation);
        if matches!(current, Page::Dashboard | Page::Settings) {
            if current == Page::Dashboard {
                home_loading.set(true);
            }
            spawn(async move {
                if current == Page::Dashboard {
                    let result = load_home_read().await;
                    if *home_generation.peek() != generation {
                        return;
                    }
                    match result {
                        Ok(read) => home_read.set(read),
                        Err(err) => {
                            home_read.set(None);
                            push_toast(format!("Could not load reading progress: {err}"));
                        }
                    }
                    home_loading.set(false);
                }
                let result = load_downloaded_series().await;
                if *home_generation.peek() != generation {
                    return;
                }
                match result {
                    Ok(items) => downloaded_series.set(items),
                    Err(err) => push_toast(format!("Could not load downloads: {err}")),
                }
            });
        }
    });

    use_effect(move || {
        let _ = selected_index();
        if page() == Page::Reader {
            document::eval("document.querySelector('.app')?.focus({ preventScroll: true });");
        }
        if matches!(
            page(),
            Page::Dashboard | Page::Libraries | Page::Series | Page::Volumes
        ) {
            document::eval(
                r#"
                setTimeout(() => {
                    const selected = document.querySelector(".series-page .selected");
                    if (selected) {
                        selected.focus({ preventScroll: true });
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

    use_effect(move || {
        let generation = toast_generation();
        if toast.read().is_some() {
            spawn(async move {
                tokio::time::sleep(Duration::from_secs(3)).await;
                if toast_generation() == generation {
                    toast.set(None);
                }
            });
        }
    });

    use_effect(move || {
        if cache_events_started() {
            return;
        }
        cache_events_started.set(true);

        spawn(async move {
            let mut rx = {
                let kavita = kavita();
                let kavita = kavita.lock().await;
                kavita.ws_sender.as_ref().map(|sender| sender.subscribe())
            };

            let Some(ref mut rx) = rx else {
                return;
            };

            loop {
                let msg = match rx.recv().await {
                    Ok(msg) => msg,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                let event = msg["event"].as_str().unwrap_or("");
                if event == "connection_status" {
                    let mut current = status.write();
                    current.offline_mode = !msg["data"]["connected"].as_bool().unwrap_or(false);
                    current.logged_as = msg["data"]["username"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    continue;
                }
                if !matches!(
                    event,
                    "volume_caching_start" | "volume_cached" | "caching_end" | "caching_cancelled"
                ) {
                    continue;
                }

                if let Some(message) = msg["message"].as_str() {
                    push_toast(message.to_string());
                }

                let event_series_id = msg["data"]["series_id"].as_i64().map(|id| id as i32);
                let should_refresh_volumes = event_series_id.map(|series_id| {
                    if page() == Page::Volumes {
                        volumes
                            .read()
                            .first()
                            .map(|volume| volume.series_id == series_id)
                            .unwrap_or(false)
                    } else if page() == Page::Reader {
                        reader_volume
                            .read()
                            .as_ref()
                            .map(|volume| volume.series_id == series_id)
                            .unwrap_or(false)
                    } else {
                        false
                    }
                });

                if should_refresh_volumes.unwrap_or(false) {
                    if let Some(series_id) = event_series_id {
                        let refreshed = {
                            let kavita = kavita();
                            let kavita = kavita.lock().await;
                            kavita
                                .get_cached_volumes(&series_id)
                                .map_err(|err| err.to_string())
                        };
                        match refreshed {
                            Ok(items) => volumes.set(items),
                            Err(err) => {
                                push_toast(format!("Failed to refresh cached volumes: {err}"))
                            }
                        }
                    }
                }

                if matches!(event, "caching_end" | "caching_cancelled") {
                    status.write().cache_mb = get_cache_size(1024 * 1024);
                    if let Some(series_id) = event_series_id {
                        let is_cached = {
                            let shared = kavita();
                            let guard = shared.lock().await;
                            guard.is_series_cached(series_id)
                        };
                        let mut updated = series.read().clone();
                        for item in &mut updated {
                            if item.id == series_id {
                                item.is_cached = is_cached;
                            }
                        }
                        series.set(updated);
                    }
                }
            }
        });
    });

    let status_snapshot = status.read().clone();
    let toast_snapshot = toast.read().clone();
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
    let app_class = if matches!(page_snapshot, Page::Dashboard | Page::Settings) {
        "app ember-mode"
    } else {
        "app series-mode"
    };

    let on_ember_action = move |action: EmberAction| {
        run_ui_action(busy, async move {
            match action {
                EmberAction::Home => {
                    selected_index.set(0);
                    page.set(Page::Dashboard);
                }
                EmberAction::Settings => {
                    selected_index.set(3);
                    page.set(Page::Settings);
                }
                EmberAction::Library => match load_libraries().await {
                    Ok(items) => {
                        libraries.set(items);
                        selected_index.set(0);
                        page.set(Page::Libraries);
                    }
                    Err(err) => show_toast(
                        &mut status,
                        &mut toast,
                        &mut toast_generation,
                        format!("Failed to load libraries: {err}"),
                    ),
                },
                EmberAction::Resume(read) => {
                    let state = snapshot().await;
                    let context = (
                        state.db.get_libraries(),
                        state.db.get_series(&read.library_id),
                        state.db.get_volumes(&read.volume.series_id),
                    );
                    match context {
                        (Ok(libs), Ok(items), Ok(siblings)) => {
                            libraries_selection.set(
                                libs.iter()
                                    .position(|l| l.id == read.library_id)
                                    .map(|i| i + 1)
                                    .unwrap_or(0),
                            );
                            series_selection.set(
                                items
                                    .iter()
                                    .filter(|s| s.read < 100)
                                    .chain(items.iter().filter(|s| s.read >= 100))
                                    .position(|s| s.id == read.volume.series_id)
                                    .map(|i| i + 1)
                                    .unwrap_or(0),
                            );
                            volumes_selection.set(
                                siblings
                                    .iter()
                                    .position(|v| v.id == read.volume.id)
                                    .map(|i| i + 1)
                                    .unwrap_or(0),
                            );
                            libraries.set(libs);
                            series.set(items);
                            volumes.set(siblings);
                            let start = initial_reader_page(&read.volume);
                            reader_page.set(start);
                            reader_first_page.set(start);
                            reader_loaded_until.set(reader_batch_end(start, read.volume.pages));
                            reader_slider_visible.set(false);
                            reader_volume.set(Some(read.volume));
                            page.set(Page::Reader);
                        }
                        _ => show_toast(
                            &mut status,
                            &mut toast,
                            &mut toast_generation,
                            "Could not restore this volume. Open it from your library.".into(),
                        ),
                    }
                }
                EmberAction::OpenDownload(item) => {
                    let state = snapshot().await;
                    match state.get_cached_volumes(&item.id) {
                        Ok(items) if !items.is_empty() => {
                            if let Ok(libs) = state.db.get_libraries() {
                                libraries_selection.set(
                                    libs.iter()
                                        .position(|l| l.id == item.library_id)
                                        .map(|i| i + 1)
                                        .unwrap_or(0),
                                );
                                libraries.set(libs);
                            }
                            if let Ok(items) = state.db.get_series(&item.library_id) {
                                series_selection.set(
                                    items
                                        .iter()
                                        .filter(|s| s.read < 100)
                                        .chain(items.iter().filter(|s| s.read >= 100))
                                        .position(|s| s.id == item.id)
                                        .map(|i| i + 1)
                                        .unwrap_or(0),
                                );
                                series.set(items);
                            }
                            let next = first_non_complete_volume_selection(&items);
                            volumes.set(items);
                            selected_index.set(next);
                            volumes_selection.set(next);
                            page.set(Page::Volumes);
                        }
                        Ok(_) => show_toast(
                            &mut status,
                            &mut toast,
                            &mut toast_generation,
                            "No downloaded volumes are ready yet.".into(),
                        ),
                        Err(err) => show_toast(
                            &mut status,
                            &mut toast,
                            &mut toast_generation,
                            format!("Could not open downloads: {err}"),
                        ),
                    }
                }
                EmberAction::ClearCache => {
                    if status.peek().offline_mode {
                        show_toast(
                            &mut status,
                            &mut toast,
                            &mut toast_generation,
                            "Go online before clearing cached pages and local library data.".into(),
                        );
                    } else {
                        match clear_cache().await {
                            Ok(()) => {
                                libraries.set(Vec::new());
                                series.set(Vec::new());
                                volumes.set(Vec::new());
                                home_read.set(None);
                                downloaded_series.set(Vec::new());
                                status.set(load_status().await);
                                show_toast(
                                    &mut status,
                                    &mut toast,
                                    &mut toast_generation,
                                    "Cache cleared. Reopen your library to refresh its data."
                                        .into(),
                                );
                            }
                            Err(err) => show_toast(
                                &mut status,
                                &mut toast,
                                &mut toast_generation,
                                format!("Failed to clear cache: {err}"),
                            ),
                        }
                    }
                }
                EmberAction::Update => match update_server_library().await {
                    Ok(()) => show_toast(
                        &mut status,
                        &mut toast,
                        &mut toast_generation,
                        "Update requested in Kavita. Your server will refresh its libraries."
                            .into(),
                    ),
                    Err(err) => show_toast(
                        &mut status,
                        &mut toast,
                        &mut toast_generation,
                        format!("Server library update failed: {err}"),
                    ),
                },
                EmberAction::ToggleOffline => {
                    let result = toggle_offline().await;
                    status.set(load_status().await);
                    let message = match result {
                        Ok(true) => "Offline mode is active".into(),
                        Ok(false) => "Connected to Kavita".into(),
                        Err(err) => format!("Could not reconnect: {err}"),
                    };
                    show_toast(&mut status, &mut toast, &mut toast_generation, message);
                }
                EmberAction::Connect(creds) => {
                    connect_settings(creds, status, toast, toast_generation).await
                }
                EmberAction::Exit => exit_process(),
            }
        });
    };

    rsx! {
        style { "{STYLE}" }
        style { "{ember_ui::STYLE}" }
        div {
            class: "{app_class}",
            "aria-busy": "{busy}",
            tabindex: "0",
            onmounted: move |event| {
                let element = event.data();
                spawn(async move {
                    if !matches!(page(), Page::Dashboard | Page::Settings) {
                        let _ = element.set_focus(true).await;
                    }
                    let mut layout = document::eval(include_str!("layout.js"));
                    while let Ok(columns) = layout.recv::<usize>().await {
                        shelf_columns.set(columns.max(1));
                    }
                });
            },
            onkeydown: move |event| {
                if matches!(page(), Page::Dashboard | Page::Settings) { return; }
                if busy() {
                    event.prevent_default();
                    return;
                }
                let key = event.key().to_string();
                if page() == Page::Reader && matches!(key.as_str(), "Enter" | " ") {
                    return;
                }
                if matches!(key.as_str(), "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Enter" | " " | "F1" | "F2" | "Backspace") {
                    event.prevent_default();
                }

                if matches!(key.as_str(), "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown") {

                    if page() == Page::Reader {
                        if let Some(volume) = { reader_volume.read().clone() } {
                            match key.as_str() {
                                "ArrowDown" => {
                                    document::eval(
                                        r#"
                                        const reader = document.querySelector(".reader-page");
                                        if (reader) {
                                            reader.scrollBy({ top: Math.floor(reader.clientHeight * 0.41), behavior: "smooth" });
                                        }
                                        "#,
                                    );
                                }
                                "ArrowUp" => {
                                    document::eval(
                                        r#"
                                        const reader = document.querySelector(".reader-page");
                                        if (reader) {
                                            reader.scrollBy({ top: -Math.floor(reader.clientHeight * 0.41), behavior: "smooth" });
                                        }
                                        "#,
                                    );
                                }
                                "ArrowLeft" | "ArrowRight" => {
                                    reader_slider_generation.set(reader_slider_generation().wrapping_add(1));
                                    if !reader_slider_visible() {
                                        reader_slider_visible.set(true);
                                    } else {
                                        let max_page = volume.pages.saturating_sub(1).max(0);
                                        let current = reader_page();
                                        let next = if key == "ArrowRight" {
                                            (current + 1).min(max_page)
                                        } else {
                                            current.saturating_sub(1).max(0)
                                        };
                                        seek_reader(next, volume.pages, reader_page, reader_first_page, reader_loaded_until, reader_slider_visible, reader_slider_generation);
                                    }
                                }
                                _ => {}
                            }
                        }
                    } else {
                        let count = visible_tile_count(page(), &libraries.read(), &series.read(), &volumes.read());
                        let current = selected_index().min(count.saturating_sub(1));
                        let next = if page() == Page::Series {
                            let series_snapshot = series.read();
                            let in_progress_count = series_snapshot
                                .iter()
                                .filter(|item| item.read < 100)
                                .count();
                            let completed_count = series_snapshot.len().saturating_sub(in_progress_count);
                            move_series_selection(
                                current,
                                key.as_str(),
                                in_progress_count,
                                completed_count,
                                shelf_columns(),
                            )
                        } else {
                            move_selection(current, key.as_str(), count, tile_columns(page(), shelf_columns()))
                        };
                        selected_index.set(next);
                        match page() {
                            Page::Dashboard => dashboard_selection.set(next),
                            Page::Libraries => libraries_selection.set(next),
                            Page::Series => series_selection.set(next),
                            Page::Volumes => volumes_selection.set(next),
                            Page::Reader | Page::Settings => {}
                        }
                    }

                    return;
                }
                if !matches!(key.as_str(), "Enter" | " " | "F1" | "F2" | "Backspace") { return; }
                run_ui_action(busy, async move {

                    match key.as_str() {

                        "Enter" | " " => {

                            let selected = selected_index().min(visible_tile_count(page(), &libraries.read(), &series.read(), &volumes.read()).saturating_sub(1));
                            match page() {
                                Page::Dashboard => {},
                                Page::Libraries => {
                                    if selected == 0 {
                                        selected_index.set(dashboard_selection());
                                        page.set(Page::Dashboard);
                                    } else if let Some(library) = { libraries.read().get(selected - 1).cloned() } {
                                        let id = library.id;
                                        match load_series(id).await {
                                            Ok(items) => {
                                                series.set(items);
                                                show_toast(
                                                    &mut status,
                                                    &mut toast,
                                                    &mut toast_generation,
                                                    format!("Loaded series for library id {id}"),
                                                );
                                                libraries_selection.set(selected);
                                                selected_index.set(0);
                                                page.set(Page::Series);
                                            }
                                            Err(err) => show_toast(
                                                &mut status,
                                                &mut toast,
                                                &mut toast_generation,
                                                format!("Failed to load series: {err}"),
                                            ),
                                        }
                                    }
                                }
                                Page::Series => {
                                    if selected == 0 {
                                        selected_index.set(libraries_selection());
                                        page.set(Page::Libraries);
                                    } else if let Some(item) = { selected_series(&series.read(), selected).cloned() } {
                                        let id = item.id;
                                        match load_volumes(id).await {
                                            Ok(items) => {
                                                let volume_selection = first_non_complete_volume_selection(&items);
                                                volumes.set(items);
                                                show_toast(
                                                    &mut status,
                                                    &mut toast,
                                                    &mut toast_generation,
                                                    format!("Loaded volumes for series id {id}"),
                                                );
                                                series_selection.set(selected);
                                                selected_index.set(volume_selection);
                                                volumes_selection.set(volume_selection);
                                                page.set(Page::Volumes);
                                            }
                                            Err(err) => show_toast(
                                                &mut status,
                                                &mut toast,
                                                &mut toast_generation,
                                                format!("Failed to load volumes: {err}"),
                                            ),
                                        }
                                    }
                                }
                                Page::Volumes => {
                                    if selected == 0 {
                                        let library_id = current_series_library_id(
                                            &series.read(),
                                            &libraries.read(),
                                            libraries_selection(),
                                        );
                                        if let Some(library_id) = library_id {
                                            match load_series(library_id).await {
                                                Ok(items) => replace_series(items, series, series_selection, selected_index, page),
                                                Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to refresh series: {err}")),
                                            }
                                        }
                                        selected_index.set(series_selection());
                                        page.set(Page::Series);
                                    } else if let Some(volume) = { volumes.read().get(selected - 1).cloned() } {
                                        volumes_selection.set(selected);
                                        let start_page = initial_reader_page(&volume);
                                        reader_page.set(start_page);
                                        reader_first_page.set(start_page);
                                        reader_loaded_until.set(reader_batch_end(start_page, volume.pages));
                                        reader_slider_visible.set(false);
                                        reader_volume.set(Some(volume.clone()));
                                        page.set(Page::Reader);
                                    }
                                }
                                Page::Reader => {}
                                Page::Settings => {},
                            }
                        }
                        "F1" => {

                            if page() == Page::Volumes {
                                let selected = selected_index().min(visible_tile_count(page(), &libraries.read(), &series.read(), &volumes.read()).saturating_sub(1));
                                if selected > 0 {
                                    let focused_volume = { volumes.read().get(selected - 1).cloned() };
                                    if let Some(volume) = focused_volume {
                                        match toggle_volume_read(&volume).await {
                                            Ok(marked_read) => {
                                                match load_volumes(volume.series_id).await {
                                                    Ok(items) => {
                                                        volumes.set(items);
                                                        selected_index.set(selected);
                                                        volumes_selection.set(selected);
                                                    }
                                                    Err(err) => show_toast(
                                                        &mut status,
                                                        &mut toast,
                                                        &mut toast_generation,
                                                        format!("Failed to refresh volumes: {err}"),
                                                    ),
                                                }
                                                let message = if marked_read {
                                                    format!("Marked {} as read", volume.title)
                                                } else {
                                                    format!("Marked {} as unread", volume.title)
                                                };
                                                show_toast(
                                                    &mut status,
                                                    &mut toast,
                                                    &mut toast_generation,
                                                    message,
                                                );
                                            }
                                            Err(err) => show_toast(
                                                &mut status,
                                                &mut toast,
                                                &mut toast_generation,
                                                format!("Failed to update volume: {err}"),
                                            ),
                                        }
                                    }
                                }
                            }
                        }
                        "F2" => {

                            let current_series = match page() {
                                Page::Series => {
                                    let series_snapshot = series.read();
                                    selected_series(&series_snapshot, selected_index())
                                        .map(|item| (item.id, item.title.clone(), Some(item.library_id)))
                                }
                                Page::Volumes => {
                                    let volumes_snapshot = volumes.read();
                                    let selected = selected_index().min(visible_tile_count(page(), &libraries.read(), &series.read(), &volumes.read()).saturating_sub(1));
                                    let volume = if selected > 0 {
                                        volumes_snapshot.get(selected - 1)
                                    } else {
                                        volumes_snapshot.first()
                                    };
                                    let library_id = current_series_library_id(
                                        &series.read(),
                                        &libraries.read(),
                                        libraries_selection(),
                                    );
                                    volume.map(|item| {
                                        (item.series_id, format!("series {}", item.series_id), library_id)
                                    })
                                }
                                Page::Reader => reader_volume
                                    .read()
                                    .as_ref()
                                    .map(|item| {
                                        let library_id = current_series_library_id(
                                            &series.read(),
                                            &libraries.read(),
                                            libraries_selection(),
                                        );
                                        (item.series_id, format!("series {}", item.series_id), library_id)
                                    }),
                                Page::Dashboard | Page::Libraries | Page::Settings => None,
                            };

                            if let Some((series_id, title, library_id)) = current_series {
                                if is_series_fully_cached(series_id).await {
                                    match remove_series_cache(series_id).await {
                                        Ok(()) => {
                                            if matches!(page(), Page::Volumes | Page::Reader) {
                                                match load_cached_volumes(series_id).await {
                                                    Ok(items) => volumes.set(items),
                                                    Err(err) => show_toast(
                                                        &mut status,
                                                        &mut toast,
                                                        &mut toast_generation,
                                                        format!("Failed to refresh volumes: {err}"),
                                                    ),
                                                }
                                            }
                                            if let Some(library_id) = library_id {
                                                match load_series(library_id).await {
                                                    Ok(items) => replace_series(items, series, series_selection, selected_index, page),
                                                    Err(err) => show_toast(
                                                        &mut status,
                                                        &mut toast,
                                                        &mut toast_generation,
                                                        format!("Failed to refresh series: {err}"),
                                                    ),
                                                }
                                            }
                                            show_toast(
                                                &mut status,
                                                &mut toast,
                                                &mut toast_generation,
                                                format!("Removed cached volumes for {}", title),
                                            );
                                        }
                                        Err(err) => show_toast(
                                            &mut status,
                                            &mut toast,
                                            &mut toast_generation,
                                            format!("Failed to remove cached volumes: {err}"),
                                        ),
                                    }
                                    return;
                                }

                                if page() == Page::Series && !is_series_caching(series_id).await {
                                    if let Err(err) = load_volumes(series_id).await {
                                        show_toast(
                                            &mut status,
                                            &mut toast,
                                            &mut toast_generation,
                                            format!("Failed to prepare cache: {err}"),
                                        );
                                        return;
                                    }
                                }

                                match toggle_series_caching(series_id).await {
                                    Ok(started) => {
                                        let message = if started {
                                            format!("Started caching {}", title)
                                        } else {
                                            format!("Stopped caching {}", title)
                                        };
                                        show_toast(
                                            &mut status,
                                            &mut toast,
                                            &mut toast_generation,
                                            message,
                                        );
                                    }
                                    Err(err) => show_toast(
                                        &mut status,
                                        &mut toast,
                                        &mut toast_generation,
                                        format!("Failed to toggle cache: {err}"),
                                    ),
                                }
                            }
                        }
                        "Backspace" => {

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
                                    let library_id = current_series_library_id(
                                        &series.read(),
                                        &libraries.read(),
                                        libraries_selection(),
                                    );
                                    if let Some(library_id) = library_id {
                                        match load_series(library_id).await {
                                            Ok(items) => replace_series(items, series, series_selection, selected_index, page),
                                            Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to refresh series: {err}")),
                                        }
                                    }
                                    selected_index.set(series_selection());
                                    page.set(Page::Series);
                                }
                                Page::Reader => {
                                    if let Some(volume) = { reader_volume.read().clone() } {
                                        let series_id = volume.series_id;
                                        match load_volumes(series_id).await {
                                            Ok(items) => volumes.set(items),
                                            Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to refresh volumes: {err}")),
                                        }
                                    }
                                    selected_index.set(volumes_selection());
                                    page.set(Page::Volumes);
                                }
                            }
                        }
                        _ => {}
                    }

                });
            },
            if busy() {
                div { class: "busy-indicator", role: "status", "Loading…" }
            }
            if let Some(message) = toast_snapshot {
                div { class: "toast", role: "status", "aria-live": "polite", "{message}" }
            }
            div { class: "tiles",
                {
                    match page_snapshot {
                        Page::Dashboard | Page::Settings => rsx! {
                            EmberScreen {
                                page: page_snapshot, status: status_snapshot.clone(), busy: busy(),
                                reading: home_read.read().clone(), reading_loading: home_loading(),
                                downloads: downloaded_series.read().clone(), onaction: on_ember_action,
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
                                                onfocus: move |_| selected_index.set(0),
                                                "aria-disabled": "{busy}",
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
                                                        onfocus: move |_| selected_index.set(index + 1),
                                                        "aria-disabled": "{busy}",
                                                        class: "{class}",
                                                        onclick: move |_| {
                                                            run_ui_action(busy, async move {
                                                                match load_series(id).await {
                                                                    Ok(items) => {
                                                                        series.set(items);
                                                                        show_toast(&mut status, &mut toast, &mut toast_generation, format!("Loaded series for library id {id}"));
                                                                        libraries_selection.set(index + 1);
                                                                        selected_index.set(0);
                                                                        page.set(Page::Series);
                                                                    }
                                                                    Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to load series: {err}")),
                                                                }

                                                            });
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
                                                    onfocus: move |_| selected_index.set(0),
                                                    "aria-disabled": "{busy}",
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
                                                let cached = item.is_cached;
                                                let cover_url = crate::local_api::media_url(&format!("series-cover/{id}"));
                                                let class = if selected_snapshot == index + 1 { "series-card selected" } else { "series-card" };
                                                let progress_class = if cached {
                                                    "series-progress-fill cached"
                                                } else {
                                                    "series-progress-fill"
                                                };
                                                let title_class = if cached {
                                                    "series-card-title cached"
                                                } else {
                                                    "series-card-title"
                                                };
                                                rsx! {
                                                    div { class: "series-card-wrap",
                                                        button {
                                                            onfocus: move |_| selected_index.set(index + 1),
                                                            "aria-disabled": "{busy}",
                                                            class: "{class}",
                                                            style: "background-image: url('{cover_url}');",
                                                            onclick: move |_| {
                                                                run_ui_action(busy, async move {
                                                                    match load_volumes(id).await {
                                                                        Ok(items) => {
                                                                            let volume_selection = first_non_complete_volume_selection(&items);
                                                                            volumes.set(items);
                                                                            show_toast(&mut status, &mut toast, &mut toast_generation, format!("Loaded volumes for series id {id}"));
                                                                            series_selection.set(index + 1);
                                                                            selected_index.set(volume_selection);
                                                                            volumes_selection.set(volume_selection);
                                                                            page.set(Page::Volumes);
                                                                        }
                                                                        Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to load volumes: {err}")),
                                                                    }

                                                                });
                                                            },
                                                            div { class: "series-progress",
                                                                div {
                                                                    class: "{progress_class}",
                                                                    style: "width: {progress}%;"
                                                                }
                                                            }
                                                            div { class: "{title_class}",
                                                                span { "{title}" }
                                                            }
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
                                                        onfocus: move |_| selected_index.set(0),
                                                        "aria-disabled": "{busy}",
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
                                                let cover_url = crate::local_api::media_url(&format!("series-cover/{id}"));
                                                let nav_index = in_progress_series.len() + index + 1;
                                                let class = if selected_snapshot == nav_index { "series-card selected" } else { "series-card" };
                                                let progress_class = if item.is_cached {
                                                    "series-progress-fill complete cached"
                                                } else {
                                                    "series-progress-fill complete"
                                                };
                                                let title_class = if item.is_cached {
                                                    "series-card-title complete cached"
                                                } else {
                                                    "series-card-title complete"
                                                };
                                                rsx! {
                                                    div { class: "series-card-wrap",
                                                        button {
                                                            onfocus: move |_| selected_index.set(nav_index),
                                                            "aria-disabled": "{busy}",
                                                            class: "{class}",
                                                            style: "background-image: url('{cover_url}');",
                                                            onclick: move |_| {
                                                                run_ui_action(busy, async move {
                                                                    match load_volumes(id).await {
                                                                        Ok(items) => {
                                                                            let volume_selection = first_non_complete_volume_selection(&items);
                                                                            volumes.set(items);
                                                                            show_toast(&mut status, &mut toast, &mut toast_generation, format!("Loaded volumes for series id {id}"));
                                                                            series_selection.set(nav_index);
                                                                            selected_index.set(volume_selection);
                                                                            volumes_selection.set(volume_selection);
                                                                            page.set(Page::Volumes);
                                                                        }
                                                                        Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to load volumes: {err}")),
                                                                    }

                                                                });
                                                            },
                                                            div { class: "series-progress",
                                                                div {
                                                                    class: "{progress_class}",
                                                                    style: "width: {progress}%;"
                                                                }
                                                            }
                                                            div { class: "{title_class}",
                                                                span { "{title}" }
                                                            }
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
                                                    onfocus: move |_| selected_index.set(0),
                                                    "aria-disabled": "{busy}",
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
                                                onfocus: move |_| selected_index.set(0),
                                                "aria-disabled": "{busy}",
                                                class: if selected_snapshot == 0 { "back-card selected" } else { "back-card" },
                                                onclick: move |_| {
                                                    run_ui_action(busy, async move {
                                                        let library_id = current_series_library_id(
                                                            &series.read(),
                                                            &libraries.read(),
                                                            libraries_selection(),
                                                        );
                                                        if let Some(library_id) = library_id {
                                                            match load_series(library_id).await {
                                                                Ok(items) => replace_series(items, series, series_selection, selected_index, page),
                                                                Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to refresh series: {err}")),
                                                            }
                                                        }
                                                        selected_index.set(series_selection());
                                                        page.set(Page::Series);

                                                    });
                                                },
                                                div { class: "back-card-arrow", "←" }
                                                div { class: "back-card-title", "Series" }
                                            }
                                        }
                                        {volumes_snapshot.iter().enumerate().map(|(index, volume)| {
                                            let cover_id = volume.id;
                                            let title = volume.title.clone();
                                            let reader_item = volume.clone();
                                            let progress = if volume.pages > 0 {
                                                ((volume.read * 100) / volume.pages).clamp(0, 100)
                                            } else {
                                                0
                                            };
                                            let complete = volume.pages > 0 && volume.read >= volume.pages;
                                            let cached = volume.is_cached;
                                            let cover_url = crate::local_api::media_url(&format!("volumes-cover/{cover_id}"));
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
                                                        onfocus: move |_| selected_index.set(index + 1),
                                                        "aria-disabled": "{busy}",
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
                                                        div { class: "{title_class}",
                                                            span { "{title}" }
                                                        }
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
                                onmounted: move |_| {
                                    if let Some(volume) = reader_volume.read().clone() {
                                        track_reader(volume, page, reader_volume, reader_page);
                                    }
                                },
                                onscroll: move |event| {
                                    if let Some(volume) = { reader_volume.read().clone() } {
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
                                        let ReaderPages { current, display_page, total_pages, first_page, loaded_until, loaded_count } =
                                            ReaderPages::new(volume.pages, reader_page(), reader_first_page(), reader_loaded_until());
                                        let title = volume.title.clone();
                                        rsx! {
                                            div { class: "reader-bar",
                                                button {
                                                    "aria-disabled": "{busy}",
                                                    class: "reader-back",
                                                    onclick: move |_| {
                                                        run_ui_action(busy, async move {
                                                            let series_id = volume.series_id;
                                                            match load_volumes(series_id).await {
                                                                Ok(items) => volumes.set(items),
                                                                Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to refresh volumes: {err}")),
                                                            }
                                                            selected_index.set(volumes_selection());
                                                            page.set(Page::Volumes);

                                                        });
                                                    },
                                                    "← Back"
                                                }
                                                div { class: "reader-title", "{title}" }
                                                button {
                                                    class: "reader-count",
                                                    "aria-label": "Choose page",
                                                    "aria-expanded": "{reader_slider_visible}",
                                                    onclick: move |_| {
                                                        reader_slider_visible.set(!reader_slider_visible());
                                                        reader_slider_generation.set(reader_slider_generation().wrapping_add(1));
                                                    },
                                                    "{display_page}/{total_pages}"
                                                }
                                            }
                                            div { class: "reader-stage",
                                                if volume.pages > 0 {
                                                    {(first_page..=loaded_until).map(|page_number| {
                                                        let image_url = crate::local_api::media_url(&format!(
                                                            "picture/{}/{}/{}/{}",
                                                            volume.series_id,
                                                            volume.volume_id,
                                                            volume.chapter_id,
                                                            page_number
                                                        ));
                                                        let page_label = page_number + 1;
                                                        rsx! {
                                                            div {
                                                                key: "{volume.chapter_id}-{page_number}",
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
                                                        span { "1" }
                                                        span { "Page {display_page} / {total_pages}" }
                                                        span { "{total_pages}" }
                                                    }
                                                    input {
                                                        r#type: "range",
                                                        min: "1",
                                                        max: "{total_pages}",
                                                        value: "{display_page}",
                                                        "aria-label": "Reader page",
                                                        "aria-valuenow": "{current + 1}",
                                                        onkeydown: move |event| event.stop_propagation(),
                                                        oninput: move |event| {
                                                            if let Ok(page) = event.value().parse::<i32>() {
                                                                seek_reader(page - 1, total_pages, reader_page, reader_first_page, reader_loaded_until, reader_slider_visible, reader_slider_generation);
                                                            }
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div { class: "reader-bar",
                                                button {
                                                    "aria-disabled": "{busy}",
                                                    class: "reader-back",
                                                    onclick: move |_| {
                                                        run_ui_action(busy, async move {
                                                            if let Some(volume) = { reader_volume.read().clone() } {
                                                                let series_id = volume.series_id;
                                                                match load_volumes(series_id).await {
                                                                    Ok(items) => volumes.set(items),
                                                                    Err(err) => show_toast(&mut status, &mut toast, &mut toast_generation, format!("Failed to refresh volumes: {err}")),
                                                                }
                                                            }
                                                            selected_index.set(volumes_selection());
                                                            page.set(Page::Volumes);

                                                        });
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
                    }
                }
            }
        }
    }
}

pub fn run_ui(kavita: SharedKavita) {
    let _ = KAVITA.set(kavita);
    let window = WindowBuilder::new()
        .with_title("Manga4Deck")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_always_on_top(false);
    #[cfg(target_os = "linux")]
    let window = window.with_decorations(false);

    // Embed the font once in the native document. This also supports `cargo run`
    // and standalone binaries without a separate asset bundling step.
    let inter = base64::engine::general_purpose::STANDARD
        .encode(include_bytes!("../assets/InterVariable.woff2"));
    let head = format!(
        "<style>@font-face {{ font-family: Inter; src: url(data:font/woff2;base64,{inter}) format('woff2'); font-weight: 100 900; font-display: swap; }}</style>"
    );
    let config = Config::new().with_window(window).with_custom_head(head);
    #[cfg(not(target_os = "macos"))]
    let config = config.with_menu(None);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(app);
}
