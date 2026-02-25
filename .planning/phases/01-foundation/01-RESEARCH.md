# Phase 1: Foundation — Research

**Phase:** 01-foundation
**Researched:** 2026-02-25
**Status:** Complete

---

## What This Research Covers

Phase 1 delivers four distinct capabilities that must work correctly from day one because later phases build on them:

1. **FLP binary parser** — reads .flp event stream, extracts BPM, time signature, channels, plugins, pattern count
2. **SQLite metadata cache** — caches parsed results, keyed correctly, survives re-scans
3. **Settings persistence** — three folder paths survive app restart
4. **GitHub Actions Windows build pipeline** — produces a .msi installer without a local Windows machine

This document answers the questions a planner needs to design all four plans in Phase 1 without stumbling into known traps.

---

## Plan 01-01: Tauri Scaffold, Rust Workspace, SQLite Schema

### What the Scaffold Looks Like

Tauri v2 projects have a split structure: frontend at the repo root (`src/`), Rust backend inside `src-tauri/`. The `create-tauri-app` CLI creates this scaffold but is not required — the structure can be set up manually. For this project with vanilla JS (no framework), the structure is:

```
flp-vault/
├── src/                         # Frontend — HTML/CSS/JS, no framework
│   ├── index.html
│   ├── styles/main.css
│   └── js/
│       ├── main.js
│       ├── api.js               # invoke() wrappers
│       └── panels/              # UI panel modules
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # App entry, Tauri builder, setup
│   │   ├── commands/            # Thin #[tauri::command] handlers
│   │   ├── services/            # Business logic (scanner, grouper, organizer)
│   │   ├── parser/              # FLP binary parser (pure: bytes in, FlpMetadata out)
│   │   ├── store/               # SQLite layer (connection, migrations, queries)
│   │   └── state.rs             # AppState struct: Mutex<Connection> + scan_status
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── capabilities/
│       └── default.json         # Tauri permission allowlisting
└── package.json                 # For @tauri-apps/api
```

### Key Cargo.toml Dependencies for Phase 1

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.38", features = ["bundled"] }
walkdir = "2"
dirs = "5"
xxhash-rust = { version = "0.8", features = ["xxh3"] }
chrono = { version = "0.4", features = ["serde"] }
```

Phase 1 does NOT need: nom (manual cursor parsing is simpler), strsim/trigram (Phase 2), notify (Phase 2 watch mode), tauri-plugin-notification (Phase 2).

### SQLite Setup Pattern

The database lives in `%APPDATA%\FLP Vault\metadata.db` — never inside the Cryptomator vault. Getting this path uses Tauri's path resolution:

```rust
// src-tauri/src/store/connection.rs
use std::path::Path;
use rusqlite::{Connection, Result};
use std::sync::Mutex;

pub fn init_db(app_data_dir: &Path) -> Result<Mutex<Connection>> {
    std::fs::create_dir_all(app_data_dir).ok();
    let db_path = app_data_dir.join("metadata.db");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA synchronous=NORMAL;"
    )?;
    run_migrations(&conn)?;
    Ok(Mutex::new(conn))
}
```

In `main.rs` setup hook:
```rust
.setup(|app| {
    let app_data_dir = app.path().app_data_dir()?;
    let db = init_db(&app_data_dir)?;
    app.manage(AppState { db: Arc::new(db) });
    Ok(())
})
```

**Critical:** `create_dir_all` before `Connection::open` — Tauri does NOT automatically create the app data directory on first run. This is a known gotcha (Tauri Discussion #11279).

### SQLite Schema for Phase 1

Phase 1 needs three tables. The cache key design is load-bearing — changing it later requires a migration that must re-parse everything.

```sql
-- files: metadata cache, keyed by content hash
CREATE TABLE IF NOT EXISTS files (
    hash         TEXT PRIMARY KEY,    -- xxhash3 of file bytes (hex)
    path         TEXT NOT NULL,       -- last known absolute path
    file_size    INTEGER NOT NULL,    -- for mtime+size shortcut
    mtime        INTEGER NOT NULL,    -- last_modified as unix timestamp
    bpm          REAL,                -- NULL if unparseable
    time_sig_num INTEGER,             -- e.g. 4 (numerator)
    time_sig_den INTEGER,             -- e.g. 4 (denominator)
    channel_count INTEGER,
    pattern_count INTEGER,
    mixer_track_count INTEGER,
    plugins_json TEXT,                -- JSON array of {name, type: "generator"|"effect"}
    warnings_json TEXT,               -- JSON array of parse warning strings
    fl_version   TEXT,                -- FL Studio version string from file
    parsed_at    INTEGER NOT NULL     -- unix timestamp of when we parsed this
);

