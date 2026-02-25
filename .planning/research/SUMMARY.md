# Project Research Summary

**Project:** FLP Vault
**Domain:** Windows desktop app — binary file organizer for FL Studio .flp project files
**Researched:** 2026-02-25
**Confidence:** HIGH (stack, architecture, pitfalls) / MEDIUM (features, fuzzy matching thresholds)

## Executive Summary

FLP Vault is a Windows desktop app for organizing FL Studio project files that solves a problem no existing tool addresses: chaotically-named .flp files (500+) scattered across a Cryptomator-encrypted vault, with no reliable way to group versions of the same song or inspect what changed between them. The expert approach is Tauri 2 + Rust backend + vanilla JS webview — a 5-10 MB binary with native Windows integration, no bundled Chromium, and zero async complexity for what is fundamentally a UI-triggered file operation tool. The FLP binary format is a TLV event stream documented by the PyFLP Python project; a Rust parser using `std::io::Cursor` manual byte reading is the implementation approach, not nom (simpler for sequential event streams). The critical path is the FLP parser — every differentiating feature (fuzzy grouping, version diff, plugin search) depends on it.

The recommended architecture is a layered Rust backend: pure parser module (bytes in, FlpMetadata out), pure fuzzy matcher (filenames + metadata in, group proposals out), file manager (copy-only, never move), SQLite store (rusqlite with bundled SQLite in `%APPDATA%\FLP Vault`), and a thin Tauri command layer exposing typed commands to the frontend. The filesystem watcher (notify-debouncer-mini) is a separate async subsystem and a v0.2 feature — the entire v0.1 ships without it. Frontend is vanilla JS driving three panels: song list, version timeline, version detail/diff.

The primary risks are (1) the FLP binary format is closed and undocumented — parsers must be explicitly permissive-forward or they break on every FL Studio release; (2) production .msi builds cannot be produced from Linux — a GitHub Actions `windows-latest` runner is required and must be wired up in phase 1 before any features are built; (3) the Cryptomator virtual filesystem generates spurious watcher events requiring strict extension filtering and 3-second debounce; (4) fuzzy grouping false positives are hard to undo if grouping is applied before manual review — the review UI is not optional polish, it is the trust mechanism that makes the tool usable.

## Key Findings

### Recommended Stack

The stack is tightly reasoned around the constraints: Windows-only target, offline operation, no bundled runtime, Cryptomator/cloud-sync-aware filesystem access. Tauri 2 (stable since Oct 2024) is the clear choice over Electron. Rust handles all backend logic with zero-cost parsing. Frontend is vanilla JS — no framework justified for a read-heavy three-panel layout. rusqlite with the `bundled` feature compiles SQLite in statically (no Windows system dependency). The fuzzy matching stack is strsim (Jaro-Winkler) + trigram crate (pg_trgm-equivalent) — lightweight for 500-file pairwise comparison. Cross-compilation from Linux produces an NSIS `.exe` installer only; `.msi` requires a Windows runner.

See `.planning/research/STACK.md` for full versions, compatibility matrix, and alternatives considered.

**Core technologies:**
- Tauri 2.10.2: Desktop framework — 5-10 MB binary vs Electron's 150 MB; v2 is stable
- Rust 1.75+: Backend runtime — zero-cost parsing, no GC pauses, single binary
- rusqlite 0.38.0 (bundled): SQLite — statically compiled, no Windows system dependency
- nom 8.0.0 / manual Cursor: FLP binary parser — sequential TLV event stream; Cursor approach is simpler than nom combinators for this format
- strsim 0.11.1 + trigram 0.4.4: Fuzzy matching — Jaro-Winkler + trigram, sufficient for 500-file pairwise
- notify 8.2.0 + notify-debouncer-mini 0.7.0: Filesystem watcher — native ReadDirectoryChangesW on Windows
- Vanilla JS (ES2022): Frontend — no framework justified; @tauri-apps/api 2.x for IPC

