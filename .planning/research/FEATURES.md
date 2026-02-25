# Feature Research

**Domain:** DAW project file organizer (FL Studio .flp)
**Researched:** 2026-02-25
**Confidence:** MEDIUM — Competitor analysis via WebSearch; no FL Studio-specific organizer tools exist, so comparison is against general-purpose DAW organizers (dBdone, SessionDock, Session Recall). Some claims are inferred from producer forum discussions.

## Competitor Landscape Summary

No FL Studio-specific project organizer exists. The market is general-purpose DAW organizers:

- **dBdone** ($119 one-time or $11.99/mo) — cross-DAW, auto-tags metadata (tempo, RMS, plugins), audio preview via VST plugin, version grouping by naming convention (v1/v2 patterns), project phases/workflow stages, notes editor. Added "Auto Versioning" in 04/2025.
- **SessionDock** (freemium, macOS/Windows) — visual library, mixdown preview, waveform annotation notes, release scheduling, cloud sync via iCloud/Dropbox. No metadata parsing of project binaries.
- **Session Recall** (free base, per-device one-time) — focused on hardware recall (mixer settings, synth patches), not project file management. Different category.
- **Splice Studio** (discontinued as standalone) — was git-style version control + cloud backup for DAW projects. Now primarily a sample marketplace.

None of these tools parse FL Studio binary metadata (BPM, plugins, channel list) from .flp files directly. None offer fuzzy grouping of chaotically-named files — they rely on the user already having organized folders or following naming conventions like `_v1`, `_v2`.

