# Phase 2: Grouping — Research

**Phase:** 02-grouping
**Researched:** 2026-02-26
**Status:** Complete

---

## What This Research Covers

Phase 2 delivers three distinct capabilities in strict dependency order:

1. **02-01: Fuzzy matcher** — trigram + BPM + temporal signals, configurable threshold, proposes groups from scanned `files` table
2. **02-02: Review UI** — paginated group proposal table sorted by confidence ascending, with merge/split/rename/assign/ignore actions
3. **02-03: Group persistence in SQLite** — confirmed groups, ignored files, manual overrides; schema migration on top of Phase 1

This document answers the questions a planner needs to design all three plans without stumbling into known traps.

---

## What Phase 1 Left Behind

The following already exists and must NOT be recreated or modified structurally.

### Rust source files

```
src-tauri/src/
├── main.rs              — Tauri builder, 5 commands registered
├── state.rs             — AppState { db: Arc<Mutex<Connection>>, scan_status }
├── commands/
│   ├── mod.rs           — re-exports all commands
│   ├── scan.rs          — scan_folder, cancel_scan
│   ├── settings.rs      — get_settings, save_settings
│   └── browse.rs        — list_scanned_files
├── services/
│   └── scanner.rs       — run_scan() background thread
├── parser/
│   ├── types.rs         — FlpMetadata, ChannelInfo (with Serialize/Deserialize/Default)
│   ├── flp.rs           — parse_flp(&[u8]) -> Result<FlpMetadata, ParseError>
│   └── events.rs        — read_varint(), FLP event ID constants
└── store/
    ├── connection.rs    — init_db(), WAL mode, migrations
    ├── migrations.rs    — CREATE TABLE IF NOT EXISTS (3 tables)
    ├── files.rs         — is_cached, hash_in_cache, update_path_index, upsert_file, list_all_files, FileRecord
    └── settings.rs      — get_setting, set_setting, get_all_settings
```

### SQLite schema (from migrations.rs)

```sql
CREATE TABLE IF NOT EXISTS files (
    hash              TEXT PRIMARY KEY,   -- xxh3 hex of file bytes
    path              TEXT NOT NULL,      -- last known absolute path
    file_size         INTEGER NOT NULL,
    mtime             INTEGER NOT NULL,   -- unix timestamp
    bpm               REAL,
    time_sig_num      INTEGER,
    time_sig_den      INTEGER,
    channel_count     INTEGER,
    pattern_count     INTEGER,
    mixer_track_count INTEGER,
    plugins_json      TEXT,               -- JSON array of plugin name strings
    warnings_json     TEXT,
    fl_version        TEXT,
    parsed_at         INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS path_index (
    path      TEXT PRIMARY KEY,
    hash      TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    mtime     INTEGER NOT NULL,
    FOREIGN KEY (hash) REFERENCES files(hash)
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### Cargo.toml dependencies (already present)

```toml
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
walkdir = "2"
dirs = "5"
xxhash-rust = { version = "0.8", features = ["xxh3"] }
chrono = { version = "0.4", features = ["serde"] }
byteorder = "1"
```

**NOT yet present** (must be added for Phase 2):

```toml
strsim = "0.11"      # Jaro-Winkler + Sorensen-Dice similarity
trigram = "0.4"      # pg_trgm-equivalent trigram similarity
```

### Frontend files (already present)

```
src/
├── index.html           — toolbar, main-content, empty-state, settings-container
├── styles/main.css      — dark theme, CSS custom properties, 442 lines
└── js/
    ├── api.js           — 5 invoke wrappers + 4 event listeners
    ├── main.js          — DOMContentLoaded wiring
    └── panels/
        ├── scan-table.js    — sortable scan results table (311 lines)
        └── settings-panel.js — slide-out folder config (with dialog plugin)
```

The scan table currently lives in `#main-content`. The grouping review UI will also live there — it replaces or sits alongside the scan table in the post-scan workflow.

---

## Plan 02-01: Fuzzy Matcher

