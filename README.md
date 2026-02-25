# FLP Vault

Organize your FL Studio project files. Scan hundreds of .flp files, auto-group versions of the same song, browse metadata (BPM, plugins, channels), and compare versions.

## Features

- **Smart grouping** — fuzzy matches filenames, BPM, and dates to group versions of the same song
- **Deep inspection** — parses .flp files to show BPM, time signature, channels, plugins (generators and effects)
- **Version diff** — compare two versions side by side to see what changed
- **Legacy import** — tame an existing folder of 500+ files with guided review
- **Watch mode** — auto-files new saves as you work in FL Studio
- **Cloud-sync friendly** — designed to work with Cryptomator + Proton Drive

## Stack

- Tauri (Rust backend + webview frontend)
- SQLite for metadata cache
- Single .msi installer, ~5-10 MB

## Status

Design phase — see [design document](docs/plans/2026-02-25-flp-vault-design.md).

## License

MIT — see [LICENSE](LICENSE).
