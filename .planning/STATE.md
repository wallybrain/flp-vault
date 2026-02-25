# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-25)

**Core value:** A producer can instantly find any version of any song and see what changed between versions — without opening FL Studio.
**Current focus:** Phase 1 — Foundation

## Current Position

Phase: 1 of 4 (Foundation)
Plan: 0 of 4 in current phase
Status: Planned — ready to execute
Last activity: 2026-02-25 — Phase 1 plans created (4 plans in 3 waves)

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: —
- Trend: —

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Pre-build]: Tauri + Rust over Electron — smaller binary, native perf for file parsing
- [Pre-build]: SQLite in %APPDATA% not vault — avoids WAL journal churn in cloud sync
- [Pre-build]: Copy files, never move — move triggers delete+create in cloud sync
- [Pre-build]: Cursor-based FLP parser not nom — simpler for sequential TLV event streams
- [Pre-build]: .msi requires GitHub Actions windows-latest runner — cannot build from Linux

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 1]: .msi installer cannot be produced from Linux — GitHub Actions windows-latest runner must be wired in Phase 1 before any feature promises
- [Phase 1]: SQLite cache key (path, size, mtime) must be correct from day one — changing later requires migrations
- [Phase 2]: Fuzzy matching threshold (0.75 trigram) needs calibration against real 500-file corpus — may need research-phase if accuracy is poor
- [Phase 4 prep]: WebView2 bootstrapper mode needed for Windows 10 targets — verify in Phase 1 build pipeline test

## Session Continuity

Last session: 2026-02-25
Stopped at: Phase 1 planned. 4 plans in 3 waves. Ready to execute.
Resume file: None