### What the Matcher Must Do

Given a `Vec<FileRecord>` loaded from the `files` table, produce a `Vec<ProposedGroup>`. Each group has:
- A `group_id` (UUID or auto-increment — see persistence section)
- A canonical name (derived from the most representative filename)
- A list of file hashes in the group
- A confidence score (0.0–1.0)

### Signal 1: Trigram + Filename Normalization (GRUP-01, GRUP-04)

Before computing similarity, normalize filenames:

1. Strip the file extension (`.flp`)
2. Extract the stem (basename without path)
3. Strip common FL Studio version suffixes:
   - Trailing digits: `Song Name 2`, `Song Name 3`, `Song Name 12`
   - Trailing `_N` pattern: `Song Name_2`
   - Leading/trailing whitespace
4. Strip common noise suffixes: `_final`, `_v2`, `_old`, `_backup`, `_copy`
5. Lowercase for comparison

**GRUP-04 specifically:** FL Studio's "Save new version" appends a space and a number to whatever the current filename is. If the name already ends in a number, FL Studio appends another number: `Trap Beat 2` → `Trap Beat 22` → `Trap Beat 222`. The normalization must strip ALL trailing digit clusters, not just one — otherwise `Trap Beat 2` and `Trap Beat 22` get different normalized forms.

```rust
fn normalize_filename(path: &str) -> String {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let mut s = stem.to_lowercase();

    // Strip FL Studio version number suffixes (one or more, greedy)
    loop {
        let trimmed = s.trim_end_matches(|c: char| c.is_ascii_digit())
                       .trim_end_matches(|c: char| c == ' ' || c == '_');
        if trimmed.len() == s.len() { break; }
        s = trimmed.to_string();
    }

    // Strip common noise suffixes
    for suffix in &["_final", "_old", "_backup", "_copy", " final", " old", " backup"] {
        if s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
        }
    }

    s.trim().to_string()
}
```

With normalized filenames, compute trigram similarity using the `trigram` crate:

```rust
use trigram::similarity;

let score: f32 = similarity(&norm_a, &norm_b);
```

`trigram::similarity()` returns 0.0–1.0. This is the `pg_trgm` algorithm — pairs with score >= 0.5 are likely the same song name.

**Short name problem (from PITFALLS.md):** Short filenames (< 6 characters after normalization) produce unreliable trigram scores. `"Hi"` and `"Ho"` will score very low, but `"Beat"` and `"Beat"` will score 1.0. Set a minimum normalized length of 4 characters; below that, require exact string match rather than trigram similarity for a confident match.

### Signal 2: BPM Agreement (GRUP-02)

If two files have non-null BPMs:
- Same BPM (within ±1.0 BPM tolerance): add `+0.15` to confidence
- Different BPM (difference > 5.0): subtract `0.10` from confidence (possible key change or different song)
- BPM not available in one or both files: no adjustment (neutral)

Tolerance of ±1.0 handles floating-point rounding in BPM storage (e.g., 128.0 vs 128.001). Use ±3.0 if the corpus tends to have tempo drift between versions.

### Signal 3: Temporal Clustering (GRUP-03)

Files with `mtime` values within N days of each other are more likely to be versions of the same song:
- Within 3 days: add `+0.10` to confidence
- Within 14 days: add `+0.05`
- More than 30 days apart: no adjustment (neutral — producers work on songs over long periods)

Use `mtime` (unix timestamp from `files` table). The formula:

```rust
let days_apart = (mtime_a - mtime_b).abs() / 86400;
let temporal_boost = match days_apart {
    0..=3   => 0.10,
    4..=14  => 0.05,
    _       => 0.0,
};
```

Do not penalize files with large time gaps. A producer may revisit a song months later — this should not reduce confidence.

### Combined Scoring Formula

