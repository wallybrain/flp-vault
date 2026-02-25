# Architecture Research

**Domain:** Tauri desktop app — binary file parsing, fuzzy matching, filesystem watching, SQLite metadata cache
**Researched:** 2026-02-25
**Confidence:** HIGH (Tauri IPC, state management, SQLite patterns) / MEDIUM (FLP parsing strategy, fuzzy matching composition)

## Standard Architecture

### System Overview

```
+=====================================================================+
|  WEBVIEW LAYER (HTML/CSS/JS)                                        |
|  +------------------+  +------------------+  +------------------+  |
|  |   Song List      |  |  Version         |  |  Version Detail  |  |
|  |   Panel          |  |  Timeline Panel  |  |  / Diff Panel    |  |
|  +------------------+  +------------------+  +------------------+  |
|  +------------------+  +------------------+                        |
|  |   Scan / Review  |  |  Settings        |                        |
|  |   Workflow       |  |  Panel           |                        |
|  +------------------+  +------------------+                        |
|                                                                     |
|  invoke() calls (commands)     listen() (events from backend)      |
+============================IPC======================================+
|  RUST BACKEND (Tauri Core + tokio)                                  |
|                                                                     |
|  +-----------------+   +------------------+  +------------------+  |
|  |  Command Layer  |   |  FS Watcher      |  |  System Tray     |  |
|  |  (handlers)     |   |  (notify crate)  |  |  (tray-icon)     |  |
|  +-----------------+   +------------------+  +------------------+  |
|          |                      |                     |            |
|          v                      v                     |            |
|  +-----------------+   +------------------+           |            |
|  |  App Services   |   |  Background      |           |            |
|  |  (orchestrates) |<--|  Task Queue      |<----------+            |
|  +-----------------+   |  (tokio::mpsc)   |                        |
|     |       |          +------------------+                        |
|     v       v                                                       |
|  +------+  +----------+  +--------------+  +-------------------+  |
|  | FLP  |  | Fuzzy    |  | File         |  | SQLite Store      |  |
|  | Parse|  | Matcher  |  | Manager      |  | (rusqlite +       |  |
|  | (nom)|  |(strsim/  |  |(copy, dedup) |  |  Mutex<Conn>)     |  |
|  |      |  | trigram) |  |              |  |                   |  |
|  +------+  +----------+  +--------------+  +-------------------+  |
+============================IPC======================================+
|  FILESYSTEM LAYER                                                   |
|  +-----------------+  +-----------------+  +-------------------+  |
|  |  Source Folder  |  |  Organized      |  |  Originals        |  |
|  |  (Cryptomator   |  |  Folder         |  |  Folder           |  |
|  |   vault mount)  |  |  (per-song dirs)|  |  (safety copies)  |  |
|  +-----------------+  +-----------------+  +-------------------+  |
|                                                                     |
|  %APPDATA%\FLP Vault\metadata.db  (SQLite, outside vault)          |
+=====================================================================+
```

### Component Responsibilities

| Component | Responsibility | Key Dependencies |
|-----------|----------------|-----------------|
| Command Layer | Route invoke() calls from webview to services, serialize responses | tauri::command macro, serde |
| App Services | Orchestrate workflows (scan, group, copy, watch), hold shared state | All services below |
| FLP Parser | Read .flp binary, extract BPM, channel names, plugin IDs, pattern count | nom or manual byte parsing |
| Fuzzy Matcher | Group filenames by song using trigram similarity + BPM + date signals | strsim or trigram crate |
| File Manager | Copy files to organized/originals dirs, track dedup by hash, batch-pace ops | std::fs, sha256 |
| FS Watcher | Monitor source folder for new .flp files, debounce events 2-3s | notify-debouncer-mini |
| SQLite Store | Persist song groups, file metadata cache, settings, copy-tracking | rusqlite, Mutex<Connection> |
| System Tray | Background presence, toast notifications, right-click menu | tauri tray-icon feature |
| Background Task Queue | Run long scans without blocking command responses | tokio::mpsc channels |

## Recommended Project Structure

