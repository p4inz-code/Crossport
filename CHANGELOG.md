# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-09-29

Product and platform polish on top of the released 1.0.0 baseline. The tagged
`v1.0.0` artifact is unchanged; this is the next development line.

### Added

- A responsive page-width system (`PageContainer` gains a `width` variant): the
  directory browser fills the whole window, working surfaces use a wide measure,
  and prose and forms keep a readable one. Views are no longer a fixed column in
  the middle of a large window, and the app shell tightens its navigation rail
  before it drops labels.
- A finalized CrossPort logo: one mark (two arrows crossing in opposite
  directions on a solid rounded field), one source of truth
  (`assets/brand/crossport-logo.svg`), and the packaged application/installer
  icons regenerated from it. The sidebar brand tile and the webview favicon use
  the same mark.
- `docs/product/PLATFORM_SUPPORT.md`: the current production platform, the
  cross-platform status of the architecture, and exactly what a Linux or macOS
  release would still require.
- `docs/product/PARAGON_CAPABILITY_GAP.md`: an honest capability matrix against
  filesystem-driver products — what CrossPort supports, what is
  platform-specific, and what would need a signed driver and is therefore out of
  scope (including native NTFS write access on macOS).

### Changed

- The Drives page hands the browser the rest of a wide window instead of
  leaving it a narrow panel, and the browser grows with the window height. On
  medium windows the volume rail becomes a horizontal strip above the browser
  rather than a column that squeezes it.
- README states the platform position directly: Windows is the only packaged
  and tested target; Linux and macOS are application foundation only.
- `docs/product/BRAND_GUIDELINES.md` documents the finalized logo and how to
  regenerate the platform icons from it.
- The whole interface now draws every colour, spacing, radius, and type value
  from the shared token set. The History and Recovery surfaces were rebuilt on
  the design system (they previously carried hard-coded pixel values), the
  volume cards, transfer cards, dialog, and sidebar were given a consistent
  hierarchy, and the transfer composer now states its destination as a panel
  rather than a muted caption.

### Fixed

- Four design variables that components referenced were never defined:
  `--color-surface-muted` (the History row hover therefore had no background at
  all), `--color-accent`, `--font-normal`, and `--space-3-5`. All four usages
  now resolve to real tokens, and a repository-wide check confirms no
  stylesheet references an undefined variable.
- A card whose header was its only content drew a stray rule beneath the
  header; the separator is now dropped when no body follows it.
- The measurement harness runs its scenarios one at a time. The idle-engine
  check reads process-wide CPU, so running it beside the other seven scenarios
  in the parallel default counted their work and reported a genuinely idle
  engine as spinning. The scenarios now serialize, and the idle engine measures
  0 ms of CPU over two seconds.

No engine, persistence, security, or IPC behaviour changed in this release.

## [1.0.0] - 2026-09-29

First production release. Windows only; feature complete, security hardened,
and frozen as the 1.0 development baseline (`v1.0.0`). Everything below is what
this release contains relative to `v0.1.0-foundation`.

### Security

- The recovery artifact scan now asks the platform's reparse-point predicate
  (`platform::is_reparse_point`) instead of `FileType::is_symlink`. On Windows a
  directory junction is a reparse point that `is_symlink` does not report, so
  the old check descended one and could report — and therefore let a cleanup
  delete — a file reached through it, outside the destination tree the scan was
  given. The scan now behaves as its own contract and the rest of the engine
  (planning and source inspection) already did. A regression test proves a linked
  directory is never descended.
- Settings writes use a unique temporary name (process id + counter) and
  `sync_all`, matching the durable document layer. The fixed `settings.json.tmp`
  name let two writers share one temporary file and race on the rename, and a
  predictable name in a user-writable directory is the shape a symlink redirection
  needs. A test proves a leftover file at the old name is neither reused nor
  removed.

### Added

- `docs/release/RELEASE_1.0.md`: the production release document — version,
  supported platform, installation, core functionality, verification behavior,
  recovery behavior, security model, artifact names and checksums, release
  procedure, known limitations, and the explicit list of what is not supported.
  `docs/release/RELEASE_CHECKLIST.md` gained a security-model spot-check and now
  points at it.
- A security-model section in the release checklist (capabilities unchanged, CSP
  unchanged, no new network dependency, no HTTP client in the Windows build, no
  file paths or contents in logs).

