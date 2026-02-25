# Requirements: FLP Vault

**Defined:** 2026-02-25
**Core Value:** A producer can instantly find any version of any song and see what changed between versions — without opening FL Studio.

## v1 Requirements

Requirements for v0.1 release. Each maps to roadmap phases.

### Parsing

- [ ] **PARS-01**: App scans a user-configured source folder recursively to discover all .flp files
- [x] **PARS-02**: App parses .flp binary format to extract BPM and time signature
- [x] **PARS-03**: App parses .flp binary format to extract channel names and plugin IDs (generators vs effects)
- [x] **PARS-04**: App parses .flp binary format to extract pattern count and mixer track count
- [x] **PARS-05**: Parser skips unknown event IDs without error (forward-compatible with future FL Studio versions)
- [x] **PARS-06**: Parsed metadata is cached in SQLite keyed by file content hash with (mtime, size) shortcut

### Grouping

- [ ] **GRUP-01**: App fuzzy-matches .flp filenames using trigram similarity after stripping version numbers, dates, and common suffixes
- [ ] **GRUP-02**: BPM matching boosts grouping confidence when two files share the same BPM
- [ ] **GRUP-03**: Temporal clustering boosts grouping confidence for files saved within days of each other
- [ ] **GRUP-04**: App recognizes FL Studio "Save new version" naming pattern (Song Name 2.flp, Song Name 3.flp)
- [ ] **GRUP-05**: User can merge two groups that are the same song
- [ ] **GRUP-06**: User can split a group that incorrectly lumps two songs
- [ ] **GRUP-07**: User can rename the canonical song name for a group
- [ ] **GRUP-08**: User can manually assign ungrouped files to existing groups
- [ ] **GRUP-09**: User can mark files as ignored (throwaway experiments)

### File Operations

- [ ] **FILE-01**: App copies .flp files into per-song folders in the organized directory (never moves)
- [ ] **FILE-02**: App copies originals to the originals directory before organizing
- [ ] **FILE-03**: App tracks what was copied to prevent duplicates on re-scan
- [ ] **FILE-04**: All file operations are read-only on .flp files — never modify project files

### Browse UI

- [ ] **BRWS-01**: Left panel displays all songs grouped by name with total song/file counts
- [ ] **BRWS-02**: Left panel provides search bar for filtering songs by name
- [ ] **BRWS-03**: Middle panel shows all versions of selected song in chronological order with version number, date, and BPM
- [ ] **BRWS-04**: Middle panel shows visual indicator when BPM or channel count changed between versions
- [ ] **BRWS-05**: Right panel shows version detail: BPM, time signature, channel count, pattern count, mixer track count
- [ ] **BRWS-06**: Right panel lists generators (instruments/synths) separately from effects
- [ ] **BRWS-07**: Right panel shows file size, modification date, and full path
- [ ] **BRWS-08**: Right panel provides "Open in FL Studio" action button
- [ ] **BRWS-09**: Right panel provides "Show in Explorer" action button
- [ ] **BRWS-10**: UI uses dark theme

### Version Diff

- [ ] **DIFF-01**: User can select two versions of a song to compare
- [ ] **DIFF-02**: Diff shows BPM changes between versions
- [ ] **DIFF-03**: Diff shows channel count delta between versions
- [ ] **DIFF-04**: Diff shows plugins added and removed between versions

### Settings

- [ ] **SETT-01**: User can configure source folder path
- [ ] **SETT-02**: User can configure organized folder path
- [ ] **SETT-03**: User can configure originals folder path
- [x] **SETT-04**: Settings persist across app restarts

### Distribution

- [x] **DIST-01**: App is distributed as a Windows installer (.msi or NSIS setup.exe)
- [x] **DIST-02**: Installer size is under 15 MB

## v2 Requirements

Deferred to v0.2 release. Not in current roadmap.

### Watch Mode