### Expected Features

No FL Studio-specific project organizer exists. Competitors (dBdone, SessionDock) do not parse .flp binary metadata and rely on users to follow naming conventions. FLP Vault's unique position is binary metadata extraction + fuzzy grouping — the two things no competitor does.

See `.planning/research/FEATURES.md` for competitor analysis, dependency graph, and prioritization matrix.

**Must have (table stakes):**
- Folder scan discovering all .flp files — any organizer must find files
- FLP binary parser (BPM, channels, plugin IDs, pattern count) — the entire product depends on this
- Fuzzy filename grouping with manual review UI — the trust mechanism before any file is copied
- Organized folder copy with originals backup — the product action; copy-only, never move
- Three-panel browse UI (song list, version timeline, version detail) — core navigation
- Version diff showing BPM delta and plugin added/removed — core differentiator
- Settings for three paths (source, organized, originals) — required day 1
- .msi installer — distribution

**Should have (competitive):**
- System tray watcher with toast notifications — turns one-time import into ongoing workflow (v0.2)
- Plugin search across library — free given SQLite index already exists (v0.2)
- BPM/date range filters — power user feature, low cost given SQLite (v0.2)
- Batch pacing option for large cloud-sync imports (v0.2)

**Defer (v2+):**
- User tags/notes on versions — adds UI scope; defer until v0.1 validated
- Export song history as report — niche use case
- Zip project support — separate parser, separate problem
- Filter by channel count, pattern count — power user, low priority

**Anti-features (do not build):**
- Audio preview — .flp files contain no audio; rendering requires FL Studio
- .flp file editing — binary format with internal references; one corrupt file = trust destroyed
- Git-style branching — producers don't think in branches; adds complexity without value
- Cross-platform (macOS/Linux) — FL Studio is Windows-primary; 2-3x scope increase
- AI-generated song names — requires API keys, internet, latency; wrong names are worse than original names

### Architecture Approach

The architecture is a clean layered Rust backend behind thin Tauri command handlers. The key structural decision is that parser and matcher are pure modules (no I/O, no DB, testable with cargo test), services orchestrate workflows, and commands are one-liners that delegate to services. Long-running scans run in tokio::spawn and emit progress events to the frontend via app_handle.emit() — never blocking a command handler. SQLite is a single Mutex<Connection> with WAL mode; the mutex is never held across await points. The filesystem watcher is an isolated async subsystem communicating via tokio::mpsc channel — designed as a v0.2 addition.

See `.planning/research/ARCHITECTURE.md` for component diagram, data flow sequences, full project structure, and anti-patterns.

**Major components:**
1. FLP Parser (`parser/`) — pure function: `&[u8]` → `FlpMetadata`; no I/O; testable
2. Fuzzy Matcher (`matcher/`) — pure function: filenames + metadata → `Vec<ProposedGroup>`; trigram + BPM + temporal signals
3. SQLite Store (`store/`) — all DB access behind service boundary; `Mutex<Connection>` with WAL; cache keyed by `(path, size, mtime)`
4. App Services (`services/`) — orchestrates scan, group, organize workflows; holds no Tauri types
5. Command Layer (`commands/`) — thin Tauri command handlers; zero business logic; one file per domain
6. File Manager — copy-only (never rename/move); batch pacing; dedup by content hash
7. Frontend (vanilla JS) — three panels driven by Tauri IPC; scan/review workflow
8. FS Watcher + System Tray — v0.2; isolated tokio subsystem; 3s debounce; .flp-only filter

### Critical Pitfalls

1. **FLP parser breaks on unknown FL Studio versions** — Build permissive-forward from day one: unknown event IDs are skipped (never errored); `ParseResult { metadata, warnings }` struct; BPM sanity range (10-999); test on FL Studio 21+ files not just the pyflp FL20 corpus.