- Windows production readiness: `tauri.conf.json` now carries the publisher,
  copyright, license file, description, and category, a current-user NSIS
  installer with a language and Start-menu folder, a WiX MSI, a silent WebView2
  bootstrapper, and refused downgrades; unused plugin commands are stripped from
  the binary; and the shipped executable is `CrossPort.exe` (`mainBinaryName`)
  rather than cargo's `crossport.exe`.
- A release-artifact smoke test (`tests/artifact_smoke.rs`): the built
  application is copied alone into a temporary directory and launched with Node,
  pnpm, Cargo, and the repository scrubbed from its environment, proving it
  writes its startup line, creates a visible window, and exits cleanly when that
  window is closed.
- A performance measurement harness (`src/measure.rs`, opt-in through
  `cargo test --lib measure -- --ignored --nocapture`) covering directory
  listings, deep trees, many small files, a 512 MiB streamed file, idle CPU, a
  full history, and the deepest path this host can create — plus frontend
  budgets in `src/test/performance.test.tsx` for the largest listing, history,
  and queue the interface can be handed. Findings are in
  `docs/development/PERFORMANCE.md`.
- A message box for a fatal startup failure or a panic, with a matching log
  entry, so a release build on a machine with no console says what went wrong
  instead of showing a window that never appears.
- `scripts/release.sh` now builds with `--locked` and writes
  `bundle/checksums.txt` (a SHA-256 digest per artifact), and CI gained a
  Windows job that runs the Rust suite where the application ships.
- Cross-volume sanity coverage: a copy and a move between two real volumes,
  skipped with a printed reason on a single-volume host.
- Product experience pass: the whole application now reads as one product.
  The browser shows a backend-resolved breadcrumb trail and remembers where you
  went (back, forward, up — buttons and `Alt`+arrow keys); the composer states
  the source-to-destination mapping, counts, free space, conflict behaviour, the
  verification policy, and the warnings before anything is queued; the queue
  reports every status including verification and links a finished job to its
  durable record; history and recovery are reachable from the notifications that
  announce them.
- Keyboard and accessibility: `Ctrl`/`Cmd`+`1`…`6` move between pages from one
  navigation table, dialogs take and return focus and answer to `Esc`, the
  close-confirmation dialog states its consequences in its button labels, and a
  global `:focus-visible` ring covers every custom control.
- Desktop lifecycle: closing the window while work is in flight is held by Rust
  (`app:close-requested`) and answered in the interface, with `exit_app` as the
  only way to end the process — and only after the user says so. A close with
  nothing in flight is not intercepted.
- Browser and composer backend support: `list_ancestors` (the path and every
  directory above it, validated and labeled so the frontend never assembles a
  path) and `exit_app`.
- Skeleton honesty: a not-found route replaces the empty frame a stale link used
  to leave behind, the home page reports live counts from the backend plus what
  the application does and which shortcuts exist, and the application version is
  shown as installed.
- Post-transfer verification (`src-tauri/src/verification/`): three policies
  (`none`, `size`, `checksum`) mapped onto the method that actually ran. `size`
  compares each written file's byte count against the bytes streamed out of the
  source; `checksum` also compares the SHA-256 computed **while copying** against
  the file on disk, so a source that changed mid-copy cannot quietly pass.
  Verification runs inside an item's own completion — the job stays `running`
  with `activity: verifying` while a file is checked — and a mismatch fails that
  item, keeps the file it wrote, and reports both sides of the discrepancy. The
  job's summary states what was checked, what was not (`unverifiedFiles`), and
  that modified times and read-only attributes are **not** reapplied, so a green
  verdict never implies metadata was preserved. A checksum that cannot be
  compared is a failure, not a pass.
- Durable documents (`src-tauri/src/persistence/`): every transfer document is
  written to a unique temporary file, flushed, and renamed over the live path,
  carries a `schemaVersion`, migrates an older schema on read, refuses to
  overwrite a newer one, and preserves a damaged file as `*.corrupt-<ms>.*`
  instead of discarding it. Load outcomes are published to the UI as
  `loaded` / `missing` / `migrated` / `recovered` / `unsupported`.
- Transfer history (`src-tauri/src/history/`): one durable record per finished
  job — identity, intent, outcome, counters, timing, bounded issue sample, the
  verification verdict, and the recovery action where one applied. Interrupted
  and recovered jobs are recorded as such rather than as failures or successes.
  Retention is by count (default 200, range 20–2000), newest first, with the
  prune reported; writes happen under the store's lock so records are never
  lost.
