> **Status: implemented (Phases 3 and 4).** Copy/move, the queue, real progress,
> pause/resume/cancel, conflict resolution, and the safety rules below exist
> under `src-tauri/src/transfer/`. Post-transfer verification, durable state,
> transfer history, and crash recovery were added in Phase 4 and are documented
> in `VERIFICATION.md`, `PERSISTENCE.md`, and `RECOVERY.md`.
# CrossPort Transfer Engine

Version: 1.0

Status: Approved

---

# Purpose

The Transfer Engine is responsible for moving data safely between locations.

It is the core operational system of CrossPort.

---

# Implementation

The engine is a plain Rust module that never depends on the Tauri windowing
layer. Progress leaves it through the `TransferPublisher` trait, which the app
implements with an event emitter, so the engine is testable without a window and
its tests link no GUI libraries.

| Concern | Where it lives |
| --- | --- |
| Wire contract (request, snapshot, preview, identifiers) | `transfer/model.rs`, mirrored by `src/types/transfer.ts` |
| Planning, sizing, item budget, free-space check | `transfer/plan.rs` |
| Source/destination validation, safe cleanup | `transfer/safety.rs` |
| Conflict strategies | `transfer/conflict.rs` |
| Streaming, temp-file commits, move semantics, per-item verification | `transfer/copy.rs` |
| Queue, workers, identifiers, published snapshots | `transfer/mod.rs` |
| Verification policies, verdicts, streaming SHA-256 | `verification/mod.rs`, `verification/hash.rs` |
| Durable state, history, and recovery | `archive.rs`, `recovery/`, `history/`, `persistence/` |
| IPC commands and `blocking` pool offload | `commands/transfer.rs`, `commands/history.rs`, `commands/recovery.rs` |

Facts worth knowing when changing it:

- **Nothing is written under a real name until it is complete.** Every file
  streams into `.crossport-<job>-<index>.partial` beside its destination and is
  committed with a rename. A cancelled or failed job discards its partial
  files, and the directories it created are removed only when they are empty.
- **One job runs at a time.** `DEFAULT_MAX_ACTIVE` is 1 on purpose: a transfer
  competing with itself for the same disk is slower, and sequential order is
  what the queue promises.
- **Streaming is bounded.** A single 1 MiB buffer is reused for every file, so
  memory does not scale with file or transfer size (a 192 MiB sanity test
  asserts the process does not grow by more than 96 MiB while it runs).
- **Moves do not delete until the copy is verified.** A same-volume move of a
  root that does not collide is a single rename; anything else copies, checks
  the copied byte count per file, and deletes the source only when nothing
  failed, skipped, or went unsupported. Otherwise the source stays and the job
  reports why.
- **Links are reported, never followed.** Symlinks and reparse points are
  recorded as `unsupported` issues; they are never read, copied, or deleted.
- **Unsafety is refused before a job exists.** A missing or unreadable source,
  a destination that is not an existing directory, a destination inside its own
  source, an unplannable item count, and a destination without room all fail the
  request with a structured error instead of queueing work that cannot finish.
- **Progress is throttled, not polled.** Snapshots are published at most every
  120 ms per job, and the UI can always resynchronize with `list_transfers`.
- **What happens is written down.** Live job state is journaled while a job
  runs (at most every 2 s), its history record is written before its live state
  is forgotten, and a job becomes durable before a worker can claim it.

---

# Implemented semantics and limits

Stated precisely enough to build on, including the parts that are deliberately
missing.

- **Pause keeps the job's worker.** A running job parks at a chunk boundary with
  its handles still open, which is what makes resume exact: the same handle
  continues at the same offset, so nothing is re-read, rewritten, or compared
  against a remembered position. With the default policy of one active job, a
  paused job therefore occupies the worker, and jobs behind it wait until it is
  resumed or cancelled. Cancelling always releases it.
- **Cancel discards only what the job wrote.** The in-progress `.partial` file
  is removed, directories the job created are removed when they are empty, and
  files the job already committed stay. An existing destination file is never
  deleted by a cancel: under `replace` it is replaced at commit time, by a
  rename over a completed temporary file.
- **Move is copy-then-delete.** A same-volume move of a root that collides with
  nothing is one rename and copies nothing. Every other move copies, compares
  each written file's byte count against the size the plan measured, and deletes
  the source only when nothing failed, was skipped, or was unsupported. A source
  that changed size mid-copy keeps its copy and its original, and is reported as
  an issue.
