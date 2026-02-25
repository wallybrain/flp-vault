# Pitfalls Research

**Domain:** Proprietary binary parser + fuzzy file organizer + Tauri desktop app on Windows
**Researched:** 2026-02-25
**Confidence:** HIGH (binary format, Tauri, filesystem watching), MEDIUM (fuzzy matching thresholds, Cryptomator interaction)

---

## Critical Pitfalls

### Pitfall 1: FLP Parser Breaks on Unknown FL Studio Versions

**What goes wrong:**
The .flp binary format is closed and undocumented. PyFLP (the reference implementation) is explicitly tested only on FL Studio 20+ and documents that "things can go wrong." When FL Studio releases a new version, it can add new event IDs or change event semantics — a Rust parser that panics or errors on unrecognized events will become unreliable every time the user updates FL Studio. The format is described by its own author as "a really bad and messy combination of Type-length-value encoded events and structs."

**Why it happens:**
Rust's exhaustive `match` idiom encourages treating unknown discriminants as errors. Binary parsers written against a known-good test corpus assume the corpus covers the format. When FL Studio 25 adds a new channel event ID, parsers written for FL Studio 21 silently produce corrupt metadata or hard-crash.

**How to avoid:**
Design the parser as explicitly permissive-forward: unknown event IDs must be skipped (not errored). Store raw event bytes for unknown IDs so they can be logged without causing failure. Parsed fields — especially BPM — must have sanity ranges (e.g., 10–999 BPM) so that misread bytes don't produce `BPM: 65535`. Emit a structured warning list alongside parse results: `ParseResult { metadata, warnings: Vec<ParseWarning> }`. Mark data from partially-parsed files as "low confidence" in the UI.

**Warning signs:**
- Parser returns Err() on any file from a new FL Studio release
- Parsed BPM values outside the 60–200 BPM range for real music projects
- No test coverage for files with event IDs not in the current spec

**Phase to address:** Phase 1 (FLP Parser) — build permissiveness in from the start, not as a patch.

---

### Pitfall 2: Cross-Compilation from Linux Cannot Produce a .msi Installer

**What goes wrong:**
The project is developed on Linux (this server) targeting Windows. Tauri's official documentation explicitly states: "you still can't build a MSI, because WiX doesn't work on Linux." The cross-compilation path for Linux → Windows MSVC is experimental, supports only NSIS installers at best, requires llvm-rc and a non-default linker, and has open bug reports for linker errors with 32-bit targets and "custom-protocol" feature flags. The result: CI passes on Linux, but the actual .msi deliverable cannot be built without a Windows environment.

**Why it happens:**
Tauri's bundler (WiX Toolset) uses Windows COM APIs for MSI generation. Linux cross-compilation support is explicitly experimental and limited. Developers assume `cargo tauri build` is the same regardless of host OS.

**How to avoid:**
Accept that the authoritative production build must run on Windows. Two practical approaches: (1) Use GitHub Actions with a `windows-latest` runner to build and produce the .msi artifact — CI handles the Windows build, Linux is for development only. (2) Use a Windows VM or Docker Windows container for final packaging. Set up the Windows build pipeline in Phase 1 before any features are built, so the build path is known-good from day one. Never promise a deliverable that hasn't been built end-to-end on the target host.

**Warning signs:**
- `cargo tauri build` has only been tested on Linux, not Windows
- No GitHub Actions CI workflow targeting `windows-latest`
- .msi installer is described as a deliverable but the build pipeline doesn't include a Windows runner

**Phase to address:** Phase 1 (Project Setup + Distribution pipeline) — wire up GitHub Actions Windows build before writing any feature code.

---

### Pitfall 3: Filesystem Watcher Fires on Cryptomator's Internal Sync Churn

**What goes wrong:**
Cryptomator on Windows mounts via WinFSP (a FUSE-like driver). When Proton Drive syncs files to/from the cloud, Cryptomator internally reads/rewrites its own `.c9r` encrypted blobs and metadata files. A filesystem watcher pointed at the Cryptomator virtual drive path receives events for this internal I/O — not just user-triggered FL Studio saves. Without careful filtering, the watcher floods the app with spurious notifications for files that aren't .flp files, and debounce logic may still fire on the encrypted metadata churn.