```rust
fn compute_confidence(
    norm_name_a: &str,
    norm_name_b: &str,
    bpm_a: Option<f64>,
    bpm_b: Option<f64>,
    mtime_a: i64,
    mtime_b: i64,
) -> f32 {
    let trigram_score = trigram::similarity(norm_name_a, norm_name_b);

    let bpm_adjustment = match (bpm_a, bpm_b) {
        (Some(a), Some(b)) => {
            if (a - b).abs() <= 1.0 { 0.15 }
            else if (a - b).abs() > 5.0 { -0.10 }
            else { 0.0 }
        }
        _ => 0.0,
    };

    let days_apart = ((mtime_a - mtime_b).abs() / 86400) as i64;
    let temporal_boost = match days_apart {
        0..=3 => 0.10,
        4..=14 => 0.05,
        _ => 0.0,
    };

    (trigram_score + bpm_adjustment as f32 + temporal_boost as f32).clamp(0.0, 1.0)
}
```

**Configurable threshold:** The minimum confidence to include a pair in a proposed group is configurable; default `0.65`. Files with no pair at or above the threshold appear as single-file "ungrouped" proposals that the user can manually assign (GRUP-08).

### Group Formation Algorithm

The matcher does not just return pairwise scores — it forms actual groups. Use union-find (disjoint set) or transitive closure:

1. Compute pairwise similarity for all file pairs (O(n²) comparisons)
2. For each pair with `confidence >= threshold`, add an edge: `(file_a, file_b, confidence)`
3. Merge all files connected transitively into groups using union-find
4. The group confidence is the **minimum** edge confidence in the group (pessimistic — forces user review of the weakest link)

**Performance note:** At n=500 files, O(n²) = 125,000 comparisons. Each comparison is two string normalizations + one `trigram::similarity()` call (~1–2 microseconds each) = ~125–250ms. This is fast enough to run synchronously in a Tauri command response (< 500ms). If n > 2000, pre-bucket by BPM (only compare files within the same BPM bucket) to reduce comparisons.

**Pre-bucketing optimization (for larger corpora):**
```
Group files into BPM buckets (e.g., BPM rounded to nearest 5)
Only compare files within the same BPM bucket (± 1 bucket)
Files with null BPM are compared against all others
```

### Canonical Name Selection

For each proposed group, select the canonical name:
1. Use the most common normalized name (mode) across the group
2. If all are unique, use the normalized name of the oldest file (earliest mtime — the original)
3. Present the unnormalized filename of that source file as the default canonical name (with the version suffix stripped — the user will clean it up if needed)

### Output Type

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ProposedGroup {
    pub id: String,               // UUID v4, ephemeral (not persisted until confirmed)
    pub canonical_name: String,   // normalized name, user-editable
    pub confidence: f32,          // 0.0–1.0, minimum edge in group
    pub file_hashes: Vec<String>, // all files in this group
    pub is_ungrouped: bool,       // true if this is a single-file "no match" proposal
}
```

**Note:** `ProposedGroup` is ephemeral — it lives in memory until the user confirms. Plan 02-03 handles persistence of confirmed groups.

### Matcher Module Structure

Following the architecture's isolation principle (matcher is pure — no DB, no filesystem):

```
src-tauri/src/matcher/
├── mod.rs          — pub fn propose_groups(files: &[FileRecord]) -> Vec<ProposedGroup>
├── normalize.rs    — normalize_filename(), strip_version_suffix()
├── signals.rs      — compute_trigram_score(), bpm_signal(), temporal_signal()
├── scorer.rs       — compute_confidence(), weighted combination
└── union_find.rs   — UnionFind struct for group formation
```

The command layer calls `matcher::propose_groups(files)` with all `FileRecord`s from the DB and returns the proposals.

---

## Plan 02-02: Review UI

### UI Flow After Scan

The current UI state after scan: `#main-content` contains the scan results table (`scan-table.js`). After scan completes, a "Review Groups" button appears. Clicking it triggers `invoke('propose_groups')` and renders the review panel.

The review panel replaces or overlays `#main-content`. The scan table remains accessible (tab or breadcrumb navigation).

### Review Panel Layout

