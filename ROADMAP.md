# Roadmap

CrossPort's long-term vision is documented in `docs/product/VISION.md`; the
feature plan lives in `docs/product/FEATURE_SPECIFICATION.md`. This page tracks
the engineering phases.

## Phase 1 — Foundation (complete)

- [x] Tauri 2 desktop shell with design-token UI system
- [x] Theming (light / dark / system)
- [x] Typed IPC layer with structured `{ code, message }` errors
- [x] Backend-owned, validated, persisted settings
- [x] Platform abstraction (OS identity, app directories)
- [x] Filesystem foundation (path normalization, directory validation, metadata)
- [x] Drive-enumeration foundation (storage roots the host exposes)
- [x] Native folder picker hosted in Rust
- [x] Windows-safe production logging with file rotation
- [x] Restrictive CSP and a minimal Tauri capability set
- [x] Test infrastructure (Vitest, cargo test) and CI
- [x] Repository hygiene and accurate documentation

## Phase 2 — Volumes and filesystem browsing (complete)

- [x] Typed volume model (kind, filesystem, capacity, read-only, mounted status)
- [x] Volume classification and capacity probing through the Win32 volume APIs
- [x] Directory listing command with entry metadata, deterministic ordering, and
      a listing limit for oversized folders
- [x] Safe navigation: path validation in Rust, backend-reported parents, and no
      symlink following during enumeration
- [x] Storage browser UI: volume rail with metadata, entry table, back / up /
      refresh / drive switching, and loading, empty, and recovery states
- [x] Test coverage for the volume model, directory listing, browser navigation,
      and the browser surface

## Phase 3 — Transfers (complete)

- [x] Copy / move engine with progress, pause, resume, and cancellation
- [x] Transfer queue with stable ordering and one active job at a time
- [x] Conflict resolution with Replace / Skip / Rename, and safe partial-output
      cleanup, including created-directory removal on cancel
- [x] Typed transfer IPC, `transfer:update` progress events, and a queue surface
      with pause, resume, cancel, and issue reporting
- [x] Safety rules: no transfer into itself, no destination inside its source,
      no symlink following, free-space checks before a copy

Phase 3 shipped without verification, persisted state, or history; Phase 4 added
all three. What the engine deliberately does not do is in
`docs/architecture/TRANSFER_ENGINE.md`.

## Phase 4 — Verification, recovery, and history (complete)

- [x] Post-transfer verification with three policies: size (default), SHA-256
      checksum, or none. Checksums compare the digest of the bytes read from the
      source against the file on disk; a mismatch fails its item, keeps the file
      it wrote, and says what was expected and what was found
- [x] Honest reporting: what was checked, what was not, and what the engine did
      not reapply (modified times and read-only attributes) travel with every
      verdict instead of being implied by a green badge
- [x] Durable documents: versioned, atomically written state and history, with
      damaged files preserved beside the original and a newer schema refused
      rather than overwritten
- [x] Crash recovery: live job state journaled while a job runs, interrupted
      jobs classified into five explicit outcomes, and restart / discard /
      confirm as the user's decision — never automatic
- [x] Transfer history persistence, per-record verification verdicts, filters,
      retention by count, and a details view
- [x] Recovery and history UI: a Recovery page with what a restart would do,
      a History page with status, sizes, durations, and verdicts, and sidebar
      counts for jobs awaiting a decision
- [x] Notifications for finished, failed, verification-failed, skipped, and
      interrupted transfers, plus recovery and history error journeys in
      `docs/architecture/ERROR_HANDLING.md`
- [x] Settings for verification policy and history retention, folded into a job
      when it is planned so a later change cannot alter queued work
- [x] Test coverage for the new modules on both sides, plus on-disk sanity runs

What Phase 4 deliberately does not do is in `docs/architecture/VERIFICATION.md`,
`docs/architecture/RECOVERY.md`, and `docs/architecture/PERSISTENCE.md`:
byte-offset resume of a partial file is refused, verification proves only the
claims it lists, metadata is still not reapplied, and history is a bounded,
pruned list rather than an audit log.