Additionally, when FL Studio writes a project file, it typically writes to a temp path then renames — generating `create`, `modify`, and `rename` events within milliseconds. Without debouncing, the same save triggers multiple "new file detected" toasts.

**Why it happens:**
Developers test the watcher with direct local filesystem paths and clean save scenarios. Cryptomator's virtual mount generates additional filesystem events invisible to a naive test. `ReadDirectoryChangesW` (used by the `notify` crate on Windows) fires on every filesystem operation at the watched path, including virtual filesystem metadata operations.

**How to avoid:**
Filter watcher events strictly to `.flp` extension before any debounce logic runs. Use a debounce window of at least 3 seconds (not the typical 500ms). Ignore events where the file doesn't exist at event-processing time (the file was a temp that already moved). Add a file-size sanity check — `.flp` files are minimally a few KB; a zero-byte or sub-1KB file is a temp artifact. Log all events that are filtered out during development to verify the filter is working. Use `notify-debouncer-full` (not `notify-debouncer-mini`) which handles rename chains correctly.

**Warning signs:**
- Watcher fires during Proton Drive sync when no FL Studio saves occurred
- Multiple "new file detected" toasts for a single FL Studio save
- Watcher fires for `.c9r` or other non-.flp extensions

**Phase to address:** Phase 2 (Watch Mode) — don't assume local filesystem behavior; test against a live Cryptomator mount.

---

### Pitfall 4: Fuzzy Matcher Produces Wrong Groupings That Are Hard to Undo

**What goes wrong:**
Trigram similarity alone produces a high false positive rate for short filenames and music-naming conventions. Examples: "Trap Beat 3" and "Trap Beat 5" score very high similarity but may be entirely different songs. "My Song v2" and "My Song v3" correctly match, but "Intro v2" and "Intro v3" from two different projects also match. Once the auto-grouper has proposed 300 groups and the user has manually reviewed 150 of them, discovering that the grouper used a too-low threshold and created 20 false-positive merges means re-reviewing from scratch.

A secondary failure: if grouping is destructive (immediately copies files based on proposed groups), a wrong group is harder to undo than if grouping is only a proposal.

**Why it happens:**
Trigram similarity is tuned on clean data. Music producers use abbreviations, numbers, and minimal names — exactly the cases where short strings produce noisy similarity scores. Temporal clustering helps but also misfires: a producer who works on multiple songs in the same week will have temporal clusters across different songs.

**How to avoid:**
Group proposals must be non-destructive — never copy files until the user has confirmed the grouping (or at least the batch is committed). Set the default similarity threshold high enough to avoid false positives (~0.75+) and require BPM agreement as a second signal before auto-grouping. Present the user with a confidence score per group (`High confidence: 94%`, `Low confidence: 61%`). Sort groups by ascending confidence in the review UI so the user spends attention where it matters. "Assign ungrouped" is equally important as "merge" — files with no confident match should be surfaced, not silently dropped.

**Warning signs:**
- Grouper puts files with clearly different BPMs into the same group
- Short filenames (under 10 characters) get merged with dissimilar songs
- No confidence score per group in the review UI

**Phase to address:** Phase 1 (Fuzzy Matcher + Review UI) — the review UI is load-bearing, not optional polish.

---

### Pitfall 5: File Operations on the Vault Trigger Cloud Sync Conflicts

**What goes wrong:**
Proton Drive syncs the Cryptomator vault continuously. If the app performs a batch copy of 500 files in rapid succession, it overwhelms the sync client — resulting in sync queue backup, "sync conflict" duplicates (e.g., `Song Name 2 (conflict).flp`), and in rare cases, partial uploads that corrupt the cloud copy. Additionally, `move` semantics (rename across directories) translate to `delete + create` in cloud sync — deleting the source before the destination is confirmed safe. If the app moves rather than copies, and the sync client hasn't yet uploaded the source, the file is effectively lost.

**Why it happens:**
Developers test on local filesystems where copy speed is disk-limited, not sync-limited. Bulk operations that complete in 2 seconds locally may take 20 minutes to sync — any crash or power loss during that window risks data loss. `fs::rename()` is cheaper than `fs::copy()` and is the default instinct.

**How to avoid:**
Copy always, never move or rename across directories. Implement configurable batch pacing: pause N seconds between batches of M files (defaults: 20 files / 5 second pause). Never delete source files — the originals backup copy is the safety net, not a convenience. After copying, write the copy record to SQLite and confirm the destination file exists and has the correct size before recording it as "done." Provide a progress UI that shows how many files remain so the user can pause if needed.

