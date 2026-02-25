# Stack Research

**Domain:** Windows desktop app — binary file organizer with fuzzy matching and filesystem watching
**Researched:** 2026-02-25
**Confidence:** HIGH for Rust backend crates; MEDIUM for frontend approach

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Tauri | 2.10.2 | Desktop app framework (Rust backend + webview frontend) | Native webview avoids bundling Chromium — target binary 5-10 MB vs Electron's 150 MB. v2 is stable (released Oct 2024), has official plugin ecosystem for notifications, SQL, shell. Windows-first use case is well-covered. |
| Rust | 1.75+ (stable) | Backend runtime | Already installed. Zero-cost abstractions, no GC pauses during file parsing, memory safety without runtime overhead. Compiles to single binary. |
| rusqlite | 0.38.0 | SQLite storage — metadata cache and song groups | Synchronous SQLite wrapper, not async. For a desktop app doing UI-driven queries this is correct — no async overhead, no extra runtime. `bundled` feature compiles SQLite in statically, no system dependency needed on Windows. |
| serde + serde_json | 1.0.228 | Tauri IPC serialization | All Tauri IPC between Rust and JS goes through JSON. serde is the Rust standard for serialization. Required by Tauri's command system. |
| nom | 8.0.0 | Binary .flp file parser | FLP format is a TLV event stream (type-length-value). nom is the Rust standard for binary parsing — combinator-based, handles variable-length events naturally, zero-copy slices into the buffer. pyflp documents the format; nom is the right tool to implement it in Rust. |
| notify | 8.2.0 | Filesystem watcher | Standard Rust fs watcher used by cargo-watch, rust-analyzer, Deno. Uses ReadDirectoryChangesW on Windows natively. v8 is current stable (9.0 is RC). |
| notify-debouncer-mini | 0.7.0 | Debounce filesystem events | Cryptomator writes + cloud sync cause duplicate file events. This companion crate collapses bursts into single events with configurable delay (use 2-3s for this use case). |

### Supporting Libraries — Rust Backend

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| strsim | 0.11.1 | String similarity metrics (Jaro-Winkler, Sørensen-Dice) | Primary fuzzy matching for filename grouping. Lighter than rapidfuzz; for 500 files compared pairwise this is plenty. Jaro-Winkler handles typos well; Sørensen-Dice is the trigram analog without the separate crate. |
| trigram | 0.4.4 | Trigram similarity (pg_trgm equivalent) | Used alongside strsim for the trigram similarity component of the grouping algorithm (as designed). The `similarity()` function matches pg_trgm behavior — consistent with the design doc's requirement. |
| tauri-plugin-sql | 2.3.2 | Expose SQLite to frontend JS | Enables JS-side database queries from webview. Needed if the frontend needs to query metadata directly. Feature: `sqlite`. Alternative: handle all DB in Rust commands (simpler, preferred). |
| tauri-plugin-notification | 2.3.3 | Windows toast notifications | Official Tauri plugin for system notifications. Used for watch mode — "New .flp detected: My Song 5.flp" toasts. Cross-platform compatible. |
| tauri-plugin-shell | 2.3.5 | Open files and folders | "Open in FL Studio" and "Show in Explorer" buttons need to launch external programs. Official Tauri plugin for this. |
| chrono | 0.4.x | Date/time handling | Filesystem mtime, temporal clustering for fuzzy grouping (files saved within days = same song confidence boost). |
| walkdir | 2.x | Recursive directory scanning | Standard crate for walking directory trees. Simpler than std::fs::read_dir recursion. Used in initial scan phase. |
| dirs | 5.x | Platform-appropriate %APPDATA% path | Gets `%APPDATA%\FLP Vault` on Windows. Standard crate, avoids hardcoding Windows paths. |
| sha2 or xxhash-rust | latest | File deduplication by hash | SQLite cache is keyed by file hash to detect re-scanned files. xxhash-rust is faster (non-cryptographic but sufficient). sha2 if you want collision resistance. |

