# Persistence Architecture

Version: 1.0
Status: Approved

## Purpose

CrossPort keeps three small documents on disk: user settings, the state of jobs
that are in flight, and the history of jobs that finished. This document covers
how any of them is written and read, and what each one stores.

There is no database. The application's durable data is small, human-readable
state, and a document layer enforces the properties that matter — atomicity,
versioning, and honest handling of damage — once, instead of leaving every
caller to get them right.

## Where the documents live

| Document | File | Written by |
| --- | --- | --- |
| Settings | `settings.json` | `settings/mod.rs` (same temp-file-then-rename rule, validated before it is written) |
| In-flight job state | `transfer-state.json` | `recovery/mod.rs` (`TransferStateStore`) |
| Finished job history | `transfer-history.json` | `history/mod.rs` (`HistoryStore`) |

All three live in the platform app-config directory resolved by
`platform::AppPaths::config_dir` and are opened during startup. The settings
document predates this layer; the two transfer documents go through
`persistence/mod.rs`.

## The rules the document layer enforces

`persistence/mod.rs` is the only code that touches a transfer document.

- **A write replaces a file or leaves the old one intact.** The value is
  serialized, written to a sibling temporary file, flushed to the device, and
  only then renamed over the live path. A crash at any point leaves either the
  previous document or the new one — never a half-written one.
- **Temporary names are unique per writer.** The temp name carries the process
  id and a counter, so two writers (or two processes) cannot land on the same
  temp path and lose a write.
- **Every document carries `schemaVersion`.** The current version is 1
  (`persistence::SCHEMA_VERSION`).
- **Reading an older document is a migration.** Absent fields are filled from
  the struct defaults and unknown fields are ignored, so a document written by
  another build still loads; the load is reported as `migrated`.
- **Reading a newer document is refused.** Its contents are unknown and must not
  be overwritten, so it is reported as `unsupported` and left untouched. The
  archive stays readable but read-only for that document.
- **A missing file is normal.** It yields the caller's default and is reported
  as `missing`.
- **A damaged file is preserved, not discarded.** An empty, malformed, or
  unreadable document is copied aside as `<name>.corrupt-<ms>.<ext>`, reported
  as `recovered`, and replaced by the default so the application still starts.

### Load states

`LoadStatus` (Rust) is the internal outcome; `LoadState` is what the frontend
receives:

| State | Meaning | Writable |
| --- | --- | --- |
| `loaded` | Read and parsed as the current schema | Yes |
| `missing` | No file yet; defaults apply | Yes |
| `migrated` | Written by an older schema and migrated on read | Yes |
| `recovered` | Unusable, preserved for diagnostics, defaults used | Yes |
| `unsupported` | Written by a newer schema | **No** |

## Using it from a store

Both `HistoryStore` and `TransferStateStore` are cheap
read-modify-write wrappers with a mutex and an atomic document underneath. Two
rules from that design are worth stating because they are load-bearing:

- **Writes happen while the store's lock is held.** History is ordered and
  append-only; writing outside the lock would let two writers read the same
  document and one silently win.
- **A failed write is reported, not swallowed.** Each store keeps the last write
  error so a caller can surface it; the archive turns that into a degraded
  document status rather than pretending the data is safe.

## What the history document stores

One record per finished job, plus one per interrupted job that recovery
accounted for.

- Identity and intent: job identifier, operation, conflict strategy, sources,
  destination.
- Outcome: `completed`, `failed`, `cancelled`, `interrupted` (the application
  stopped before the job proved it finished), or `recovered` (an interrupted job
  the archive proved had finished).
- Numbers: planned and transferred bytes, planned and completed files and
  directories, skipped and failed items.
- Timing: queued, started, and finished timestamps, and `durationMs` for time
  spent working (paused time excluded).
- Failure detail: the job-level error in storable form, a bounded sample of the
  job's issues (40, with a truncation flag), and
  `issuesTruncated`.
- Verification: status, method, policy, algorithm, counts, verified bytes, and
  the backend-rendered verdict.
- Recovery: the action taken (`pending`, `discard`, `restart`, `confirm`) and
  the interrupted job a restart came from.

Records are validated before they are written: a record without an identifier,
a source, or a destination is refused, because it could not be displayed
meaningfully.

### Retention

Retention is by count, configured as `historyLimit` in settings (default 200,
accepted range 20–2000, see `docs/architecture/SETTINGS.md`).

- The **newest** records are kept and the oldest are pruned — never the other
  way round.
- A prune is reported back to the caller (`PruneOutcome`), so a trim is logged
  rather than discovered later.
- Lowering the limit prunes immediately and writes the trimmed list, so what is
  on disk matches what the application will read back. A document that cannot be
  written (newer schema) is left alone and keeps its own contents.
- A limit outside the accepted range is clamped rather than obeyed, and a zero
  limit cannot empty the history.

### Filters

History can be requested as `all`, `completed`, `failed`, `cancelled`, or
`interrupted` (interrupted and recovered entries — what recovery accounted for).
The filter is a closed set on both sides of the IPC boundary.

## Degraded operation

An application that cannot read its own history still has to start. When a
document loads as `recovered` or `unsupported`, the archive reports it through
`get_archive_status` and the History page says which document is degraded, why,
and whether it can be written to — instead of showing an empty list that looks
like "nothing ever happened".

## Tests

`persistence/mod.rs` tests cover the atomic write and the absence of the temp
file afterwards, corrupt and empty documents being preserved beside the
original, older-schema migration, newer-schema refusal, and unique temporary
names. `history/mod.rs` and `recovery/mod.rs` tests cover ordering, retention,
filters, update and delete, and the write ordering that keeps a finished job
from ever looking unfinished.
