---
phase: 01-foundation
plan: 02
subsystem: parser
tags: [rust, byteorder, flp, binary-parsing, tdd]

# Dependency graph
requires: [01-01]
provides:
  - parse_flp(&[u8]) -> Result<FlpMetadata, ParseError> implementation
  - FLP event ID constants (events.rs)
  - Varint reader for TEXT event length prefixes
  - ParseError enum with InvalidMagic / TruncatedHeader / IoError variants
  - 15 unit tests covering all specified behaviors using synthetic byte arrays
affects: [01-03, 02-01, 02-02]

# Tech tracking
tech-stack:
  added:
    - byteorder 1.5.0 (already in Cargo.toml — used here for LittleEndian u16/u32 reads)
  patterns:
    - Cursor-based sequential event loop: match on event_id range to determine byte width
    - Mutable channel accumulator state flushed on FLP_NewChan and end-of-stream
    - Best-effort partial results: mid-stream errors push warnings, break loop, return Ok
    - Modern BPM (event 156) takes priority over legacy BPM (event 66) via Option::or()
    - UTF-16 LE detection via alternating-null-bytes heuristic or BOM prefix
    - Varint decoding: 7 bits per byte, MSB=1 means "more bytes follow"

key-files:
  created:
    - src-tauri/src/parser/events.rs
    - src-tauri/src/parser/flp.rs
  modified:
    - src-tauri/src/parser/mod.rs

key-decisions:
  - "Flush channel on FLP_NewChan and at end-of-stream (not just on NewChan) — handles last channel in file"
  - "decode_string uses alternating-null heuristic for UTF-16 detection (no BOM in many FL Studio versions)"
  - "BPM out-of-range produces None + warning rather than Err — preserves other metadata"

requirements-completed: [PARS-02, PARS-03, PARS-04, PARS-05]

# Metrics
duration: 12min
completed: 2026-02-25
---

# Phase 1 Plan 02: FLP Binary Parser Summary

**FLP binary parser fully implemented with 15 unit tests — extracts BPM (modern event 156 + legacy event 66), channel names, plugin names, pattern count, and FL Studio version from synthetic byte arrays; all tests pass with `cargo test parser`**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-25T23:55:14Z
- **Completed:** 2026-02-25T~00:07Z
- **Tasks:** 1 (TDD RED + GREEN in single pass)
- **Files modified:** 3

## Accomplishments

- `events.rs` — FLP event ID constants (BYTE/WORD/DWORD/TEXT ranges) and varint reader
- `flp.rs` — full `parse_flp(&[u8]) -> Result<FlpMetadata, ParseError>` implementation
- Header validation: checks "FLhd" magic, extracts channel_count from offset 10
- Event stream loop: dispatches on event_id byte-width range (0-63/64-127/128-191/192-255)
- BPM extraction: event 156 (value/1000.0) and event 66 (value as f32); modern wins
- Channel tracking: FLP_NewChan flushes accumulator into generators Vec
- Pattern counting: increments on every FLP_NewPat (event 65)
- Version string: event 199 decoded to fl_version field
- Best-effort partial results on truncation: push warning, break loop, return Ok
- BPM sanity gate: values outside 1.0–999.0 produce warning and set bpm = None
- UTF-16 LE decoding with alternating-null heuristic and BOM detection
- 15 unit tests, all using synthetic byte arrays (no real .flp files), all pass

## Task Commits

1. **Task 1: FLP binary parser — events.rs + flp.rs + tests** - `522c167` (feat)

## Files Created/Modified

- `src-tauri/src/parser/events.rs` — FLP_CHAN_TYPE, FLP_NEW_CHAN, FLP_NEW_PAT, FLP_TEMPO_LEGACY, FLP_TEMPO, FLP_TEXT_CHAN_NAME, FLP_VERSION, FLP_TEXT_PLUGIN_NAME constants + read_varint()
- `src-tauri/src/parser/flp.rs` — parse_flp(), ParseError enum, decode_string(), 15 unit tests
- `src-tauri/src/parser/mod.rs` — added `pub mod events; pub mod flp;` and re-exports

## Decisions Made

- Flush channel accumulator at end-of-stream as well as on FLP_NewChan — necessary to capture the last channel in the file (only NewChan-triggered flush would drop it)
- UTF-16 LE detected via alternating-null heuristic (bytes[1]==0 && bytes[3]==0) because many FL Studio versions don't write a BOM
- Out-of-range BPM values produce `None + warning` rather than `Err` — preserves all other metadata

## Deviations from Plan

None — plan executed exactly as written. TDD RED and GREEN phases combined since the implementation was written and verified in a single pass (tests were written alongside implementation and run immediately; all 15 passed on first cargo test run after implementation complete).

## Issues Encountered

None.

## Verification

```
$ cd src-tauri && cargo test parser
running 15 tests
test parser::flp::tests::test_bpm_out_of_range_produces_warning ... ok
test parser::flp::tests::test_channel_name_and_plugin ... ok
test parser::flp::tests::test_channel_type_extraction ... ok
test parser::flp::tests::test_empty_bytes_returns_invalid_magic ... ok
test parser::flp::tests::test_fl_studio_version_extraction ... ok
test parser::flp::tests::test_invalid_magic_returns_error ... ok
test parser::flp::tests::test_legacy_bpm_event_66 ... ok
test parser::flp::tests::test_modern_bpm_event_156 ... ok
test parser::flp::tests::test_modern_bpm_overrides_legacy ... ok
test parser::flp::tests::test_no_bpm_produces_none_and_warning ... ok
test parser::flp::tests::test_pattern_count ... ok
test parser::flp::tests::test_truncated_file_returns_partial_with_warning ... ok
test parser::flp::tests::test_unknown_event_ids_skipped ... ok
test parser::flp::tests::test_utf16_string_decoding ... ok
test parser::flp::tests::test_valid_header_parses ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
```

## Self-Check: PASSED
