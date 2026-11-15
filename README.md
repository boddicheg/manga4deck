# Manga4Deck

A manga, manhwa, and PDF reader for Steam Deck and self-hosted Kavita servers, built with Rust and Dioxus.

- Horizontal library gallery and a volume list for each series
- Scrolling reader with a page slider
- Offline reading and progress sync

## Getting started

Install Rust and Cargo, plus the native UI dependencies on Linux.

From the repository root:

```bash
cargo run --manifest-path src/Cargo.toml
```

Open **Settings → Connection**, enter your Kavita address as `host:port` (for example, `192.168.1.100:5000`), your credentials and API key, then select **Save & connect**. Connections currently use HTTP.

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
| Toggle focused volume as read/unread (online) | Y or F1 |
| Manage series downloads | F2 |

The first Left or Right press opens the slider; subsequent presses change the page. You can also tap the page counter and drag the slider. It hides after five seconds of inactivity.

F2 starts a series download, stops an active download, or removes a fully cached series. It works from the library gallery, series detail, and reader.

In series detail, the button shows **Remove downloaded** as soon as any volume is saved. The button and F2 then remove that series' downloaded pages, including partial series downloads, and stop any active download. Removal also works offline.

Home, Library, series detail, and Settings also support standard gamepads directly: the D-pad or left stick moves focus, A opens or applies a filter, B goes back, X opens Downloads, and Y opens Connection from Settings. On a focused library title, X manages its download. In the volume list, Y toggles the focused volume as read/unread; X has no action. With a keyboard, use arrows and Enter/Space, Esc to go back, X for Downloads, and S for Settings. Text fields keep their normal editing keys.

Libraries open with **Unread**, which includes unstarted and partially read series. Switch to **All series**, **Downloaded**, or **Completed** as needed. The series count appears beside **Your library**. Narrow portrait cards show progress below covers: blue for partial progress, green for completed series. Long titles scroll like a bus display when focused or hovered; reduced-motion mode shows wrapped titles. Left/Right browses the full title row; Up moves from titles to filters and then to navigation tabs. Extra titles scroll into view, and your filter and title selection are restored when returning from a volume.

Series sort newest to oldest by when Kavita last added content to them. Dates refresh when you open a library online and are cached for offline browsing. Series without a date appear last; equal dates sort by title. Reading progress does not change this order.

Opening a series shows its cover, author and genres when available, and a scrolling volume list. The first unfinished volume receives focus; if every volume is complete, the first volume receives focus. Use Up/Down to browse, A/Enter to read, B/Esc to return to the library, and Y or F1 to toggle the focused volume as read/unread while online. Use the download button or F2 to manage the series download. Left moves to the download button; Right returns to the volume list. Returning from the reader restores the volume you opened. Author and genre metadata are cached for offline use.

Volume rows show an animated download indicator beside reading progress. It changes to a green **Downloaded** checkmark once every page is saved. A divider separates download status from reading progress; smaller screens show the status icon without its text label. Reduced-motion mode keeps the download indicator still.

Home shows your most recent volume and a horizontal row of your Kavita libraries. Use Left/Right or the scroll arrows to browse folders, then A/Enter to open one. Back returns to the selected folder on Home. Settings contains connection status, cache cleanup, server updates, connection details, online/offline mode, and Exit.

## Offline reading

Download a series before disconnecting, then use **Settings → Online** to switch to offline mode. Downloads skip completed chapters. Offline browsing shows series whose remaining unread chapters are fully cached.

Download failures show the HTTP status and Kavita's message. If extraction fails, check the affected archive and its permissions on the server. The app skips the remaining pages of a failed volume, continues with other volumes, and keeps saved pages for retry.

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