**FLP Vault's unique position:** binary metadata extraction + fuzzy grouping = the two things no competitor does.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Folder scan — discover all .flp files | Any organizer must find files to organize | LOW | Recursive scan; handle Cryptomator mount paths |
| Display filename, date modified, file size | Basic file browser expectation | LOW | Already in filesystem metadata, no parsing needed |
| Open file in FL Studio | Core action — every project manager does this | LOW | `ShellExecute` or `opener::open()` in Rust |
| Show file in Windows Explorer | Universal expectation for desktop file tools | LOW | `explorer /select,<path>` |
| Settings: configure source/organized/originals paths | Multi-folder tools require path configuration | LOW | First-launch wizard + settings screen |
| Non-destructive operation — never modify .flp files | Trust requirement; producers are paranoid about data loss | LOW (policy) | Read-only file access is an architectural constraint, not a feature toggle |
| Persist state across sessions — remember groups, names | Would be useless if user re-reviews every launch | MEDIUM | SQLite in %APPDATA% already planned |
| Search by song name | Table stakes for any library with >50 items | LOW | SQL LIKE query on song name |
| Organized folder output — per-song subfolders | The stated product value; everything else supports this | MEDIUM | File copy with dedup tracking |
| Safety backup of originals | Data loss fear; producers will not use a tool that could destroy files | LOW-MEDIUM | Copy to originals folder before organizing; document clearly |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required by convention, but where FLP Vault wins.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| .flp binary parsing — BPM, time signature, channels, plugin IDs | No competitor extracts metadata from .flp files; this is the entire basis for smart grouping and version diff | HIGH | pyflp is the reference; Rust port needed. Priority fields: BPM/time sig, channel names, plugin IDs, pattern count |
| Fuzzy filename grouping — trigram + BPM + temporal signals | 500+ chaotically-named files cannot be organized manually; no competitor solves this | HIGH | Trigram similarity after stripping version suffixes; BPM match as confidence boost; date proximity as tiebreaker |
| Manual review UI — merge, split, rename, assign, ignore | Fuzzy matching will be wrong sometimes; user must be able to fix it before copy executes | MEDIUM | Three-step workflow: scan → review → execute. This is the trust mechanism. |
| Version diff — compare two .flp versions showing plugin delta | Unique capability enabled by binary parsing; no DAW organizer does this | MEDIUM | Requires parsed metadata from both files; display added/removed plugins, BPM change, channel count delta |
| FL Studio "Save new version" pattern awareness | FL Studio appends `_2`, `_3` etc.; grouping must understand this specific convention | LOW-MEDIUM | Strip trailing `_N` and date suffixes before fuzzy comparison |
| System tray watcher — auto-file new saves with toast notification | Turns one-time import into ongoing workflow; no manual re-scan after every save session | HIGH | `ReadDirectoryChangesW` via Rust `notify` crate; debounce 2-3s; confidence-based auto-file vs prompt |
| Cloud-sync-aware file operations | Cryptomator + Proton Drive users will encounter data corruption if tools move files; copy-only with batch pacing is differentiated behavior | LOW (policy) + MEDIUM (pacing UI) | Copy not move; batch pacing option in legacy import; debounce watcher |
| Plugin search across entire library | "Which songs used Serum?" is a question no other organizer answers because they don't parse plugin metadata | LOW (given parsing) | SQL query on plugin_id from parsed metadata |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Audio preview / playback | Producers want to hear a project without opening FL Studio | .flp files contain no audio — they reference samples stored elsewhere. Rendering a preview requires running FL Studio. Any preview system requires a separate audio export workflow outside this tool. | Show file path in organized folder; user opens in FL Studio to hear it. If preview is needed later, integrate with a mixdown file convention (look for `<song>_mixdown.mp3` in same folder). |
| Edit .flp files — rename channels, change BPM | Seems useful once metadata is visible | .flp is a binary format with checksums and internal references. Editing risks corruption. pyflp's write support is experimental and fragile across FL Studio versions. One corrupted project file = trust destroyed. | Read-only always. If the user wants to change a name, they change it in FL Studio. |
| Git-style branching / merging of versions | Developers assume git is the right mental model | Producers don't think in branches. .flp is binary so diffs are meaningless to humans. The "branch" a producer wants is already in the filename (_mixdown, _vocals_done). Adding this complexity harms the core use case. | The version timeline IS the linear history. Diff shows what changed. That's sufficient. |
| Cloud sync features — Proton Drive API, sync status | User wants to see sync status in the app | Cryptomator operates at filesystem level; there is no API into Proton Drive sync state. Building this means maintaining integrations with multiple cloud providers. | Document the copy-not-move convention. Trust the cloud provider's UI for sync status. |
| Cross-platform — macOS / Linux support | Producers on other platforms want it | FL Studio is Windows-primary (macOS support is recent and secondary). The .flp format, watcher API (ReadDirectoryChangesW), and .msi distribution are all Windows-specific. Adding cross-platform adds 2-3x scope. | Windows only. Revisit after v1.0 ships. |
| Duplicate detection across the library | Producers want to find exact duplicate .flp files | Fuzzy grouping already handles near-duplicates by song. Exact dedup is a separate problem that adds complexity (byte-level hash or content hash) with low marginal value — the grouping already surfaces them as versions of the same song. | The version timeline implicitly surfaces duplicates. If two files land in the same song group with same date, that's a duplicate — surface it there. |
| Mobile companion app | "I want to browse my library on my phone" | Source files live in Cryptomator vault on Windows. A mobile app requires either cloud API access (see above) or sync of the SQLite database. Out of scope for a file organizer. | Out of scope. The tool runs on the same Windows machine as FL Studio. |
| Collaboration features — share projects with other producers | Producers collaborate | Shared vaults, access control, conflict resolution on .flp files — this is a fundamentally different product. Splice used to do this; it's a multi-year project. | Out of scope. Use Splice or share .zip exports manually. |
| AI-generated song names from metadata | Producers want better names for chaotically-named files | LLM calls require API keys, internet access, latency, and cost. The tool is designed for offline use. Wrong AI-generated names are worse than leaving original names. | Show metadata (BPM, top plugins, date) in review UI so user can make an informed rename decision. |

---

## Feature Dependencies