```
flp-vault/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # App entry, Tauri builder, setup, state init
│   │   ├── commands/
│   │   │   ├── mod.rs           # generate_handler! registration
│   │   │   ├── scan.rs          # scan_folder, get_scan_progress
│   │   │   ├── groups.rs        # get_groups, merge_groups, split_group, rename_group
│   │   │   ├── files.rs         # execute_organize, get_copy_status
│   │   │   ├── browse.rs        # list_songs, get_versions, get_version_detail, diff_versions
│   │   │   ├── search.rs        # search_songs, filter_by_plugin, filter_by_bpm
│   │   │   └── settings.rs      # get_settings, save_settings
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── scanner.rs       # Orchestrates scan: walk dir, parse FLPs, emit progress
│   │   │   ├── grouper.rs       # Fuzzy grouping logic, group review operations
│   │   │   └── organizer.rs     # Copy logic, dedup, batch pacing
│   │   ├── parser/
│   │   │   ├── mod.rs
│   │   │   ├── flp.rs           # Top-level FLP binary parser
│   │   │   ├── events.rs        # TLV event reader (event ID + value)
│   │   │   └── types.rs         # FlpMetadata, Channel, Plugin structs
│   │   ├── matcher/
│   │   │   ├── mod.rs
│   │   │   ├── trigram.rs       # Filename trigram similarity
│   │   │   ├── signals.rs       # BPM signal, temporal clustering signal
│   │   │   └── scorer.rs        # Combine signals into group score
│   │   ├── store/
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs    # DB init, WAL mode, migration, Mutex<Connection>
│   │   │   ├── songs.rs         # Song group queries
│   │   │   ├── files.rs         # File metadata + copy-tracking queries
│   │   │   └── settings.rs      # Settings persistence
│   │   ├── watcher/
│   │   │   ├── mod.rs
│   │   │   └── handler.rs       # notify-debouncer-mini setup, event dispatch
│   │   └── state.rs             # AppState struct (DB conn, watcher handle, scan status)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/
└── src/                         # Frontend (HTML/CSS/JS, no framework needed)
    ├── index.html
    ├── styles/
    │   └── main.css             # Dark theme
    └── js/
        ├── main.js              # App init, panel management
        ├── panels/
        │   ├── song-list.js     # Left panel: list, search, filters
        │   ├── timeline.js      # Middle panel: version list
        │   └── detail.js        # Right panel: metadata, diff
        ├── workflow/
        │   ├── scan.js          # Scan progress UI, review UI
        │   └── organize.js      # Execute organize, progress
        └── api.js               # invoke() wrappers for all commands
```

### Structure Rationale

- **commands/:** One file per domain area. Each file contains only `#[tauri::command]` functions — no business logic. Commands are thin wrappers that delegate to services.
- **services/:** Business logic lives here. Services hold no Tauri-specific types — they accept plain Rust types and return Results, making them testable without a Tauri runtime.
- **parser/:** Isolated from everything else. Parser takes `&[u8]` and returns `FlpMetadata`. No filesystem I/O, no DB access. Pure, testable.
- **matcher/:** Isolated scorer. Takes filenames + metadata, returns group proposals. No DB, no filesystem.
- **store/:** All SQLite access behind a service boundary. No SQL in commands or services.
- **watcher/:** Isolated async subsystem. Communicates via tokio channels only.
- **state.rs:** Single `AppState` struct registered with `app.manage()`. All shared mutable state centralised here.

## Architectural Patterns

### Pattern 1: Command as Thin Dispatcher

**What:** `#[tauri::command]` functions contain zero business logic. They lock state, call a service, return the result.

**When to use:** Always. Business logic in commands cannot be unit tested without spinning up a Tauri app.

**Trade-offs:** Slightly more files, but services become independently testable with `cargo test`.

**Example:**
```rust
// commands/scan.rs
#[tauri::command]
pub async fn scan_folder(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<ScanStarted, String> {
    let db = state.db.clone();
    // Spawn long-running scan — don't block the command
    tauri::async_runtime::spawn(async move {
        scanner::run_scan(&path, db, app_handle).await;
    });
    Ok(ScanStarted { job_id: Uuid::new_v4().to_string() })
}
```