**Warning signs:**
- Any code path that calls `fs::rename()` across directory boundaries
- No batch pacing in the import workflow
- Source files deleted after copy

**Phase to address:** Phase 1 (File Manager component) — copy-not-move and pacing must be enforced at the lowest level, not patched on later.

---

### Pitfall 6: SQLite Metadata Cache Becomes Stale When Source Files Change Outside the App

**What goes wrong:**
The SQLite metadata cache stores parsed .flp metadata keyed by file path. If a user renames a file in Explorer, moves it in FL Studio's browser, or if cloud sync resolves a conflict by creating a renamed copy, the cache entry is orphaned. The app shows stale metadata for a path that now contains different data, or shows no metadata for a file that was renamed. Hash-keying partially solves this, but hashing 500 .flp files on every scan is slow if done naively.

A secondary issue: if the cache is keyed by path only, re-scanning after a reorganization causes every file to be re-parsed from scratch because all paths changed.

**Why it happens:**
Path-based caching is the obvious first implementation. Content-hash caching requires reading every file byte to compute the hash, which seems expensive at scan time.

**How to avoid:**
Key cache entries by `(path, file_size, last_modified_timestamp)` — not by path alone and not by content hash. This three-tuple is essentially free to compute (single `stat()` call), is stable when the file is unchanged, and correctly invalidates when the file is modified. Content hash is useful for deduplication (detecting when two paths contain the same project) but should be computed lazily, not on every scan. Track file moves by observing that `(content_hash, old_path)` now appears at a new path — optional, but prevents losing metadata after a rename.

**Warning signs:**
- Cache keyed by path string only
- Re-scanning after reorganization re-parses all files
- No stale-entry cleanup when cached paths no longer exist on disk

