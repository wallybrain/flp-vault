---
phase: 02-grouping
plan: 03
subsystem: database
tags: [sqlite, rusqlite, tauri, rust, ipc, persistence, uuid, chrono]

requires:
  - phase: 02-01
    provides: ProposedGroup struct, run_grouper service, matcher module
  - phase: 01-foundation
    provides: AppState, init_db, store patterns (Mutex<Connection>), settings table

provides:
  - song_groups and group_files SQLite tables (idempotent Phase 2 migration)
  - store/groups.rs CRUD module (confirm_groups, list_confirmed_groups, get_group_for_file, has_confirmed_groups, mark_file_ignored, clear_all_groups)
  - commands/groups.rs Tauri IPC commands (propose_groups, confirm_groups, list_groups, reset_groups)
  - Configurable grouping_threshold from settings table (default 0.65)
  - All 9 Tauri commands registered in main.rs

affects:
  - 02-04
  - 03-organizer
  - frontend-review-panel

tech-stack:
  added: []
  patterns:
    - "unchecked_transaction() for SQLite transactions when Mutex already held"
    - "GroupConfirmation input type separate from ConfirmedGroup output type"
    - "BTreeMap aggregation for JOIN rows into grouped structs"
    - "Threshold read from settings table — no code change needed to reconfigure"

key-files:
  created:
    - src-tauri/src/store/groups.rs
    - src-tauri/src/commands/groups.rs
  modified:
    - src-tauri/src/store/migrations.rs
    - src-tauri/src/store/mod.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/main.rs
    - src-tauri/src/matcher/mod.rs

key-decisions:
  - "unchecked_transaction() used for confirm_groups because Mutex lock is already held — rusqlite transaction() borrows &mut self which conflicts with MutexGuard"
  - "ProposedGroup gained Serialize/Deserialize derives to satisfy Tauri IPC return type requirement (auto-fix Rule 1)"
  - "Separate GroupConfirmation (input) and ConfirmedGroup (output) types — input needs ignored_hashes to control is_ignored flags, output needs both lists for UI display"
  - "BTreeMap keyed by group_id for list_confirmed_groups aggregation — preserves stable ordering without extra sort pass"

patterns-established:
  - "Store CRUD functions take &Mutex<Connection> and return Result<T, String> for Tauri command compatibility"
  - "Transactions use unchecked_transaction() + explicit commit() — consistent with settings/files pattern"

requirements-completed:
  - GRUP-05
  - GRUP-06
  - GRUP-07
  - GRUP-08
  - GRUP-09

duration: 3min
completed: 2026-02-26
---

# Phase 2 Plan 03: Grouping Persistence Summary

**SQLite group persistence layer (song_groups + group_files tables) and four Tauri IPC commands wiring the fuzzy matcher to the review frontend**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-26T02:56:10Z
- **Completed:** 2026-02-26T02:59:37Z
- **Tasks:** 2
- **Files modified:** 6 (plus 1 created per task = 8 total)

## Accomplishments
- Phase 2 SQLite migration adds song_groups and group_files tables with FK constraints (idempotent IF NOT EXISTS)
- store/groups.rs implements all 6 CRUD functions with single-transaction batch confirm and FK-order clear
- commands/groups.rs exposes propose_groups, confirm_groups, list_groups, reset_groups as Tauri IPC commands
- grouping_threshold read from settings table with 0.65 default — configurable without code change
- All 9 commands (5 Phase 1 + 4 Phase 2) registered in main.rs generate_handler

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Phase 2 SQLite migration and store/groups.rs CRUD module** - `8dea546` (feat)
2. **Task 2: Create commands/groups.rs Tauri commands and register in main.rs** - `c2869d6` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src-tauri/src/store/migrations.rs` - Added song_groups and group_files CREATE TABLE IF NOT EXISTS
- `src-tauri/src/store/groups.rs` - New CRUD module: confirm_groups, list_confirmed_groups, get_group_for_file, has_confirmed_groups, mark_file_ignored, clear_all_groups + unit tests
- `src-tauri/src/store/mod.rs` - Added pub mod groups
- `src-tauri/src/commands/groups.rs` - New Tauri commands: propose_groups, confirm_groups, list_groups, reset_groups
- `src-tauri/src/commands/mod.rs` - Added pub mod groups and re-exports
- `src-tauri/src/main.rs` - Extended use statement and invoke_handler with 4 new commands
- `src-tauri/src/matcher/mod.rs` - Added Serialize/Deserialize derives to ProposedGroup (auto-fix)

## Decisions Made
- Used `unchecked_transaction()` for confirm_groups because the Mutex lock is already held — rusqlite's `transaction()` requires `&mut self` which conflicts with the MutexGuard
- Separate `GroupConfirmation` (input) and `ConfirmedGroup` (output) types: input needs explicit ignored_hashes list to set is_ignored flags; output needs both lists for review UI
- BTreeMap keyed by group_id for list aggregation — stable ordering, no extra sort pass

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added Serialize/Deserialize derives to ProposedGroup**
- **Found during:** Task 2 (creating propose_groups Tauri command)
- **Issue:** ProposedGroup only derived Debug and Clone — Tauri commands returning custom types must implement Serialize; the command would fail to compile
- **Fix:** Added `serde::Serialize, serde::Deserialize` derives to ProposedGroup in matcher/mod.rs
- **Files modified:** src-tauri/src/matcher/mod.rs
- **Verification:** cargo check passes cleanly after fix
- **Committed in:** c2869d6 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — missing derive for IPC type)
**Impact on plan:** Required for correct compilation — no scope creep.

## Issues Encountered
None beyond the ProposedGroup derive fix above.

## User Setup Required
None - no external service configuration required.

## Self-Check: PASSED

All files exist and both task commits verified (8dea546, c2869d6).

## Next Phase Readiness
- IPC layer is complete: frontend can now call propose_groups to run the matcher and confirm_groups to persist results
- Plan 02-02 (review UI) can wire its confirm button to the confirm_groups command
- Phase 3 organizer can call get_group_for_file to resolve which folder each file belongs in
- grouping_threshold is adjustable via save_settings — no code changes needed for threshold tuning

---
*Phase: 02-grouping*
*Completed: 2026-02-26*