2. **.msi installer cannot be built from Linux** — Wire up GitHub Actions `windows-latest` runner in Phase 1 before any feature code. Never promise a deliverable that hasn't been built end-to-end on Windows. NSIS `.exe` is the Linux cross-compile output; `.msi` requires WiX on Windows.

3. **Filesystem watcher fires on Cryptomator sync churn** — Filter strictly to `.flp` extension before any debounce. Use 3-second debounce window (not 500ms). Ignore events for files that don't exist at processing time. Consider `notify-debouncer-full` over `notify-debouncer-mini` for rename chain handling. Test explicitly against a live Cryptomator mount with active Proton Drive sync.

4. **Fuzzy grouping wrong proposals are hard to undo** — Group proposals are non-destructive until user confirms. Default threshold 0.75+ with BPM agreement as second signal. Sort review UI by ascending confidence. Never auto-apply groupings; the manual review step is mandatory. Paginate: 20 groups at a time to prevent user abandonment.

5. **File operations trigger cloud sync conflicts** — Copy always, never `fs::rename()` across directory boundaries. Configurable batch pacing (default: 20 files / 5s pause). Verify destination file exists and correct size before recording as done. Source files are never deleted.

6. **SQLite cache goes stale after file renames** — Key cache by `(path, file_size, last_modified_timestamp)` not by path alone. Content hash computed lazily for dedup detection only. Changing the cache key later requires a migration; get it right in Phase 1.

7. **Tauri v2 window.hide() crash** — App crashes ~50 minutes after hiding to tray if using `window.hide()`. Use `WindowEvent::CloseRequested` handler to hide instead. Soak test: leave hidden 2+ hours.

## Implications for Roadmap

The architecture research provides an explicit build order: SQLite Store → FLP Parser → Fuzzy Matcher → App Services + Commands → Frontend → FS Watcher (v0.2). Every phase is dependency-gated on the one before. The pitfall research confirms that Phase 1 must include not just code but also the Windows build pipeline and the cache key design decision — both are expensive to change later.

### Phase 1: Foundation — Parser, Store, Build Pipeline

**Rationale:** The FLP parser is on the critical path for every differentiating feature. SQLite store design decisions (cache key, schema) are expensive to change after data is written. The Windows build pipeline must be confirmed working before any deliverable is promised — the .msi pitfall has high recovery cost.
**Delivers:** Working Tauri app skeleton, FLP parser with unit tests on real .flp files, SQLite schema with migrations, GitHub Actions Windows build producing a signed .msi, parser forward-compatibility verified on FL Studio 21+ files.
**Addresses:** FLP Binary Parser (P1 feature), Settings path config (P1), .msi installer (P1)
**Avoids:** Pitfall 1 (parser breaks on new FL Studio), Pitfall 2 (.msi not buildable from Linux), Pitfall 6 (stale SQLite cache)

### Phase 2: Core Grouping + Review

**Rationale:** Fuzzy matcher depends on FlpMetadata types from Phase 1. The manual review UI is load-bearing — it is the trust mechanism that makes the tool usable and must be built before the organize step. This is the highest-risk feature phase (fuzzy matching quality determines product viability).
**Delivers:** Trigram + BPM + temporal fuzzy grouper, manual review UI (merge/split/rename/assign/ignore with confidence scores, paginated by confidence ascending), proposed groups persisted in SQLite.
**Uses:** strsim + trigram crates, SQLite store from Phase 1
**Avoids:** Pitfall 4 (wrong fuzzy groupings hard to undo)

### Phase 3: File Operations + Browse UI

**Rationale:** Once groups are confirmed via review UI, the organize step is the product action. The browse UI is the post-organize experience. Both depend on confirmed groups from Phase 2.
**Delivers:** Organized folder copy with originals backup, copy-tracking in SQLite, three-panel browse UI (song list, version timeline, version detail), version diff (BPM delta, plugin added/removed), search by song name and plugin.
**Implements:** File Manager component, Frontend panels
**Avoids:** Pitfall 5 (cloud sync conflicts from file operations)

