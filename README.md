# Manga4Deck

A manga, manhwa, and PDF reader for Steam Deck and self-hosted Kavita servers, built with Rust and Dioxus.

- Library and chapter shelves
- Scrolling reader with a page slider
- Offline reading and progress sync

## Getting started

Install Rust and Cargo, plus the native UI dependencies on Linux.

From the repository root:

```bash
cargo run --manifest-path src/Cargo.toml
```

Open **Settings**, enter your Kavita address as `host:port` (for example, `192.168.1.100:5000`), your credentials and API key, then select **Save**. Connections currently use HTTP.

Leave the password or API key blank to keep its saved value.

## Controls

Map Steam Deck buttons to these keys:

| Action | Key |
| --- | --- |
| Activate selected tile | Enter or Space |
| Go back | Backspace |
| Move shelf selection | Arrow keys |
| Scroll reader | Up / Down |
| Show or move page slider | Left / Right |
| Toggle chapter or volume as read/unread (online) | F1 |
| Manage series downloads | F2 |

The first Left or Right press opens the slider; subsequent presses change the page. You can also tap the page counter and drag the slider. It hides after five seconds of inactivity.

F2 starts a series download, stops an active download, or removes a fully cached series. It works from the series shelf, volume shelf, and reader.

## Offline reading

Download a series before disconnecting, then switch the dashboard's Online/Offline tile to offline. Downloads skip completed chapters. Offline browsing shows series whose remaining unread chapters are fully cached.

Progress follows the visible page, and reopening a chapter resumes there. Updates are saved locally before upload; failed uploads are retried on the next page update or reconnect.

PDF reading requires Kavita's rendered PDF image support (`extractPdf=true`). The first page may take longer to render. Multi-chapter volumes appear as separate chapter entries.

On Steam Deck/Linux, settings and cached pages are stored in `~/.local/share/manga4deck-cache/`.

## Build

```bash
cargo check --manifest-path src/Cargo.toml --locked
cargo build --manifest-path src/Cargo.toml --release --locked
```

The app uses authenticated local services on `127.0.0.1:11337` (HTTP) and `127.0.0.1:11338` (WebSocket).

## Screenshots

![Dashboard](images/1.png)

![Series shelf](images/2.png)

![Reader](images/3.png)