-- path_index: maps current path → hash (for "has this path changed?" shortcut)
CREATE TABLE IF NOT EXISTS path_index (
    path         TEXT PRIMARY KEY,
    hash         TEXT NOT NULL,
    file_size    INTEGER NOT NULL,
    mtime        INTEGER NOT NULL,
    FOREIGN KEY (hash) REFERENCES files(hash)
);

-- settings: key/value store for user configuration
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

**Why two tables instead of one:** `files` is the authoritative record keyed by content hash. `path_index` enables the mtime+size shortcut: on re-scan, look up path in `path_index` — if mtime and size match, return the linked hash without opening the file. If they don't match, open the file, compute hash, look up in `files`. This design handles renames (same hash, new path) correctly.

### Settings Keys

```
source_folder       — absolute path string
organized_folder    — absolute path string
originals_folder    — absolute path string
```

Smart defaults (from CONTEXT.md):
- `source_folder` defaults to FL Studio's default: `%USERPROFILE%\Documents\Image-Line\FL Studio\Projects`
- `organized_folder` defaults to `%USERPROFILE%\Documents\FLP Vault`
- `originals_folder` defaults to `%USERPROFILE%\Documents\FLP Vault Originals`

These defaults are applied the first time settings are read if no value is set.

### AppState Pattern

```rust
// src-tauri/src/state.rs
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub scan_status: Mutex<ScanStatus>,
}

pub struct ScanStatus {
    pub total: usize,
    pub done: usize,
    pub running: bool,
}
```

Registered with `app.manage(AppState { ... })` in setup. Commands receive it via `state: State<'_, AppState>`.

---

## Plan 01-02: FLP Binary Parser

### FLP Format Structure

The .flp binary format has two sections:

**Header chunk (16 bytes total):**
```
[0..4]  Magic: b"FLhd"
[4..8]  Size: u32_le = 6 (always 6)
[8..10] Format: u16_le (project format version)
[10..12] num_channels: u16_le (channel rack count — this IS the channel count we need)
[12..14] ppq: u16_le (pulses per quarter note — timing resolution, not time signature)
[14..16] (end of header, size was 6 so 6 bytes after offset 8 = up to offset 14)
```

Wait — size=6 means the data section after "FLhd" is 6 bytes, so the header is: magic(4) + size(4) + data(6) = 14 bytes. Then "FLdt" starts at byte 14.

**Data chunk:**
```
[0..4]  Magic: b"FLdt"
[4..8]  Size: u32_le (total bytes of event data that follows)
[8..]   Event stream
```

### Event Encoding

Every event starts with a 1-byte event ID. The ID determines how many bytes of value follow:

| ID Range | Value Size | Notes |
|----------|-----------|-------|
| 0–63 | 1 byte | BYTE events |
| 64–127 | 2 bytes (u16 LE) | WORD events |
| 128–191 | 4 bytes (u32 LE) | DWORD events |
| 192–255 | varint + payload | Variable-length (strings, structs) |

Variable-length events (192–255): after the 1-byte ID, the next bytes encode the payload length as a varint. The varint encoding: read bytes while the high bit is set; each 7 bits contribute to the length (little-endian). Then read that many bytes of payload.

**Varint decode (from pyflp documentation):**
```rust
fn read_varint(cursor: &mut Cursor<&[u8]>) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0;
    loop {
        let byte = cursor.read_u8().unwrap();
        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 { break; }
        shift += 7;
    }
    result
}
```

### Known Event IDs for Phase 1

These are the event IDs needed to satisfy PARS-02, PARS-03, PARS-04:

**Tempo/BPM:**
- Event ID `156` (= 128 + 28, DWORD): `ProjectID.Tempo` — stored as `BPM * 1000` as u32. To get BPM: `value as f32 / 1000.0`. Supports fractional BPM (e.g., 128.5 BPM stored as 128500).
- Event ID `66` (= 64 + 2, WORD): `FLP_Tempo` — older format, coarse BPM as u16. Present in files from FL Studio 4 era.
- Event ID `93` (= 64 + 29, WORD): `ProjectID._TempoFine` — fine tempo adjustment.

