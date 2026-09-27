# CrossPort Data Flow

Version: 4.0
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

## Transfer flow (implemented)

```
User selects entries and chooses Copy to… / Move to…
    ↓
invoke("plan_transfer", { request }) → Rust plans the whole job (item budget,
      collisions resolved, free space checked) and returns a preview
    ↓
the composer shows what would move, what collides, and how much room is left
    ↓
invoke("start_transfer", { request }) → Rust queues the job and returns a snapshot
    ↓
engine runs the job one item at a time, publishing throttled "transfer:update"
      events carrying a typed snapshot
    ↓
transfer store merges each snapshot by job identifier; the queue renders it
```

Every number the UI shows — sizes, speeds, ETAs, collisions — comes from the
backend. Pause, resume, cancel, and remove are commands; cancel discards partial
output and asks first.

## Verification flow (implemented)

```
The job's verification policy (from settings, folded into the request) reaches
      the engine with the plan
    ↓
an item finishes writing: the temporary file is renamed onto its destination
    ↓
verification compares what the plan measured and the bytes streamed out of the
      source against what is on disk (size, and SHA-256 under the checksum policy)
    ↓
the job's activity becomes "verifying" while a file is checked; the result is
      recorded on the job and published with the next "transfer:update"
    ↓
a mismatch fails that item (and the job) with verification_failed, keeps the
      file it wrote, and reports what was expected and what was found
```

The job's summary states what was checked, what was not (`unverifiedFiles`), and
that modified times and read-only attributes were not reapplied. See
`VERIFICATION.md`.

## Durable state, history, and recovery flow (implemented)

```
job state changes and throttled progress (every 2 s)
    ↓
engine publishes to the archive through the journal port
    ↓
archive writes transfer-state.json through the atomic, versioned document layer

job reaches a terminal status
    ↓
archive writes its history record first, then forgets the live state

the application starts again
    ↓
setup opens the archive, reads both documents, and reports each one's load
      state (loaded / missing / migrated / recovered / unsupported)
    ↓
recovery classifies every interrupted job (completed_before_crash,
      restart_required, source_missing, destination_unavailable, unsupported)
    ↓
the shell announces it once; the Recovery page offers Discard / Restart / Confirm
    ↓
invoke("recover_transfer", { id, action }) → leftovers removed, request queued
      again, or nothing run at all, and the episode is recorded in history
```

Reaching the document directly is impossible from the UI: `get_archive_status`,
`list_history`, and the recovery commands are the only way in, and they report a
degraded document instead of an empty list. See `PERSISTENCE.md` and
`RECOVERY.md`.

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
| Transfer jobs (live) | Rust engine | In memory, with throttled snapshots published over `transfer:update` |
| Transfer state (in flight) | Rust archive | `transfer-state.json` in the app-config directory, replaced atomically while a job runs |
| Transfer history | Rust archive | `transfer-history.json` in the app-config directory, written on completion and pruned to the configured limit |
| Verification verdicts | Rust engine, stored by the archive | Carried in the snapshot and in each history record |
| Notifications | Frontend notification store | In memory, bounded, raised once per terminal transition |
| Navigation history | Frontend browser store | In-memory stack of locations the backend produced |
| App metadata (name, version) | Frontend | In-memory store |
| Theme resolution | Frontend (`useThemeMode`) | Derived from settings + OS preference |

## Forbidden flows

- UI components modifying storage directly.
- Frontend filesystem, drive, or dialog access.
- Components importing stores from sibling feature folders.
