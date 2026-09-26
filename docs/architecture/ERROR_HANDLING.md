# CrossPort Error Handling

Version: 3.0
Status: Approved

Errors must help users solve problems. An error is a communication point, not
just a technical failure.

## User-facing principle

Every user-facing error should answer:

1. What happened?
2. Why did it happen?
3. What can the user do?

## Structured backend errors (implemented)

All backend commands return `AppResult<T>`. Errors cross the IPC boundary as a
structured object:

```
{ "code": "path_not_found", "message": "path not found: D:\\Media" }
```

- `code` — stable machine-readable identifier. The frontend switches on it and
  never string-matches `message`.
- `message` — human-readable detail for display, logging, and bug reports.

### Backend codes

| Code | Meaning |
| --- | --- |
| `invalid_input` | Input failed backend validation (empty, relative, escapes root, unknown theme) |
| `path_not_found` | The path does not exist |
| `path_not_directory` | The path exists but is not a directory |
| `permission_denied` | The OS refused access |
| `io` | A platform/filesystem operation failed for another reason |
| `unsafe_relationship` | A transfer was refused because the destination is its own source or lives inside it (with `rename`, a destination equal to the source becomes a sibling instead) |
| `not_enough_space` | The destination volume cannot hold the planned transfer |
| `disk_full` | The destination ran out of space while data was being written |
| `too_many_items` | A transfer covers more entries than the engine will plan (100,000) |
| `transfer_not_found` | No transfer job with that identifier is known to the engine |
| `transfer_failed` | A job finished without completing every item |
| `internal` | An internal failure (resolved directory, poisoned lock, dialog failure) |

`std::io::Error` is mapped onto these categories, so callers get the closest
meaningful code instead of a bare message.

### Frontend codes

| Code | Meaning |
| --- | --- |
| `invalid_response` | The backend replied with a payload that breaks the published schema |
| `unavailable` | The command only exists in the desktop app and we are in a browser |
| `unknown` | No structured information; `message` carries the detail |

The frontend type is `IpcError` (`src/services/ipc.ts`) and carries `code`,
`message`, and — when the backend reports a code outside the published contract
— the original code in `unsupportedCode` so diagnostics lose nothing.

The frontend mirrors the backend code list in `BACKEND_ERROR_CODES`. Drift
between the two is a bug: the Rust list is covered by
`errors::tests::codes_are_stable_identifiers` and the TypeScript list by
`src/services/ipc.test.ts`.

## Recovery

- Corrupt settings file → logged warning, defaults used, rewritten on next save.
- Backend unreachable during hydration → defaults used, error kept in the store
  and shown where the user can act on it.
- Drive enumeration failure → the drives page shows the backend message and a
  retry action.
- Cancelled native dialog → `null`, not an error.
- Rejected transfer request → the reason is shown where the request was made
  (missing source, destination inside its source, not enough room, too many
  items) and nothing is queued.
- Per-item transfer failure → recorded as an `issue` on the job, the job keeps
  going, and the job ends as `failed` only after the rest was attempted. A
  failed move keeps the source in place.
- A cancel discards the job's partial output and removes only the directories
  the job created, so an interrupted transfer never leaves a half-written file
  under a real name.
- No silent failures anywhere: every failure path logs or surfaces.

## Logging

Backend logs go to stdout and to a rotating per-app log file (5 MiB × 3) in the
platform log directory, so release builds on Windows — which have no console —
remain diagnosable. Startup logs the version, platform, loaded preferences, and
the resolved config/log directories.

## Principle

A good error message reduces frustration. A bad error message creates more
problems.