The review UI must handle a paginated, actionable list of group proposals.

```
[Review Groups] [20 of 47 groups remaining] [Approve All High Confidence]

Group 1 of 20  — Confidence: 61% (LOW)
┌─────────────────────────────────────────────────────────────────┐
│ Canonical name: [  Acid Bass Line_______  ]  [Rename]          │
│                                                                  │
│  File                           BPM    Date                     │
│  Acid Bass Line.flp             128.0  2024-03-10               │
│  Acid Bass Line 2.flp           128.0  2024-03-12               │
│  Acid Bass Line(copy).flp       128.0  2024-03-10               │
│                                                                  │
│  [Split] [Merge with another group] [Mark files as ignored]    │
│  [Assign ungrouped file to this group]                         │
└─────────────────────────────────────────────────────────────────┘

[← Previous Group]  [Confirm this group →]  [Next Group →]
```

Key UX decisions derived from PITFALLS.md (UX Pitfalls section):

1. **Sort ascending by confidence** — show lowest-confidence groups first. High confidence groups need the least attention. The user starts with the hard ones.
2. **Pagination** — 20 groups per page maximum. A flat list of 300+ groups causes abandonment.
3. **"Approve All High Confidence" batch action** — confirm all groups above 0.85 confidence in one click. This handles the obvious cases without requiring per-group review. Show count: "Approve 31 high-confidence groups."
4. **Confidence label** — show both the numeric score AND a label: `< 0.65` = LOW (red), `0.65–0.84` = MEDIUM (yellow), `≥ 0.85` = HIGH (green).

### Actions (GRUP-05, GRUP-06, GRUP-07, GRUP-08, GRUP-09)

**Merge (GRUP-05):** User selects two group cards and presses "Merge". A dropdown/searchable list shows other group names. Selected groups collapse into one with combined file lists; confidence becomes the lower of the two original confidences.

**Split (GRUP-06):** User selects one or more files in a group and presses "Split". A sub-panel appears: "Which files should stay in this group?" The user checks/unchecks files. Confirmed split creates a new group from the unchecked files.

**Rename (GRUP-07):** Inline editable text field for `canonical_name`. Edit in place, confirmation on blur or Enter.

**Assign ungrouped (GRUP-08):** Ungrouped files appear at the bottom of the review list with `is_ungrouped: true`. User can drag them to an existing group card or use "Assign to..." dropdown. Alternatively, ungrouped files can be confirmed as their own single-file group (they will become a song with one version).

**Ignore (GRUP-09):** Per-file "Ignore" button within a group card. Ignored files are moved to a separate "Ignored Files" section and excluded from group confirmation. An ignored file will not be organized in Phase 3. The user can un-ignore from the ignored files section.

### State Management in JS

The review UI manages in-memory state:

```javascript
// review-panel.js
let proposals = [];       // Vec<ProposedGroup> from invoke('propose_groups')
let confirmedGroups = [];  // Groups confirmed by user
let ignoredFiles = [];     // Hashes of ignored files
let currentPage = 0;
const PAGE_SIZE = 20;
```

The review panel does NOT persist anything to SQLite until the user clicks "Confirm All Groups" (the final step). All merge/split/rename operations are in-memory edits to the `proposals` array.

**Non-destructive guarantee (success criterion 5):** No file is copied or moved during the review step. The backend has no write operations during this phase — only reads (`propose_groups`) and the final `confirm_groups` write.

### Frontend Module

```
src/js/workflow/
└── review-panel.js     — group review UI, merge/split/rename/assign/ignore
```

This follows the existing `workflow/` namespace that was outlined in ARCHITECTURE.md. Phase 1 created only `panels/` — Phase 2 adds `workflow/`.

The module exports:
```javascript
export function init(container, options) {}  // render the review panel
export function show() {}
export function hide() {}
```

Called from `main.js` after scan completes.

---

## Plan 02-03: Group Persistence in SQLite

### New Tables Required

