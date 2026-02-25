# FLP Vault — Design Document

**Date**: 2026-02-25
**Status**: Approved

## Overview

FLP Vault is a Windows desktop application that organizes FL Studio project files (.flp) into per-song folders, using fuzzy name matching to group versions of the same song. It parses .flp binary metadata (BPM, plugins, channels) and presents a browsable library with version comparison.

## Problem

Producers accumulate hundreds of .flp files in a single folder — different versions, inconsistent naming, abbreviations, typos. Finding the right version of a song means scrolling through a wall of files. There's no quick way to see what changed between versions without opening each one in FL Studio.

## Target User

A producer using FL Studio on Windows, storing projects in a Cryptomator-encrypted vault synced to Proton Drive. 500+ .flp files, chaotic naming conventions, VST-heavy workflow (minimal audio recordings).

## Technical Stack

- **Backend**: Rust (Tauri framework)
- **Frontend**: HTML/CSS/JS in Tauri webview, dark theme
- **Storage**: SQLite for metadata cache and song groups
- **Distribution**: Single .msi installer or portable .exe (~5-10 MB)
- **Reference**: pyflp Python library for .flp binary format documentation

## Architecture

```
+-------------------+
|   Tauri Webview   |  <- Dark theme UI (HTML/CSS/JS)
+-------------------+
         |
    IPC commands
         |
+-------------------+
|   Rust Backend    |
|                   |
|  +- FLP Parser    |  <- Reads .flp binary format, extracts metadata
|  +- Fuzzy Matcher |  <- Groups files by song using name/BPM/date signals
|  +- File Manager  |  <- Copy operations, dedup tracking
|  +- FS Watcher    |  <- ReadDirectoryChangesW for new file detection
|  +- SQLite Store  |  <- Persists song groups, metadata cache, settings
+-------------------+
         |
+-------------------+
|   Filesystem      |  <- Cryptomator mount / local folders
+-------------------+
```

## User-Configured Paths

Three directories, all user-configurable at first launch and in settings:

| Path | Purpose |
|------|---------|
| Source folder | Where FL Studio saves .flp files (e.g., V:\FL Projects) |
| Organized folder | Where songs get filed into per-song folders (e.g., V:\FLP Vault) |
| Originals folder | Safety copies of files before organizing (e.g., V:\FLP Vault Originals) |

All three can be anywhere — inside or outside the Cryptomator vault, on any drive. The app doesn't care about encryption or cloud sync; it just sees normal filesystem paths.

## Cloud Sync Considerations

When the vault syncs to Proton Drive:

- **Copy, don't move**: Moving triggers delete + create in sync. Copying only triggers creates.
- **SQLite outside the vault**: The metadata database lives in %APPDATA%\FLP Vault to avoid WAL journal sync churn.
- **Batch pacing**: Legacy import offers a pacing option (batches of 20-30 with pauses) to avoid overwhelming cloud sync.
- **Debounce file watcher**: Wait 2-3 seconds after file events before acting, to avoid reacting to partial writes or sync operations.

## UI Layout

Three-panel design with dark theme:

### Left Panel — Song List
- All songs grouped by name
- Search bar
- Filter by date range, plugin, BPM
- Total song/file count

### Middle Panel — Version Timeline
- All versions of selected song, chronological
- Quick-glance: version number, date, BPM
- Visual indicator when BPM or channel count changed between versions

### Right Panel — Version Detail
- BPM, time signature
- Channel count, pattern count
- Generators (instruments/synths) listed separately from effects
- File size, modification date, full path
- Action buttons: Open in FL Studio, Show in Explorer

### Version Diff
Select two versions to compare:
- BPM changes
- Channel count delta
- Plugins added/removed

## Legacy Import Workflow

### Step 1 — Scan
Point app at source folder. Reads every .flp and .zip, extracts filenames, dates, parses binary metadata.

### Step 2 — Auto-group
Fuzzy matcher proposes song groups using:
- Filename similarity (trigram comparison after stripping version numbers, dates, common suffixes)
- BPM matching (same BPM = confidence boost)
- Temporal clustering (files saved within days of each other)

### Step 3 — Manual review
User fixes grouping mistakes:
- Merge: combine two groups that are the same song
- Split: separate two songs that were incorrectly lumped
- Rename: set canonical song name
- Assign ungrouped: manually place files the matcher couldn't confidently group
- Ignore: mark throwaway files (test beats, experiments)

### Step 4 — Execute
Copies files into per-song folders in the organized directory. Copies originals to the originals directory. Tracks what was copied to prevent duplicates on re-scan. Originals are the user's responsibility to delete when satisfied.

## Ongoing Watch Mode

After initial import, a filesystem watcher monitors the source folder.

### System Tray
Runs as system tray icon with minimal footprint. Optional launch-with-Windows. Right-click menu: open app, pause watching, settings, quit.

### New File Detection
When FL Studio writes a new .flp:
- **High confidence match**: Toast notification, auto-files after 10 seconds unless user intervenes
- **Ambiguous**: Popup with suggested song and alternatives, waits for user input
- **New song**: Prompts to name a new song or file to _Unsorted

### FL Studio Save Pattern
Understands FL Studio's "Save new version" pattern (Song Name 2.flp, Song Name 3.flp) and auto-matches to the correct song group.

## FLP Parser Scope

Priority metadata (MVP):
1. BPM / time signature
2. Channel names and plugin IDs (generators vs effects)
3. Pattern count
4. Mixer track count

Not parsing (fragile across FL Studio versions):
- Note data within patterns
- Automation data
- Audio clip references

Reference implementation: pyflp Python library (reverse-engineered .flp format).

## Phased Delivery

### v0.1 — MVP
- Scan folder, parse .flp filenames and binary metadata
- Fuzzy grouping with review UI (merge, split, rename, assign)
- Copy files into organized song folders + originals backup
- Three-panel browse UI with plugin list
- Version diff (compare two versions)
- Settings: configure three paths
- .msi installer

### v0.2 — Quality of Life
- System tray watcher mode
- Launch with Windows option
- Batch pacing for cloud-sync-friendly imports

### v0.3 — Power Features
- Search across all songs by plugin name
- Filter by BPM range, date range, channel count
- User tags/notes on versions ("drums done", "sent to vocalist")
- Export song history as report

### Not Planned
- Audio preview/playback (FL Studio does this better)
- Editing .flp files (read-only always)
- Git-style branching/merging of versions
- Cloud sync features (Cryptomator + Proton Drive handle this)
- Cross-platform (Windows only)