**Phase to address:** Phase 1 (SQLite Store design) — cache key design is load-bearing; changing it later requires a migration.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Parsing only the known-good FL Studio 20 event IDs | Simpler parser code | Parser fails on every new FL Studio release | Never — forward-unknown events must be skipped from day one |
| Keying SQLite cache by path only | Simpler queries | Stale cache after any rename or move | Never — use (path, size, mtime) triple from the start |
| Using `fs::rename()` for "move to organized folder" | Faster than copy | Delete + create in cloud sync; data loss risk | Never — copy always |
| Skipping manual review step, auto-applying grouping | Simpler UX | Wrong groupings silently applied to 500 files | Never — review UI is the product |
| Hardcoding debounce to 500ms | Feels responsive | Fires during cloud sync churn | Set to 3 seconds as configurable default |
| Building on Linux, assuming cross-compile produces .msi | Faster dev iteration | .msi cannot be built from Linux (WiX doesn't work on Linux) | Acceptable for dev; never for release builds |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Cryptomator virtual mount | Watch the vault root directly | Watch only the source folder path, filter to `.flp` extension, use 3s debounce |
| FL Studio "Save new version" | Assume `Song Name N.flp` pattern only | Handle: `Song Name 2.flp`, `Song Name_2.flp`, `Song Name v2.flp`, `Song Name2.flp` — FL Studio appends to whatever the current name is, so if name already contains a number it appends a second number |
| WebView2 on Windows | Assume it's always installed | Configure MSI to use `downloadBootstrapper` mode; WebView2 is preinstalled on Windows 11 but not guaranteed on Windows 10 without updates |
| Tauri system tray | Use `window.hide()` and leave process running | Known crash bug in Tauri v2: app hidden via `window.hide()` crashes ~50 minutes later — use `WindowEvent::CloseRequested` to hide to tray instead |
| Tauri single-instance plugin | Assume it prevents duplicate tray icons | Single-instance prevents app re-launch but the tray icon can duplicate if the app creates it twice at startup — create tray icon once on first `setup()` |
| SQLite in %APPDATA% | Use WAL mode by default | WAL mode is fine here (it's not in the cloud-synced vault) — but if %APPDATA% is itself synced (e.g., OneDrive folder redirect), WAL -wal and -shm files cause sync corruption |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Parsing all 500 .flp files serially on app launch | 30+ second startup time | Parse lazily: scan file list first (fast), parse metadata on demand or in background thread | Noticeable with 100+ files |
| JSON-serializing large scan results over Tauri IPC | 200ms+ IPC roundtrip on Windows for >10MB payloads | Stream results as they're parsed using Tauri events (`emit()`), don't return a 500-item array from a single command | Noticeable at 50+ files returned in one IPC call |
| O(n²) pairwise fuzzy comparison of 500 filenames | Group-proposals take 60+ seconds | Pre-bucket by BPM and prefix before comparing; only compare within buckets | Breaks at ~200 files with naive all-pairs comparison |
| Content-hashing all files at scan time | Scan takes minutes for large files | Hash lazily for dedup detection; use (path, size, mtime) for cache invalidation | Breaks at ~50 large (>10MB) .flp files |
| Watcher re-scanning the entire folder on any event | Constant I/O during cloud sync | Only re-parse the specific file that changed, not the whole folder | Breaks during any Proton Drive sync activity |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Constructing file paths by string concatenation with user-supplied folder paths | Path traversal: `../../Windows/System32` | Use `std::path::PathBuf::join()` — it sanitizes traversal components |
| Reading .flp binary data without byte-range bounds checks | Malformed .flp triggers buffer overflow in parser | Use `nom` or `bytes` crate — never index raw slices manually |
| Storing absolute paths in exported reports or logs | Leaks user's Windows username and folder structure | Strip to relative paths in any exported artifact |
| Allowing Tauri shell commands via `tauri::api::shell` without allowlist | Frontend JS can execute arbitrary shell commands | Allowlist in `tauri.conf.json` — only permit `open` for "Open in FL Studio" and "Show in Explorer" |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Showing the review UI as a flat list of 300+ groups | User abandons the review step entirely | Paginate: show 20 groups at a time, sorted by confidence (lowest first); allow "approve all high confidence" in one click |
| Auto-filing immediately when watcher detects a new .flp | Wrong group applied without user input; file is in the wrong folder | Always show a toast with a 10-second countdown to cancel; "ambiguous" files go to `_Unsorted` with no auto-filing |
| Progress bar for file copy that counts files not bytes | Bar stalls on large files, jumps on small ones | Count operations not bytes; show "File 47 of 312: Song Name 3.flp" |
| Showing all plugins including built-in FL Studio instruments | Plugin list is noise; user cares about VSTs | Distinguish built-in plugins (Fruity Kick, etc.) from third-party VSTs; default filter to VSTs only in diff view |
| No undo for "Execute" step of import workflow | One wrong click copies/organizes wrong files | "Execute" should be a dry-run first: show what will be copied, require explicit confirmation with file count |

---

## "Looks Done But Isn't" Checklist

- [ ] **FLP Parser:** Handles unknown event IDs gracefully — verify with a file from the latest FL Studio version, not just FL20 test corpus
- [ ] **File Copy:** No `fs::rename()` across directory boundaries anywhere in the codebase — grep for `rename` before every phase
- [ ] **Fuzzy Matcher:** Handles the edge case where FL Studio appends a number to a name that already ends in a number (e.g., `Trap Beat 2` → `Trap Beat 22` not `Trap Beat 2 2`)
- [ ] **Watcher Debounce:** Fires zero spurious events during a Proton Drive sync with no FL Studio activity — test this explicitly
- [ ] **Windows Build:** The `.msi` installer has actually been built on a Windows runner, not just assumed to build
- [ ] **SQLite Location:** The database file is in `%APPDATA%\FLP Vault`, not inside the Cryptomator vault — verify path in settings
- [ ] **Tray Behavior:** App does not crash after 50 minutes when hidden to tray — test this against the known Tauri v2 window.hide() crash bug
- [ ] **Manual Review:** Review step is always required before Execute; there is no "skip review" fast path that auto-applies groupings
- [ ] **Originals Backup:** Originals are copied (not moved) before the organized copy is made — both copies exist simultaneously

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Parser breaks on new FL Studio version | MEDIUM | Add unknown-event logging, identify new event IDs from pyflp issue tracker, add to spec; no user data lost if parser is non-destructive |
| Wrong fuzzy groupings applied to 500 files | HIGH | Originals backup is the recovery; re-run grouping from scratch against originals; user must re-do manual review |
| Cloud sync conflict duplicates created by batch copy | MEDIUM | Sync conflicts appear as renamed files; SQLite tracks what was copied so re-scan can identify duplicates; user manually resolves conflicts in Proton Drive |
| SQLite database corrupted (WAL mismatch from sync) | MEDIUM | Database is a cache only — drop and rebuild from filesystem scan; no unique user data is lost (song groups can be re-proposed) |
| .msi build only discovered to be non-functional at release time | HIGH | Requires setting up Windows build environment from scratch under time pressure; prevention is the only acceptable strategy |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| FLP parser breaks on unknown FL Studio versions | Phase 1: Parser | Run parser against FL Studio 21+ files; verify zero panics and warning list is populated |
| Cross-compilation cannot produce .msi | Phase 1: Build pipeline | GitHub Actions Windows runner produces signed .msi artifact before any features are built |
| Filesystem watcher fires on Cryptomator sync churn | Phase 2: Watch Mode | Test against live Cryptomator mount with Proton Drive syncing; verify zero spurious toasts |
| Fuzzy matcher wrong groupings | Phase 1: Fuzzy Matcher | Manual review of grouping on real 500-file corpus; measure false positive rate before shipping |
| File operations trigger cloud sync conflicts | Phase 1: File Manager | Verify copy-not-move at code review; batch pacing tested with 100+ file import |
| SQLite cache goes stale after renames | Phase 1: SQLite Store | Unit test: rename a file, re-scan, verify cache entry updated not orphaned |
| Tray crash after 50 minutes | Phase 2: System Tray | Soak test: leave app hidden in tray for 2+ hours; verify process still alive and tray icon responds |

---

## Sources

- [PyFLP Limitations Documentation](https://pyflp.readthedocs.io/en/latest/limitations.html) — confirmed FLP is closed/undocumented, "best-effort" parsing
- [PyFLP Architecture: FLP Format](https://pyflp.readthedocs.io/en/latest/architecture/flp-format.html) — "really bad and messy combination of TLV events and structs"
- [PyFLP GitHub](https://github.com/demberto/PyFLP) — tested only on FL Studio 20+, backup recommended before modifications
- [Tauri Cross-Platform Compilation v1](https://v1.tauri.app/v1/guides/building/cross-platform/) — "you still can't build a MSI, because WiX doesn't work on Linux"
- [Tauri Cross-Compilation Discussion #3291](https://github.com/orgs/tauri-apps/discussions/3291) — "cross-platform compilation is experimental and does not support all features"
- [Tauri Issue #1114](https://github.com/tauri-apps/tauri/issues/1114) — cross-compilation from Linux confirmed limited
- [notify-rs: Cross-platform filesystem notification library for Rust](https://github.com/notify-rs/notify) — Windows backend uses ReadDirectoryChangesW
- [How to Build a File Watcher with Debouncing in Rust](https://oneuptime.com/blog/post/2026-01-25-file-watcher-debouncing-rust/view) — confirmed need for debounce; single save = multiple events
- [Cryptomator Broken Filesystem Node Issue #3871](https://github.com/cryptomator/cryptomator/issues/3871) — .c9r churn events during sync
- [Cryptomator Volume Types Documentation](https://docs.cryptomator.org/en/latest/desktop/volume-type/) — WinFSP virtual mount on Windows
- [Tauri System Tray](https://v2.tauri.app/learn/system-tray/) — v2 tray implementation
- [Tauri Issue #14088](https://github.com/tauri-apps/tauri/issues/14088) — app crashes after all windows hidden; 50-minute window
- [Tauri Issue #8982](https://github.com/tauri-apps/tauri/issues/8982) — duplicate tray icon bug in v2
- [Tauri IPC Discussion #11915](https://github.com/orgs/tauri-apps/discussions/11915) — IPC JSON serialization ~200ms on Windows for 10MB
- [FL Studio Forums: Save New Version naming](https://forum.image-line.com/viewtopic.php?t=151019) — appends number to end; doesn't detect existing trailing numbers
- [SQLite WAL and Cloud Sync](https://github.com/skilion/onedrive/issues/346) — WAL requires shared memory; network filesystems/synced folders cause corruption
- [SQLite How To Corrupt](https://www.sqlite.org/howtocorrupt.html) — -wal and -shm must be copied with .db file; cloud sync that misses aux files corrupts DB

---

*Pitfalls research for: FL Studio project file organizer — Tauri + Rust, Windows desktop*
*Researched: 2026-02-25*