### Supporting Libraries — Frontend (HTML/CSS/JS)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Vanilla JS (no framework) | ES2022 | Webview UI | The UI is a read-heavy three-panel layout driven by Rust data. No reactive state beyond what Tauri's IPC provides. A framework would add 30-100 KB of JS for little benefit. Tauri's `@tauri-apps/api` handles all backend calls. |
| @tauri-apps/api | 2.x (matches Tauri) | Tauri JS API — IPC, events, window management | Required. All Rust command invocations, event listeners, and window operations go through this. Install via npm. |
| CSS custom properties + flexbox | Native | Dark theme three-panel layout | The three-panel layout (song list, version timeline, detail) is straightforward flexbox. No CSS framework needed — adds unnecessary bulk and opinionated resets. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| cargo-tauri (tauri-cli) | Dev server, build, bundle | `cargo install tauri-cli` — use this for `tauri dev` and `tauri build`. Current: 2.10.0. |
| cargo-xwin | Cross-compile Linux → Windows | `cargo install cargo-xwin` — downloads Windows SDK headers locally. Required for building from the Linux dev server. |
| rustup target x86_64-pc-windows-msvc | Windows compilation target | `rustup target add x86_64-pc-windows-msvc` — must be added before cross-compile. |
| clang + lld + llvm | C/C++ linker for cross-compile | Required on Linux for xwin. `apt install clang lld llvm`. |

---

## Installation

