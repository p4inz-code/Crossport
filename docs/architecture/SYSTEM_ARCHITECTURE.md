# CrossPort System Architecture

Version: 3.0
Status: Approved

## Purpose

CrossPort is a cross-platform desktop file transfer utility built as a single
Tauri 2 application. This document describes the architecture that exists
today and the principles that keep it stable.

## Stack

- **Shell:** Tauri 2 (Rust backend + system webview)
- **Frontend:** React 19, TypeScript (strict), Vite, React Router, Zustand, Zod
- **Backend:** Rust with typed Tauri commands
- **Quality:** Vitest (frontend), cargo test (backend), Biome, ESLint, CI

## Layers

```
React UI (features, layouts, components)
        ↓  stores (Zustand)
        ↓  services (typed IPC / validated storage)
        ↓  [IPC: structured JSON commands]
Rust command layer (settings, system, drives, filesystem, dialog)
        ↓
Rust domain modules (settings, platform volumes, filesystem)
        ↓
Operating system
```

Each layer communicates only with the layer beneath it. The webview has no
direct filesystem or config access; all privileged work happens in Rust.
The platform abstraction (`platform/`) is the single module that knows about the
host OS: it resolves application directories through Tauri's public path
resolver, reports OS identity, and detects and classifies storage volumes.

## Key decisions

### Settings ownership (backend)

User preferences are owned by the Rust backend, persisted to a JSON file in the
platform app-config directory, and exposed via `get_settings` /
`update_settings`. Inputs are validated on both sides of the boundary. The
frontend uses a schema-validated localStorage fallback only when running in a
plain browser during development. See `docs/architecture/SETTINGS.md`.

### Volume metadata and browsing (backend)

Volumes are detected in Rust and described with what the platform can actually
report: kind, volume name, filesystem, total/free/used capacity, read-only
flag, and mounted status. On Windows the probing uses the Win32 volume APIs
(`GetDriveTypeW`, `GetDiskFreeSpaceExW`, `GetVolumeInformationW`); other targets
keep mount-point discovery and report the same fields as unknown. Facts the
platform does not report stay unknown rather than being inferred.

Directory browsing lists exactly one directory per request and never recurses.
The path is validated in Rust (absolute, no null bytes, no root escape, must be
an existing directory) before anything is read, symlinks and reparse points are
reported but not followed, and the listing is capped so a huge folder cannot
exhaust memory. Volume probing and every filesystem command run on the blocking
pool, so the event loop stays responsive while the OS walks a directory or
queries a slow volume.

### Structured errors

All commands return `AppResult<T>`. Errors serialize to
`{ "code": "...", "message": "..." }` with stable codes, so the frontend maps
codes to typed errors instead of parsing prose. See
`docs/architecture/ERROR_HANDLING.md`.

### Minimal capability surface

The frontend is granted only `core:default`
(`src-tauri/capabilities/default.json`). Because filesystem, drive, and dialog
work lives in Rust behind CrossPort's own commands, no plugin permissions are
granted to the webview, and new privileged features do not widen its attack
surface.

### Security

A restrictive Content Security Policy is enforced in production; a separate dev
CSP permits Vite HMR. See `SECURITY.md`.

## Extensibility

New capabilities are introduced by adding a Rust domain module + command
surface and a matching frontend feature folder — never by widening existing
modules. The roadmap in `ROADMAP.md` describes the planned phases.

## Non-Goals

See `NON_GOALS.md` and `docs/product/FEATURE_SPECIFICATION.md`.