```
[FLP Binary Parser]
    └──required by──> [Fuzzy Grouping] (BPM signal)
    └──required by──> [Version Diff] (plugin/channel delta)
    └──required by──> [Plugin Search] (plugin index)
    └──required by──> [Version Detail Panel] (BPM, channels, patterns display)

[Fuzzy Grouping]
    └──required by──> [Manual Review UI] (groups to review)
    └──required by──> [Organized Folder Output] (groups define folder structure)

[Manual Review UI]
    └──required by──> [Organized Folder Output] (confirmed groups trigger copy)

[SQLite Store]
    └──required by──> [Manual Review UI] (persists group decisions)
    └──required by──> [Plugin Search] (indexes parsed metadata)
    └──required by──> [Version Diff] (caches parsed metadata)
    └──required by──> [System Tray Watcher] (persists known files, confidence scores)

[Organized Folder Output]
    └──enhances──> [System Tray Watcher] (watcher files to existing song folders)

[System Tray Watcher]
    └──enhances──> [Song List UI] (new file triggers UI refresh)
```

### Dependency Notes

- **FLP Binary Parser is the critical path:** Everything that makes FLP Vault different from a glorified file browser depends on successfully parsing .flp binary data. This is the highest-risk deliverable and must be phase 1.
- **Fuzzy Grouping requires Parser for BPM signal:** Without BPM as a grouping signal, fuzzy matching degrades to filename-only — still useful, but lower accuracy. Parser and grouping should ship together.
- **Manual Review UI is the trust mechanism:** Users will not commit to the organized folder copy without being able to inspect and fix the proposed groupings. The review step is mandatory before execute.
- **System Tray Watcher depends on everything else:** It's the ongoing workflow feature that assumes the library is already organized. It's a quality-of-life addition, not an MVP requirement.
- **Plugin Search is free given the Parser:** Once metadata is indexed in SQLite, cross-library plugin search is a single SQL query. Low cost, high value — add alongside search/filter UI.

---

## MVP Definition

### Launch With (v0.1)

Minimum viable product — what's needed to validate the concept.

- [ ] FLP binary parser — BPM, time signature, channel names, plugin IDs, pattern count
- [ ] Fuzzy filename grouping — trigram + BPM + temporal clustering
- [ ] Manual review UI — merge, split, rename, assign, ignore
- [ ] Organized folder copy with originals backup and dedup tracking
- [ ] Three-panel browse UI: song list, version timeline, version detail
- [ ] Version diff — compare two versions (BPM delta, plugin added/removed, channel count)
- [ ] Settings: configure three paths (source, organized, originals)
- [ ] .msi installer

**Why these are MVP:** Without the parser and grouper, there is no product — it's just a file browser. Without the review UI, users will not trust the copy step. Without the diff, version comparison requires opening FL Studio — the core value proposition is gone.

### Add After Validation (v0.2)

Features to add once core is working.

- [ ] System tray watcher mode with toast notifications — trigger: users start using the organized library and want ongoing maintenance without manual re-scans
- [ ] Launch with Windows option — trigger: same as above (daily driver use)
- [ ] Batch pacing option in legacy import — trigger: cloud-sync users report overwhelming sync queue during large imports
- [ ] Search by plugin name across all songs — trigger: low implementation cost given SQLite index already exists

### Future Consideration (v0.3+)

Features to defer until product-market fit is established.

