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

What Phase 3 deliberately does not do is in
`docs/architecture/TRANSFER_ENGINE.md` — byte-count rounding is the only
verification, jobs live in memory, and metadata is not reapplied.

## Phase 4 — Verification & polish

- [ ] Optional post-transfer verification (size, metadata, checksum)
- [ ] Crash recovery: resuming an interrupted transfer from persisted state
- [ ] Transfer history persistence and retention policy
- [ ] Notifications and error journeys per `docs/architecture/ERROR_HANDLING.md`

See `docs/product/FEATURE_SPECIFICATION.md` for the full feature breakdown
through the 2030 roadmap.