### Pattern 2: Progress via Emitted Events

**What:** Long-running tasks (scan, organize) run in `tokio::spawn`. They emit named events back to the frontend via `app_handle.emit()` with progress payloads.

**When to use:** Any operation that takes more than ~100ms. Never block a command handler.

**Trade-offs:** Frontend must listen for events and update state. Cannot use invoke() return value for streaming data.

**Example:**
```rust
// services/scanner.rs
pub async fn run_scan(path: &str, db: Arc<Mutex<Connection>>, app: AppHandle) {
    let files = walk_dir(path);
    let total = files.len();
    for (i, file) in files.iter().enumerate() {
        let meta = parser::parse_flp(file);
        store::upsert_file(&db, &meta);
        app.emit("scan:progress", json!({ "done": i + 1, "total": total })).ok();
    }
    app.emit("scan:complete", json!({})).ok();
}
```

### Pattern 3: Mutex-Protected Single SQLite Connection

**What:** One `Connection` wrapped in `Mutex<Connection>`, stored in `AppState`. All database access locks this mutex, runs the query, releases. WAL mode enabled at startup.

**When to use:** This is the standard pattern for Tauri + SQLite desktop apps. Not a connection pool — a single connection with WAL is sufficient for a single-user desktop app.

**Trade-offs:** Simple. No connection pool overhead. WAL mode means reads and writes don't block each other. The mutex is never held across await points — only during synchronous DB calls.

**Example:**
```rust
// store/connection.rs
pub fn init_db(app_data_dir: &Path) -> Result<Mutex<Connection>> {
    let db_path = app_data_dir.join("metadata.db");
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    run_migrations(&conn)?;
    Ok(Mutex::new(conn))
}

// store/files.rs
pub fn upsert_file(db: &Mutex<Connection>, meta: &FlpMetadata) -> Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO files (hash, path, bpm, ...) VALUES (?1, ?2, ?3, ...)",
        params![meta.hash, meta.path, meta.bpm],
    )?;
    Ok(())
}
```

### Pattern 4: FLP Binary Parsing via Manual TLV Reader

**What:** FLP files are structured as a header (magic + 6-byte fixed data) followed by a stream of TLV events. Each event has a 1-byte ID and a value whose size is determined by the ID range (1/2/4/variable bytes). Parse events sequentially, pick out only the event IDs you need.

**When to use:** This project. The FLP format is not a well-structured binary format — nom parser combinators add overhead without benefit for this specific sequential event-stream format.

**Trade-offs:** Manual `Cursor<&[u8]>` with `read_u8/u16/u32/le` calls is simpler and faster than nom for this use case. The pyflp Python library's event table is the authoritative reference for event IDs.

**Example:**
```rust
// parser/flp.rs
pub fn parse_flp(bytes: &[u8]) -> Result<FlpMetadata> {
    let mut cursor = Cursor::new(bytes);
    validate_header(&mut cursor)?;  // "FLhd" magic + channel count
    let mut meta = FlpMetadata::default();
    while let Ok(event_id) = cursor.read_u8() {
        let value = read_event_value(&mut cursor, event_id)?;
        match event_id {
            // BPM stored as fixed-point in event 28 (tempo)
            28 => meta.bpm = Some(decode_bpm(&value)),
            // Channel names in variable-length string events
            192..=255 => handle_variable_event(event_id, &value, &mut meta),
            _ => {} // Skip unknown events
        }
    }
    Ok(meta)
}
```

### Pattern 5: Watcher as Isolated Async Subsystem

**What:** The filesystem watcher runs in its own tokio task from app setup. It sends detected file paths through a `tokio::mpsc` channel to a handler task, which applies 2-3s debounce, parses the new file, queries the DB for a match, and emits a frontend event.

**When to use:** Watch mode. The watcher must survive window close (tray mode) and must not block the UI.

**Trade-offs:** Slightly more complex startup code in `main.rs` setup. But the watcher is completely decoupled from the command system — it never touches the command layer.

