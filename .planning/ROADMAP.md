# Roadmap: FLP Vault

## Overview

FLP Vault ships in four phases gated by dependency: the FLP binary parser is on the critical path for everything, so it ships first alongside the build pipeline and SQLite store. Fuzzy grouping depends on parsed metadata, so the grouper and its mandatory manual review UI come second. File organization and browsing come third — they need confirmed groups to work. Version diff comes last because it requires both the browse UI and the parsed metadata to already exist. Each phase delivers a self-contained, user-verifiable capability.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Foundation** - FLP parser, SQLite store, settings, and Windows build pipeline
- [ ] **Phase 2: Grouping** - Fuzzy matcher and manual review UI
- [ ] **Phase 3: Organize and Browse** - File operations and three-panel browse UI
- [ ] **Phase 4: Version Diff** - Two-version comparison with BPM and plugin delta

## Phase Details

### Phase 1: Foundation
**Goal**: A working Tauri app skeleton that parses .flp binary metadata, caches results in SQLite, persists settings, and produces a signed Windows installer from CI
**Depends on**: Nothing (first phase)
**Requirements**: PARS-01, PARS-02, PARS-03, PARS-04, PARS-05, PARS-06, SETT-01, SETT-02, SETT-03, SETT-04, DIST-01, DIST-02
**Success Criteria** (what must be TRUE):
  1. User can point the app at a folder and see all .flp files discovered with their BPM, channel count, and plugin list
  2. Re-scanning the same folder is fast (SQLite cache returns metadata without re-parsing unchanged files)
  3. User can configure source, organized, and originals folder paths, and the settings survive an app restart
  4. GitHub Actions produces a runnable Windows installer (.msi or NSIS .exe) on every push without manual steps
  5. Parser processes .flp files from FL Studio 21+ without crashing on unknown event IDs
**Plans**: TBD

Plans:
- [ ] 01-01: Tauri project scaffold, Rust workspace, SQLite schema and migrations
- [ ] 01-02: FLP binary parser (BPM, time sig, channels, plugins, pattern count, forward-unknown handling)
- [ ] 01-03: Scan command, SQLite cache layer, settings persistence
- [x] 01-04: GitHub Actions Windows build pipeline and installer verification

### Phase 2: Grouping
**Goal**: Users can review and confirm fuzzy grouping proposals before any file is touched
**Depends on**: Phase 1
**Requirements**: GRUP-01, GRUP-02, GRUP-03, GRUP-04, GRUP-05, GRUP-06, GRUP-07, GRUP-08, GRUP-09
**Success Criteria** (what must be TRUE):
  1. After scanning, the app proposes groups with confidence scores — similar filenames with the same BPM cluster together
  2. User can merge two groups that belong to the same song
  3. User can split a group that incorrectly combines two songs
  4. User can rename the canonical song name, manually assign ungrouped files, and mark files as ignored
  5. No files are copied or moved until the user explicitly confirms groups — proposals are non-destructive
**Plans**: TBD

Plans:
- [ ] 02-01: Fuzzy matcher (trigram + BPM + temporal signals, configurable threshold)
- [ ] 02-02: Review UI (paginated group proposals sorted by confidence ascending, with merge/split/rename/assign/ignore actions)
- [ ] 02-03: Group persistence in SQLite (confirmed groups, ignored files, manual overrides)

### Phase 3: Organize and Browse
**Goal**: Users can organize confirmed groups into per-song folders and browse their library in a three-panel UI
**Depends on**: Phase 2
**Requirements**: FILE-01, FILE-02, FILE-03, FILE-04, BRWS-01, BRWS-02, BRWS-03, BRWS-04, BRWS-05, BRWS-06, BRWS-07, BRWS-08, BRWS-09, BRWS-10
**Success Criteria** (what must be TRUE):
  1. User can execute the organize step and .flp files are copied (not moved) into per-song folders with originals backed up
  2. Re-organizing after adding new files copies only new files — files already organized are not duplicated
  3. User can browse all songs in the left panel, select a song to see its versions in the middle panel, and select a version to see full metadata in the right panel
  4. User can search songs by name and see which versions changed BPM or channel count at a glance
  5. Right panel "Open in FL Studio" and "Show in Explorer" buttons work on the selected version
**Plans**: TBD

Plans:
- [ ] 03-01: File manager (copy-only, originals backup, dedup tracking, batch pacing)
- [ ] 03-02: Three-panel browse UI shell (dark theme, layout, Tauri IPC wiring)
- [ ] 03-03: Song list panel (grouped songs, search bar, file counts)
- [ ] 03-04: Version timeline panel (chronological versions, BPM/channel change indicators)
- [ ] 03-05: Version detail panel (full metadata, generators vs effects list, Open/Explorer actions)

### Phase 4: Version Diff
**Goal**: Users can compare any two versions of a song and see exactly what changed
**Depends on**: Phase 3
**Requirements**: DIFF-01, DIFF-02, DIFF-03, DIFF-04
**Success Criteria** (what must be TRUE):
  1. User can select two versions of a song from the version timeline and trigger a comparison
  2. Diff view shows BPM change between the two versions
  3. Diff view shows channels added and removed between the two versions
  4. Diff view shows plugins added and removed between the two versions
**Plans**: TBD

Plans:
- [ ] 04-01: Diff computation (BPM delta, channel count delta, plugin set diff)
- [ ] 04-02: Diff UI panel (two-column or inline diff view integrated with browse UI)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 3/4 | In Progress|  |
| 2. Grouping | 0/3 | Not started | - |
| 3. Organize and Browse | 0/5 | Not started | - |
| 4. Version Diff | 0/2 | Not started | - |