Phase 2 adds two new tables via a migration. The existing Phase 1 `migrations.rs` handles this via `CREATE TABLE IF NOT EXISTS` — the same pattern can be extended with a second batch.

**Migration approach:** Append to the `run_migrations` function with a new `conn.execute_batch()` call for the new tables. Since Phase 1 uses `CREATE TABLE IF NOT EXISTS` idempotently, this is safe to run at every startup.

```sql
-- song_groups: confirmed groups from the review step
CREATE TABLE IF NOT EXISTS song_groups (
    group_id      TEXT PRIMARY KEY,    -- UUID v4
    canonical_name TEXT NOT NULL,      -- user-confirmed song name
    confirmed_at  INTEGER NOT NULL,    -- unix timestamp when confirmed
    is_ignored    INTEGER NOT NULL DEFAULT 0  -- 1 if this "group" is an ignored singleton
);

-- group_files: maps file hashes to groups
CREATE TABLE IF NOT EXISTS group_files (
    hash          TEXT NOT NULL,        -- references files.hash
    group_id      TEXT NOT NULL,        -- references song_groups.group_id
    is_ignored    INTEGER NOT NULL DEFAULT 0,  -- 1 if this specific file is ignored
    manually_assigned INTEGER NOT NULL DEFAULT 0,  -- 1 if user manually assigned (not from matcher)
    assigned_at   INTEGER NOT NULL,     -- unix timestamp
    PRIMARY KEY (hash, group_id),
    FOREIGN KEY (hash) REFERENCES files(hash),
    FOREIGN KEY (group_id) REFERENCES song_groups(group_id)
);
```

**Design decisions:**

- `is_ignored` on both tables: A group can be ignored (all files in it), or individual files can be ignored within a kept group.
- `manually_assigned` flag: Tracks which assignments were manual vs auto-matched. Useful for analytics and for re-grouping if the user wants to re-run the matcher later.
- `confirmed_at` and `assigned_at`: Timestamps enable future features (e.g., "show me what I organized last week") without schema changes.
- No `confidence` stored in `song_groups`: The matcher's confidence was for the review step. After confirmation, the user has vouched for the group — storing the original confidence would only cause confusion ("why does this group show 61% confidence if I confirmed it?").

### Store Module Structure

Phase 2 adds a new store module:

```
src-tauri/src/store/
├── connection.rs    (existing — no changes)
├── migrations.rs    (MODIFIED — add song_groups and group_files tables)
├── files.rs         (existing — no changes)
├── settings.rs      (existing — no changes)
└── groups.rs        (NEW — CRUD for song_groups and group_files)
```

Key functions in `groups.rs`:

```rust
// Insert a confirmed group and its file assignments
pub fn confirm_group(db: &Mutex<Connection>, group: &ConfirmedGroup) -> Result<()>

// Get all confirmed groups with their file hashes
pub fn list_confirmed_groups(db: &Mutex<Connection>) -> Vec<ConfirmedGroup>

// Get the group for a specific file hash (for Phase 3 organize step)
pub fn get_group_for_file(db: &Mutex<Connection>, hash: &str) -> Option<String>  // group_id

// Check if any groups have been confirmed (Phase 3 gating)
pub fn has_confirmed_groups(db: &Mutex<Connection>) -> bool

// Mark a file as ignored (standalone or within a group)
pub fn mark_file_ignored(db: &Mutex<Connection>, hash: &str) -> Result<()>

// Reset all groups (allow re-grouping from scratch)
pub fn clear_all_groups(db: &Mutex<Connection>) -> Result<()>
```

### New Tauri Commands

Phase 2 adds to `commands/`:

```
src-tauri/src/commands/
├── mod.rs           (MODIFIED — export new commands)
├── scan.rs          (existing)
├── settings.rs      (existing)
├── browse.rs        (existing)
└── groups.rs        (NEW)
```

Commands in `groups.rs`:

```rust
// Compute and return group proposals from current files table
#[tauri::command]
pub fn propose_groups(state: State<'_, AppState>) -> Result<Vec<ProposedGroup>, String>

// Confirm a batch of groups (called once when user clicks "Confirm All Groups")
#[tauri::command]
pub fn confirm_groups(
    groups: Vec<GroupConfirmation>,
    state: State<'_, AppState>,
) -> Result<(), String>

// Get already-confirmed groups (for Phase 3 use, and for re-opening the review)
#[tauri::command]
pub fn list_groups(state: State<'_, AppState>) -> Result<Vec<ConfirmedGroupSummary>, String>

// Reset groups (re-run matching from scratch)
#[tauri::command]
pub fn reset_groups(state: State<'_, AppState>) -> Result<(), String>
```

**Input type for `confirm_groups`:**

```rust
#[derive(Deserialize)]
pub struct GroupConfirmation {
    pub canonical_name: String,
    pub file_hashes: Vec<String>,
    pub ignored_hashes: Vec<String>,  // files the user marked as ignored
}
```

All grouping happens in-memory on the frontend. The Rust backend just receives the final confirmed state and persists it. This keeps the Rust backend stateless during review.

### Re-grouping Behavior

If `confirm_groups` is called and groups already exist in SQLite:
- **Option A (Replace):** Clear all existing groups, insert the new batch. Simple, but destroys previous manual work if called accidentally.
- **Option B (Merge/Update):** Update canonical names for matching group_ids, insert new groups, leave unchanged groups alone. Complex.

**Recommendation: Option A with a confirmation modal.** "You have 47 confirmed groups. Re-grouping will replace them. Continue?" This avoids accidental data loss. The `reset_groups` command handles the explicit reset case. `confirm_groups` adds to existing groups (does not clear first) to allow incremental workflow: user confirms some groups, scans more files, confirms the rest.

---

## Cross-Cutting Concerns

### Services Module (grouper.rs)

Following the architecture pattern: business logic lives in `services/`, not commands. Add:

```
src-tauri/src/services/
├── scanner.rs      (existing)
└── grouper.rs      (NEW) — calls matcher::propose_groups(), handles large-corpus optimization
```

`grouper.rs` orchestrates:
1. Load all `FileRecord`s from `store::files::list_all_files()`
2. Call `matcher::propose_groups(&files)`
3. Return `Vec<ProposedGroup>`

Keeping matcher pure (no DB access) means it can be unit tested independently with synthetic `FileRecord` data.

### AppState Changes

Phase 2 does NOT need new fields in `AppState`. The group proposal is computed on-demand via `invoke('propose_groups')` and is stateless on the Rust side (the frontend holds the in-progress review state). Confirmed groups live only in SQLite.

No need for a `group_status: Mutex<GroupStatus>` analog to `scan_status` — the grouping operation is fast enough (< 500ms) to run synchronously in the command handler without spawning a thread.

### main.rs Registration

Add the new group commands to `generate_handler![]`:

```rust
.invoke_handler(tauri::generate_handler![
    scan_folder,
    cancel_scan,
    get_settings,
    save_settings,
    list_scanned_files,
    // Phase 2 additions:
    propose_groups,
    confirm_groups,
    list_groups,
    reset_groups,
])
```

---

## Key Decisions and Tradeoffs

### Decision 1: Stateless review (frontend holds all in-progress state)

**Chosen:** Frontend holds the `proposals` array and all edits (merge, split, rename) in JS memory. Rust backend is only called for the initial `propose_groups` and the final `confirm_groups`.

**Alternative:** Backend holds proposals in an `Arc<Mutex<Vec<ProposedGroup>>>` in `AppState`, and each UI action (merge, split, rename) is an IPC call that mutates the backend state.

**Why frontend-stateless wins:** The review session is transient — if the user closes the app mid-review, proposals are re-computed on next open (matcher is deterministic). No need for partial state persistence. IPC roundtrip per UI action would add 10–20ms latency for every drag-and-drop or button click, making the review UI feel sluggish. Frontend-local state is instant.

