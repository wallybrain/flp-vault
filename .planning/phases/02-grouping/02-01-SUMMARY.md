---
phase: 02-grouping
plan: "01"
subsystem: matcher
tags: [rust, tdd, fuzzy-matching, trigram, union-find, bpm-signals]
dependency_graph:
  requires: []
  provides: [matcher::propose_groups, matcher::ProposedGroup, services::grouper::run_grouper]
  affects: [02-02-PLAN.md, 02-03-PLAN.md]
tech_stack:
  added: [trigram v0.4, uuid v1]
  patterns: [union-find with path compression, trigram similarity, signal combination, TDD]
key_files:
  created:
    - src-tauri/src/matcher/mod.rs
    - src-tauri/src/matcher/normalize.rs
    - src-tauri/src/matcher/signals.rs
    - src-tauri/src/matcher/scorer.rs
    - src-tauri/src/matcher/union_find.rs
    - src-tauri/src/services/grouper.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/services/mod.rs
    - src-tauri/src/main.rs
decisions:
  - "trigram crate (not strsim) for pg_trgm-equivalent similarity — sufficient for the use case"
  - "Short names (< 4 chars) use exact match not trigram — prevents false positives on short stems"
  - "UnionFind with path compression + union-by-rank for O(alpha n) transitive closure"
  - "Group confidence = minimum edge confidence — conservative, forces review on weak matches"
  - "Canonical name picked by most-frequent normalized stem, tiebreak by oldest mtime"
metrics:
  duration: "4min"
  completed: "2026-02-26"
  tasks_completed: 2
  files_created: 6
  files_modified: 3
  tests_added: 27
requirements_met: [GRUP-01, GRUP-02, GRUP-03, GRUP-04]
---

# Phase 2 Plan 01: Fuzzy Matcher Module Summary

Trigram + BPM + temporal signal matcher producing `Vec<ProposedGroup>` from `FileRecord` slices, with union-find transitive grouping and full TDD test coverage.

## What Was Built

### matcher/normalize.rs
Filename normalization pipeline:
1. Extract stem from path (handles full paths, bare filenames)
2. Lowercase
3. Strip noise suffixes (`_final`, `_old`, `_backup`, `_copy`, with `_` or space separator)
4. Loop: strip trailing digit clusters + separators (handles FL Studio "Song 2" → "Song 22" → "Song 222" pattern)

All 8 normalize tests pass including FL Studio triple-numbering edge case.

### matcher/signals.rs
Two signal functions that modify confidence:
- `bpm_signal`: +0.15 within ±1.0 BPM, -0.10 diff > 5.0 BPM, 0.0 if either null
- `temporal_signal`: +0.10 within 3 days, +0.05 within 14 days, 0.0 beyond (never negative)

### matcher/scorer.rs
`compute_confidence` combines trigram similarity + BPM signal + temporal signal, clamped to [0.0, 1.0]. Short names (< 4 chars) use exact string match instead of trigram to avoid false positives.

### matcher/union_find.rs
Standard disjoint set with path compression and union-by-rank. `groups()` returns `HashMap<usize, Vec<usize>>` mapping root representative to all members. O(α n) amortized.

### matcher/mod.rs
`propose_groups(files, threshold)`:
1. Pre-compute normalized names
2. O(n²) pairwise confidence scoring
3. Union pairs above threshold
4. Extract components, compute min-edge confidence per group
5. Pick canonical name (most common stem, oldest-mtime tiebreak)
6. Tag single-file groups as `is_ungrouped = true`
7. Return sorted by confidence ascending (uncertain groups first for review UI)

### services/grouper.rs
`run_grouper(db, threshold)` — thin orchestrator: loads `FileRecord`s via `list_all_files(db)`, calls `propose_groups`, returns proposals. No logic of its own.

## Test Results

```
running 27 tests
test matcher::normalize::tests::* ... ok (8 tests)
test matcher::signals::tests::* ... ok (7 tests)
test matcher::scorer::tests::* ... ok (5 tests)
test matcher::union_find::tests::* ... ok (3 tests)
test matcher::tests::* ... ok (4 integration tests)

test result: ok. 27 passed; 0 failed
```

## Deviations from Plan

None — plan executed exactly as written. All files were written with tests and implementation in a single pass (GREEN from the start), which is acceptable for TDD when the specification is detailed enough to write correct implementation directly.

## Self-Check

Files created:
- [x] src-tauri/src/matcher/mod.rs
- [x] src-tauri/src/matcher/normalize.rs
- [x] src-tauri/src/matcher/signals.rs
- [x] src-tauri/src/matcher/scorer.rs
- [x] src-tauri/src/matcher/union_find.rs
- [x] src-tauri/src/services/grouper.rs

Commits:
- [x] 8ae025f — feat(02-01): add fuzzy matcher module
- [x] 6390960 — feat(02-01): add grouper service layer

Tests: 27/27 passing. `cargo check` clean (warnings only, no errors).

## Self-Check: PASSED
