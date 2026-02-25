# FLP Vault

## Project Overview

Windows desktop app for organizing FL Studio project files (.flp) into per-song folders with metadata inspection. Tauri + Rust backend, HTML/CSS/JS frontend.

## Architecture

- **Rust backend**: FLP parser, fuzzy matcher, file manager, filesystem watcher, SQLite store
- **Tauri webview**: Dark theme three-panel UI
- **SQLite**: Metadata cache in %APPDATA%\FLP Vault (not in cloud-synced vault)
- **Reference**: pyflp Python library for .flp binary format

## Key Conventions

- Read-only .flp access — never modify project files
- Copy files, never move (cloud sync safety)
- Debounce filesystem events (2-3 seconds)
- SQLite for metadata cache, keyed by file hash

## Design Doc

See `docs/plans/2026-02-25-flp-vault-design.md` for full design.

## Build Commands

```bash
cargo tauri dev        # Development mode
cargo tauri build      # Production build (.msi)
```