- Crash recovery (`src-tauri/src/recovery/`): live job state is journaled while
  a job runs and dropped only after its history record is written, so a crash
  can never leave a finished job looking unfinished. Interrupted jobs are
  classified into five explicit outcomes (`completed_before_crash`,
  `restart_required`, `source_missing`, `destination_unavailable`,
  `unsupported`), with the request stored whole so a restart replays the policy
  and conflict strategy the user chose. Artifacts are matched exactly to their
  own job (`.crossport-<job>-<index>.partial`), never by name resemblance, and
  byte-offset resume is refused: a restart removes the leftovers and starts the
  affected files from zero.
- The archive (`src-tauri/src/archive.rs`): the one module that knows both the
  live engine and the durable side. It journals live state, writes history
  first and forgets state second, answers recovery questions, and reports each
  document's load state so a damaged file is visible instead of looking like an
  empty history.
- Recovery and history commands: `list_history`, `get_history_record`,
  `delete_history_record`, `clear_history`, `get_archive_status`,
  `list_recovery_candidates`, `get_recovery_candidate`, `recover_transfer`
  (with `discard`, `restart`, and `confirm`, and a `RecoveryReport` in reply).
- New backend error categories `verification_failed`, `state_unavailable`, and
  `recovery_unavailable`, mirrored in the frontend code list and documented in
  `docs/architecture/ERROR_HANDLING.md`.
- Settings for verification policy and history retention, validated on both
  sides. The policy is folded into a job's request when it is planned or
  started, so changing the setting later cannot alter queued work.
- Frontend verification, history, recovery, and archive layers:
  `src/types/{verification,archive,history,recovery}.ts` (the wire contract as
  zod schemas), `history-service.ts` and `recovery-service.ts`, and the
  history, recovery, and notification stores.
- History UI: a list of finished transfers with status, size, duration, and
  verdict, filters, empty and degraded states, and a details view with the
  request, counters, issues, and recovery action.
- Recovery UI: a page listing interrupted jobs with their outcome, progress
  before the interruption, artifacts left on disk, and what a restart would do,
  with Discard and Restart (or Confirm) exactly where they apply, plus a shell
  banner and sidebar count for jobs awaiting a decision.
- Notifications: finished, failed, verification-failed, skipped-item, and
  interrupted-transfer feedback, raised once per transition and deduplicated,
  with a verification failure never presented as a plain transfer failure.
- The transfer queue surface now says `Verifying` while a job checks its output
  and shows the verdict and what was not reapplied.
- Documentation: `docs/architecture/VERIFICATION.md`,
  `docs/architecture/RECOVERY.md`, and `docs/architecture/PERSISTENCE.md`.
- Transfer engine (`src-tauri/src/transfer/`): a real copy/move engine with a
  planned destination for every item. `plan.rs` walks sources in Rust with a
  100,000-item budget and resolves collisions before anything is touched;
  `copy.rs` streams through a reused 1 MiB buffer, writes each file to a
  `.crossport-<job>-<index>.partial` file beside its destination, and commits it
  with a rename, so an interrupted or cancelled transfer never leaves a
  half-written file under a real name. `safety.rs` validates sources and the
  destination (must exist, must be a directory, must be writable), refuses a
  destination that is its own source or lives inside it, and removes only the
  directories a job created when it unwinds. `conflict.rs` implements the three
  conflict strategies, including `Data (2).txt`-style renaming.
- Move semantics: a same-volume move of a clean root is a single rename and
  copies nothing; every other move copies first, checks the copied byte count
  per file, and only then deletes the source — and keeps the source (reporting
  it as an issue) when anything failed or was skipped, so a move can never lose
  data it did not place.
- Transfer queue (`src-tauri/src/transfer/mod.rs`): jobs are queued with
  monotonic identifiers and run in the order they were accepted, one at a time.
  Pause and resume work on queued and running jobs alike; cancel unwinds
  cooperatively and discards partial output; finished jobs can be pruned
  individually or in bulk. A worker panic is caught and reported as a failed
  job instead of poisoning the engine.
- Progress reporting: byte, file, and directory counters, current file, speed
  over a recent window, whole-run average, and an ETA that is only reported
  while a job is running. Progress is published as throttled `transfer:update`
  events carrying a typed `TransferSnapshot`, so the UI stays live without
  polling.
- Transfer commands (`src-tauri/src/commands/transfer.rs`): `plan_transfer`,
  `start_transfer`, `list_transfers`, `get_transfer`, `pause_transfer`,
  `resume_transfer`, `cancel_transfer`, `remove_transfer`, and
  `clear_finished_transfers`. Planning and queueing run on the blocking pool so
  the event loop never waits on a slow volume.
