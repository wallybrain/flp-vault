# FLP Vault

## What This Is

A Windows desktop application that organizes FL Studio project files (.flp) into per-song folders using fuzzy name matching, parses binary metadata (BPM, plugins, channels), and provides a browsable library with version comparison. Built for a producer with 500+ .flp files stored in a Cryptomator-encrypted vault synced to Proton Drive.

## Core Value

A producer can instantly find any version of any song and see what changed between versions — without opening FL Studio.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Scan a folder of .flp files and parse binary metadata (BPM, time sig, channels, plugins, patterns)
- [ ] Fuzzy-match filenames to group .flp files by song (trigram similarity + BPM + temporal clustering)
- [ ] Manual review UI for fixing grouping mistakes (merge, split, rename, assign, ignore)
- [ ] Copy files into organized per-song folders with originals backup
- [ ] Three-panel browse UI: song list, version timeline, version detail
- [ ] Version diff: compare two versions showing BPM changes, channel delta, plugins added/removed
- [ ] Search and filter songs by plugin, BPM range, date range
- [ ] System tray watcher mode for new .flp file detection with toast notifications
- [ ] Settings: configure source folder, organized folder, originals folder
- [ ] .msi installer for Windows distribution

### Out of Scope

- Audio preview/playback — FL Studio does this better
- Editing .flp files — read-only always, never modify project files
- Git-style branching/merging of versions — overcomplicated for the use case
- Cloud sync features — Cryptomator + Proton Drive handle this externally
- Cross-platform — Windows only, FL Studio is Windows-primary
- Note data / automation data parsing — fragile across FL Studio versions
- Audio clip references parsing — fragile across FL Studio versions

## Context

- FL Studio saves projects as .flp binary files; the format is reverse-engineered and documented by the pyflp Python library
- The producer uses FL Studio's "Save new version" pattern (Song Name 2.flp, Song Name 3.flp) — the app must understand this convention
- Files live in a Cryptomator vault which means: copy don't move (move = delete + create in sync), debounce file events (2-3s), pace batch operations to avoid overwhelming sync
- SQLite metadata cache lives in %APPDATA%\FLP Vault (not in the cloud-synced vault) to avoid WAL journal sync churn
- The workflow is VST-heavy with minimal audio recordings, so plugin metadata is the most useful differentiator between versions
- Reference implementation: pyflp Python library for .flp binary format documentation

## Constraints

- **Platform**: Windows only — Tauri + Rust backend, HTML/CSS/JS webview frontend
- **File safety**: Read-only .flp access, copy files never move, originals backup before organizing
- **Cloud sync**: Operations must be sync-friendly (copy not move, debounce events, batch pacing)
- **Distribution**: Single .msi installer or portable .exe, target ~5-10 MB
- **Development**: Built on Linux server, cross-compilation needed for Windows targets; UI testing requires Windows

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tauri + Rust over Electron | Smaller binary (~5-10 MB vs ~150 MB), native perf for file parsing | — Pending |
| SQLite in %APPDATA% not vault | Avoids WAL journal churn in cloud sync | — Pending |
| Copy files, never move | Move triggers delete+create in cloud sync, copy only triggers create | — Pending |
| pyflp as format reference | Most complete reverse-engineering of .flp binary format | — Pending |
| Trigram fuzzy matching | Good balance of accuracy and simplicity for filename grouping | — Pending |

---
*Last updated: 2026-02-25 after initialization*