**Example:**
```rust
// main.rs setup
let (tx, mut rx) = tokio::sync::mpsc::channel::<PathBuf>(32);
let app_handle_clone = app_handle.clone();
tauri::async_runtime::spawn(async move {
    while let Some(path) = rx.recv().await {
        tokio::time::sleep(Duration::from_secs(2)).await; // debounce
        watcher::handle_new_file(path, &app_handle_clone).await;
    }
});
let _watcher = setup_notify_watcher(source_dir, tx);
app.manage(WatcherHandle(_watcher));
```

### Pattern 6: File Hashing for Dedup and Cache Keying

**What:** Every .flp file is identified by its SHA-256 content hash, not its path. The metadata cache is keyed on hash. Copy-tracking records (hash, destination) to prevent re-copying on re-scan.

**When to use:** This project specifically — files may be renamed or moved between scans. Hash-based identity is stable across renames.

**Trade-offs:** SHA-256 of a ~10 MB .flp file takes ~5ms. With 500 files on first scan, that's ~2.5s hashing overhead. Acceptable and cacheable (mtime + size shortcut for subsequent scans).

## Data Flow

### Flow 1: Initial Scan and Group

```
User clicks "Scan"
    |
    v
invoke("scan_folder", { path })          [JS → Rust command]
    |
    v
commands::scan::scan_folder()
    |-- tokio::spawn(scanner::run_scan())
    |
    v  (immediately returns ScanStarted to frontend)
    |
    scanner::run_scan() [background task]
        |-- walk_dir(source_path) → Vec<PathBuf>
        |-- for each file:
        |       hash = sha256(file_bytes)
        |       if store::file_in_cache(hash) → skip
        |       meta = parser::parse_flp(file_bytes)
        |       store::upsert_file(hash, meta)
        |       app.emit("scan:progress", { done, total })
        |
        v
    app.emit("scan:complete", {})        [Rust → JS event]
        |
        v
    invoke("get_groups")                 [JS → Rust command]
        |
        v
    commands::groups::get_groups()
        |-- matcher::propose_groups(all_files_from_db)
        |       trigram_similarity + bpm_signal + temporal_clustering
        |       → Vec<ProposedGroup>
        |-- return groups to frontend
        |
        v
    Review UI renders for user
```

### Flow 2: Browse / Version Detail

```
User clicks song in Song List
    |
    v
invoke("get_versions", { song_id })
    |
    v
store::songs::get_versions(song_id)
    |-- SELECT files WHERE group_id = ? ORDER BY mtime ASC
    |-- return Vec<VersionSummary>
    |
    v
Timeline Panel renders versions

User clicks version → invoke("get_version_detail", { file_hash })
    |-- return FlpMetadata from cache (no re-parse needed)
    |
User selects two versions → invoke("diff_versions", { hash_a, hash_b })
    |-- load both FlpMetadata from store
    |-- compute delta: bpm change, channel delta, plugins added/removed
    |-- return DiffResult
```

### Flow 3: Watch Mode New File Detection

```
FL Studio saves Song Name 5.flp to source folder
    |
    v
notify-debouncer-mini fires event (Create/Modify)
    |-- 2s debounce window elapses
    |
    v
watcher::handler::handle_new_file(path)
    |-- hash = sha256(file)
    |-- if hash in store → ignore (already known)
    |-- meta = parser::parse_flp(file)
    |-- candidates = matcher::find_candidates(meta, store::all_songs())
    |
    +-- High confidence (score > 0.85):
    |       app.emit("watcher:match", { path, song_name, song_id, confidence })
    |       frontend shows toast "Song Name 5 → Song Name" [auto-file in 10s]
    |
    +-- Ambiguous (0.4 < score < 0.85):
    |       app.emit("watcher:ambiguous", { path, candidates })
    |       frontend shows popup with suggestions
    |
    +-- No match (score < 0.4):
            app.emit("watcher:unknown", { path })
            frontend prompts: "Name new song or file to _Unsorted"
```

### State Management Flow

