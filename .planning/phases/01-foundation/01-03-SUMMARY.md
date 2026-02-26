---
phase: 01-foundation
plan: 03
subsystem: ui
tags: [tauri, rust, walkdir, xxhash-rust, sqlite, vanilla-js, tauri-plugin-dialog]

# Dependency graph
requires:
  - phase: 01-01
    provides: AppState, SQLite store (is_cached, upsert_file, list_all_files, get_all_settings, set_setting), ScanStatus, frontend shell
  - phase: 01-02
    provides: parse_flp(&[u8]) -> Result<FlpMetadata, ParseError>
provides:
  - scan_folder Tauri command: spawns background scanner thread
  - cancel_scan Tauri command: sets Arc<Mutex<bool>> running flag
  - get_settings / save_settings Tauri commands with folder validation warnings
  - list_scanned_files Tauri command returns Vec<FileRecord> from SQLite
  - scanner service: WalkDir traversal, mtime+size cache shortcut, xxh3 hash, parse_flp, upsert_file, progress events
  - scan:started / scan:progress / scan:complete / scan:cancelled Tauri events
  - api.js: typed invoke wrappers and event listener helpers
  - scan-table.js: sortable table with live scan streaming, progress bar, empty state
  - settings-panel.js: slide-out panel with native OS folder pickers
  - Full dark-theme CSS: table, progress bar, settings overlay