## Phase 5 — Complete product experience (complete)

- [x] End-to-end flow: drives → browse → select → destination → review → start →
      progress → pause/resume/cancel → verification → completion/failure →
      history → recovery, with a real state and a real next step at every step
- [x] File browser: a breadcrumb trail resolved by the backend, back / forward /
      up with buttons and `Alt`+arrow keys, roving focus over the entries,
      loading, empty, truncated, unreachable, and permission states, and a clear
      statement that the selection is the source
- [x] Transfer composer: source mapping per item, destination, operation, item
      and byte counts, free space, the conflict strategies with what each one
      does, the verification policy, and the safety warnings — all before
      anything is queued, with the backend still the only authority
- [x] Transfer queue: every status including `Verifying`, the verdict and what
      was not preserved, the issues, the controls that apply, and a link to a
      finished job's durable record; an empty queue offers the way to start one
- [x] History and recovery integrated: discoverable in the sidebar, a link from
      the queue, filters, readable details, explicit consequences for every
      recovery action, degraded documents surfaced instead of hidden, and
      interrupted work that can never read as completed
- [x] Notifications that lead somewhere: one per terminal transition, bounded,
      dismissible, assertively announced when they are errors, distinct for a
      verification failure, and carrying the action that opens the surface they
      are about
- [x] Keyboard and accessibility: a single navigation table driving the sidebar,
      the top bar, and `Ctrl`/`Cmd`+digit shortcuts; dialogs that take and return
      focus, answer to `Esc`, and label their buttons with consequences
- [x] Desktop behaviour: the close guard (Rust holds the close while work is in
      flight and says what it would set aside), a home page that reports the real
      state, and a not-found route instead of an empty frame
- [x] Settings grouped by what they change (Appearance, Region, Verification,
      History) with the backend still owning and validating every value
- [x] Visual consistency pass over typography, spacing, controls, status tones,
      empty/loading/error states, and terminology
- [x] Tests: journey tests across the real pages, plus unit tests for the new
      browser, queue, composer, notification, shortcut, and close-guard behaviour

What Phase 5 deliberately does not do is in `docs/design/COMPONENTS.md` and
`docs/design/ACCESSIBILITY.md`: the shortcut set stays small (nothing that
changes what is on disk is a keystroke), the composer reports the engine's plan
rather than predicting a different one, and the notification stack is a toast
surface, not a log.

## Phase 6 — Performance and Windows production (complete)

- [x] Performance measured on real Windows workloads and recorded:
      startup (settings plus archive), a 10,000-entry directory listing, a
      500-level tree plan, 2,000 small files, a 512 MiB streamed file, idle CPU,
      and the largest listing / history / queue the interface can render — see
      `docs/development/PERFORMANCE.md`
- [x] The one genuine interface problem the measurements found: the browser
      rendered every entry of every listing on every change, which cost 1.7 s to
      paint and 0.65 s per click at the 10,000-entry cap. It now renders a
      window of rows, states how much of the listing is shown, and memoizes the
      rows: 0.17 s and 0.006 s, with budgets enforced by tests
- [x] Transfer robustness on real Windows volumes: nested trees, a move, all
      three conflict strategies, pause/resume/cancel cleanliness, streaming
      under a memory ceiling, queue order, verification, and — new — a copy and
      a move between two real volumes, skipped with a printed reason on a
      single-volume host
- [x] Startup and shutdown exercised across the lifecycle: clean start, start
      after successful work, start with interrupted work, start with a damaged
      document, close with nothing in flight, close with work in flight, and
      restart after a recovery decision
- [x] Production build hardened: publisher, copyright, descriptions, license,
      category, current-user NSIS with a language and Start-menu folder, WiX
      MSI, WebView2 bootstrapper, downgrades refused, and unused plugin
      commands stripped from the binary; the shipped executable is
      `CrossPort.exe` rather than cargo's `crossport.exe`; release profile
      pinned with thin LTO and no debug info (rustc 1.97.1 crashes inside a
      fat-LTO link of this crate on Windows, reproducibly — see
      `apps/desktop/src-tauri/Cargo.toml`)