**Strategy:** Check for event `156` first (modern format). If not found, fall back to event `66`. Both may appear; prefer `156` for precision.

**Channel count:** Stored directly in the file header `num_channels` field (bytes 10–12). This is the number of channels in the channel rack. No event needed — just read the header.

**Channel names:** Event ID `192` (TEXT, variable-length): `FLP_Text_ChanName`. A new channel begins when event `64` (WORD: `FLP_NewChan`) fires. The channel name event `192` names the most recently opened channel. Channels are parsed sequentially — maintain a mutable current channel in the parser loop.

**Channel type (generator vs effect):** Event ID `21` (BYTE): `FLP_ChanType`. Values map to:
- `0` = Sampler
- `1` = Native plugin (generator/instrument)
- `2` = Layer
- `3` = Instrument (VST/AU generator)
- `4` = Automation clip
- Mixer effects appear as separate InsertID events, not via FLP_ChanType.

For the purpose of Phase 1: generators are channel types 0, 1, 2, 3 (in the channel rack). Effects are on mixer inserts (separate event stream). The channel rack count from the header counts generators only.

**Plugin name (channel rack plugins):** Event ID `201` (TEXT, variable-length): `FLP_Text_PluginName`. Contains the plugin file name (without path). For native FL Studio plugins, this is the built-in name. For VSTs, this is the VST DLL filename. Combined with `FLP_ChanType`, this tells you what plugin is on each channel.

**Plugin name (new-style):** Event ID `212` (TEXT, variable-length): `FLP_NewPlugin`. Used in newer FL Studio versions.

**Pattern count:** Event ID `65` (WORD): `FLP_NewPat`. Each occurrence creates a new pattern. Count the number of times this event fires — that is the pattern count.

**Mixer track count:** The mixer in newer FL Studio versions uses `InsertID.Flags` events to denote insert tracks. Count occurrences of `InsertID.Flags` = event `159` (= 128 + 31? — exact ID needs verification from pyflp source). For a simpler approach in Phase 1: count occurrences of event `129` (`FLP_PlayListItem`) is not right either. **Practical Phase 1 approach:** Count events with ID `202` (TEXT: `FLP_Text_FXName` or mixer channel name). The number of distinct mixer channel name events = mixer track count.

**Time signature:** The FLP format does not appear to store time signature as discrete header fields that are cleanly accessible. PyFLP's `Project` class has no `TimeSig` in `ProjectID`. The `PPQ` (pulses per quarter) in the header controls timing resolution. `FLP_PatLength` (event 17, BYTE) stores steps per bar and is the closest approximation of time signature numerator. `FLP_BlockLength` (event 18, BYTE) relates to step count. For Phase 1, parse what's available and document "time_sig may be unavailable for older files."

**FL Studio version string:** Event ID `199` (TEXT, variable-length): `FLP_Version`. Contains the FL Studio version that saved this file (e.g., "21.2.3.4109"). Extract this for display and debugging.

### Parser Implementation Pattern