**Tradeoff accepted:** If the user crashes mid-review with 200 manually edited groups, they lose that work. Mitigation: show a "You have unsaved group edits — confirm before closing?" warning in the `onCloseRequested` Tauri event.

### Decision 2: Union-find for group formation (not hierarchical clustering)

**Chosen:** Union-find with a threshold cut. If confidence(A,B) >= threshold, merge A and B into one group.

**Alternative:** Complete-linkage hierarchical clustering with a dendrogram that the user can cut at different thresholds.

**Why union-find wins:** Simpler to implement and explain. The user sees "these files are grouped together" not "these files are at 73% similarity on the dendrogram." A configurable threshold slider in settings achieves the same tuning capability without the UX complexity of a dendrogram.

### Decision 3: Threshold default of 0.65

**Chosen:** Default to 0.65 trigram + signals combined.

**Rationale:** PITFALLS.md warns that 0.75 trigram-alone is too conservative when the BPM and temporal signals are also in play. A combined score of 0.65 with a BPM boost means two files with the same BPM and trigram score of 0.50 (borderline) still group together. A trigram score of 0.65 alone (no BPM match) is approximately "similar enough that the names share a common stem."

**Note from STATE.md:** "Fuzzy matching threshold (0.65 trigram) needs calibration against real 500-file corpus — may need research-phase if accuracy is poor." This is a known known. Phase 2 should implement the threshold as a configurable value (stored in `settings` table key `grouping_threshold`) so it can be adjusted without a code change.

### Decision 4: Confirmation is batch-atomic

**Chosen:** User confirms ALL groups at once with a single "Confirm All Groups" click. The `confirm_groups` command receives the full final state.

**Alternative:** Per-group incremental confirmation (user confirms groups one at a time, each call persists immediately).

**Why batch wins:** Avoids partial state in SQLite (e.g., 20 groups confirmed, user wants to undo a rename that was confirmed 15 groups ago). Batch atomicity means either all groups are confirmed or none are. It also simplifies the UI — there's a clear "review phase" and a clear "confirmed" state, matching the mental model of "I review everything, then commit."

**SQLite transaction:** Wrap the entire `confirm_groups` insertion in a single SQLite transaction for true atomicity.

---

## Known Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| Trigram produces wrong groups on short names ("Beat", "Loop", "Hit") | HIGH | MEDIUM | Require minimum 4-char normalized name for auto-grouping; below this, only exact-match groups; short-name files surface as ungrouped for manual assignment |
| FL Studio version number append-on-append produces `Song 22`, `Song 222` | MEDIUM | MEDIUM | Normalization strips ALL trailing digit clusters in a loop (not just one), verified by unit test |
| 500+ files causes noticeable (> 1s) matcher delay | LOW | LOW | At 500 files, 125,000 comparisons at ~2µs each = ~250ms; fast enough; for 2000+ files, add BPM pre-bucketing |
| User closes app during review, loses 30 minutes of edit work | MEDIUM | MEDIUM | `onCloseRequested` event handler checks for pending edits; show "Unsaved group edits — confirm before closing?" dialog |
| Re-grouping (reset + re-scan) destroys manually confirmed groups | LOW | HIGH | "Re-group" action shows count of existing groups and requires explicit confirmation; confirmed groups are never silently overwritten |
| Frontend proposals array gets out of sync with Rust state after a crash | LOW | LOW | On app restart, `list_groups` command shows confirmed groups; proposals are always re-computed fresh from `propose_groups` |
| `trigram` crate produces different scores than expected pg_trgm behavior | LOW | MEDIUM | Unit test the scorer against known filename pairs with known expected groupings; calibrate threshold against 10–20 real filenames from the user's corpus before Phase 2 ships |

---

## What Phase 2 Does NOT Include

Clarifying scope to prevent creep into Phase 3:

- No file copying or moving (Phase 3)
- No three-panel browse UI (Phase 3)
- No filesystem watcher/system tray (v0.2 — Phase 2 watch mode reference in PITFALLS.md is about v0.2 watch mode, not this Phase 2)
- No "Open in FL Studio" or "Show in Explorer" buttons (Phase 3)
- No version diff view (Phase 4)
- No tauri-plugin-shell (not needed until Phase 3)
- No tauri-plugin-notification (not needed until v0.2 watch mode)

The Phase 2 UI entry point is a "Review Groups" button that appears after a scan completes. The three-panel browse UI is Phase 3.

---

## Sources and Confidence

| Claim | Source | Confidence |
|-------|--------|-----------|
| `trigram::similarity()` matches pg_trgm behavior | [trigram crates.io](https://crates.io/crates/trigram) v0.4.4 | MEDIUM |
| `strsim` Sorensen-Dice and Jaro-Winkler for string similarity | [strsim-rs GitHub](https://github.com/rapidfuzz/strsim-rs) v0.11.1 | HIGH |
| O(n²) pairwise = 125,000 comparisons at n=500 causes 250ms latency | PITFALLS.md performance traps section | MEDIUM (estimate, unverified on this hardware) |
| FL Studio "Save new version" appends digit, doubles if already ends in digit | [PITFALLS.md integration gotchas](../research/PITFALLS.md) + [FL Studio Forums](https://forum.image-line.com/viewtopic.php?t=151019) | HIGH |
| Configurable threshold, non-destructive proposals required | REQUIREMENTS.md GRUP-01 through GRUP-09; PITFALLS.md Pitfall 4 | HIGH |
| Frontend-holds-state pattern for review UI | ARCHITECTURE.md Pattern 1 (thin commands) anti-pattern reasoning | HIGH |
| Union-find for transitive group closure | Standard algorithms — no external source needed | HIGH |
| Mutex not held across await (SQLite pattern) | ARCHITECTURE.md Anti-Pattern 4 | HIGH |
| matcher/ module isolation (pure: no DB, no filesystem) | ARCHITECTURE.md Structure Rationale | HIGH |
| Phase 1 SQLite schema: files, path_index, settings tables | `src-tauri/src/store/migrations.rs` (verified from source) | HIGH |
| Phase 1 FileRecord fields: hash, path, file_size, mtime, bpm, channel_count, plugins_json, fl_version | `src-tauri/src/store/files.rs` (verified from source) | HIGH |
| Cargo.toml is missing strsim and trigram | `src-tauri/Cargo.toml` (verified from source) | HIGH |
| Short name threshold 4 chars, trigram unreliable below this | PITFALLS.md Pitfall 4; standard trigram behavior | MEDIUM |

---

## RESEARCH COMPLETE

**Summary:** Phase 2 is well-understood with one calibration risk (threshold tuning on real corpus) and one moderate UX risk (user loses mid-review work on crash). All three plans have clear implementation paths:

- **02-01:** Pure Rust `matcher/` module with normalize/signal/score/union-find submodules; no DB or filesystem access; testable with synthetic data
- **02-02:** Frontend `workflow/review-panel.js` holding all in-progress state; paginated with confidence sorting; 5 actions (merge/split/rename/assign/ignore); batch-atomic final confirmation
- **02-03:** Two new SQLite tables (`song_groups`, `group_files`); new `store/groups.rs` module; 4 new Tauri commands in `commands/groups.rs`; no `AppState` changes needed

The most important non-obvious findings:
1. The normalization loop must strip ALL trailing digit clusters (not just one) to handle FL Studio's double-numbering edge case
2. Short filenames (< 4 chars normalized) must fall back to exact match — trigram similarity is meaningless below this length
3. The `trigram` and `strsim` crates are NOT in `Cargo.toml` yet — they must be added in Plan 02-01
4. The default threshold of 0.65 (combined score) should be stored in the `settings` table as `grouping_threshold` so it can be adjusted without a code change

**Planning can proceed.**

---

*Phase 2 research for: FLP Vault — fuzzy file grouper with manual review UI and SQLite group persistence*
*Researched: 2026-02-26*