### Phase 4: Watch Mode + System Tray

**Rationale:** This phase assumes the library is already organized (Phases 1-3 complete). The watcher is the ongoing workflow feature — it cannot be validated until core import works. Cryptomator event filtering is complex enough to deserve its own phase.
**Delivers:** System tray presence, filesystem watcher with 3s debounce and .flp-only filter, confidence-tiered toast notifications (auto-file vs prompt vs unsorted), Windows launch-on-startup option.
**Avoids:** Pitfall 3 (Cryptomator sync churn), Pitfall 7 (Tauri tray crash after 50 minutes)

### Phase 5: Polish + Distribution

**Rationale:** UX pitfalls (review UI pagination, dry-run for execute, progress counting) and remaining P2 features (BPM/date filters, batch pacing option) are best addressed as a dedicated polish phase after the core loop is validated.
**Delivers:** UX refinements (dry-run before execute, paginated review, file-count progress), BPM/date range filters, batch pacing UI, signed .msi with WebView2 bootstrapper, installer tested on clean Windows 10 + Windows 11.

### Phase Ordering Rationale

- SQLite store before parser because the store schema cannot be changed without migrations — get it right first.
- Parser before fuzzy matcher because the matcher uses `FlpMetadata` types; building them simultaneously creates interface churn.
- Fuzzy matcher + review UI before file operations because copying files to wrong groups is the highest-severity user-visible failure. The review step is the gate.
- File operations before browse UI because browse depends on organized folder structure existing.
- Watch mode last because it assumes the library is organized, and the Cryptomator testing requirement is isolated from the main build.
- Build pipeline (Phase 1) not last — this is the counterintuitive but critical decision from pitfall research: the .msi must be proven buildable before any feature development, not discovered broken at release time.

### Research Flags

Phases needing deeper research during planning:

- **Phase 2 (Fuzzy Matching):** Threshold tuning is domain-specific. The 0.75 trigram + BPM agreement heuristic is reasonable but needs calibration against a real 500-file corpus before the algorithm is locked. The exact scoring formula (weight of trigram vs BPM vs temporal signal) will require experimentation. Flag for `/gsd:research-phase` if the grouper accuracy on real data is lower than expected.
- **Phase 4 (Watch Mode):** Cryptomator event filtering behavior is MEDIUM confidence (sourced from GitHub issues, not official docs). Test against a live Cryptomator + Proton Drive mount early in this phase before building the confidence-tiering logic.

Phases with standard patterns (skip research-phase):