- [x] Real artifacts built (NSIS installer and MSI), checked for version
      consistency and hashed into `bundle/checksums.txt` by `scripts/release.sh`
- [x] Clean-environment verification: the built executable starts from an
      isolated directory with no Node, pnpm, Cargo, repository, or development
      environment variable in sight, writes its startup line, creates a visible
      window, and exits cleanly when the window is closed
- [x] Install, upgrade, and uninstall exercised against the built installer,
      including what happens to user data across each step: it installs into
      `%LOCALAPPDATA%\CrossPort` with a Start-menu entry, a desktop shortcut,
      and an entry in **Apps → Installed apps**; reinstalling the same version
      replaces the program files; uninstalling removes the program, the
      shortcuts, and the registry entry and leaves user data alone
- [x] Logging reviewed for production: startup context, application
      directories, archive health, and job identifiers — never the user's file
      paths. A startup failure or a panic now shows a message box with a
      matching log entry instead of a window that never appears
- [x] CI gained a Windows job for the Rust suite, and the release process,
      checklist, and documentation were corrected where they described
      workflows that did not exist

What Phase 6 deliberately does not do: no telemetry, no update checker, no code
signing (there is no certificate in this repository), no installer publishing,
and no V2 features. The remaining limits are written down in `README.md` under
"Known limitations".

## Phase 7 — Security and release hardening (complete)

- [x] Security boundary reviewed end to end: typed IPC → Tauri commands → Rust
      domain → filesystem → persistence/logging. Every security-relevant
      decision stays in Rust; the webview holds only `core:default`
- [x] Path handling reviewed: absolute-only normalization, null-byte and
      `..`-escape rejection, lexical containment, destination-inside-source and
      source-inside-destination refusal, and reparse points reported but never
      planned through, copied, or removed
- [x] Persistence and recovery reviewed: atomic unique-temp writes with fsync,
      schema versioning with newer-schema refusal, damaged documents preserved
      (`*.corrupt-<ms>.*`), history written before live state is forgotten, and
      artifact matching by exact job-owned name
- [x] Temporary artifacts reviewed: collision-safe names carrying the job
      identifier, cleanup bounded to CrossPort-owned artifacts, and no false
      completion — an unverified operation is never reported as successful
- [x] Two hardening fixes: the recovery artifact scan now uses
      `platform::is_reparse_point` (a Windows junction is not reported by
      `FileType::is_symlink`, so the old check could descend one) and settings
      writes use a unique temp name and `sync_all` like the document layer
- [x] Dependency and capability surface reviewed: no HTTP client in the Windows
      build, no fs/dialog/shell permission for the webview, no network access,
      no telemetry, no updater, no secrets
- [x] Release correctness: version consistency enforced across all four
      manifests, README/FAQ/versioning/template corrected to describe the
      product that exists, and the misleading cross-platform release workflow
      removed
- [x] The full gate re-run green (frontend lint/check/test/build, Rust
      fmt/check/clippy/test, version sync), the production release built, and
      both installers checksummed and verified
- [x] `docs/release/RELEASE_1.0.md`: the production release document

What Phase 7 deliberately did not do is add any feature. The next step is the
1.0 freeze, which bumps the version to `1.0.0` and records the completed
baseline. The interactive install/upgrade/uninstall pass recorded under Phase 6
was not repeated in this pass (see `docs/release/RELEASE_1.0.md`).

## Version 2 candidates

Not started. Candidates from `docs/product/FEATURE_SPECIFICATION.md`: folder
synchronization, watched folders, batch operations, and advanced transfer rules.
Keyboard shortcuts and the product-experience work they belonged to shipped in
Phase 5.

See `docs/product/FEATURE_SPECIFICATION.md` for the full feature breakdown
through the 2030 roadmap.
