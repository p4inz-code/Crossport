# Recovery Architecture

Version: 1.0
Status: Approved

## Purpose

Recovery is what CrossPort does about a transfer that was running when the
application — or Windows — stopped. It answers three questions:

1. What was in flight, and how far did it get?
2. Can it be run again, and what would running it again do to the destination?
3. What is left on disk from the interrupted attempt?

It never claims more than it can prove. **A transfer that did not prove it
finished is treated as unfinished.** No part of recovery looks at a destination
file and concludes that a transfer succeeded; the only proof accepted is the
archive's own record of the job reaching a terminal state.

## What is written while a job runs

Every state change of a job is journaled, and progress is journaled on a
throttled cadence (`JOURNAL_INTERVAL`, 2 s) so a crash costs at most a couple of
seconds of reporting while a long transfer stays quiet on disk.

The durable state document (`transfer-state.json`) holds up to 100 interrupted
jobs (`MAX_STATE_JOBS`); reaching that bound means the application died
mid-transfer a hundred times without the list ever being cleared, so the oldest
entries are dropped and the drop is logged. Each entry stores:

- the **whole request** — sources, destination, operation, conflict strategy, and
  verification policy — because that is what a restart replays, unchanged;
- the job's status when it was last written;
- queue/start/update timestamps;
- counters (`PersistedProgress`), deliberately without speed, ETA, or elapsed
  time: those describe a process that no longer exists, and persisting them
  would only invite someone to display them after a restart;
- verification as it stood, when any had run.

### Write ordering is the correctness property

1. While a job runs, its live state is replaced on its cadence.
2. When it finishes, its **history record is written first**.
3. Only then is the live state forgotten.

A crash between 2 and 3 leaves a job recovery can recognise as finished from the
archive's own record. It can never leave a finished job looking unfinished.

A job also becomes durable before it can be claimed by a worker, so a job that
was queued but never started is still known after a crash and is never silently
dropped.

## What an interrupted job looks like

Because the engine writes every file to a temporary path and renames it into
place only when it is complete, an interrupted job leaves a destination
containing only complete files, plus `.crossport-<job>-<index>.partial`
leftovers.

**Byte-offset continuation of those leftovers is not offered.** Nothing
persisted proves which prefix of a partial file is valid, so a restart starts
the affected files from zero after the leftovers are removed. Correctness is
worth more than resume speed — and the partial files are exactly what would make
resuming wrong.

## Classifying a job

`recovery::classify` decides one of five outcomes. Every one of them is a
verdict, not a suggestion:

| Outcome | Meaning | Restart offered |
| --- | --- | --- |
| `completed_before_crash` | The archive's own record shows the job reached a terminal state before the application stopped | No — nothing needs to run |
| `restart_required` | The job did not finish and can be run again from the beginning | Yes |
| `source_missing` | A source the job needs is gone | No |
| `destination_unavailable` | The destination is missing, is not a directory, or cannot be written | No |
| `unsupported` | The recorded request cannot be planned again (for example, it now covers more items than the engine plans) | No |

Each candidate carries the backend's own explanation (`detail`), so the surface
never has to invent one.

Two further pieces of information are offered, and both are labelled as the
weaker kind of statement they are:

- `destinationLooksComplete` — the result of re-planning the job and finding
  every planned item in place with the expected size. It is **evidence, not
  proof**, it is bounded (20,000 items, then the answer is `null`, "unknown"),
  and it never on its own turns a job into `completed_before_crash`.
- `restartImpact` — what running the job again would do: the strategy in force,
  how many items collide, how many would be left alone, the bytes and files
  involved, and whether the restart would overwrite anything. A restart is
  re-planned under the strategy the user originally chose.

## Artifacts

The scan looks for the engine's own temporary files in the directories a job
planned to write into (`artifact_directories`), and matches them **exactly** to
the job being examined (`is_artifact_of`): only `.crossport-<job>-<index>.partial`
for that job's identifier. Another job's leftovers — or a file that merely looks
like an artifact — are never touched.

The scan is bounded and reports `artifactsTruncated` when it hit the bound, so
"no artifacts found" is never a guess.

## Actions

| Action | What it does |
| --- | --- |
| `discard` | Runs nothing again. The job's partial output is removed and a history record is written with status `interrupted` and action `discard` |
| `restart` | Queues the recorded request again, from the beginning, after removing the job's partial files. The new job gets a new identifier, and the history record carries `recoveredFrom` so the lineage is visible |
| `confirm` | The archive already proved the job finished (`completed_before_crash`). Nothing runs; the existing history record is revised to status `recovered` with action `confirm` instead of a second record being inserted |

Illegal requests are refused with structured errors rather than treated as "do
nothing": `recovery_unavailable` when an interrupted job cannot be restored,
discarded, or confirmed as asked, and `invalid_input` when an action identifier
is not one of the three.

## Why an interrupted job is in the history

An interrupted job is a fact about the past, so it is recorded as one: history
has `interrupted` and `recovered` statuses alongside `completed`, `failed`, and
`cancelled`. The History page shows what happened and which decision accounted
for it, and the details view shows the request, the counters, and the recovery
action.

## Startup experience

- The archive is opened during setup; the recovery candidates are read once and
  exposed through `list_recovery_candidates` / `get_recovery_candidate`.
- The shell checks for candidates on startup and raises **one** notification
  saying what happened and that a decision is waiting. Recovery never runs
  anything by itself — restarting a transfer is the user's call.
- The Recovery page lists each interrupted job with its outcome, its progress
  before the interruption, what a restart would do, and what is left on disk,
  with Discard and Restart (or Confirm) exactly where they apply.
- The sidebar shows how many jobs are waiting for a decision.

## Commands

| Command | Purpose |
| --- | --- |
| `list_recovery_candidates` | Every interrupted job with its outcome, evidence, artifacts, and restart impact |
| `get_recovery_candidate` | One candidate, refusing an unknown identifier with `recovery_unavailable` |
| `recover_transfer` | Applies `discard`, `restart`, or `confirm` and returns a `RecoveryReport` |
| `get_archive_status` | How each document loaded, and whether it can be written to |

## Tests

`recovery/tests.rs` covers classification for all five outcomes, illegal action
identifiers, artifact matching (including another job's leftovers and a
similarly named file being left alone), state bounds and validation, and the
`TransferStateStore` write-and-reload path. `archive/tests.rs` covers recovery
end to end against a real directory: a job interrupted mid-copy, a job that
completed before the crash, a restart that removes leftovers and queues the
request again, a discard, and a confirm that revises the existing record instead
of duplicating it. `transfer/tests.rs` covers the journal write ordering the
whole design rests on.