```
AppState (registered via app.manage())
    |
    +-- db: Mutex<Connection>          (all DB access via lock/unlock)
    +-- watcher_handle: Option<...>    (keep watcher alive)
    +-- scan_status: Mutex<ScanStatus> (current/total for progress polling fallback)

Commands access state via:
    State<'_, AppState> parameter injection  (Tauri manages Arc internally)

Background tasks access state via:
    AppHandle passed at spawn time
    app_handle.state::<AppState>()
```

## Build Order Implications

Building in dependency order prevents rework:

1. **SQLite Store first** — Every other component depends on it. Stand up the DB schema, migration system, and `Mutex<Connection>` pattern before writing anything else.

2. **FLP Parser second** — Core value is metadata extraction. Must work before scan, fuzzy matching, or any UI. Write with unit tests on sample .flp files.

3. **Fuzzy Matcher third** — Depends on `FlpMetadata` types from parser. Develop and tune against real filenames before integrating with UI.

4. **App Services + Command Layer fourth** — Wire parser, matcher, and store together. Implement scan workflow, group workflow, organize workflow as service functions. Expose via commands.

5. **Frontend last** — Build against the real command API. No mocking needed — commands are already real. Three-panel layout, scan workflow, review UI.

6. **FS Watcher + System Tray** — These are v0.2 features. Entire v0.1 ships without them. Add after the core browse + organize loop is working.

## Integration Points

### Internal Boundaries

| Boundary | Communication Method | Notes |
|----------|---------------------|-------|
| Webview ↔ Rust commands | `invoke()` / `listen()` — JSON-serialized | All types must derive serde Serialize/Deserialize |
| Commands ↔ Services | Direct Rust function calls | Services return `Result<T, AppError>` |
| Services ↔ Parser | Direct call: `parser::parse_flp(&bytes)` | Parser is pure — no side effects |
| Services ↔ Matcher | Direct call: `matcher::propose_groups(&files)` | Matcher is pure — no side effects |
| Services ↔ Store | Direct call with `&Mutex<Connection>` | Store locks mutex per operation, not per call-site |
| Background tasks ↔ Frontend | `app_handle.emit(event, payload)` | One-way push. Frontend registers `listen()` handlers |
| Watcher ↔ Handler | `tokio::mpsc::channel::<PathBuf>` | Decouples OS events from application logic |

### External Boundaries

| Boundary | Integration | Notes |
|----------|------------|-------|
| Filesystem | `std::fs` read + copy only | Never write to source folder. Copy, never move. |
| SQLite | rusqlite direct, single connection | DB lives in `%APPDATA%\FLP Vault\` not in vault |
| FL Studio | None — no process communication | App detects FL Studio's output files; does not call FL Studio |
| Cryptomator mount | Treated as normal filesystem path | App has no knowledge of encryption layer |

## Anti-Patterns

### Anti-Pattern 1: Business Logic in Command Handlers

**What people do:** Write SQL queries and matching logic directly inside `#[tauri::command]` functions.

**Why it's wrong:** `tauri::command` functions cannot be unit-tested without running a full Tauri app. Scan logic buried in commands is impossible to test with real .flp files.

**Do this instead:** Commands are one-liners that call into `services::`. All real logic lives in services and is tested with `cargo test`.

### Anti-Pattern 2: Blocking Long Ops in Commands

**What people do:** Call `scan_folder()` synchronously in the command handler, returning only after all files are parsed.

**Why it's wrong:** Blocks the Tauri IPC thread. UI freezes. On 500 files, the scan takes 10-30 seconds. The `invoke()` call will appear to hang.

**Do this instead:** `tokio::spawn` the scan immediately. Return a job ID. Emit progress events. Frontend uses `listen()` to update a progress bar.

### Anti-Pattern 3: Moving Files Instead of Copying

**What people do:** Use `std::fs::rename()` to move files into organized folders because it's atomic and fast.

**Why it's wrong:** In a Cryptomator/cloud sync context, rename = delete + create. The source file is deleted from the vault, triggering a sync delete event. If the copy to organized folder hasn't been uploaded yet, the file is at risk during any network interruption.

