# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
- CI pipeline (lint, biome, build, frontend/backend tests, version sync) and a
  release workflow.
- `windows-sys` (Windows-only) for drive-letter enumeration.
- Repository hygiene: root README, MIT license, changelog, contributor and
  security docs, editor configs, and operational scripts.

### Changed

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

- Empty placeholder workspace packages (`packages/*`), placeholder Rust
  commands/modules/models that returned `NotImplemented`, dead frontend stores
  and types, and empty documentation stubs.