- New backend error categories for transfers: `unsafe_relationship`,
  `not_enough_space`, `disk_full`, `too_many_items`, `transfer_not_found`, and
  `transfer_failed`, mirrored in the frontend code list and documented in
  `docs/architecture/ERROR_HANDLING.md`.
- Platform support for the engine: reparse-point detection, disk-full error
  recognition, free-space probing per destination volume, volume identity
  comparison (`same_volume`), and path containment/equality helpers.
- Frontend transfer layer: `src/types/transfer.ts` (the snapshot, preview, and
  request contract as zod schemas), `transfer-service.ts` (typed commands plus a
  validated `transfer:update` subscription), and `transfer-store.ts` (the queue,
  merged by identifier, with per-job control state and dismissals that survive
  late events).
- Transfer UI: multi-select checkboxes and Copy to… / Move to… in the directory
  browser, a composer dialog that shows the backend's dry run (size, item
  counts, collisions, free space, same-volume note) and re-plans whenever the
  conflict strategy changes, and a Transfers page listing every job with its
  progress, speed, ETA, issues, and pause / resume / cancel / remove controls.
  Cancelling asks first, because it discards partial output.
- Tests for the transfer work: queue ordering, pause/resume/cancel on running
  and queued jobs, the conflict matrix, move behavior, failure isolation, prune
  rules, bounded memory while streaming a large file, real end-to-end copy/move
  sanity runs on disk, plus frontend service, store, presentation, and component
  tests.
- Volume model (`src-tauri/src/platform/volume.rs`): typed `DriveInfo` with a
  volume kind (`fixed`, `removable`, `network`, `optical`, `ram`, `unknown`),
  volume name, filesystem type, total/free/used capacity, read-only flag, and
  mounted status. Facts the platform does not report stay `null` instead of
  being guessed at.
- Windows volume probing through the Win32 volume APIs (`GetDriveTypeW`,
  `GetDiskFreeSpaceExW`, `GetVolumeInformationW`). Non-Windows targets keep the
  mount-point discovery and report the same fields as unknown.
- Directory listing (`src-tauri/src/filesystem/directory.rs` and the
  `list_directory` command): validates the path, lists exactly one directory
  (never recursing, never following symlinks or reparse points), returns typed
  entries with size, modification time, and read-only flag, orders directories
  before files, and flags listings truncated at 10,000 entries.
- Storage browser on the Drives page: volume rail with capacity and status, an
  entry table with per-entry metadata, back / up / refresh navigation, the
  native folder picker, and loading, empty, and error states with recovery.
- Browser store (`src/stores/browser-store.ts`): current location, navigation
  history, and listing state, with a request token so a slow listing can never
  overwrite a newer location.
- Backend tests for volume classification, capacity derivation, Windows volume
  probing, directory ordering and metadata, listing limits, symlink handling,
  and read-error mapping; frontend tests for the volume schema, directory
  listing service, browser store, volume list, directory browser, and the
  rebuilt drives page.
- Platform abstraction (`src-tauri/src/platform/`): OS identity, app directory
  resolution through Tauri's public path resolver, and drive-root enumeration
  (`GetLogicalDrives` on Windows, conventional mount points elsewhere).
- Filesystem foundation (`src-tauri/src/filesystem/`): strict path
  normalization (absolute, no null bytes, no root escape), directory
  validation, and metadata inspection.
- Native folder picker command (`pick_directory`) hosted in Rust via
  `tauri-plugin-dialog`; the webview holds no dialog or filesystem permission.
- New commands: `get_system_info`, `list_drives`, `inspect_path`.
- Additional backend error categories: `path_not_found`,
  `path_not_directory`, `permission_denied`, and `io`, with `io::Error`
  mapping. The frontend mirrors them as `IpcError` codes.
- Frontend typed IPC transport (`src/services/ipc.ts`) with payload schema
  validation, runtime detection, and structured error normalization.
- Frontend services, stores, and pages for system info and drives, plus a real
  settings page backed by the backend.
- Settings migration behavior: a settings file written by another version still
  loads, filling absent fields with defaults and ignoring unknown ones.
- Log rotation (5 MiB, three files) and dependency log levels.
- Tests: service, store, schema, formatter, and page tests on the frontend;
  platform, filesystem, drive, state, and error tests in Rust.
- Frontend test infrastructure (Vitest + Testing Library) and Rust unit tests.
- CI pipeline (lint, biome, build, frontend/backend tests, version sync).
- `windows-sys` (Windows-only) for drive-letter enumeration.
- Repository hygiene: root README, MIT license, changelog, contributor and
  security docs, editor configs, and operational scripts.

