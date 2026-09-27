# Verification Architecture

Version: 1.0
Status: Approved

## Purpose

Verification answers one question after a file has been written: did what the
engine placed at the destination match what it read from the source?

It is deliberately narrow. It reports what it checked, what it found, and what
it did **not** check. Nothing in this module infers a claim from a status, and a
verdict the engine cannot prove is never rendered as a pass.

## Implementation

| Concern | Where it lives |
| --- | --- |
| Policy, method, statuses, mismatch reasons, per-file result | `verification/mod.rs` |
| Job-wide accumulation and the one-line verdict | `verification/mod.rs` (`VerificationLog`, `VerificationSummary`) |
| Streaming SHA-256 | `verification/hash.rs` |
| Calling it from the copy path | `transfer/copy.rs` (`verify_committed`) |
| Tests | `verification/tests.rs`, plus engine-level cases in `transfer/tests.rs` |
| Wire contract | `apps/desktop/src/types/verification.ts` |

## Policies

A policy is what the user chooses; a method is what actually ran. The two are
stored separately because "checksum was requested" and "a checksum was compared"
are different facts.

| Policy | Method | What it proves | What it does not prove |
| --- | --- | --- | --- |
| `none` | `none` | Nothing beyond the transfer's own commit. Every file is recorded as `skipped` and the job's verdict is `not verified`. | Anything about the bytes |
| `size` (default) | `size` | Every planned file exists at its destination as a file, with exactly the byte count that was streamed out of the source | That the contents are the source's contents; anything about metadata |
| `checksum` | `size_and_checksum` | As `size`, plus: the SHA-256 of the bytes read from the source equals the SHA-256 of the file on disk | Metadata; anything about a file that changed after its check |

`ChecksumAlgorithm` has exactly one value today (`sha256`). It exists so adding
an algorithm later is an addition to the contract rather than a redefinition of
it — and so the checksum that was used travels with the result.

## When it runs

Verification runs inside an item's own completion, after the temporary file has
been renamed onto its destination and before the item counts as completed.

- The job's status stays `running` while a file is checked; its
  `progress.activity` becomes `verifying` and `progress.currentFile` names the
  path being checked, so the surface can say what is happening instead of
  showing a finished-looking bar over unfinished verification.
- No bytes move while a check runs, so no progress is reported for the file
  under check.
- Pause and cancel are honoured during a verification read: the checkpoint is
  tested between chunks of the hash, and a stopped check is a cancellation, not
  a verification failure.
- One file at a time, in the job's own worker; verification never touches the
  queue lock, so checkpoints and control commands stay responsive.

### Checksums are of what was read, not of a second read

Under the `checksum` policy the digest of the source bytes is computed **while
the file is copied** — the same buffer that is written to the destination is
hashed. Verification then hashes the file on disk and compares the two. A source
that changed mid-copy therefore cannot quietly satisfy the check, and the copy
is never re-read from the source to "make sure".

If a checksum comparison is demanded but no digest was ever computed, that is a
`failed` verification with the reason attached. It is never reported as
`verified`.

### A destination is inspected, not followed

The destination is read with `symlink_metadata`, so a link or junction standing
where the plan wrote a file is a mismatch (`destination_not_a_file`) rather than
being followed to whatever it points at. Verification never reads through a
link, and it never reads the source at all.

## Statuses

| Status | Meaning |
| --- | --- |
| `pending` | Nothing has been checked yet; the job has not reached a file that needs checking |
| `verifying` | A check is running right now |
| `verified` | Every checked file matched. Not the same as "every planned file was checked" — see `unverifiedFiles` |
| `mismatch` | A file exists at the destination but does not match (size, content, missing, or the wrong kind of entry) |
| `failed` | The check could not be completed (an I/O read failed, or a checksum was required and never computed) |
| `skipped` | Nothing was checked: the policy is `none`, or no file needed checking (a same-volume move rename writes no bytes) |

Mismatch reasons: `size_mismatch`, `checksum_mismatch`, `destination_missing`,
`destination_not_a_file`. Each carries both sides of the discrepancy in its own
words — expected and actual — so the UI never has to reconstruct them.