- **Verification proves what it says it proves.** The default `size` policy
  compares each written file's byte count with the bytes streamed out of the
  source; `checksum` also compares the SHA-256 computed while copying against
  the file on disk; `none` records every file as skipped and ends with `not
  verified` rather than a green badge. A job's own summary reports how many
  planned files were checked and what was not — see `VERIFICATION.md`.
- **Interrupted work is known, not guessed at.** Jobs are journaled while they
  run, so the next start knows what was in flight and how far it had reported
  getting. Nothing ever infers success from a destination file: a job that did
  not prove it finished is treated as unfinished, classified as one of five
  outcomes, and left for the user to restart, discard, or confirm — byte-offset
  resume of a `.partial` file is deliberately refused. See `RECOVERY.md`.
- **Links are reported, never followed.** Symlinks, junctions, and other reparse
  points are recorded as `unsupported` and are never read, copied, or deleted,
  which is what keeps a transfer inside the tree it was given.
- **Metadata is not preserved — and says so.** Contents and directory structure
  are copied; the source's modification time and read-only attribute are not
  reapplied, and every verification summary reports those two claims as
  explicitly false instead of leaving a verdict to imply otherwise.
- **Read-only destinations are refused, not overridden.** Replacing an existing
  read-only entry fails with a structured error rather than clearing the
  attribute to make room — a visible failure beats silently changing an
  attribute the user set.
- **One job at a time, by policy.** `DEFAULT_MAX_ACTIVE` is 1, and the engine
  takes the limit as a parameter, so raising it is a one-line change once there
  is evidence it helps.

---

# Responsibilities

The Transfer Engine handles:

- File copying
- File moving
- Transfer queue management
- Pause and resume
- Retry handling
- Conflict resolution
- Progress reporting
- Post-transfer verification
- Publishing what happened to the archive (durable state, history, recovery)

---

# Non-Responsibilities

The Transfer Engine does not handle:

- User interface
- Notifications
- Operating system UI
- File browsing interface
- Reading or writing its own documents: it publishes snapshots to the archive
  through the `TransferJournal` port and never touches a file for durability

---

# Transfer Pipeline

A transfer follows this lifecycle:

```
Discover

↓

Prepare

↓

Validate

↓

Transfer

↓

Verify

↓

Complete
```

---

# Prepare Stage

Before transfer:

Check:

- Source availability
- Destination availability
- Available space
- Permissions
- Conflicts

---

# Transfer Stage

Requirements:

- Efficient streaming
- Cancellation support
- Progress reporting
- Error recovery

---

# Pause and Resume

Pause keeps the job's worker and its open handles, so resuming continues the
same file at the same offset: nothing is re-read or compared against a
remembered position. A paused job therefore holds the worker, and jobs behind it
wait until it is resumed or cancelled.

That is in-process resume. It is not crash recovery: nothing persisted proves
which prefix of a partially written file is valid, so after a crash the affected
files are written again from zero — see `RECOVERY.md`.

Recovery of a live job across a device that comes back, a temporary failure, or
(later) a network interruption is the same mechanism: the job is planned again
and its partial output is discarded, never resumed mid-file.

---

# Error Handling

Failures should be isolated.

A single failed file should not automatically cancel the entire transfer.

---

# Conflict Handling

When files already exist, the strategy is chosen in the UI before the job is
queued and applies to every item in it:

- `replace` — overwrite what is already there (including a file where a folder
  goes, or the other way round). Nothing else is removed.
- `skip` — leave the existing entry exactly as it is. This is the default:
  a transfer never destroys data the user did not ask it to replace.
- `rename` — write beside the existing entry under a unique name
  (`report (2).txt`, `notes (2)`).

Future options:

- Remember preference per job
- Smart resolution

---

# Verification

Verification is optional and chosen per job (from the configured policy at the
time the job is planned or started).

Implemented methods:

- `none` — nothing is checked, and every file is recorded as skipped so the job
  never reports a verdict it did not earn
- `size` — each written file's byte count against the bytes read from the source
- `checksum` — the size check plus a SHA-256 comparison of the bytes read from
  the source against the file on disk

Metadata is **not** compared or reapplied: the summary reports modified times
and read-only attributes as not preserved. Strategies can expand without
changing the transfer architecture; the full contract, cost, and failure
behavior are in `VERIFICATION.md`.

---

# Performance Requirements

The engine should:

- Avoid unnecessary memory usage
- Support large transfers
- Remain responsive
- Avoid blocking the UI

---

# Security Requirements

The engine must:

- Never modify files unexpectedly
- Never delete without confirmation
- Preserve user control

---

# Future Extensions

Designed to support:

- Folder synchronization
- Automated transfers
- Transfer scheduling
- Background operations

without redesigning the core engine.
