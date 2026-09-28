# Performance

What was measured, on what, and what the numbers mean. The point of this file is
to make a performance claim falsifiable: every figure below can be reproduced
with the commands in it.

## How to measure

| What | Command |
| --- | --- |
| Backend (Windows, real files) | `cd apps/desktop/src-tauri && cargo test --lib measure -- --ignored --nocapture --test-threads=1` |
| Backend, one scenario | `cargo test --lib measure::measure_large_file_streaming -- --ignored --nocapture` |
| Frontend budgets | `pnpm --filter desktop test src/test/performance.test.tsx` |
| Packaged artifact | `cd apps/desktop/src-tauri && cargo test --test artifact_smoke -- --nocapture` |

The backend scenarios are `#[ignore]`d, so a normal `cargo test` skips them — a
measurement that fails on a busy machine would help nobody — but they are
compiled with the rest of the suite, and every one of them still asserts the
catastrophe a regression would cause (a job that never finishes, an idle engine
that spins, private memory that grows with the file it copies).

## What the numbers are not

- They come from one machine in September 2026, in a **debug** profile
  (`cargo test`). An optimized build is faster; these are the pessimistic
  numbers, which is the right direction for a regression guard.
- The volume they ran on was **98% full** at the time. Throughput figures on a
  healthy disk are higher; the shape of the numbers is what matters.
- Frontend figures are **jsdom**, not a browser engine. They compare like with
  like (the same harness before and after a change), which is what the budgets
  are for; they are not a browser rendering benchmark.

## Backend (`src-tauri/src/measure.rs`)

| Scenario | Result |
| --- | --- |
| Directory listing, 10,000 entries | 10 ms cold, 9 ms warm |
| Deep tree plan — 500 levels, 2,000 files, 501 directories | 580 ms |
| Deep tree copy, same tree | 242 ms |
| Small files — 2,000 × 4 KiB | 3,848 ms (1.92 ms/file, 2.0 MiB/s) |
| Large file — 512 MiB | 339 ms (1,510 MiB/s), private memory **+1 MiB** |
| Idle engine | 0 ms of CPU over 2 s wall (0.00% of one core) |
| History record, default bound (200) | 4.5 ms/record, 199 KiB document |
| History record, maximum bound (2,000) | 64.4 ms/record, 1,999 KiB document |
| Archive open | 0 ms first run, 6 ms at 200 records, 120 ms at 2,000 |
| History listing | 0 ms at either bound |
| Deepest directory this host creates | 3,475 characters, and a file inside it writes |

Three of these are worth reading carefully.

**Streaming holds.** Copying 512 MiB cost the process **1 MiB** of private
memory. The engine reads and writes in fixed-size chunks; it never holds a
file's contents, and the ceiling is asserted so a future change that buffers a
whole file fails the suite instead of a user's machine.

**Nothing spins.** An idle engine uses 0 ms of CPU across two seconds of wall
time. Its worker sleeps on a condition variable between polls, so a queue
waiting for work costs nothing.

**A history record is a durability cost, not a leak.** `record()` persists the
whole document — serialize, write, flush, rename — because a finished job must
be on disk before its live state is forgotten, and a crash must never expose a
partial file. So the cost of one record grows with how many are retained: 4.5 ms
at the default bound of 200, 64.4 ms at the largest bound a user can configure.
At most one write happens per finished transfer, and the document stays under
2 MiB, so this is deliberate and bounded. An append-only journal would make the
write cheaper and the recovery more complex; that trade is not made here.

## Frontend (`apps/desktop/src/test/performance.test.tsx`)

The budgets are set far above the measured cost: they detect a regression, they
do not benchmark.

| Scenario | Measured | Budget |
| --- | --- | --- |
| zod validation of a 10,000-entry listing | 6 ms | 250 ms |
| DirectoryBrowser, 10,000 entries: first render | 173 ms | 1,200 ms |
| DirectoryBrowser, 10,000 entries: one checkbox | 6 ms | 250 ms |
| HistoryPage, 2,000 records: render | 584 ms | 2,500 ms |
| HistoryPage, 2,000 records: one selection | 207 ms | 1,000 ms |
| TransferQueue, 500 jobs | 366 ms | 2,000 ms |

The browser was the one genuine problem this pass found, and it is fixed. The
listing cap is 10,000 entries (`MAX_DIRECTORY_ENTRIES`) and the browser used to
render all of them: **1,672 ms to paint and 649 ms for a single checkbox click**,
because every row re-rendered and every row scanned a selected-path array. Three
changes fixed it:

- the rows are **rendered in a window** of 400 (`RENDER_STEP`), with the count
  stated on screen and **Show 400 more** / **Show all N** to extend it, so
  nothing is hidden silently;
- each row is a **memoized component**, so toggling one checkbox re-renders one
  row instead of the table;
- the checked paths are a **set**, so membership is a lookup rather than a scan
  per row.

Same dataset, after: **173 ms to paint, 6 ms per click.**

## Packaged artifact

`cargo test --test artifact_smoke -- --nocapture` starts the built executable
the way a user does — alone in a directory, with Node, pnpm, Cargo and the
repository scrubbed from its environment — and reports:

```
ARTIFACT startup line: 1255ms, window: 1256ms
ARTIFACT clean close: 100ms, exit code Some(0)
```

The first number is the backend reaching its startup log line; the second is a
visible window. Across runs on this host the startup line landed between 0.7 s
and 1.3 s with the window right behind it, and the close was 100 ms every time.
The window took about a minute on the very first run on this host, when WebView2
had never started on it — a cost that belongs to the runtime, not to CrossPort,
which is why the check allows for it rather than asserting a warm number.