**Do this instead:** `std::fs::copy()` always. The source file remains intact. Originals folder provides a safety copy. Deletion is the user's explicit action after review.

### Anti-Pattern 4: Holding Mutex Across Await Points

**What people do:** `let guard = db.lock().unwrap(); do_async_thing().await; use guard;`

**Why it's wrong:** A `std::sync::Mutex` guard held across `.await` is not `Send`. It won't compile in async contexts. Even with a tokio `Mutex`, it blocks other DB access for the duration of the async operation.

**Do this instead:** Lock, do the synchronous DB work, release immediately. Never hold a DB lock across an await point. If an async operation requires multiple DB queries, do them in separate lock/unlock cycles.

### Anti-Pattern 5: Keying File Cache by Path

**What people do:** Use `filepath → metadata` as the cache key.

**Why it's wrong:** FLP files get renamed frequently (producer workflow). Renaming a file produces a cache miss and triggers re-parsing + re-grouping for a file the app has already seen.

**Do this instead:** Key by SHA-256 content hash. A shortcut: skip full hash if `(mtime, size)` matches a known file — only compute hash on first encounter or when `(mtime, size)` changes.

## Scaling Considerations

This is a single-user desktop app. Scaling is about file count, not concurrent users.

| File Count | Concern | Approach |
|------------|---------|---------|
| 0-500 files | Initial target | In-memory matching, synchronous scan acceptable at this scale for most ops |
| 500-2000 files | Scan time | Background task with progress (Pattern 2) handles this. Hash shortcut (mtime+size) eliminates re-parsing known files |
| 2000+ files | Memory pressure from loading all metadata for matching | Stream matching from DB rather than loading all into Vec. Index BPM and mtime columns. |

### Scaling Priorities

1. **First bottleneck:** Scan time on initial import of 500+ files. Mitigation: background task + progress events + hash-based skip for known files (already designed in).
2. **Second bottleneck:** Fuzzy matching quality on files with ambiguous names. Mitigation: multi-signal scoring (trigram + BPM + temporal) and manual review UI to correct mistakes. Not a performance problem.

## Sources

- [Tauri IPC Architecture](https://v2.tauri.app/concept/inter-process-communication/) — HIGH confidence, official docs
- [Tauri Calling Rust from Frontend](https://v2.tauri.app/develop/calling-rust/) — HIGH confidence, official docs
- [Tauri Calling Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) — HIGH confidence, official docs
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) — HIGH confidence, official docs
- [Tauri System Tray](https://v2.tauri.app/learn/system-tray/) — HIGH confidence, official docs
- [Tauri Project Structure](https://v2.tauri.app/start/project-structure/) — HIGH confidence, official docs
- [notify-rs/notify (filesystem watching)](https://github.com/notify-rs/notify) — HIGH confidence, official GitHub
- [File Watcher with Debouncing in Rust](https://oneuptime.com/blog/post/2026-01-25-file-watcher-debouncing-rust/view) — MEDIUM confidence, verified against notify docs
- [rusqlite documentation](https://docs.rs/rusqlite/latest/rusqlite/) — HIGH confidence, official docs
- [PyFLP FLP Format Architecture](https://pyflp.readthedocs.io/en/latest/architecture/flp-format.html) — HIGH confidence for format structure; used as reference for parser design
- [strsim-rs](https://github.com/rapidfuzz/strsim-rs) — HIGH confidence, official GitHub
- [nucleo fuzzy matcher](https://github.com/helix-editor/nucleo) — MEDIUM confidence, used by Helix editor
- [Tauri async runtime patterns](https://rfdonnelly.github.io/posts/tauri-async-rust-process/) — MEDIUM confidence, community post verified against Tauri docs
- [Long-running background tasks in Tauri v2](https://sneakycrow.dev/blog/2024-05-12-running-async-tasks-in-tauri-v2) — MEDIUM confidence, community post

---
*Architecture research for: FLP Vault — Tauri + Rust desktop app with binary parsing, fuzzy matching, filesystem watching, SQLite*
*Researched: 2026-02-25*