- [ ] User tags/notes on versions ("drums done", "sent to vocalist") — value is clear but adds UI scope; defer until v0.1 is validated
- [ ] Filter by BPM range, date range, channel count — useful power feature, low complexity given SQLite, but adds UI work
- [ ] Export song history as a report — producer client work use case; defer
- [ ] Zip file support (.zip projects with samples) — FL Studio's .zip project format includes samples; parsing is different from .flp; defer

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| FLP binary parser | HIGH | HIGH | P1 — MVP gate |
| Fuzzy grouping algorithm | HIGH | HIGH | P1 — MVP gate |
| Manual review UI | HIGH | MEDIUM | P1 — trust mechanism |
| Organized folder copy + backup | HIGH | MEDIUM | P1 — the product action |
| Three-panel browse UI | HIGH | MEDIUM | P1 — navigation |
| Version diff | HIGH | LOW (given parser) | P1 — core differentiator |
| Settings + path config | HIGH | LOW | P1 — must work day 1 |
| .msi installer | HIGH | MEDIUM | P1 — distribution |
| System tray watcher | MEDIUM | HIGH | P2 — ongoing workflow |
| Plugin search | MEDIUM | LOW (given index) | P2 — cheap win |
| Filter by BPM/date | MEDIUM | LOW | P2 — power user |
| User tags/notes on versions | MEDIUM | MEDIUM | P2 — after validation |
| Batch pacing option | LOW-MEDIUM | LOW | P2 — cloud-sync users |
| Export report | LOW | LOW | P3 — niche use case |
| Zip project support | MEDIUM | HIGH | P3 — separate parser |

**Priority key:**
- P1: Must have for v0.1 launch
- P2: Add in v0.2 once core is stable
- P3: Future consideration

---

## Competitor Feature Analysis

| Feature | dBdone | SessionDock | FLP Vault |
|---------|--------|-------------|-----------|
| Multi-DAW support | Yes | Yes | No — FL Studio only (intentional) |
| .flp binary metadata parsing | No — reads filesystem metadata only | No | Yes — BPM, plugins, channels, patterns |
| Fuzzy filename grouping | Partial — pattern matching on `_v1`/`_v2` suffixes (added 04/2025) | No — user organizes manually | Yes — trigram + BPM + temporal signals |
| Manual grouping review UI | No | No | Yes — merge, split, rename, assign, ignore |
| Version diff (plugin delta) | No | No | Yes |
| Audio preview | Yes — VST plugin records a section | Yes — mixdown file preview | No (anti-feature) |
| Project workflow status | Yes — Idea/In Progress/Finished etc. | No | No (anti-feature for v0.1) |
| User notes on versions | Yes | Yes — waveform annotation | Planned v0.3 |
| Search/filter by metadata | Tags only (not parsed metadata) | Tags only | Plugin name, BPM range, date range |
| File system watcher | No | No | Yes — system tray watcher |
| Cloud sync aware | No | Yes — iCloud/Dropbox | Yes — copy-not-move, batch pacing |
| Windows support | Yes | Yes (v1.1+) | Yes (only) |
| Offline / no account | Yes | Partial | Yes |
| Price | $119 one-time | Freemium | Free / self-distributed |

---

## Sources

- [dBdone documentation](https://dbdone.com/documentation/) — competitor feature set
- [dBdone review — GearNews](https://www.gearnews.com/dbdone-freeware-organize-software-studio/) — feature overview
- [SessionDock homepage](https://sessiondock.com/) — competitor feature set
- [SessionDock review — GearNews](https://www.gearnews.com/sessiondock-studio-tech/) — feature overview
- [SessionDock — MusicRadar](https://www.musicradar.com/music-tech/its-a-pain-that-every-producer-knows-a-desktop-full-of-old-sessions-forgotten-mixdowns-and-endless-folders-struggling-to-keep-track-of-your-daw-projects-this-free-app-can-help) — producer pain point framing
- [pyflp — GitHub](https://github.com/demberto/PyFLP) — FL Studio binary parser reference
- [FL Studio "Save new version" forum thread](https://forum.image-line.com/viewtopic.php?t=151019) — naming convention behavior
- [KVR Audio — DAW version control discussion](https://www.kvraudio.com/forum/viewtopic.php?t=429799) — producer version tracking pain points
- [Splice Studio announcement — CDM](https://cdm.link/splice-studio-is-free-backup-version-control-and-collaboration-for-your-daw/) — historical git-style DAW version control

---

*Feature research for: FL Studio .flp project file organizer*
*Researched: 2026-02-25*
