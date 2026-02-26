---
phase: 02-grouping
plan: 02
subsystem: ui
tags: [tauri, vanilla-js, dom, review-panel, grouping]

requires:
  - phase: 02-01
    provides: propose_groups Rust command + fuzzy matcher returning ProposedGroup structs

provides:
  - review-panel.js module with merge, split, rename, assign, ignore actions
  - proposeGroups/confirmGroups/listGroups/resetGroups API wrappers in api.js
  - Review Groups toolbar button wired to review lifecycle in main.js
  - Close guard protecting unsaved group edits

affects:
  - 02-03 (confirm_groups Tauri command receives GroupConfirmation structs from this panel)

tech-stack:
  added: []
  patterns:
    - "Review panel uses makeEl()+textContent only — no innerHTML (Tauri IPC safety)"
    - "Custom events (review:cancel, review:confirmed) decouple panel from main.js"
    - "All review edits are in-memory mutations — no file I/O until confirmGroups() call"
    - "Close guard via onCloseRequested with try/catch fallback for non-Tauri environments"

key-files:
  created:
    - src/js/workflow/review-panel.js
  modified:
    - src/js/api.js
    - src/js/main.js
    - src/styles/main.css
    - src/index.html

key-decisions:
  - "Custom events (review:cancel, review:confirmed) over callback props — cleaner decoupling from main.js"
  - "Split mode uses in-card checkbox UI (not modal) — less disruptive, stays in context"
  - "Merge dropdown is inline (appended after button) not a global modal — simpler, no z-index issues"
  - "Close guard wrapped in try/catch — onCloseRequested unavailable in browser dev mode"
  - "Approve All High Confidence calls confirmGroups immediately and removes approved groups from proposals — not a local mark, a real backend confirm"

patterns-established:
  - "review-panel.js: in-memory proposals array mutated by action handlers, full re-render on each change"
  - "Non-destructive guarantee: reviewPanel never invokes copy/move operations, only propose_groups + confirm_groups"

requirements-completed: [GRUP-05, GRUP-06, GRUP-07, GRUP-08, GRUP-09]

duration: 3min
completed: 2026-02-26
---

# Phase 2 Plan 2: Group Review UI Summary

**Paginated review panel with in-memory merge/split/rename/assign/ignore actions and batch high-confidence approval, wired into main.js lifecycle with close guard**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-26T02:56:06Z
- **Completed:** 2026-02-26T02:59:34Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Built `review-panel.js` (609 lines) with full group review lifecycle — proposals fetched, sorted ascending by confidence (hardest first), rendered as paginated cards
- Implemented all 5 GRUP actions: merge (combines file_hashes, takes lower confidence), split (new group via crypto.randomUUID()), rename (inline edit on blur/Enter), assign ungrouped (dropdown select), ignore (per-file with un-ignore)
- Wired review panel into main.js with toolbar "Review Groups" button appearing post-scan, review:cancel/confirmed events, and close guard via onCloseRequested

## Task Commits

1. **Task 1: Create review-panel.js with group display, pagination, and confidence coloring** - `c227f75` (feat)
2. **Task 2: Implement merge/split/assign/ignore actions and wire review flow into main.js** - `5ba407e` (feat)

**Plan metadata:** (see final docs commit)

## Files Created/Modified

- `src/js/workflow/review-panel.js` - Full review panel module (609 lines): init/show/hide/hasUnsavedEdits exports, renderPage/renderGroupCard, all 5 action handlers, confirm/approve batch actions
- `src/js/api.js` - Added proposeGroups, confirmGroups, listGroups, resetGroups invoke wrappers
- `src/js/main.js` - Imports reviewPanel, wires lifecycle (show button post-scan, showReviewView/showScanView, close guard)
- `src/styles/main.css` - Review panel styles: group cards, confidence colors (LOW=#ff4444/MEDIUM=#ffaa00/HIGH=#44ff44), action buttons, merge dropdown, ungrouped/ignored sections
- `src/index.html` - Added btn-review-groups toolbar button and review-container div

## Decisions Made

- Custom events (`review:cancel`, `review:confirmed`) over callback props — cleaner decoupling from main.js, panel doesn't need reference to outer functions
- Split mode uses in-card checkbox UI — less disruptive than a modal, keeps user in context of the group
- Merge dropdown is inline (appended after the Merge button) — avoids global modal and z-index complexity
- `onCloseRequested` wrapped in try/catch — not available in browser dev mode, should not crash app
- "Approve All High Confidence" calls `confirmGroups` immediately and removes those groups from proposals — gives real-time feedback that they are confirmed, not just locally marked

## Deviations from Plan

None — plan executed exactly as written. All action handlers were implemented in review-panel.js during Task 1 as a cohesive module; Task 2 verified completeness and wired main.js.

## Issues Encountered

- `scanTable.updateEmptyState()` called in initial draft of `showScanView()` — that function is not exported from scan-table.js. Fixed by removing the call (scan table manages its own empty state visibility internally).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Review panel sends `GroupConfirmation[]` to `confirm_groups` Tauri command
- Phase 02-03 must implement `confirm_groups` Rust handler that persists confirmed groups to SQLite
- The `GroupConfirmation` struct (canonical_name, file_hashes, ignored_hashes) is the contract between this frontend and the backend

---
*Phase: 02-grouping*
*Completed: 2026-02-26*