## What a failure does

- The item fails with a `verification_failed` error carrying the reason, and the
  job continues with its remaining items. The job ends `failed`.
- **Nothing is rolled back.** The file that failed verification stays where it
  is, next to the evidence of what was expected and what was found, because
  deleting a file the user asked to place — even a wrong one — is a worse
  outcome than reporting it.
- A cross-volume move keeps its source, exactly as it does for any other failed
  item.
- The mismatch list on the job is bounded (100 entries) and flagged as
  truncated; the counts in the summary are complete regardless.

## Honest reporting

The summary (`VerificationSummary`) is built so a reader cannot mistake it for
more than it is:

- `plannedFiles`, `checkedFiles`, `verifiedFiles`, `mismatchedFiles`,
  `failedFiles`, `skippedFiles`, and `unverifiedFiles` are all reported.
  `unverifiedFiles` counts planned files with no result, so `checkedFiles` can
  never be read as "everything".
- `coverage` states its claims explicitly: `size`, `structure`, `checksum`, and
  two that are **always false** — `modifiedTimePreserved` and
  `readonlyPreserved`. This engine does not reapply modified times or the
  read-only attribute, so it says so instead of letting a green badge imply the
  metadata came along.
- `verdict` is one line rendered by the backend (`verified (size, 2 files, 4096
  bytes)`, `1 of 3 files did not verify`, `not verified`). The UI displays it; it
  never rebuilds it from counts.
- `durationMs` measures only the time spent checking, and is frozen when the job
  finishes.

## Cost

- `size`: one metadata read per file. Effectively free.
- `checksum`: one additional full read of each destination file, streamed
  through a reused 1 MiB buffer, so memory does not scale with file or transfer
  size. Disk throughput is the cost, and the UI says the job is verifying while
  it is paid.

## Policy selection

The configured policy lives in settings (`verification`, default `size`, see
`docs/architecture/SETTINGS.md`) and is **folded into the job's request** when it
is planned or started. Changing the setting afterwards cannot change what a
queued or running job does, and history records the policy a job actually ran
under.

## Surfaces

| Surface | What it shows |
| --- | --- |
| Transfer queue | A running job's status reads `Verifying` while it checks; the card shows the verdict line and, when any claim is false, what was not reapplied |
| History | The verdict and the policy that produced it, per record |
| Notifications | A verification failure is its own notification (`verification-failed`), distinct from a transfer that could not run |
| Settings | The policy picker, which also states that modified times and read-only attributes are not reapplied |

## Tests

- `verification/tests.rs` covers the policy-to-method mapping, every status
transition of the log, a match and each mismatch reason, a checksum that cannot
be compared being a failure, a link at the destination being a mismatch, the
coverage claims (including the two that are always false), the verdict wording,
cancellation during a check, real files that are truncated, corrupted in place,
and streamed past the buffer size — all against real temporary directories.
- `transfer/tests.rs` covers the policy in the engine's own copy path: a `size`
job that verifies its bytes, a `checksum` job whose digests are recorded and
whose copy really holds the source's bytes, and a `none` job that reports
`skipped` / `not verified` instead of a pass.
- `transfer/sanity.rs` runs it as a milestone check on real files:
`sanity_verification_proves_what_it_claims_on_real_files` copies a real tree
under `size` and asserts what was checked and what was not, copies a file under
`checksum`, tampering with the copy in a way only a checksum can catch (same
size, different bytes) and asserting `checksum_mismatch` plus a
`verification_failed` error, and runs a `none` job that ends `not verified`. It
also re-reads source and destination independently and compares them byte for
byte.
- `archive/tests.rs` runs whole jobs under `size` and `checksum` policies
  through the engine and the archive, and asserts the verdict that lands in
  history.

A mismatch produced by the copy path itself cannot be manufactured
deterministically — the engine always compares against the bytes it just wrote
— so that path is not faked with a race. What is asserted instead is the layer
below it (a real mismatched file, on disk) and the consequence above it (the
`verification_failed` code and the failed item).
