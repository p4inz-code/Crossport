# CrossPort Data Flow

Version: 3.0
Status: Approved

This document defines how information moves through CrossPort today.

## General flow

```
User Action
    ↓
UI Layer (React components)
    ↓
Zustand stores
    ↓
Services (typed IPC / validated storage)
    ↓
Rust command layer
    ↓
Domain modules (settings, platform, filesystem)
    ↓
Operating system
```

Every backend call goes through `src/services/ipc.ts`, which invokes the
command, validates the payload against the matching zod schema, and normalizes
failures into `IpcError`.

## Settings flow (implemented)

```
User changes a setting (theme, locale)
    ↓
settings store (Zustand) validates locally and updates immediately
    ↓
settings service validates with zod
    ↓
Tauri: invoke("update_settings", { settings })
    ↓
Rust: validate → persist JSON to app-config directory → update managed state
```

On startup:

```
Rust setup loads settings.json (defaults when missing; serde defaults fill
fields written by other versions)
    ↓
store hydrates via invoke("get_settings")
    ↓
UI reflects the loaded settings
```

Browser dev mode substitutes the Tauri steps with schema-validated
localStorage reads/writes, keeping the store and UI code path identical.

## Platform and volume flow (implemented)

```
Drives page mounts
    ↓
drives store → invoke("list_drives")
    ↓
Rust: platform::drives enumerates candidate roots on the blocking pool,
      probes each one (kind, volume name, filesystem, capacity, read-only,
      mounted) and keeps the volumes that exist
    ↓
UI renders each volume with its capacity, kind, and status
```

```
StoreProvider mounts
    ↓
system store → invoke("get_system_info")
    ↓
Rust: platform::SystemInfo (from std constants, resolved once at startup)
    ↓
Status bar and home page show the real host facts
```

## Directory browsing flow (implemented)

```
User clicks a volume, a folder row, Back, Up, or Refresh
    ↓
browser store picks a path — a volume root, an entry path the backend
reported, or the parent the backend reported (never one the UI built)
    ↓
invoke("list_directory", { path })
    ↓
Rust: normalize → must_be_directory → list one directory (no recursion, no
      link following) on the blocking pool
    ↓
UI adopts the normalized path from the response and renders the entries
```

The store drops the previous entries while a new location loads, ignores a
listing that arrives after a newer navigation started, and clears the entries
when a listing fails — a disconnected volume shows a structured error with
retry and "back to volumes" instead of stale content.

## Path inspection flow (implemented)

```
invoke("inspect_path", { path }) → Rust normalizes the path, validates it,
and returns name, kind, size, modification time, and read-only flag
```

`inspect_path` is the single-path primitive later milestones use (transfer
sources and destinations); browsing goes through `list_directory`.

## Native folder picker flow (implemented)

```
User clicks "Open folder…"
    ↓
invoke("pick_directory") → Rust opens the native dialog on its own thread
    ↓
Rust normalizes the selection and asserts it is a directory
    ↓
invoke("list_directory", { path }) opens it in the browser
```

Cancelling the dialog resolves with `null`, which is not an error.

## Error flow

- Every `invoke` rejection is normalized to `IpcError` carrying the backend
  error `code`; unknown codes are kept in `unsupportedCode`.
- A payload that does not match the schema becomes `invalid_response` rather
  than corrupting state.
- Failures are logged and surfaced in the UI; none are swallowed.

## Data ownership

| Data | Owner | Storage |
| --- | --- | --- |
| Settings | Rust backend | `settings.json` in the app-config directory |
| Platform facts | Rust backend | Derived at startup, cached in `AppState` |
| Volume list | Rust backend | Derived per request; UI keeps the last result |
| Path metadata | Rust backend | Derived per request; never cached |
| Directory listing | Rust backend | Derived per request; the browser keeps only the current location |
| Navigation history | Frontend browser store | In-memory stack of locations the backend produced |
| App metadata (name, version) | Frontend | In-memory store |
| Theme resolution | Frontend (`useThemeMode`) | Derived from settings + OS preference |

## Forbidden flows

- UI components modifying storage directly.
- Frontend filesystem, drive, or dialog access.
- Components importing stores from sibling feature folders.