### Changed

- The browser no longer renders every entry of a listing: it renders a window
  of rows, states how much is shown with **Show 400 more** / **Show all N**,
  memoizes the rows, and keeps the checked paths in a set. At the 10,000-entry
  listing cap that took a first paint from 1.7 s to 0.17 s and a checkbox from
  0.65 s to 6 ms.
- The release profile uses `lto = "thin"` instead of fat LTO, and `tauri-utils`
  is pinned to `opt-level = 0`: rustc 1.97.1 on Windows dies inside the fat-LTO
  link of this crate and while optimizing that crate, reproducibly. The
  reasoning and a note to revert are in `apps/desktop/src-tauri/Cargo.toml`.
- README, the release process, the release checklist, and the CI documentation
  were corrected where they described workflows that do not exist — a release
  workflow building macOS and Linux bundles, an update server, publishing
  automation — and now describe what the repository actually does.
- The FAQ's backend contract was brought back in step with the code: it listed
  seven commands and six error codes, and now lists the command surface by
  domain and every error category, with `lib.rs` named as the authoritative
  command list. `README.md` records lexical (not resolved) path containment as a
  limitation, including that a destination traversing an existing junction is
  resolved by the operating system.
- A verification failure fails its item and the job, and reports what was
  expected and what was found, instead of a job ending as a plain success with
  only a byte count behind it.
- Finished jobs write a history record before their live state is forgotten, so
  an interrupted application can tell a finished job from an unfinished one.
- The transfer engine journals live state before a job can be claimed by a
  worker and before its terminal event is published, closing two races where a
  crash could have lost a queued job or shown a finished one as unfinished.
- Temporary document names carry the process id and a counter, fixing a real
  collision between concurrent writers.
- The Drives page can now compose transfers: entries are multi-selected and
  copied or moved to a destination chosen in the native dialog, and the
  backend's dry run is shown — what moves, what collides, how much space the
  destination has — before anything is queued. The frontend never estimates
  sizes or plans a transfer itself.
- The application shell feeds the transfer queue, so a running job keeps its
  progress wherever the user navigates; the Transfers page is listed in the
  sidebar and renders only snapshots the engine reported.
- Cancelling a transfer asks for confirmation before it discards partial
  output, and the default conflict strategy is Skip: a transfer never destroys
  data unless the user chose Replace.
- `list_drives` returns the full volume contract (identity, kind, filesystem,
  capacity, status) instead of a root and a label. A volume whose media is not
  ready is reported as unavailable rather than being dropped or invented.
- Volume enumeration and all filesystem commands run on the blocking pool, so
  the Tauri event loop never waits on a slow or disconnected volume.
- The Drives page is now the storage browser: volumes are listed with their
  metadata, and folders can be opened, navigated with back/up, and refreshed.
  The native folder picker opens the picked folder in the browser instead of
  only displaying its metadata.
- Settings are now owned and persisted by the Rust backend and exposed over
  typed IPC commands (`get_settings`, `update_settings`) returning structured
  errors (`code` + `message`). A validated localStorage fallback keeps browser
  dev mode working.
- Errors from the backend serialize as `{ code, message }` objects instead of
  opaque strings; placeholder `NotImplemented` errors were removed.
- Logging writes to a rotating file (5 MiB × 3) in the platform log directory
  as well as stdout; dev builds log at `debug`, release builds at `info`, and
  dependency crates are pinned to `warn`.
- A restrictive Content Security Policy (with a separate dev CSP) is enforced;
  the production policy also denies `object-src`, `base-uri`, `form-action`,
  and `frame-ancestors`.
- Cross-service error type unified on `IpcError` (replacing
  `SettingsServiceError`), so every failure carries a stable code.
- Routing and navigation list only implemented pages; the placeholder
  transfer/history features were removed until Phase 3 implements them.
- Platform detection uses the official `isTauri()` API instead of internal
  globals.

### Removed

- The `.github/workflows/release.yml` workflow. It built desktop bundles on
  Ubuntu, Windows, and macOS and created a draft GitHub release, which the
  project's own release process, CI documentation, and README say does not
  happen — releases are built locally on Windows and published by hand, and no
  macOS or Linux artifact is built or tested. `VERSIONING.md` now describes the
  actual process, and `RELEASE_TEMPLATE.md` no longer promises macOS and Linux
  downloads.
- Empty placeholder workspace packages (`packages/*`), placeholder Rust
  commands/modules/models that returned `NotImplemented`, dead frontend stores
  and types, and empty documentation stubs.