- **Phase 1 (Foundation):** Tauri IPC, rusqlite, Tauri build pipeline — all HIGH confidence from official docs. The manual Cursor-based FLP parser approach is well-defined from pyflp's format documentation.
- **Phase 3 (File Ops + Browse):** File copy operations and three-panel UI layout are standard patterns. SQLite query patterns for browse/filter are straightforward.
- **Phase 5 (Polish):** UX refinements and installer configuration are standard; no novel patterns.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Tauri v2, rusqlite, strsim, notify — all verified against official docs and crates.io on 2026-02-25. Version compatibility matrix verified. |
| Features | MEDIUM | No FL Studio-specific organizer exists for direct comparison; competitor analysis via WebSearch on dBdone and SessionDock. Anti-features grounded in forum research (KVR, FL Studio forums). Core feature set is unambiguous. |
| Architecture | HIGH | Tauri IPC, state management, SQLite Mutex patterns from official Tauri docs. FLP parser approach from pyflp architecture docs. Data flow sequences are well-reasoned from first principles. |
| Pitfalls | HIGH | Critical pitfalls backed by specific GitHub issues: Tauri tray crash (issue #14088), .msi Linux limitation (official Tauri docs), Cryptomator churn (issue #3871), SQLite WAL sync corruption (SQLite official docs). Fuzzy matching threshold pitfall is MEDIUM confidence. |

**Overall confidence:** HIGH

### Gaps to Address

- **FLP event ID completeness:** The pyflp event table documents FL Studio 20+ events. Unknown event IDs from FL Studio 21-25 will be encountered in real use. The parser's forward-unknown handling makes this safe, but the set of extractable metadata fields may be incomplete until tested against current FL Studio versions. Validate during Phase 1 with the user's actual .flp corpus.

- **Fuzzy matching threshold values:** The 0.75 trigram similarity threshold and BPM-agreement weight are informed guesses, not empirically validated. The actual optimal threshold depends on the naming conventions in the user's library. Plan for a calibration step in Phase 2 where the matcher is tuned against a sample of real files before the organize step is built.

- **WebView2 availability on target Windows installs:** WebView2 is preinstalled on Windows 11 but not guaranteed on all Windows 10 configurations. The .msi installer must use `downloadBootstrapper` mode. Verify this in the Phase 1 build pipeline test.

- **Cryptomator event filtering specifics:** The exact set of spurious events generated during Proton Drive sync is MEDIUM confidence from GitHub issues. Actual event patterns may differ. Plan for an explicit test session against a live Cryptomator mount in Phase 4.

## Sources

### Primary (HIGH confidence)
- [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/) — v2 stability confirmed Oct 2024
- [Tauri Windows Installer docs](https://v2.tauri.app/distribute/windows-installer/) — MSI Linux limitation
- [Tauri IPC Architecture](https://v2.tauri.app/concept/inter-process-communication/) — command/event patterns
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) — AppState Mutex pattern
- [Tauri System Tray docs](https://v2.tauri.app/learn/system-tray/) — tray implementation
- [Tauri Issue #14088](https://github.com/tauri-apps/tauri/issues/14088) — window.hide() crash confirmed
- [rusqlite docs](https://docs.rs/rusqlite/latest/rusqlite/) — bundled feature, connection patterns
- [PyFLP FLP Format Architecture](https://pyflp.readthedocs.io/en/latest/architecture/flp-format.html) — TLV event stream format
- [PyFLP Limitations](https://pyflp.readthedocs.io/en/latest/limitations.html) — closed format, FL Studio 20+ only
- [notify-rs GitHub](https://github.com/notify-rs/notify) — Windows ReadDirectoryChangesW, v8.2.0
- [strsim-rs GitHub](https://github.com/rapidfuzz/strsim-rs) — Jaro-Winkler, Sørensen-Dice
- [SQLite How To Corrupt](https://www.sqlite.org/howtocorrupt.html) — WAL sync hazard
- crates.io API — all versions verified 2026-02-25

### Secondary (MEDIUM confidence)
- [Cryptomator Issue #3871](https://github.com/cryptomator/cryptomator/issues/3871) — .c9r churn events during sync
- [Tauri Cross-Compilation Discussion #3291](https://github.com/orgs/tauri-apps/discussions/3291) — experimental status
- [trigram crates.io](https://crates.io/crates/trigram) — pg_trgm-equivalent behavior
- [CrabNebula: UI Libraries for Tauri](https://crabnebula.dev/blog/the-best-ui-libraries-for-cross-platform-apps-with-tauri/) — vanilla JS validation
- [FL Studio Forums: Save New Version naming](https://forum.image-line.com/viewtopic.php?t=151019) — naming convention behavior
- [KVR Audio: DAW version control discussion](https://www.kvraudio.com/forum/viewtopic.php?t=429799) — producer pain points
- [dBdone documentation](https://dbdone.com/documentation/) — competitor feature set
- [SessionDock homepage](https://sessiondock.com/) — competitor feature set

### Tertiary (LOW confidence)
- [Rust forum: SQLite library choice for desktop](https://users.rust-lang.org/t/rust-and-sqlite-which-one-to-use/90780) — rusqlite recommendation context

---
*Research completed: 2026-02-25*
*Ready for roadmap: yes*