- **WTCH-01**: System tray watcher monitors source folder for new .flp files
- **WTCH-02**: High-confidence matches auto-file after 10 seconds with toast notification
- **WTCH-03**: Ambiguous matches show popup with suggested song and alternatives
- **WTCH-04**: New songs prompt user to name or file to _Unsorted
- **WTCH-05**: Optional launch-with-Windows setting
- **WTCH-06**: Right-click tray menu: open app, pause watching, settings, quit

### Power Features

- **PWRF-01**: Search across all songs by plugin name
- **PWRF-02**: Filter songs by BPM range
- **PWRF-03**: Filter songs by date range
- **PWRF-04**: Batch pacing option for cloud-sync-friendly legacy imports

## v3 Requirements

Deferred to v0.3+.

- **TAGS-01**: User tags/notes on versions ("drums done", "sent to vocalist")
- **FILT-01**: Filter by channel count
- **RPRT-01**: Export song history as report

## Out of Scope

| Feature | Reason |
|---------|--------|
| Audio preview/playback | .flp files contain no audio; rendering requires FL Studio |
| Editing .flp files | Binary format with internal references; risk of corruption destroys trust |
| Git-style branching/merging | Producers don't think in branches; .flp is binary so diffs are meaningless |
| Cloud sync features (Proton Drive API) | Cryptomator operates at filesystem level; no API available |
| Cross-platform (macOS/Linux) | FL Studio is Windows-primary; adding platforms = 2-3x scope |
| Mobile companion app | Source files live in encrypted vault on Windows; different product |
| Collaboration features | Shared vaults + access control + conflict resolution = multi-year project |
| AI-generated song names | Requires internet, API keys, cost; wrong names are worse than original names |
| Duplicate detection | Fuzzy grouping already surfaces near-duplicates as versions of same song |
| Zip project support | Different parser for FL Studio .zip format; defer to v0.3+ |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PARS-01 | Phase 1 | Pending |
| PARS-02 | Phase 1 | Complete |
| PARS-03 | Phase 1 | Complete |
| PARS-04 | Phase 1 | Complete |
| PARS-05 | Phase 1 | Complete |
| PARS-06 | Phase 1 | Complete |
| SETT-01 | Phase 1 | Pending |
| SETT-02 | Phase 1 | Pending |
| SETT-03 | Phase 1 | Pending |
| SETT-04 | Phase 1 | Complete |
| DIST-01 | Phase 1 | Complete (01-04) |
| DIST-02 | Phase 1 | Complete (01-04) |
| GRUP-01 | Phase 2 | Pending |
| GRUP-02 | Phase 2 | Pending |
| GRUP-03 | Phase 2 | Pending |
| GRUP-04 | Phase 2 | Pending |
| GRUP-05 | Phase 2 | Pending |
| GRUP-06 | Phase 2 | Pending |
| GRUP-07 | Phase 2 | Pending |
| GRUP-08 | Phase 2 | Pending |
| GRUP-09 | Phase 2 | Pending |
| FILE-01 | Phase 3 | Pending |
| FILE-02 | Phase 3 | Pending |
| FILE-03 | Phase 3 | Pending |
| FILE-04 | Phase 3 | Pending |
| BRWS-01 | Phase 3 | Pending |
| BRWS-02 | Phase 3 | Pending |
| BRWS-03 | Phase 3 | Pending |
| BRWS-04 | Phase 3 | Pending |
| BRWS-05 | Phase 3 | Pending |
| BRWS-06 | Phase 3 | Pending |
| BRWS-07 | Phase 3 | Pending |
| BRWS-08 | Phase 3 | Pending |
| BRWS-09 | Phase 3 | Pending |
| BRWS-10 | Phase 3 | Pending |
| DIFF-01 | Phase 4 | Pending |
| DIFF-02 | Phase 4 | Pending |
| DIFF-03 | Phase 4 | Pending |
| DIFF-04 | Phase 4 | Pending |

**Coverage:**
- v1 requirements: 31 total
- Mapped to phases: 31
- Unmapped: 0

---
*Requirements defined: 2026-02-25*
*Last updated: 2026-02-25 — traceability populated by roadmapper*
