# Phase 1: Foundation - Context

**Gathered:** 2026-02-25
**Status:** Ready for planning

<domain>
## Phase Boundary

A working Tauri app skeleton that parses .flp binary metadata, caches results in SQLite, persists settings, and produces a signed Windows installer from CI. This phase delivers the FLP parser, SQLite cache, settings UI, scan results table, and GitHub Actions build pipeline. Grouping, file organization, and browse UI are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Scan results display
- Simple table layout with columns: filename, BPM, channels, plugins, modified date
- All columns sortable, default sort by filename
- Plugin column truncates after 2-3 plugins with "(+N more)" — full list on hover/click
- Filename only in Name column — full path available as hover tooltip
- Dark theme table

### Settings UI
- Gear icon in toolbar opens a slide-out side panel (not a separate page)
- Three folder pickers: source, organized, originals
- Smart defaults: source defaults to FL Studio's default project folder; organized and originals default to subfolders near source
- Changing source folder triggers an automatic rescan
- Validation with warnings: check folder exists/writable, warn if source = organized, warn about Cryptomator vault latency. Warnings inform but don't block.

### Parser resilience
- Best-effort parsing for all .flp versions — files going back to FL Studio 4 should be attempted
- Extract whatever metadata is readable; never skip a file entirely
- Warning icon on files with incomplete/unparseable data, tooltip explains what couldn't be read
- Unknown event IDs are skipped silently (forward-compatible with future FL Studio versions)

### Scan progress
- Progress bar with file count ("Scanning... 142/873 files") at top
- Results stream into the table as they're parsed (live accumulation)
- Cancel button stops scan — already-parsed files are shown and cached
- Re-scanning picks up where it left off (cached files not re-parsed)

### First run experience
- No onboarding wizard — app opens straight to the main view
- Empty state: large centered message "No .flp files found" with button linking to settings
- No special first-run flow — settings panel is the entry point

### Installer
- MSI installer (Tauri native)
- GitHub Actions produces installer on every push

### Claude's Discretion
- App icon and branding choices
- Exact table styling, spacing, typography within dark theme
- Loading skeleton / shimmer design during scan
- Error state handling details
- Compression and temp file handling during build

</decisions>

<specifics>
## Specific Ideas

- User has .flp files going back to FL Studio 4 (very old Fruity Loops) — parser must handle legacy binary format gracefully, not just FL 20+
- The scan results table should feel like a file manager — dense, scannable, functional
- Auto-rescan on folder change is the expected flow — "why would you change the folder and NOT want to see what's in it?"

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-02-25*
