//! EMBER Home, library, series detail and Settings views.
use std::sync::LazyLock;

use base64::Engine;
use dioxus::prelude::*;

use super::{Page, UiStatus};
use crate::kavita::{ConnectionCreds, Library, Series, SeriesMetadata, Volume};

pub(super) const STYLE: &str = include_str!("ember.css");

#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum SeriesFilter {
    #[default]
    Unread,
    All,
    Downloaded,
    Completed,
}

impl SeriesFilter {
    fn includes(self, series: &Series) -> bool {
        match self {
            Self::Unread => series.read < 100,
            Self::All => true,
            Self::Downloaded => series.is_cached,
            Self::Completed => series.read >= 100,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct HomeRead {
    pub volume: Volume,
    pub title: String,
    pub library_id: i32,
}

#[derive(Clone)]
pub(super) enum EmberAction {
    Home,
    OpenLibrary(Library),
    OpenSeries(Series),
    OpenVolume(Volume),
    BackFromSeries,
    RefreshLibraries,
    Settings,
    Resume(HomeRead),
    OpenDownload(Series),
    ClearCache,
    Update,
    RefreshSeriesCovers,
    ToggleOffline,
    Connect(ConnectionCreds),
    Exit,
}

#[derive(Clone, Copy, PartialEq)]
enum View {
    Main,
    Connection,
    Downloads,
}

#[derive(Clone, Copy, PartialEq)]
enum Confirmation {
    Cache,
    Exit,
}

#[component]
pub(super) fn EmberScreen(
    page: Page,
    status: UiStatus,
    busy: bool,
    cover_refresh: Option<String>,
    reading: Option<HomeRead>,
    reading_loading: bool,
    downloads: Vec<Series>,
    library_folders: Vec<Library>,
    libraries_loading: bool,
    libraries_error: bool,
    focused_library: Option<i32>,
    series_items: Vec<Series>,
    focused_series: Option<i32>,
    series_filter: SeriesFilter,
    onfilter: EventHandler<SeriesFilter>,
    onseriesfocus: EventHandler<i32>,
    active_series: Option<Series>,
    series_metadata: SeriesMetadata,
    volume_items: Vec<Volume>,
    downloading_volume: Option<i32>,
    focused_volume: Option<i32>,
    onvolumefocus: EventHandler<i32>,
    onaction: EventHandler<EmberAction>,
) -> Element {
    let mut view = use_signal(|| View::Main);
    let mut confirmation = use_signal(|| None::<Confirmation>);
    let mut server = use_signal(|| status.server_ip.clone());
    let mut username = use_signal(|| status.saved_username.clone());
    let mut password = use_signal(String::new);
    let mut api_key = use_signal(String::new);
    let settings = page == Page::Settings;
    let library = page == Page::Series;
    let detail = page == Page::Volumes;
    let refreshing_covers = cover_refresh.is_some();
    let main = view() == View::Main;
    let home = page == Page::Dashboard && main;
    let centered_footer = main && (home || library || detail);
    let connection = view() == View::Connection;
    let showing_downloads = view() == View::Downloads;
    let active_nav = if showing_downloads {
        "downloads"
    } else if settings {
        "settings"
    } else {
        "home"
    };
    let series_count = series_items
        .iter()
        .filter(|s| series_filter.includes(s))
        .count();
    let title = if connection {
        "Connection settings"
    } else if showing_downloads {
        "Downloads"
    } else if library {
        "Your library"
    } else if detail {
        active_series
            .as_ref()
            .map(|s| s.title.as_str())
            .unwrap_or("Series")
    } else if settings {
        "Settings"
    } else {
        "Home"
    };
    let account = if status.offline_mode {
        "Offline mode · Saved volumes available".to_string()
    } else if status.logged_as.is_empty() {
        "Kavita · Not signed in".to_string()
    } else {
        format!("Kavita online · Signed in as {}", status.logged_as)
    };
    let connection_ip = status.server_ip.clone();
    let connection_username = status.saved_username.clone();
    let open_connection = use_callback(move |_: ()| {
        server.set(connection_ip.clone());
        username.set(connection_username.clone());
        password.set(String::new());
        api_key.set(String::new());
        view.set(View::Connection);
    });

    rsx! {
        div {
            class: "ember-screen",
            "data-ember-view": if detail && main { "Series detail" } else { title },
            "data-series-detail-id": active_series.as_ref().map(|s| s.id.to_string()).unwrap_or_default(),
            "aria-busy": busy,
            onkeydown: move |event| {
                // Retain the existing download and read/unread commands.
                let key = event.key().to_string();
                if !(main && ((library && key == "F2") || (detail && matches!(key.as_str(), "F1" | "F2")))) { event.stop_propagation(); }
            },
            onmounted: move |_| {
                spawn(async move {
                    let _ = document::eval(include_str!("ember.js")).await;
                });
            },
            header { class: "ember-header",
                div { class: "ember-brand", "manga4deck" }
                nav { class: "ember-nav", "aria-label": "Main navigation",
                    for (tab, label) in [("home", "Home"), ("downloads", "Downloads"), ("settings", "Settings")] {
                        button {
                            r#type: "button",
                            class: "ember-nav-item",
                            "aria-current": if tab == active_nav { "page" } else { "false" },
                            "data-nav": tab,
                            disabled: busy,
                            onclick: move |_| {
                                confirmation.set(None);
                                password.set(String::new());
                                api_key.set(String::new());
                                view.set(if tab == "downloads" { View::Downloads } else { View::Main });
                                match tab {
                                    "home" => onaction.call(EmberAction::Home),
                                    "settings" => onaction.call(EmberAction::Settings),
                                    _ => {}
                                }
                            },
                            "{label}"
                        }
                    }
                }
                div { class: "ember-connection",
                    if settings || library || detail || showing_downloads {
                        if library || detail { img { class: "ember-status-dot", src: ONLINE_ICON.as_str(), width: "8", height: "8", alt: "", "aria-hidden": "true" } }
                        else { span { class: "ember-status-dot", "aria-hidden": "true" } }
                        if status.offline_mode { "Offline mode" } else if status.logged_as.is_empty() { "Not signed in" } else { "Kavita online" }
                    }
                }
            }
            main { class: "ember-content", "aria-labelledby": "ember-title",
                if library && main {
                    div { class: "ember-series-heading",
                        h1 { id: "ember-title", "{title}" }
                        span { class: "ember-series-count", "aria-live": "polite", "{series_count} series" }
                    }
                } else {
                    h1 { id: "ember-title", class: if detail { "ember-detail-title" } else { "" }, "{title}" }
                }
                if showing_downloads {
                    if downloads.is_empty() {
                        section { class: "ember-empty",
                            h2 { "Your offline library starts here." }
                            p { "Download a series from your library to read without a connection." }
                            button {
                                class: "ember-primary", r#type: "button", disabled: busy,
                                "data-autofocus": "true", "data-focus-row": "1", "data-focus-col": "0",
                                onclick: move |_| { view.set(View::Main); onaction.call(EmberAction::Home); },
                                "Browse library"
                            }
                        }
                    } else {
                        div { class: "ember-downloads",
                            for (index, item) in downloads.into_iter().enumerate() {
                                button {
                                    key: "{item.id}", class: "ember-download", r#type: "button", disabled: busy,
                                    "data-autofocus": if index == 0 { "true" } else { "false" },
                                    "data-focus-row": "{index + 1}", "data-focus-col": "0",
                                    onclick: move |_| onaction.call(EmberAction::OpenDownload(item.clone())),
                                    strong { "{item.title}" }
                                    span { "Open downloaded volumes" }
                                }
                            }
                        }
                    }
                } else if connection {
                    p { class: "ember-note", "Your library, wherever you read." }
                    form {
                        class: "ember-form",
                        onsubmit: move |event| {
                            event.prevent_default();
                            if !busy {
                                onaction.call(EmberAction::Connect(ConnectionCreds {
                                    ip: server(), username: username(), password: password(), api_key: api_key(),
                                }));
                            }
                        },
                        h2 { "Kavita server" }
                        div { class: "ember-fields",
                            label { r#for: "ember-server", "Server address"
                                input {
                                    id: "ember-server", r#type: "text", name: "server", value: "{server}",
                                    "aria-describedby": "ember-server-hint", autocomplete: "off", spellcheck: "false",
                                    readonly: busy, "data-autofocus": "true", "data-focus-row": "1", "data-focus-col": "0",
                                    oninput: move |e| server.set(e.value()),
                                }
                                small { id: "ember-server-hint", "Host and port, without http://" }
                            }
                            label { r#for: "ember-username", "Username"
                                input {
                                    id: "ember-username", r#type: "text", name: "username", value: "{username}",
                                    autocomplete: "username", spellcheck: "false", readonly: busy,
                                    "data-focus-row": "1", "data-focus-col": "1",
                                    oninput: move |e| username.set(e.value()),
                                }
                            }
                            label { r#for: "ember-password", "Password"
                                input {
                                    id: "ember-password", r#type: "password", name: "password", value: "{password}",
                                    placeholder: "Keep saved password", autocomplete: "current-password", readonly: busy,
                                    "aria-describedby": "ember-credential-note", "data-focus-row": "2", "data-focus-col": "0",
                                    oninput: move |e| password.set(e.value()),
                                }
                            }
                            label { r#for: "ember-api-key", "API key"
                                input {
                                    id: "ember-api-key", r#type: "password", name: "api_key", value: "{api_key}",
                                    placeholder: "Keep saved API key", autocomplete: "off", readonly: busy,
                                    "aria-describedby": "ember-credential-note", "data-focus-row": "2", "data-focus-col": "1",
                                    oninput: move |e| api_key.set(e.value()),
                                }
                            }
                        }
                        p { id: "ember-credential-note", "Leave password and API key blank to keep saved credentials." }
                        div { class: "ember-form-actions",
                            button {
                                class: "ember-primary settings-save", r#type: "submit", disabled: busy,
                                "data-focus-row": "3", "data-focus-col": "0",
                                if busy { "Connecting…" } else { "Save & connect" }
                            }
                            button {
                                class: "ember-secondary", r#type: "button", disabled: busy,
                                "data-back": "true", "data-focus-row": "3", "data-focus-col": "1",
                                onclick: move |_| { password.set(String::new()); api_key.set(String::new()); view.set(View::Main); },
                                "Back to settings"
                            }
                        }
                    }
                } else if library {
                    LibrarySeries { items: series_items, focused_series, series_filter, busy, onfilter, onseriesfocus, onaction }
                } else if detail {
                    if let Some(item) = active_series.clone() {
                        SeriesDetail {
                            key: "{item.id}",
                            library_name: library_folders.iter().find(|l| l.id == item.library_id).map(|l| l.title.clone()).unwrap_or_default(),
                            item, metadata: series_metadata, volumes: volume_items, focused_volume, downloading_volume, busy,
                            offline: status.offline_mode, onvolumefocus, onaction,
                        }
                    } else {
                        section { class: "ember-empty",
                            h2 { "Series unavailable." }
                            button { class: "ember-secondary", r#type: "button", "data-autofocus": "true", "data-back": "true",
                                onclick: move |_| onaction.call(EmberAction::BackFromSeries), "Back to library" }
                        }
                    }
                } else if settings {
                    section { class: "ember-status", role: "status", "aria-live": "polite",
                        strong { class: if status.offline_mode || status.logged_as.is_empty() { "" } else { "ember-online" }, "{account}" }
                        p {
                            if status.offline_mode { "Reconnect to browse your Kavita server." } else { "{status.server_ip}" }
                        }
                    }
                    div { class: "ember-actions",
                        for (position, (index, label, detail)) in [
                            (1, "Clean cache".to_string(), format!("{} MB on this device", status.cache_mb)),
                            (2, "Update".to_string(), "Update server library".to_string()),
                            (3, "Connection".to_string(), "Server and credentials".to_string()),
                            (4, if status.offline_mode { "Offline" } else { "Online" }.to_string(), if status.offline_mode { "Switch to online" } else { "Switch to offline" }.to_string()),
                            (6, "Refresh series covers".to_string(), if status.offline_mode { "Connect to Kavita to refresh" } else { "Download fresh previews for every series" }.to_string()),
                            (5, "Exit".to_string(), "Close manga4deck".to_string()),
                        ].into_iter().enumerate() {
                            button {
                                r#type: "button", class: "ember-action",
                                disabled: busy || (index == 6 && status.offline_mode) || (refreshing_covers && matches!(index, 1 | 3 | 4)),
                                "aria-disabled": index == 6 && refreshing_covers,
                                "data-autofocus": if (index == 3 && !refreshing_covers) || (index == 6 && refreshing_covers) { "true" } else { "false" },
                                "data-connection": if index == 3 { "true" } else { "false" },
                                "data-focus-row": "{position / 3 + 1}", "data-focus-col": "{position % 3}",
                                onclick: move |_| match index {
                                    1 => { confirmation.set(Some(Confirmation::Cache)); },
                                    2 => onaction.call(EmberAction::Update),
                                    3 => open_connection.call(()),
                                    4 => onaction.call(EmberAction::ToggleOffline),
                                    5 => { confirmation.set(Some(Confirmation::Exit)); },
                                    6 if !refreshing_covers => onaction.call(EmberAction::RefreshSeriesCovers),
                                    _ => {}
                                },
                                strong { "{label}" }
                                span { "{detail}" }
                            }
                        }
                    }
                    p { class: "ember-note", role: "status",
                        if let Some(message) = cover_refresh { "{message}" }
                        else { "Manage your device and Kavita connection." }
                    }
                } else if home {
                    if let Some(read) = reading {
                        HomeProgress { key: "{read.volume.id}", read, busy, onaction }
                    } else {
                        section { class: "ember-empty", "aria-live": "polite",
                            h2 { if reading_loading { "Loading your reading progress…" } else { "Your next volume is waiting." } }
                            if !reading_loading {
                                p { "Start a volume from your library. Your reading progress will appear here." }
                                if !library_folders.is_empty() {
                                    button {
                                        class: "ember-primary", r#type: "button", disabled: busy,
                                        "data-browse-library": "true", "aria-controls": "ember-library-rail",
                                        "Browse library"
                                    }
                                }
                            }
                        }
                    }
                    LibraryFolders {
                        folders: library_folders, busy, loading: libraries_loading, error: libraries_error,
                        focused_library, onaction,
                    }
                }
            }
            footer { class: if centered_footer { "ember-footer ember-footer-centered" } else { "ember-footer" }, "aria-label": "Controller shortcuts",
                div { class: "ember-hints",
                    if library && main {
                        span { class: "ember-hint", kbd { "↔" } span { span { class: "ember-series-filter-hint", "Filters" } span { class: "ember-series-card-hint", "Browse" } } }
                        span { class: "ember-hint", kbd { "A" } span { span { class: "ember-series-filter-hint", "Apply" } span { class: "ember-series-card-hint", "Open" } } }
                        span { class: "ember-hint", kbd { "B" } span { "Back" } }
                        span { class: "ember-hint ember-series-download-hint",
                            kbd { "X" }
                            span { "Download" }
                        }
                    } else { for (key, label) in [("↕", "Move"), ("A", "Open"), ("B", "Back"), (if (settings || detail) && main { "Y" } else { "X" }, if settings && main { "Connection" } else if detail && main { "Read/Unread" } else { "Downloads" })].into_iter().take(if home { 3 } else { 4 }) {
                        span { class: "ember-hint",
                            if key == "↕" {
                                kbd { span { class: "ember-move-label", "↕" } span { class: "ember-browse-label", "↔" } }
                                span { span { class: "ember-move-label", "Move" } span { class: "ember-browse-label", "Browse" } }
                            } else { kbd { "{key}" } span { "{label}" } }
                        }
                    } }
                }
                if !centered_footer { span { class: "ember-keyboard-hint", "Enter / Esc" } }
            }
            if let Some(kind) = confirmation() {
                div { class: "ember-overlay",
                    section {
                        class: "ember-dialog", role: "dialog", "aria-modal": "true", "aria-labelledby": "ember-confirm-title",
                        h2 { id: "ember-confirm-title", if kind == Confirmation::Cache { "Clear cached pages?" } else { "Close manga4deck?" } }
                        p {
                            if kind == Confirmation::Cache { "Cached pages and local library data will be removed. Reconnect to restore your library." }
                            else { "Your reading progress is saved. You can continue next time." }
                        }
                        div { class: "ember-form-actions",
                            button {
                                class: "ember-secondary", r#type: "button", disabled: busy,
                                "data-back": "true", "data-autofocus": "true", "data-focus-row": "0", "data-focus-col": "0",
                                onclick: move |_| confirmation.set(None), "Cancel"
                            }
                            button {
                                class: "ember-primary", r#type: "button", disabled: busy,
                                "data-focus-row": "0", "data-focus-col": "1",
                                onclick: move |_| {
                                    confirmation.set(None);
                                    onaction.call(if kind == Confirmation::Cache { EmberAction::ClearCache } else { EmberAction::Exit });
                                },
                                if kind == Confirmation::Cache { "Clear cache" } else { "Close app" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SeriesCoverImage(
    series_id: i32,
    title: String,
    width: u16,
    height: u16,
    fallback: &'static str,
    #[props(default)] lazy: bool,
) -> Element {
    let revision_signal = use_context::<Signal<u64>>();
    let revision = revision_signal();
    let mut failed = use_signal(|| None);
    let cover =
        crate::local_api::media_url(&format!("series-cover/{series_id}?revision={revision}"));
    rsx! {
        if failed() == Some((series_id, revision)) {
            span { "{fallback}" }
        } else {
            img {
                key: "{series_id}-{revision}",
                src: (!lazy).then(|| cover.clone()),
                "data-cover-src": lazy.then_some(cover),
                alt: "{title} cover", width: "{width}", height: "{height}",
                decoding: if lazy { "async" } else { "auto" },
                onerror: move |_| failed.set(Some((series_id, revision))),
            }
        }
    }
}

#[component]
fn HomeProgress(read: HomeRead, busy: bool, onaction: EventHandler<EmberAction>) -> Element {
    let current = read.volume.read.clamp(0, read.volume.pages.max(0));
    let title = read.volume.title.trim();
    let volume_label = if title.is_empty() {
        "Volume".to_string()
    } else if title
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | ' '))
    {
        format!("Volume {title}")
    } else if let Some(number) = title.strip_prefix("Chapter ") {
        format!("Volume {number}")
    } else {
        title.to_string()
    };
    rsx! {
        article { class: "ember-resume", "aria-label": "Continue reading",
            div { class: "ember-cover",
                SeriesCoverImage { series_id: read.volume.series_id, title: read.title.clone(), width: 48, height: 72, fallback: "No cover" }
            }
            div { class: "ember-reading-copy",
                h2 { "{read.title}" }
                p { "{volume_label} · Page {current} of {read.volume.pages}" }
            }
            button {
                class: "ember-primary", r#type: "button", disabled: busy,
                "data-autofocus": "true", "data-focus-row": "1", "data-focus-col": "0",
                onclick: move |_| onaction.call(EmberAction::Resume(read.clone())),
                "Keep reading"
            }
        }
    }
}

fn svg_data(bytes: &[u8]) -> String {
    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}
static FOLDER_ICON: LazyLock<String> =
    LazyLock::new(|| svg_data(include_bytes!("../assets/ember-folder.svg")));
static PREVIOUS_ICON: LazyLock<String> =
    LazyLock::new(|| svg_data(include_bytes!("../assets/ember-chevron-left.svg")));
static NEXT_ICON: LazyLock<String> =
    LazyLock::new(|| svg_data(include_bytes!("../assets/ember-chevron-right.svg")));

#[component]
fn LibraryFolders(
    folders: Vec<Library>,
    busy: bool,
    loading: bool,
    error: bool,
    focused_library: Option<i32>,
    onaction: EventHandler<EmberAction>,
) -> Element {
    let preferred = focused_library
        .filter(|id| folders.iter().any(|f| f.id == *id))
        .or_else(|| folders.first().map(|f| f.id));
    let count = folders.len();
    rsx! {
        section { class: "ember-library-folders", "aria-labelledby": "ember-libraries-title",
            header { class: "ember-library-heading",
                div { class: "ember-library-heading-copy",
                    h2 { id: "ember-libraries-title", "Your libraries" }
                    span { class: "ember-library-count", "aria-live": "polite",
                        if loading && count == 0 { "Loading…" }
                        else if count == 1 { "1 folder" }
                        else { "{count} folders" }
                    }
                }
                if count > 0 {
                    div { class: "ember-carousel-arrows",
                        for (direction, label, icon) in [("previous", "Previous folders", PREVIOUS_ICON.as_str()), ("next", "Next folders", NEXT_ICON.as_str())] {
                            button {
                                class: "ember-carousel-arrow", r#type: "button", disabled: busy,
                                "aria-label": label, "aria-controls": "ember-library-rail", "data-library-scroll": direction,
                                img { src: icon, width: "24", height: "24", alt: "", "aria-hidden": "true" }
                            }
                        }
                    }
                }
            }
            if count > 0 {
                div { id: "ember-library-rail", class: "ember-library-carousel", role: "group", "aria-label": "Library folders",
                    for folder in folders {
                        button {
                            key: "{folder.id}", class: "ember-folder", r#type: "button", disabled: busy,
                            "data-library-id": "{folder.id}", "data-autofocus": if Some(folder.id) == preferred { "true" } else { "false" },
                            "aria-label": "Open {folder.title} library", title: "{folder.title}",
                            onclick: move |_| onaction.call(EmberAction::OpenLibrary(folder.clone())),
                            img { class: "ember-folder-icon", src: FOLDER_ICON.as_str(), width: "40", height: "40", alt: "", "aria-hidden": "true" }
                            div { class: "ember-folder-labels",
                                strong { "{folder.title}" }
                                span { "Open library" }
                            }
                        }
                    }
                }
                p { class: "ember-carousel-hint", "Left / Right to browse · A to open" }
            } else if loading {
                p { class: "ember-library-placeholder", role: "status", "Loading your libraries…" }
            } else if !error {
                p { class: "ember-library-placeholder", "Your Kavita libraries will appear here." }
            }
            if error {
                div { class: "ember-library-error", role: "status",
                    p { if count > 0 { "Could not refresh libraries. Showing saved folders." } else { "Could not load libraries. Check your connection and try again." } }
                    button {
                        class: "ember-secondary", r#type: "button", disabled: busy || loading,
                        onclick: move |_| onaction.call(EmberAction::RefreshLibraries), "Retry"
                    }
                }
            }
        }
    }
}

static ONLINE_ICON: LazyLock<String> =
    LazyLock::new(|| svg_data(include_bytes!("../assets/ember-online.svg")));

#[component]
fn SeriesDetail(
    item: Series,
    metadata: SeriesMetadata,
    library_name: String,
    volumes: Vec<Volume>,
    downloading_volume: Option<i32>,
    focused_volume: Option<i32>,
    busy: bool,
    offline: bool,
    onvolumefocus: EventHandler<i32>,
    onaction: EventHandler<EmberAction>,
) -> Element {
    let preferred = focused_volume
        .filter(|id| volumes.iter().any(|v| v.id == *id))
        .or_else(|| {
            volumes
                .iter()
                .find(|v| v.pages <= 0 || v.read < v.pages)
                .map(|v| v.id)
        })
        .or_else(|| volumes.first().map(|v| v.id));
    let next_unread = volumes
        .iter()
        .find(|v| v.pages <= 0 || v.read < v.pages)
        .map(|v| v.id);
    let count = volumes.len();
    let has_downloaded = volumes.iter().any(|volume| volume.is_cached);
    let authors = metadata.authors.join(", ");
    let genres = metadata.genres.join(", ");
    rsx! {
        section { class: "ember-series-detail", "aria-label": "Series volumes",
            button { class: "ember-detail-breadcrumb", r#type: "button", disabled: busy,
                "aria-label": "Back to library", "data-detail-breadcrumb": "true",
                onclick: move |_| onaction.call(EmberAction::BackFromSeries),
                "Library" if !library_name.is_empty() { " / {library_name}" }
            }
            div { class: "ember-detail-layout",
                aside { class: "ember-detail-sidebar", "aria-label": "Series information",
                    div { class: "ember-detail-cover",
                        SeriesCoverImage { series_id: item.id, title: item.title.clone(), width: 240, height: 336, fallback: "Cover unavailable" }
                    }
                    if !authors.is_empty() { strong { class: "ember-detail-author", "{authors}" } }
                    p { class: "ember-detail-meta",
                        if !genres.is_empty() { "{genres} · " }
                        if count == 1 { "1 volume" } else { "{count} volumes" }
                    }
                    button { class: "ember-primary ember-detail-download", r#type: "button",
                        disabled: busy || count == 0 || (offline && !has_downloaded),
                        "data-series-download": "true",
                        "aria-label": if has_downloaded { format!("Remove downloaded volumes for {}", item.title) } else { format!("Download {}", item.title) },
                        if has_downloaded { "Remove downloaded" } else { "Download series" }
                    }
                }
                if volumes.is_empty() {
                    section { class: "ember-empty", role: "status",
                        h2 { "No volumes available." }
                        p { "Scan this series in Kavita, then reopen it." }
                        button { class: "ember-secondary", r#type: "button", disabled: busy, "data-autofocus": "true",
                            onclick: move |_| onaction.call(EmberAction::BackFromSeries), "Back to library" }
                    }
                } else {
                    div { class: "ember-volume-list", role: "group", "aria-label": "Volumes",
                        for (index, volume) in volumes.into_iter().enumerate() {
                            VolumeRow { key: "{volume.id}",
                                preferred: preferred == Some(volume.id), next_unread: next_unread == Some(volume.id),
                                downloading: downloading_volume == Some(volume.id),
                                volume, number: index + 1, busy, offline, onvolumefocus, onaction,
                            }
                        }
                    }
                }
            }
            button { class: "ember-detail-back", r#type: "button", hidden: true,
                onclick: move |_| onaction.call(EmberAction::BackFromSeries), "Back" }
        }
    }
}

#[component]
fn VolumeRow(
    volume: Volume,
    number: usize,
    preferred: bool,
    next_unread: bool,
    downloading: bool,
    busy: bool,
    offline: bool,
    onvolumefocus: EventHandler<i32>,
    onaction: EventHandler<EmberAction>,
) -> Element {
    let complete = volume.pages > 0 && volume.read >= volume.pages;
    let progress = if volume.pages > 0 {
        ((i64::from(volume.read) * 100) / i64::from(volume.pages)).clamp(0, 100)
    } else {
        0
    };
    let detail = if complete {
        format!("{} pages · completed", volume.pages)
    } else if volume.read > 0 {
        format!(
            "{} pages · page {} saved",
            volume.pages.max(0),
            volume.read.clamp(0, volume.pages.max(0))
        )
    } else {
        format!(
            "{} pages · {}",
            volume.pages.max(0),
            if next_unread {
                "ready to start"
            } else {
                "not started"
            }
        )
    };
    let label = if complete {
        "Completed".to_string()
    } else if progress > 0 {
        format!("{progress}%")
    } else if next_unread {
        "START".into()
    } else {
        "NEW".into()
    };
    let number = format!("{number:02}");
    let id = volume.id;
    let open_volume = volume.clone();
    let download_label = if volume.is_cached {
        "Downloaded"
    } else if downloading {
        "Downloading"
    } else {
        ""
    };
    rsx! {
        button { class: "ember-volume-row", r#type: "button", disabled: busy,
            "data-volume-id": "{id}", "data-autofocus": if preferred { "true" } else { "false" },
            "data-reading": !complete && volume.read > 0,
            "aria-label": "Open {volume.title}, {detail}, {download_label}",
            onfocus: move |_| onvolumefocus.call(id),
            onclick: move |_| onaction.call(EmberAction::OpenVolume(open_volume.clone())),
            span { class: "ember-volume-number", "{number}" }
            span { class: "ember-volume-copy",
                strong { "{volume.title}" }
                span { "{detail}" if offline { if volume.is_cached { " · available offline" } else { " · connect to read" } } }
            }
            span { class: "ember-volume-status",
                if !download_label.is_empty() {
                    span { class: "ember-volume-download", "data-downloaded": volume.is_cached,
                        "aria-label": download_label, title: download_label,
                        if volume.is_cached {
                            svg { class: "ember-download-check", width: "20", height: "20", view_box: "0 0 24 24",
                                fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                                circle { cx: "12", cy: "12", r: "9" }
                                path { d: "m8 12 3 3 5-6" }
                            }
                        } else {
                            span { class: "ember-download-spinner", "aria-hidden": "true" }
                        }
                        span { class: "ember-volume-download-label", "{download_label}" }
                    }
                    span { class: "ember-volume-status-divider", "aria-hidden": "true" }
                }
                if complete {
                    span { class: "ember-volume-completed", role: "img", "aria-label": "{label}", title: "{label}",
                        svg { width: "24", height: "24", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                            path { d: "M18 6 7 17l-5-5" }
                            path { d: "m22 10-7.5 7.5L13 16" }
                        }
                    }
                } else {
                    span { class: "ember-volume-state", "{label}" }
                }
            }
        }
    }
}

#[component]
fn LibrarySeries(
    items: Vec<Series>,
    focused_series: Option<i32>,
    series_filter: SeriesFilter,
    busy: bool,
    onfilter: EventHandler<SeriesFilter>,
    onseriesfocus: EventHandler<i32>,
    onaction: EventHandler<EmberAction>,
) -> Element {
    let empty_library = items.is_empty();
    let visible: Vec<_> = items
        .into_iter()
        .filter(|s| series_filter.includes(s))
        .collect();
    let count = visible.len();
    let preferred = focused_series
        .filter(|id| visible.iter().any(|s| s.id == *id))
        .or_else(|| visible.first().map(|s| s.id));
    rsx! {
        section { class: "ember-series-library", "aria-label": "Series in this library",
            div { class: "ember-series-filters", role: "group", "aria-label": "Filter series",
                for (filter, value, label) in [
                    (SeriesFilter::Unread, "unread", "Unread"),
                    (SeriesFilter::All, "all", "All series"),
                    (SeriesFilter::Downloaded, "downloaded", "Downloaded"),
                    (SeriesFilter::Completed, "completed", "Completed"),
                ] {
                    button {
                        class: "ember-series-filter", r#type: "button", disabled: busy,
                        "aria-pressed": series_filter == filter,
                        "data-series-filter": value,
                        "data-autofocus": if preferred.is_none() && series_filter == filter { "true" } else { "false" },
                        onclick: move |_| onfilter.call(filter), "{label}"
                    }
                }
                if count > 3 {
                    div { class: "ember-series-scroll-controls",
                        for (direction, label, icon) in [("previous", "Previous titles", PREVIOUS_ICON.as_str()), ("next", "Next titles", NEXT_ICON.as_str())] {
                            button { class: "ember-carousel-arrow", r#type: "button", disabled: busy,
                                "data-library-scroll": direction, "aria-label": label, "aria-controls": "ember-series-rail",
                                img { src: icon, width: "24", height: "24", alt: "", "aria-hidden": "true" }
                            }
                        }
                    }
                }
            }
            if count == 0 {
                div { class: "ember-empty", role: "status",
                    h2 {
                        if empty_library { "This library is empty." }
                        else { match series_filter {
                            SeriesFilter::Unread => "No unread series.",
                            SeriesFilter::Completed => "No completed series yet.",
                            SeriesFilter::Downloaded => "No downloaded series yet.",
                            SeriesFilter::All => "This library is empty.",
                        } }
                    }
                    p {
                        if empty_library { "Add titles in Kavita, then reopen this library." }
                        else { match series_filter {
                            SeriesFilter::Unread => "You’re up to date with every series in this library.",
                            SeriesFilter::Completed => "Finish reading a series to find it here.",
                            SeriesFilter::Downloaded => "Download a series to read it offline.",
                            SeriesFilter::All => "Add titles in Kavita, then reopen this library.",
                        } }
                    }
                    if !empty_library && series_filter != SeriesFilter::All {
                        button { class: "ember-secondary", r#type: "button", disabled: busy, onclick: move |_| onfilter.call(SeriesFilter::All), "Show all series" }
                    }
                }
            } else {
                div { id: "ember-series-rail", class: "ember-library-carousel ember-series-carousel", role: "group", "aria-label": "Series",
                    for item in visible {
                        LibrarySeriesCard { key: "{item.id}", preferred: Some(item.id) == preferred, item, busy, onseriesfocus, onaction }
                    }
                }
            }
        }
    }
}

#[component]
fn LibrarySeriesCard(
    item: Series,
    preferred: bool,
    busy: bool,
    onseriesfocus: EventHandler<i32>,
    onaction: EventHandler<EmberAction>,
) -> Element {
    let id = item.id;
    let open_item = item.clone();
    let progress = item.read.clamp(0, 100);
    rsx! {
        button {
            class: "ember-series-card", r#type: "button", disabled: busy, title: "{item.title}",
            "aria-label": "Open {item.title}, {progress}% read", "data-series-id": "{id}",
            "data-autofocus": if preferred { "true" } else { "false" },
            onfocus: move |_| onseriesfocus.call(id),
            onclick: move |_| onaction.call(EmberAction::OpenSeries(open_item.clone())),
            div { class: "ember-series-art",
                SeriesCoverImage { series_id: id, title: item.title.clone(), width: 228, height: 304, fallback: "Cover unavailable", lazy: true }
            }
            div {
                class: "ember-series-progress", role: "progressbar",
                "aria-label": "{item.title} reading progress",
                "aria-valuemin": "0", "aria-valuemax": "100", "aria-valuenow": "{progress}",
                "aria-valuetext": if progress == 100 { "Completed".to_string() } else { format!("{progress}% read") },
                div { class: "ember-series-progress-fill", "data-completed": progress == 100, style: "width: {progress}%;" }
            }
            div { class: "ember-series-info",
                strong { class: "ember-series-title",
                    span { class: "ember-series-title-track",
                        span { class: "ember-series-title-text", "{item.title}" }
                        span { class: "ember-series-title-copy", "aria-hidden": "true", "{item.title}" }
                    }
                }
            }
        }
    }
}
