> **Status: core engine implemented (Phase 3).** Copy/move, the queue, real
> progress, pause/resume/cancel, conflict resolution, and the safety rules below
> exist today under `src-tauri/src/transfer/`. History persistence, crash
> recovery, and post-transfer verification are not implemented yet; the sections
> that describe them are planning material.
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
| Streaming, temp-file commits, move semantics | `transfer/copy.rs` |
| Queue, workers, identifiers, published snapshots | `transfer/mod.rs` |
| IPC commands and `blocking` pool offload | `commands/transfer.rs` |

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

---

# Implemented semantics and limits (Phase 3)

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
- **Verification is byte-count only.** There is no checksum yet: a transfer
  proves it wrote the number of bytes it planned, not that those bytes are the
  ones the source held. Content verification is a later milestone.
- **Nothing survives the process.** Jobs live in the engine's memory: no history,
  no persistence of interrupted transfers, no crash recovery. If the app exits
  mid-transfer, an uncommitted `.partial` file stays on disk and the next run
  does not know about it.
- **Links are reported, never followed.** Symlinks, junctions, and other reparse
  points are recorded as `unsupported` and are never read, copied, or deleted,
  which is what keeps a transfer inside the tree it was given.
- **Metadata is not preserved.** Contents and directory structure are copied;
  the source's modification time and read-only attribute are not reapplied.
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

---

# Non-Responsibilities

The Transfer Engine does not handle:

- User interface
- Notifications
- Operating system UI
- File browsing interface

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

The system should support recovery when possible.

Examples:

- Drive reconnect
- Temporary failure
- Network interruption (future)

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

Verification is optional.

Possible methods:

- Size comparison
- Metadata comparison
- Checksum verification

Verification strategies can expand without changing the transfer architecture.

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
