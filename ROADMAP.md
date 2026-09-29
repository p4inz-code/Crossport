# Roadmap

CrossPort's long-term vision is in [`docs/product/VISION.md`](docs/product/VISION.md);
the feature plan is in
[`docs/product/FEATURE_SPECIFICATION.md`](docs/product/FEATURE_SPECIFICATION.md).
This page tracks the engineering direction and the phases already completed.

Directions are not commitments, and no dates are promised.

## NOW — CrossPort 1.1.x (shipped)

The Windows product, complete and packaged.

- Copy and move files and folders between mounted volumes, with a reviewed plan
  before anything is written.
- Conflict handling: Replace / Skip / Rename.
- Post-transfer verification: size (default) or SHA-256, with honest verdicts
  about what was and was not checked.
- A single-job queue with live progress, pause, resume, and cancel.
- Durable, bounded transfer history and crash recovery with explicit outcomes.
- A responsive interface, light/dark/system themes, notifications, and keyboard
  navigation.
- Windows installers (NSIS + WiX MSI) built locally with SHA-256 checksums.

The current release is **1.1.1**; see
[`docs/release/RELEASE_1.1.1.md`](docs/release/RELEASE_1.1.1.md). The **1.0.0**
baseline remains frozen and untouched.

## NEXT — realistic engineering work on the Windows product

Work that strengthens the product that exists rather than widening it. None of
these is started unless noted.

- Broader verification coverage, including optional metadata reapplication
  (modified times, read-only attributes), always stated in the verdict.
- Queue ergonomics that do not change what lands on disk.
- Packaging polish: installer hardening and a documented, reproducible release
  procedure.
- Continued performance measurement against real Windows workloads.

## FUTURE / PLATFORM-SPECIFIC

Capabilities that need platform builds, signing, or deeper OS integration.
None of this exists yet.

- **Linux and macOS production packaging and QA.** The architecture is
  platform-neutral — one `platform` module isolates every OS fact — but no
  bundle is produced and no platform QA has run. See
  [`docs/product/PLATFORM_SUPPORT.md`](docs/product/PLATFORM_SUPPORT.md).
- **Platform code signing.** There is no certificate in this repository, so
  installers are unsigned and Windows warns about an unknown publisher.
- **An update path.** There is no update checker and no update server today.
- **Version 2 feature candidates** (folder synchronization, watched folders,
  batch operations, advanced transfer rules). Candidates, not promises; see
  [`docs/product/FEATURE_SPECIFICATION.md`](docs/product/FEATURE_SPECIFICATION.md).

**Explicitly out of scope:** filesystem-driver functionality — mounting,
unmounting, formatting, repairing volumes, and native NTFS write access beyond
what the OS mounts. These require a signed driver or a privileged helper and are
recorded as out of scope in
[`docs/product/PARAGON_CAPABILITY_GAP.md`](docs/product/PARAGON_CAPABILITY_GAP.md)
and [`NON_GOALS.md`](NON_GOALS.md).

---

## Completed phases

The engineering record. Every phase below is finished and its work is in the
shipped product.

### Phase 1 — Foundation (complete)

- [x] Tauri 2 desktop shell with a design-token UI system, theming (light /
      dark / system), and a typed IPC layer with structured `{ code, message }`
      errors
- [x] Backend-owned, validated, persisted settings; platform abstraction (OS
      identity, app directories); filesystem foundation (path normalization,
      directory validation, metadata)
- [x] Drive-enumeration foundation, native folder picker hosted in Rust,
      Windows-safe logging with rotation, a restrictive CSP and a minimal
      Tauri capability set
- [x] Test infrastructure (Vitest, `cargo test`), CI, and repository hygiene

### Phase 2 — Volumes and filesystem browsing (complete)

- [x] Typed volume model (kind, filesystem, capacity, read-only, mounted) and
      classification through the Win32 volume APIs
- [x] Directory listing with entry metadata, deterministic ordering, and a
      listing limit; safe navigation with path validation in Rust and no
      symlink following
- [x] Storage browser UI: volume rail, entry table, back / up / refresh / drive
      switching, and loading, empty, and recovery states

### Phase 3 — Transfers (complete)

- [x] Copy / move engine with progress, pause, resume, and cancellation
- [x] Transfer queue with stable ordering and one active job at a time
- [x] Conflict resolution with Replace / Skip / Rename, and safe partial-output
      cleanup