```bash
# Tauri CLI
cargo install tauri-cli

# Cross-compile toolchain (run on dev Linux machine)
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
# apt install clang lld llvm  (tell user, no sudo available)

# Frontend (in project root)
npm install @tauri-apps/api

# Cargo.toml — Rust dependencies
# [dependencies]
# tauri = { version = "2", features = ["tray-icon"] }
# tauri-plugin-notification = "2"
# tauri-plugin-shell = "2"
# rusqlite = { version = "0.38", features = ["bundled"] }
# serde = { version = "1", features = ["derive"] }
# serde_json = "1"
# nom = "8"
# strsim = "0.11"
# trigram = "0.4"
# notify = "8"
# notify-debouncer-mini = "0.7"
# chrono = { version = "0.4", features = ["serde"] }
# walkdir = "2"
# dirs = "5"
# xxhash-rust = { version = "0.8", features = ["xxh3"] }
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| rusqlite 0.38 (bundled) | tauri-plugin-sql (sqlx backend) | Use tauri-plugin-sql only if you need JS-side SQL queries AND don't want to expose every query as a Rust command. For this app, all SQL is in Rust — rusqlite directly is simpler and has no async overhead. |
| rusqlite (bundled) | sqlx with sqlite feature | sqlx is async and adds tokio runtime complexity. Desktop apps with UI-triggered queries don't need async DB access. |
| nom 8.0 | Manual byte slicing | Manual parsing works for very simple formats. FLP has 4 event size categories plus variable-length blocks — nom's combinators model this cleanly and are easier to extend as more event IDs are reverse-engineered. |
| strsim + trigram | rapidfuzz 0.5 | Use rapidfuzz if comparing tens of thousands of strings where microseconds matter. 500 files pairwise = 125,000 comparisons — strsim handles this in milliseconds. rapidfuzz would be overkill. |
| strsim + trigram | nucleo | nucleo is optimized for interactive fuzzy search (helix editor's picker). Not the right tool for batch grouping by song — it's a UI fuzzy-finder, not a similarity scorer. |
| notify 8.2 (stable) | notify 9.0.0-rc.2 | Use 9.0 when it hits stable. As of 2026-02-25 it's RC — don't ship production code on an RC. |
| Vanilla JS | Svelte 5 / Preact | Use a framework if the UI becomes interactive and stateful enough to warrant it (e.g., drag-and-drop grouping review with live updates). Starting vanilla is correct — add a framework later if needed. |
| Vanilla JS | React | React adds ~40 KB gzipped for a three-panel read-mostly display. Not justified. |
| NSIS (setup.exe) | WiX .msi | .msi can only be built on Windows. Building from Linux requires NSIS (setup.exe). Both are valid Windows installers. If .msi is required, the final build step must run on a Windows machine or GitHub Actions Windows runner. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Electron | 150+ MB binary, ships its own Chromium. Violates the 5-10 MB target. | Tauri 2 |
| Tauri v1 | EOL approach — v2 is the stable release since Oct 2024, v1 plugin ecosystem is split. | Tauri 2 |
| sqlx for this app | Async SQL adds tokio runtime complexity; compile-time query checking requires a live DB at build time, which complicates cross-compilation. | rusqlite (bundled) |
| warp/axum as a local HTTP server | Some Tauri v1 tutorials used a local HTTP server for IPC. Tauri v2's command system replaces this entirely. | Tauri commands + IPC |
| notify 9.0.0-rc.2 | Release candidate — API may change before stable. | notify 8.2.0 (stable) |
| pyflp at runtime | It's Python. Shipping a Python interpreter alongside a Rust binary defeats the purpose. Use it as a format reference only — implement the parser in Rust with nom. | nom + pyflp docs |
| Git for version storage | The design doc ruled this out — overcomplicated for the use case, and .flp files aren't text-diffable. | SQLite metadata cache + file copies |
| tauri-plugin-rusqlite2 (community) | Third-party community fork. The official Tauri SQL plugin or direct rusqlite are better-supported options. Only needed if you specifically need transaction support from the JS side. | rusqlite directly |

---

## Stack Patterns by Variant

**For all DB access from Rust commands only (recommended):**
- Use rusqlite directly in Rust backend
- No tauri-plugin-sql needed
- Cleaner separation: Rust owns data, JS only receives serialized results
- Each query is a typed Tauri command with defined return struct

**If JS needs direct DB queries:**
- Add tauri-plugin-sql with sqlite feature
- Use JS `Database.load('sqlite:...')` for ad-hoc queries
- Still need rusqlite for scan/parse operations in Rust

**For cross-compile build from Linux (dev workflow):**
- `tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`
- Produces NSIS setup.exe (not .msi — WiX is Windows-only)
- For .msi: use GitHub Actions windows-latest runner

---

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| tauri 2.10.2 | @tauri-apps/api 2.10.1 | Always keep Rust crate and JS package on same minor version. tauri-cli 2.10.0 manages this. |
| rusqlite 0.38.0 | bundled SQLite 3.49.x | bundled feature pins SQLite version — no system sqlite3 dependency needed on Windows. |
| notify 8.2.0 | notify-debouncer-mini 0.7.0 | These are companion crates from the same repo — versions are coordinated. Match them. |
| tauri-plugin-sql 2.3.2 | tauri 2.x | All tauri-plugin-* crates are major-version locked to Tauri. Don't mix v1 plugins with Tauri v2. |
| nom 8.0.0 | Rust 1.65+ | nom 8 requires Rust edition 2021. Already satisfied by standard toolchain. |

---

## Sources

- [Tauri 2.0 Stable Release announcement](https://v2.tauri.app/blog/tauri-20/) — version confirmed stable Oct 2024 (HIGH confidence)
- [Tauri GitHub releases](https://github.com/tauri-apps/tauri/releases) — v2.10.2 confirmed latest as of 2026-02-25 (HIGH confidence)
- [Tauri Windows Installer docs](https://v2.tauri.app/distribute/windows-installer/) — MSI Linux limitation confirmed (HIGH confidence)
- [Tauri SQL Plugin docs](https://v2.tauri.app/plugin/sql/) — feature flags and SQLite setup (HIGH confidence)
- [Tauri Notification Plugin](https://v2.tauri.app/plugin/notification/) — toast notifications on Windows (HIGH confidence)
- [rusqlite GitHub](https://github.com/rusqlite/rusqlite) — bundled feature, v0.38.0 (HIGH confidence)
- [Rust forum: SQLite library choice for desktop](https://users.rust-lang.org/t/rust-and-sqlite-which-one-to-use/90780) — rusqlite recommendation for desktop apps (MEDIUM confidence)
- [PyFLP format documentation](https://pyflp.readthedocs.io/en/latest/architecture/flp-format.html) — FLP binary format: TLV event stream confirmed (HIGH confidence)
- [nom GitHub](https://github.com/rust-bakery/nom) — v8.0.0, binary parsing approach (HIGH confidence)
- [notify-rs GitHub](https://github.com/notify-rs/notify) — v8.2.0 stable, Windows ReadDirectoryChangesW (HIGH confidence)
- [strsim-rs GitHub](https://github.com/rapidfuzz/strsim-rs) — v0.11.1, Jaro-Winkler and Sørensen-Dice (HIGH confidence)
- [trigram crates.io](https://crates.io/crates/trigram) — v0.4.4, pg_trgm-equivalent (MEDIUM confidence)
- [CrabNebula: Best UI Libraries for Tauri](https://crabnebula.dev/blog/the-best-ui-libraries-for-cross-platform-apps-with-tauri/) — vanilla JS confirmed viable (MEDIUM confidence)
- crates.io API — all version numbers verified live on 2026-02-25 (HIGH confidence)

---

*Stack research for: FLP Vault — Windows desktop app, Tauri 2 + Rust, binary .flp parsing, fuzzy song grouping, filesystem watching, SQLite metadata cache*
*Researched: 2026-02-25*