affects: [01-04, 02-01, 02-02, 02-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - ScanStatus.running is Arc<Mutex<bool>> not plain bool — cloned into scanner thread for cancel support
    - Scanner spawned in std::thread::spawn (not tokio) — no async runtime needed for sync file I/O
    - Scanner emits events via AppHandle.emit() — zero-copy IPC to frontend
    - Cache shortcut: is_cached(path, size, mtime) checked first; hash only computed if mtime/size changed
    - Hash shortcut: hash_in_cache(hash) before parse_flp — avoids re-parsing moved files
    - Frontend panels use safe DOM APIs only (makeEl helper, textContent, appendChild) — no innerHTML
    - Settings panel uses dynamic import('@tauri-apps/plugin-dialog') for native folder picker
    - Scan table streams rows live from scan:progress events, re-sorts on each batch

key-files:
  created:
    - src-tauri/src/commands/scan.rs
    - src-tauri/src/commands/settings.rs
    - src-tauri/src/commands/browse.rs
    - src-tauri/src/services/scanner.rs
    - src/js/panels/scan-table.js
    - src/js/panels/settings-panel.js
  modified:
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/services/mod.rs
    - src-tauri/src/main.rs
    - src-tauri/src/state.rs
    - src-tauri/src/store/settings.rs
    - src-tauri/src/store/files.rs
    - src/index.html
    - src/js/main.js
    - src/js/api.js
    - src/styles/main.css

key-decisions:
  - "ScanStatus.running changed to Arc<Mutex<bool>> so cancel_scan and scanner thread can share it without unsafe"
  - "Scanner uses std::thread::spawn not tokio — Tauri commands are sync; no async runtime needed"
  - "Frontend uses makeEl() helper with textContent/appendChild — avoids innerHTML XSS risk even for trusted Tauri IPC data"
  - "Plugin folder picker uses dynamic import('@tauri-apps/plugin-dialog') inside click handler — defers to runtime Tauri context"
  - "Scan table re-renders full tbody on each progress event — acceptable for <10k files; virtualisation deferred"

patterns-established:
  - "Pattern: Arc<Mutex<bool>> for cross-thread flags in Tauri commands — clone Arc before spawn, check inside loop"
  - "Pattern: makeEl(tag, props) factory with textContent/appendChild — safe DOM creation without innerHTML"
  - "Pattern: onScanX(callback) wrappers in api.js — hides Tauri listen() from panel code"

requirements-completed: [PARS-01, PARS-06, SETT-01, SETT-02, SETT-03]

# Metrics
duration: 5min
completed: 2026-02-26
---

# Phase 1 Plan 03: Tauri Commands and Frontend Summary

**Scan workflow wired end-to-end: five Tauri commands (scan_folder, cancel_scan, get_settings, save_settings, list_scanned_files), background scanner with mtime+size cache and xxh3 hash shortcuts, settings slide-out panel with native folder pickers, and sortable scan results table streaming live from scan events**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-25T23:59:37Z
- **Completed:** 2026-02-26T00:04:40Z
- **Tasks:** 2
- **Files modified:** 16

## Accomplishments

- Five Tauri commands registered and functional: scan_folder, cancel_scan, get_settings, save_settings, list_scanned_files
- Scanner service: WalkDir traversal, mtime+size cache shortcut (path_index), xxh3 hash for content-addressed lookup, parse_flp() on uncached files, progress event stream
- Settings panel: slide-out overlay from gear icon, three folder fields with native OS folder picker via @tauri-apps/plugin-dialog, validation warnings (non-existent dirs, conflicting paths), auto-rescan on source folder change
- Scan results table: 5 sortable columns (Name, BPM, Channels, Plugins, Modified), live streaming from scan:progress events, progress bar with cancel button, empty state with Settings link, warning indicator on parse-error rows
- ScanStatus.running changed from plain bool to Arc<Mutex<bool>> to safely share between cancel command and scanner thread
- All frontend DOM operations use safe textContent/appendChild via makeEl() — no innerHTML

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement Tauri commands and scanner service** - `24d239d` (feat)
2. **Task 2: Build scan results table and settings panel frontend** - `efe5461` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src-tauri/src/commands/scan.rs` — scan_folder (spawns thread) and cancel_scan (sets running=false)
- `src-tauri/src/commands/settings.rs` — get_settings and save_settings with SettingsValidation warnings
- `src-tauri/src/commands/browse.rs` — list_scanned_files from SQLite
- `src-tauri/src/commands/mod.rs` — module declarations and re-exports
- `src-tauri/src/services/scanner.rs` — run_scan: WalkDir, cache check, xxh3, parse_flp, emit events
- `src-tauri/src/services/mod.rs` — declares scanner module
- `src-tauri/src/main.rs` — all 5 commands registered in generate_handler![]
- `src-tauri/src/state.rs` — ScanStatus.running changed to Arc<Mutex<bool>>
- `src-tauri/src/store/settings.rs` — Settings derives Serialize/Deserialize for Tauri IPC
- `src-tauri/src/store/files.rs` — FileRecord derives Serialize for Tauri IPC
- `src/index.html` — main-content, settings-container, empty-state with icon and hint
- `src/js/main.js` — startup init: load settings, populate cache, wire buttons
- `src/js/api.js` — invoke wrappers and event listener helpers for all 5 commands + 4 events
- `src/js/panels/scan-table.js` — sortable table with live streaming, progress bar, empty state
- `src/js/panels/settings-panel.js` — slide-out overlay with folder pickers and save logic
- `src/styles/main.css` — dark theme: progress bar, sortable table, settings overlay, warning styles

## Decisions Made

- `ScanStatus.running` changed from `bool` to `Arc<Mutex<bool>>` so `cancel_scan` command and the scanner thread can share it safely. The Arc is cloned before thread spawn and checked between file iterations.
- Scanner runs in `std::thread::spawn` (not tokio) — Tauri commands are synchronous in this app; spawning an OS thread for I/O-bound scanning is simpler and avoids async runtime complexity.
- Frontend DOM uses a `makeEl()` helper factory with `textContent`/`appendChild` throughout. No `innerHTML` is used, avoiding XSS risk even though data comes from trusted Tauri IPC.
- `@tauri-apps/plugin-dialog` is dynamically imported inside the Browse button click handler, deferring resolution to runtime when Tauri context is available.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Security] Replaced innerHTML with safe DOM methods in frontend panels**
- **Found during:** Task 2 (settings-panel.js and scan-table.js)
- **Issue:** Pre-commit security hook flagged innerHTML usage as XSS risk. Even though FLP Vault data comes from local files via trusted Tauri IPC, using innerHTML as a habit is unsafe practice.
- **Fix:** Created `makeEl(tag, props)` factory function that builds DOM elements with `textContent` and `appendChild`. All dynamic content uses this pattern throughout both panel modules.
- **Files modified:** src/js/panels/scan-table.js, src/js/panels/settings-panel.js
- **Verification:** No innerHTML in either panel; security hook passes
- **Committed in:** efe5461 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 — security)
**Impact on plan:** Correct security practice. No scope change.

## Issues Encountered

- `Settings` struct in store/settings.rs was missing `Serialize/Deserialize` derives — required for Tauri command return type. Added as part of Task 1 (Rule 3 auto-fix, blocking).
- `FileRecord` struct in store/files.rs was missing `Serialize` derive — required for command return. Added as part of Task 1.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- All Phase 1 Plans 01-03 complete. Plan 04 (GitHub Actions CI/CD) can run `cargo check` as CI step with confidence.
- Phase 2 (fuzzy matching, version detection) can build on scanner service and SQLite cache directly.
- The scan pipeline is production-capable: handles parse errors gracefully, caches correctly, emits cancellable progress.

## Self-Check: PASSED

All key files verified present. Task commits 24d239d and efe5461 verified in git log.

---
*Phase: 01-foundation*
*Completed: 2026-02-26*