- [x] Safety rules: no transfer into itself, no destination inside its source,
      no symlink following, free-space checks before a copy

### Phase 4 — Verification, recovery, and history (complete)

- [x] Post-transfer verification with three policies (size, SHA-256, none), and
      honest reporting of what was and was not checked
- [x] Durable, versioned, atomically written state and history, with damaged
      files preserved beside the original
- [x] Crash recovery: journaled job state, five explicit outcomes, and
      restart / discard / confirm as the user's decision — never automatic
- [x] Transfer history with per-record verdicts, filters, retention, and a
      details view; recovery and history UI and notifications

### Phase 5 — Complete product experience (complete)

- [x] End-to-end flow: drives → browse → select → destination → review → start →
      progress → pause/resume/cancel → verification → completion/failure →
      history → recovery
- [x] File browser with breadcrumbs and keyboard navigation; a transfer composer
      that reports the backend's plan before anything is queued; a queue surface
      with verdicts, issues, controls, and durable-record links
- [x] Notifications that lead somewhere; keyboard and accessibility pass;
      desktop behaviour including the close guard; settings grouped by effect;
      a visual consistency pass over the whole interface

### Phase 6 — Performance and Windows production (complete)

- [x] Performance measured on real Windows workloads and recorded
      ([`docs/development/PERFORMANCE.md`](docs/development/PERFORMANCE.md))
- [x] The browser's 10,000-entry rendering cost fixed (1.7 s → 0.17 s paint);
      budgets enforced by tests
- [x] Transfer robustness on real volumes; the full startup/shutdown lifecycle
- [x] Production build hardened: publisher, copyright, descriptions, license,
      current-user NSIS, WiX MSI, WebView2 bootstrapper, unused commands
      stripped, and the executable named `CrossPort.exe`
- [x] Real NSIS and MSI artifacts built and hashed; clean-environment launch
      verified; install / reinstall / uninstall exercised (including data
      preservation)
- [x] Logging reviewed for production; a startup failure or panic shows a
      message box with a matching log entry

### Phase 7 — Security and release hardening (complete)

- [x] Security boundary reviewed end to end: typed IPC → Tauri commands → Rust
      domain → filesystem → persistence/logging. The webview holds only
      `core:default`
- [x] Path handling reviewed: absolute-only normalization, null-byte and
      `..`-escape rejection, lexical containment, destination-inside-source
      refusal, and reparse points reported but never planned through
- [x] Persistence and recovery reviewed: atomic writes, schema versioning,
      damaged-document preservation, and no false completion
- [x] Dependency and capability surface reviewed: no HTTP client, no network
      access, no telemetry, no updater, no secrets
- [x] The full gate re-run green and both installers checksummed and verified

### Release 1.0 — production baseline (complete, frozen)

- [x] Version bumped to `1.0.0` across all four manifests and `Cargo.lock`
- [x] The full gate re-run green; production artifacts rebuilt with SHA-256
      checksums; documentation brought in step with the release
- [x] Annotated tag `v1.0.0` recording the baseline

The 1.0 development baseline is **frozen**. Only critical security fixes,
critical production bugs, and release-blocking corrections may change the
`1.0.x` line. See [`VERSIONING.md`](VERSIONING.md).

### Release 1.1 — product and platform polish (complete)

- [x] Responsive page widths and a Drives layout that gives the browser the
      rest of a wide window
- [x] One finalized logo (`assets/brand/crossport-logo.svg`), with the sidebar
      tile, favicon, and packaged icons derived from it
- [x] [`docs/product/PLATFORM_SUPPORT.md`](docs/product/PLATFORM_SUPPORT.md) and
      [`docs/product/PARAGON_CAPABILITY_GAP.md`](docs/product/PARAGON_CAPABILITY_GAP.md)
- [x] The whole interface drawn from the shared design tokens
- [x] Annotated tag `v1.1.0`

### Release 1.1.1 — corrective release (current)

- [x] Fixed a wire-contract defect in the live transfer snapshot: the rendered
      verification verdict is now serialized with the job, so the queue renders
      and a transfer can be started from the interface
- [x] A regression test asserts the serialized summary carries its verdict
- [x] Windows installers rebuilt and checksummed; annotated tag `v1.1.1`

The `v1.0.0` and `v1.1.0` tags and their artifacts are untouched.
