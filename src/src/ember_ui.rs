//! The EMBER Home and Settings views. Library shelves and the reader retain their own UI.
use dioxus::prelude::*;

use super::{Page, UiStatus};
use crate::kavita::{ConnectionCreds, Series, Volume};

pub(super) const STYLE: &str = include_str!("ember.css");

#[derive(Clone, Debug, PartialEq)]
pub(super) struct HomeRead {
    pub volume: Volume,
    pub title: String,
    pub library_id: i32,
}

#[derive(Clone)]
pub(super) enum EmberAction {
    Home,
    Library,
    Settings,
    Resume(HomeRead),
    OpenDownload(Series),
    ClearCache,
    Update,
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
    reading: Option<HomeRead>,
    reading_loading: bool,
    downloads: Vec<Series>,
    onaction: EventHandler<EmberAction>,
) -> Element {
    let mut view = use_signal(|| View::Main);
    let mut confirmation = use_signal(|| None::<Confirmation>);
    let mut server = use_signal(|| status.server_ip.clone());
    let mut username = use_signal(|| status.saved_username.clone());
    let mut password = use_signal(String::new);
    let mut api_key = use_signal(String::new);
    let settings = page == Page::Settings;
    let main = view() == View::Main;
    let connection = view() == View::Connection;
    let showing_downloads = view() == View::Downloads;
    let title = if connection {
        "Connection settings"
    } else if showing_downloads {
        "Downloads"
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
            "data-ember-view": "{title}",
            "aria-busy": busy,
            onkeydown: move |event| event.stop_propagation(),
            onmounted: move |_| {
                spawn(async move {
                    let _ = document::eval(include_str!("ember.js")).await;
                });
            },
            header { class: "ember-header",
                div { class: "ember-brand", "manga4deck" }
                nav { class: "ember-nav", "aria-label": "Main navigation",
                    for (index, label) in ["Home", "Library", "Downloads", "Settings"].into_iter().enumerate() {
                        button {
                            r#type: "button",
                            class: "ember-nav-item",
                            "aria-current": if (index == 0 && !settings && !showing_downloads) || (index == 3 && settings && !showing_downloads) || (index == 2 && showing_downloads) { "page" } else { "false" },
                            "data-nav": "{index}",
                            "data-focus-row": "0", "data-focus-col": "{index}",
                            disabled: busy,
                            onclick: move |_| {
                                confirmation.set(None);
                                password.set(String::new());
                                api_key.set(String::new());
                                view.set(if index == 2 { View::Downloads } else { View::Main });
                                match index {
                                    0 => onaction.call(EmberAction::Home),
                                    1 => onaction.call(EmberAction::Library),
                                    3 => onaction.call(EmberAction::Settings),
                                    _ => {}
                                }
                            },
                            "{label}"
                        }
                    }
                }
                div { class: "ember-connection",
                    if settings || showing_downloads {
                        span { class: "ember-status-dot", "aria-hidden": "true" }
                        if status.offline_mode { "Offline mode" } else if status.logged_as.is_empty() { "Not signed in" } else { "Kavita online" }
                    }
                }
            }
            main { class: "ember-content", "aria-labelledby": "ember-title",
                h1 { id: "ember-title", "{title}" }
                if showing_downloads {
                    if downloads.is_empty() {
                        section { class: "ember-empty",
                            h2 { "Your offline library starts here." }
                            p { "Download a series from your library to read without a connection." }
                            button {
                                class: "ember-primary", r#type: "button", disabled: busy,
                                "data-autofocus": "true", "data-focus-row": "1", "data-focus-col": "0",
                                onclick: move |_| { view.set(View::Main); onaction.call(EmberAction::Library); },
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
                } else if settings {
                    section { class: "ember-status", role: "status", "aria-live": "polite",
                        strong { class: if status.offline_mode || status.logged_as.is_empty() { "" } else { "ember-online" }, "{account}" }
                        p {
                            if status.offline_mode { "Reconnect to browse your Kavita server." } else { "{status.server_ip}" }
                        }
                    }
                    div { class: "ember-actions",
                        for (index, label, detail) in [
                            (0, "Kavita".to_string(), "Browse your libraries".to_string()),
                            (1, "Clean cache".to_string(), format!("{} MB on this device", status.cache_mb)),
                            (2, "Update".to_string(), "Update server library".to_string()),
                            (3, "Connection".to_string(), "Server and credentials".to_string()),
                            (4, if status.offline_mode { "Offline" } else { "Online" }.to_string(), if status.offline_mode { "Switch to online" } else { "Switch to offline" }.to_string()),
                            (5, "Exit".to_string(), "Close manga4deck".to_string()),
                        ] {
                            button {
                                r#type: "button", class: "ember-action", disabled: busy,
                                "data-autofocus": if index == 3 { "true" } else { "false" },
                                "data-connection": if index == 3 { "true" } else { "false" },
                                "data-focus-row": "{index / 3 + 1}", "data-focus-col": "{index % 3}",
                                onclick: move |_| match index {
                                    0 => onaction.call(EmberAction::Library),
                                    1 => { confirmation.set(Some(Confirmation::Cache)); },
                                    2 => onaction.call(EmberAction::Update),
                                    3 => open_connection.call(()),
                                    4 => onaction.call(EmberAction::ToggleOffline),
                                    5 => { confirmation.set(Some(Confirmation::Exit)); },
                                    _ => {}
                                },
                                strong { "{label}" }
                                span { "{detail}" }
                            }
                        }
                    }
                    p { class: "ember-note", "Manage your device and Kavita connection." }
                } else {
                    if let Some(read) = reading {
                        HomeProgress { key: "{read.volume.id}", read, busy, onaction }
                    } else {
                        section { class: "ember-empty", "aria-live": "polite",
                            h2 { if reading_loading { "Loading your reading progress…" } else { "Your next volume is waiting." } }
                            if !reading_loading {
                                p { "Start a volume from your library. Your reading progress will appear here." }
                                button {
                                    class: "ember-primary", r#type: "button", disabled: busy,
                                    "data-autofocus": "true", "data-focus-row": "1", "data-focus-col": "0",
                                    onclick: move |_| onaction.call(EmberAction::Library),
                                    "Browse library"
                                }
                            }
                        }
                    }
                }
            }
            footer { class: "ember-footer", "aria-label": "Controller shortcuts",
                div { class: "ember-hints",
                    for (key, label) in [("↕", "Move"), ("A", "Open"), ("B", "Back"), (if settings && main { "Y" } else { "X" }, if settings && main { "Connection" } else { "Downloads" })] {
                        span { class: "ember-hint", kbd { "{key}" } span { "{label}" } }
                    }
                }
                span { class: "ember-keyboard-hint", "Enter / Esc" }
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
fn HomeProgress(read: HomeRead, busy: bool, onaction: EventHandler<EmberAction>) -> Element {
    let mut cover_failed = use_signal(|| false);
    let cover = crate::local_api::media_url(&format!("series-cover/{}", read.volume.series_id));
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
                if !cover_failed() {
                    img { src: "{cover}", alt: "{read.title} cover", width: "48", height: "72", onerror: move |_| cover_failed.set(true) }
                } else { span { "No cover" } }
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