Use `std::io::Cursor` with the `byteorder` crate (or Rust's `read_u8/u16_le/u32_le`). Do NOT use nom — the architecture research (ARCHITECTURE.md, Pattern 4) explicitly chose manual cursor parsing as simpler for this sequential TLV format.

```rust
// src-tauri/src/parser/flp.rs
use std::io::{Cursor, Read};

pub struct FlpMetadata {
    pub bpm: Option<f32>,
    pub time_sig_num: Option<u8>,
    pub time_sig_den: Option<u8>,
    pub channel_count: u16,       // from header
    pub pattern_count: u16,
    pub mixer_track_count: u16,
    pub generators: Vec<ChannelInfo>,
    pub effects: Vec<String>,     // mixer effect names
    pub fl_version: Option<String>,
    pub warnings: Vec<String>,
}

pub struct ChannelInfo {
    pub name: String,
    pub plugin_name: Option<String>,
    pub channel_type: u8,
}

pub fn parse_flp(bytes: &[u8]) -> Result<FlpMetadata, ParseError> {
    let mut cursor = Cursor::new(bytes);
    let header = parse_header(&mut cursor)?;

    // Validate FLdt magic
    // ... read FLdt + size

    let mut meta = FlpMetadata {
        channel_count: header.num_channels,
        ..Default::default()
    };

    let mut current_channel: Option<ChannelInfo> = None;
    let mut current_plugin_name: Option<String> = None;
    let mut current_channel_type: Option<u8> = None;

    loop {
        let event_id = match read_u8(&mut cursor) {
            Ok(id) => id,
            Err(_) => break,  // EOF = done
        };

        match event_id {
            // BYTE events (0-63)
            0..=63 => {
                let value = read_u8(&mut cursor)?;
                match event_id {
                    21 => current_channel_type = Some(value),  // FLP_ChanType
                    17 => meta.time_sig_num = Some(value),      // FLP_PatLength
                    18 => meta.time_sig_den = Some(value),      // FLP_BlockLength
                    _ => {}  // skip unknown
                }
            }
            // WORD events (64-127)
            64..=127 => {
                let value = read_u16_le(&mut cursor)?;
                match event_id {
                    64 => {  // FLP_NewChan — flush previous channel, start new
                        if let Some(ch) = current_channel.take() {
                            meta.generators.push(ch);
                        }
                        current_channel = Some(ChannelInfo {
                            name: String::new(),
                            plugin_name: current_plugin_name.take(),
                            channel_type: current_channel_type.unwrap_or(0),
                        });
                        current_plugin_name = None;
                        current_channel_type = None;
                    }
                    65 => { meta.pattern_count += 1; }  // FLP_NewPat
                    66 => {  // FLP_Tempo (legacy BPM)
                        if meta.bpm.is_none() {
                            meta.bpm = Some(value as f32);
                        }
                    }
                    _ => {}
                }
            }
            // DWORD events (128-191)
            128..=191 => {
                let value = read_u32_le(&mut cursor)?;
                match event_id {
                    156 => {  // ProjectID.Tempo = 128+28
                        meta.bpm = Some(value as f32 / 1000.0);
                    }
                    _ => {}
                }
            }
            // Variable-length events (192-255)
            192..=255 => {
                let len = read_varint(&mut cursor)? as usize;
                let mut payload = vec![0u8; len];
                cursor.read_exact(&mut payload)?;

                match event_id {
                    192 => {  // FLP_Text_ChanName
                        let name = decode_string(&payload);
                        if let Some(ch) = current_channel.as_mut() {
                            ch.name = name;
                        }
                    }
                    199 => {  // FLP_Version
                        meta.fl_version = Some(decode_string(&payload));
                    }
                    201 => {  // FLP_Text_PluginName
                        current_plugin_name = Some(decode_string(&payload));
                    }
                    _ => {}  // PARS-05: skip unknown variable events silently
                }
            }
        }
    }

    // Flush last channel
    if let Some(ch) = current_channel.take() {
        meta.generators.push(ch);
    }

    Ok(meta)
}
```

### PARS-05: Forward Compatibility (Skip Unknown Events)

The critical requirement: "Parser skips unknown event IDs without error." The pattern above achieves this naturally — the `_ => {}` arms handle all unknown event IDs by skipping the correctly-sized payload. This is correct because:
- Unknown byte events: consumed 1 byte (correct)
- Unknown word events: consumed 2 bytes (correct)
- Unknown dword events: consumed 4 bytes (correct)
- Unknown variable events: consumed `len` bytes via `read_exact` (correct)

**Never add a panic or error for unknown event IDs.** The architecture must allow FL Studio 25, 26, etc. to add new event IDs without breaking the parser.

### String Encoding

FLP strings in text events are null-terminated. Some are ASCII, some are UTF-16 LE (newer FL Studio versions). Detection approach:
- If the payload starts with a UTF-16 BOM or contains alternating null bytes, decode as UTF-16 LE
- Otherwise, decode as UTF-8/ASCII, treating unknown bytes as replacement characters
- Strip the trailing null byte(s)

### Parser Resilience (Best-Effort Extraction)

From CONTEXT.md: "best-effort parsing for all .flp versions — files going back to FL Studio 4 should be attempted." This means:
- Return partial results if parsing fails partway through (some fields populated, some None)
- Add to `warnings` list: `"Unknown events skipped: [list of IDs]"`, `"BPM not found"`, etc.
- Never return an error for a parseable file — even an empty `FlpMetadata` with all-None fields is better than an error that prevents the file from appearing in the UI at all
- Only return `Err(ParseError)` for completely invalid files (wrong magic bytes, truncated header)

### Parser Sanity Ranges

From PITFALLS.md: add sanity checks to reject garbage data:
- BPM: clamp to 1.0–999.0 BPM. Reject values outside this range with a warning.
- Channel count (from header): 0–999. More than 999 channels is malformed.
- Pattern count: count up events naturally — no artificial limit needed.

---

## Plan 01-03: Scan Command, SQLite Cache, Settings Persistence

### Scan Architecture

The scan is a background task — from ARCHITECTURE.md Pattern 2. The `scan_folder` command immediately spawns a tokio task and returns. Progress streams via emitted events.

**Command signature:**
```rust
#[tauri::command]
pub async fn scan_folder(
    path: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn(async move {
        services::scanner::run_scan(&path, db, app_handle).await;
    });
    Ok(())
}
```

**Scanner service:**
```rust
pub async fn run_scan(path: &str, db: Arc<Mutex<Connection>>, app: AppHandle) {
    // 1. Walk directory recursively, collect .flp paths
    let files: Vec<PathBuf> = walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "flp"))
        .map(|e| e.path().to_path_buf())
        .collect();

    let total = files.len();
    app.emit("scan:started", json!({ "total": total })).ok();

    for (i, file_path) in files.iter().enumerate() {
        // 2. Stat the file
        let stat = match std::fs::metadata(&file_path) { Ok(s) => s, Err(_) => continue };
        let mtime = stat.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let file_size = stat.len() as i64;

        // 3. Check path_index for mtime+size shortcut
        let path_str = file_path.to_string_lossy().into_owned();
        if store::is_cached(&db, &path_str, file_size, mtime) {
            // Cache hit — emit progress and continue without re-parsing
            app.emit("scan:progress", json!({ "done": i+1, "total": total, "path": path_str, "cached": true })).ok();
            continue;
        }

        // 4. Read file bytes and parse
        let bytes = match std::fs::read(&file_path) { Ok(b) => b, Err(_) => continue };
        let hash = xxhash3(&bytes);

        // 5. Check if hash is known (file content identical, possibly renamed)
        if store::hash_in_cache(&db, &hash) {
            // Known content at new path — update path_index only
            store::update_path_index(&db, &path_str, &hash, file_size, mtime);
            app.emit("scan:progress", json!({ "done": i+1, "total": total, "path": path_str })).ok();
            continue;
        }

        // 6. Parse FLP
        let meta = parser::parse_flp(&bytes);

        // 7. Store in DB
        store::upsert_file(&db, &hash, &path_str, file_size, mtime, &meta);

        // 8. Emit progress with metadata for streaming into UI table
        app.emit("scan:progress", json!({
            "done": i+1,
            "total": total,
            "path": path_str,
            "meta": &meta   // FlpMetadata must implement Serialize
        })).ok();
    }

    app.emit("scan:complete", json!({ "total": total })).ok();
}
```

### Cache Key Design (Critical Decision)

From STATE.md: "SQLite cache key (path, size, mtime) must be correct from day one." The two-table design:

1. **Check `path_index(path, file_size, mtime)`** — O(1) lookup, costs only a `stat()` call on the file
2. **If mismatch:** compute hash, check `files(hash)` — handles renames where content is the same
3. **If new content:** parse and insert into `files` + update `path_index`

This means a scan of 500 already-cached files costs: 500 stat() calls + 500 SQLite lookups. No file reading, no parsing. This satisfies success criterion 2 ("re-scanning the same folder is fast").

### Settings Commands

```rust
#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    store::settings::get_all(&state.db)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(settings: Settings, state: tauri::State<'_, AppState>) -> Result<(), String> {
    store::settings::save_all(&state.db, &settings)
        .map_err(|e| e.to_string())
}
```

```rust
pub struct Settings {
    pub source_folder: String,
    pub organized_folder: String,
    pub originals_folder: String,
}
```

From CONTEXT.md: "Changing source folder triggers an automatic rescan." This means the frontend, on receiving a successful `save_settings` response where `source_folder` changed, immediately calls `scan_folder`.

### Settings Validation

From CONTEXT.md: "Validation with warnings: check folder exists/writable, warn if source = organized, warn about Cryptomator vault latency. Warnings inform but don't block."

```rust
pub struct SettingsValidation {
    pub warnings: Vec<String>,  // non-blocking issues
    // No errors — save always succeeds if path is a valid string
}
```

Validate: `std::fs::metadata(path).is_ok()` to check existence. Warn if source == organized or organized == originals.

### Scan Results UI (Phase 1 Minimal)

From CONTEXT.md: The scan results view is a table with columns: filename, BPM, channels, plugins, modified date. All sortable, default by filename. Plugin column truncates to 2-3 plugins with "(+N more)" on hover.

The frontend listens for `scan:progress` events and appends rows to the table as they arrive:
```javascript
import { listen } from '@tauri-apps/api/event';

await listen('scan:progress', (event) => {
    const { path, meta } = event.payload;
    appendRowToTable(path, meta);
});
```

---

## Plan 01-04: GitHub Actions Windows Build Pipeline

### Why This Must Be Plan 4, Not an Afterthought

From STATE.md blocker: ".msi installer cannot be produced from Linux — GitHub Actions windows-latest runner must be wired in Phase 1 before any feature promises." From PITFALLS.md Pitfall 2: if the .msi build is only discovered to be broken at release time, setting up a Windows build environment under time pressure is HIGH recovery cost.

**Wire up the CI pipeline before writing the FLP parser or the UI.** This is unconventional but correct — confirming the artifact delivery pipeline works first means every subsequent push produces a testable installer.

### The Cross-Compilation Problem

From STACK.md: "Building from Linux requires NSIS (setup.exe), not .msi — WiX doesn't work on Linux." The authoritative DIST-01/DIST-02 requirement says ".msi or NSIS .exe" — both are valid. The GitHub Actions approach produces the .msi directly on a Windows runner, which is the cleanest solution.

For local dev on Linux:
```bash
# Cross-compile produces NSIS setup.exe (NOT .msi)
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
# User must run: apt install clang lld llvm (no sudo available to claude)
cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc
```

**This is for dev iteration only.** The deliverable .msi comes from CI.

### GitHub Actions Workflow

Complete working workflow:

```yaml
# .github/workflows/build.yml
name: Build Windows Installer

on:
  push:
    branches: [main, release]
  pull_request:
    branches: [main]

jobs:
  build-windows:
    runs-on: windows-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - name: Install frontend dependencies
        run: npm install

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: v__VERSION__
          releaseName: 'FLP Vault v__VERSION__'
          releaseBody: 'Windows installer produced by CI.'
          releaseDraft: true
          prerelease: false
```

**What this produces:** The `tauri-action` on `windows-latest` runs `cargo tauri build` on a real Windows machine and uploads the `.msi` installer as a GitHub Release artifact. Draft releases are created — user promotes to published.

**Secrets required:** Only `GITHUB_TOKEN`, which GitHub provides automatically. No code signing certificates needed for the initial build (unsigned installer). SmartScreen will warn on the unsigned binary, but it's runnable for testing.

**WebView2 consideration:** From STATE.md: "WebView2 bootstrapper mode needed for Windows 10 targets." In `tauri.conf.json`:
```json
{
  "bundle": {
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      }
    }
  }
}
```
`downloadBootstrapper` adds ~0 MB to installer size and downloads WebView2 at install time if missing. WebView2 is pre-installed on Windows 11 and most updated Windows 10 systems. This satisfies DIST-02 (under 15 MB).

### Installer Size Budget

From STACK.md: target 5-10 MB. Tauri v2 MSI baseline is 2-10 MB. With `downloadBootstrapper` (not offline installer), the installer itself stays small. The Rust binary for this app (FLP parser, SQLite, Tauri) will be 3-6 MB. Total installer: ~5-8 MB — within budget.

### VBSCRIPT Requirement

From Tauri Windows Installer docs: MSI packaging requires the VBSCRIPT optional feature to be enabled on Windows. The `windows-latest` GitHub Actions runner has this enabled. Users installing the .msi on Windows 10/11 typically have it enabled by default; add to release notes if issues arise.

---

## Cross-Cutting: What to Get Right in Every Plan

### Tauri IPC Pattern

All Rust commands registered in `main.rs`:
```rust
tauri::Builder::default()
    .manage(app_state)
    .invoke_handler(tauri::generate_handler![
        commands::scan::scan_folder,
        commands::scan::cancel_scan,
        commands::settings::get_settings,
        commands::settings::save_settings,
        commands::browse::list_scanned_files,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

Frontend invoke:
```javascript
import { invoke } from '@tauri-apps/api/core';
const result = await invoke('scan_folder', { path: '/path/to/folder' });
```

### Permission Allowlisting (tauri.conf.json)

Tauri v2 requires explicit capability declarations. For Phase 1, minimal capabilities:
```json
// src-tauri/capabilities/default.json
{
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default"
  ]
}
```

No filesystem plugin needed for Phase 1 — file operations go through Rust commands that use `std::fs` directly. The frontend doesn't touch the filesystem.

### Error Handling

All command return types: `Result<T, String>`. The `String` error is displayed in the UI. Never panic in a command. Parse failures return `Ok(FlpMetadata::partial())` with warnings populated, not `Err`.

---

## Decision Points That Need Resolution Before Planning

These are gaps or ambiguities that could derail planning:

### 1. Mixer track count event ID (Medium risk)

The exact event ID for "mixer insert track count" is uncertain from the pyflp source that was accessible. The `MixerID` enum has `APDC = 29` and `Params = DATA+17`. The actual mixer insert count mechanism uses `InsertID.Flags` events. This needs verification against a real .flp file or the pyflp source code.

**Resolution during Plan 01-02:** Write the parser, test against sample files, and empirically determine the correct event ID. Count occurrences of `InsertID.Flags` (likely around event ID `159` = 128+31) or use a heuristic (count distinct mixer channel name events at 202 or 203).

**Fallback:** If mixer track count is unreliable, Phase 1 reports it as a best-effort field with a `?` warning. Mixer track count is required by PARS-04 but not by the scan table UI (CONTEXT.md shows: filename, BPM, channels, plugins, modified date — no mixer column).

### 2. Time signature in header vs events (Low risk)

The FLP header contains `ppq` (pulses per quarter note) which is NOT the time signature numerator/denominator. The time signature may be stored in `FLP_PatLength` (event 17) or not stored explicitly at all. PyFLP's `Project` class has no time signature event in `ProjectID`.

**Resolution during Plan 01-02:** Parse FLP_PatLength (event 17) and FLP_BlockLength (event 18) experimentally. If they don't map cleanly to a 4/4 vs 3/4 distinction, document that time signature is parsed on a best-effort basis. The CONTEXT.md says "BPM and time signature" under PARS-02 — but the format may not support this cleanly for all file versions.

### 3. Code signing for the installer (Low risk, but must be decided)

The Phase 1 CI pipeline produces an **unsigned** installer. Users on Windows will see a SmartScreen warning ("Windows protected your PC"). This is acceptable for development/testing but not for shipping to end users.

**Resolution:** Phase 1 produces unsigned .msi for internal use. Code signing (OV or EV certificate, or Azure Trusted Signing) is a DIST-01 concern that can be addressed before v0.1 release, not in Phase 1 foundational build.

### 4. Frontend approach: script modules or bundled? (Low risk)

The frontend is vanilla JS. Tauri expects a `dist/` directory with static files. Options:
- **No bundler:** Write ESM modules directly in `src/`, point `devPath` and `distDir` at `src/`. No build step needed. Simplest for Phase 1.
- **Vite:** Bundling + HMR. Adds ~100ms build time. Overkill for Phase 1.

**Recommendation:** No bundler for Phase 1. `tauri.conf.json` `devPath: "src"`, `distDir: "src"`. Import `@tauri-apps/api` from CDN or copy the dist file into `src/js/vendor/`. Add a bundler in Phase 3 if the frontend grows complex.

---

## Known Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| Parser breaks on FL Studio 4-era files (very old format) | HIGH | LOW | Best-effort parsing: return partial metadata with warnings; never crash |
| BPM event ID 156 not present in old files | HIGH | LOW | Fall back to event ID 66 (older WORD tempo event) |
| Mixer track count ID is wrong | MEDIUM | LOW | Mixer count is a nice-to-have in Phase 1 UI; PARS-04 requires it but it can be 0/unknown |
| Cross-compile from Linux fails for dev iteration | HIGH | LOW | Acceptable — CI produces the real .msi. Local NSIS is for smoke tests only |
| DB path not created on Windows before first `Connection::open` | HIGH | MEDIUM | `create_dir_all` before `Connection::open` (known gotcha) |
| Time signature not readable from FLP format cleanly | MEDIUM | LOW | Document as best-effort; show "?" in UI for files where it can't be determined |
| unsigned .msi triggers SmartScreen on every install | HIGH | LOW | Expected for dev builds; code signing added before v0.1 public release |

---

## What Phase 1 Does NOT Include

Clarifying scope from CONTEXT.md to avoid scope creep in plans:

- No fuzzy matching — that is Phase 2
- No file copying/organization — that is Phase 3
- No three-panel browse UI — that is Phase 3 (Phase 1 has a scan results table only)
- No filesystem watcher/system tray — that is Phase 2 (v0.2)
- No toast notifications
- No version diff view
- No "Open in FL Studio" button (requires tauri-plugin-shell, add in Phase 3)

The Phase 1 UI is intentionally minimal: settings panel + scan results table. That is all.

---

## Sources and Confidence

| Claim | Source | Confidence |
|-------|--------|-----------|
| FLP header: FLhd magic, 6-byte data, num_channels, ppq | [PyFLP format docs](https://pyflp.readthedocs.io/en/v2.2.1/architecture/flp-format.html) | HIGH |
| Event ID ranges (0-63/64-127/128-191/192-255) | [PyFLP format docs](https://pyflp.readthedocs.io/en/v2.2.1/architecture/flp-format.html) + [FLP_Format reference](https://github.com/andrewrk/PyDaw/blob/master/FLP_Format) | HIGH |
| Event 156 = Tempo (DWORD, BPM*1000) | [PyFLP project.py source](https://pyflp.readthedocs.io/en/latest/_modules/pyflp/project.html) | HIGH |
| Event 66 = FLP_Tempo (WORD, legacy BPM) | [FLP_Format reference](https://github.com/andrewrk/PyDaw/blob/master/FLP_Format) | HIGH |
| Event 192 = FLP_Text_ChanName | [FLP_Format reference](https://github.com/andrewrk/PyDaw/blob/master/FLP_Format) | HIGH |
| Event 64 = FLP_NewChan | [FLP_Format reference](https://github.com/andrewrk/PyDaw/blob/master/FLP_Format) | HIGH |
| Event 21 = FLP_ChanType (generator vs effect) | [PyFLP channel.py](https://raw.githubusercontent.com/demberto/PyFLP/master/pyflp/channel.py) | HIGH |
| Event 201 = FLP_Text_PluginName | [FLP_Format reference](https://github.com/andrewrk/PyDaw/blob/master/FLP_Format) | HIGH |
| Event 65 = FLP_NewPat (pattern count by counting fires) | [FLP_Format reference](https://github.com/andrewrk/PyDaw/blob/master/FLP_Format) | HIGH |
| Mixer track count via InsertID.Flags event | [PyFLP mixer.py](https://raw.githubusercontent.com/demberto/PyFLP/master/pyflp/mixer.py) | MEDIUM (exact ID unverified) |
| WiX MSI cannot be built from Linux | [Tauri cross-platform docs](https://v1.tauri.app/v1/guides/building/cross-platform/) + [Discussion #3291](https://github.com/orgs/tauri-apps/discussions/3291) | HIGH |
| GitHub Actions windows-latest produces .msi via tauri-action | [Tauri GitHub pipeline docs](https://v2.tauri.app/distribute/pipelines/github/) | HIGH |
| WebView2 downloadBootstrapper mode | [Tauri Windows Installer docs](https://v2.tauri.app/distribute/windows-installer/) | HIGH |
| Tauri app_data_dir → %APPDATA% on Windows | [Tauri PathResolver docs](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html) | HIGH |
| create_dir_all needed before Connection::open | [Tauri Discussion #11279](https://github.com/orgs/tauri-apps/discussions/11279) | HIGH |
| Mutex<Connection> + WAL pattern for Tauri+SQLite | [ARCHITECTURE.md](../research/ARCHITECTURE.md) (pre-research) | HIGH |
| rusqlite bundled feature = no system sqlite3 dependency | [STACK.md](../research/STACK.md) (pre-research) | HIGH |

---

## RESEARCH COMPLETE

**Summary:** Phase 1 is well-understood with one medium-risk unknown (mixer track count exact event ID) and one low-risk unknown (time signature extraction). All four plans have clear implementation patterns. The most important non-obvious finding is that the GitHub Actions Windows pipeline must be wired up in Plan 01-04 (the first plan, before any features) to prove the delivery path works. The FLP parser should be built with explicit permissive-forward design: `_ => {}` for all unknown event IDs in every size category.

**Planning can proceed.**
