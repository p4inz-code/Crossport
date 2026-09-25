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

## Platform and drives flow (implemented)

```
Drives page mounts
    ↓
drives store → invoke("list_drives")
    ↓
Rust: platform::drives enumerates candidate roots and keeps the readable ones
    ↓
UI renders roots and labels
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

## Path inspection flow (implemented)

```
User clicks "Browse for a folder…"
    ↓
invoke("pick_directory") → Rust opens the native dialog
    ↓
Rust normalizes the selection and asserts it is a directory
    ↓
invoke("inspect_path", { path }) → Rust returns metadata
    ↓
UI renders the path, kind, size, and modification time
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
| Drive list | Rust backend | Derived per request; UI keeps the last result |
| Path metadata | Rust backend | Derived per request; never cached |
| App metadata (name, version) | Frontend | In-memory store |
| Theme resolution | Frontend (`useThemeMode`) | Derived from settings + OS preference |

## Forbidden flows

- UI components modifying storage directly.
- Frontend filesystem, drive, or dialog access.
- Components importing stores from sibling feature folders.
