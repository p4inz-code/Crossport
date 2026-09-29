# CrossPort

A fast, reliable, cross-platform file transfer utility. Shipped for Windows
today; the application is built to be portable.

Free forever. Offline first. No accounts, no ads, no telemetry.

## Status

**CrossPort 1.0.0 is released** (tag `v1.0.0`); that tagged baseline is frozen,
and only critical fixes change it. Development continues on the **1.1.0** line —
product and platform polish on top of 1.0.0: a layout that uses the whole
window, one finalized logo, and documentation of exactly which platforms and
capabilities are supported. Phases 0–7 are complete: the engine, the interface,
verification, recovery, and history all work end to end, and the Windows
production build is hardened and released from a committed lockfile. The application builds into Windows
installers (NSIS and MSI), starts on a machine with no Node, pnpm, Cargo, or
repository anywhere in sight, and its performance at the largest datasets the
backend can produce is measured and bounded by tests. The repository currently
provides:

- A Tauri 2 desktop shell (React 19 + TypeScript + Vite frontend, Rust backend)
- A design-token-driven UI system with light/dark/system themes
- A typed IPC layer with structured `{ code, message }` errors end to end
- Backend-owned, validated, persisted settings
- Volume detection with real metadata: kind (fixed, removable, network, optical,
  RAM disk, or unknown), volume name, filesystem type, total/free/used capacity,
  read-only flag, and mounted status. Windows volumes are probed through the
  Win32 volume APIs; anything a platform cannot report stays unknown
- A storage browser: pick a volume, open its folders, walk a breadcrumb trail,
  go back, forward, and up (buttons or `Alt`+arrow keys), refresh, and see every
  entry with its size, modification time, and kind
- Safe navigation: every path is validated in Rust before it is read, listings
  never recurse, and symlinks/reparse points are reported but never followed
- A filesystem foundation (path normalization, directory validation, metadata,
  single-directory listing)
- A platform abstraction (OS identity, app directories, volumes)
- A native folder picker hosted in Rust
- A transfer engine: copy and move files and folders recursively, with a queue
  that runs jobs in order, live byte/speed/ETA progress, pause, resume, cancel
  with partial-output cleanup, and three conflict strategies (Replace, Skip,
  Rename)
- Transfer safety: destination and source validation in Rust, refusal to
  transfer into itself, free-space checks, symlinks and reparse points reported
  but never followed or deleted, and moves that keep the source when anything
  failed
- A transfer surface: multi-select in the browser, a composer that shows the
  backend's dry run — source mapping, counts, free space, conflict behaviour,
  verification policy, and warnings — before anything is queued, and a queue
  page with per-job progress, issues, controls, the verification verdict, and a
  link to the finished job's durable record
- Post-transfer verification with three policies (`size` by default, SHA-256
  `checksum`, or `none`): the checksum compares the digest of the bytes read from
  the source against the file on disk, a mismatch fails its item while keeping
  the file it wrote, and every verdict states what was checked and what was not
- Durable state and history: versioned, atomically written documents that
  survive a crash, with damaged files preserved beside the original rather than
discarded
- Crash recovery: interrupted transfers are classified into explicit outcomes
  and left for the user to restart, discard, or confirm. Nothing is restarted
automatically, and byte-offset resume of a partial file is deliberately refused
- Transfer history with per-record verdicts, filters, retention by count, and a
  details view
- Notifications for finished, failed, verification-failed, skipped, and
  interrupted transfers, each able to open the surface it is about
- Desktop behaviour that respects the work in flight: closing the window while a
  transfer runs is held by Rust and answered in the interface, and `Ctrl`/`Cmd`+
  `1`…`6` move between pages without a mouse
- A Windows installer (NSIS) and a WiX MSI, built from `scripts/release.sh`
  into `apps/desktop/src-tauri/target/release/bundle/` with SHA-256 checksums,
  and a shipped executable named `CrossPort.exe` after the product rather than
  after the crate
- Windows-safe production logging (stdout + rotating per-app log file) that
  records startup context, app directories, and job identifiers — never the
  contents of the user's files, and never the paths of the files a transfer
  moves
- A user-visible failure surface: a startup failure or a panic shows a message
  box (with a matching log entry) instead of a window that never appears
- Measured performance: startup, directory listings, deep trees, many small
  files, a large streamed file, idle CPU, and the largest listing/history/queue
  the interface can render, with budgets enforced by tests (see
  `docs/development/PERFORMANCE.md`)
- A restrictive CSP and a minimal Tauri capability set
- Real test suites (Vitest + `cargo test`), a release-artifact smoke test, and
  CI on Linux and Windows

`docs/release/RELEASE_1.0.md` is the production release document: version,
supported platform, installation, functionality, verification, recovery, the
security model, artifacts, and the explicit list of what is not supported.

Not implemented yet: folder synchronization, scheduling, and the other Version 2+
candidates in
[`docs/product/FEATURE_SPECIFICATION.md`](docs/product/FEATURE_SPECIFICATION.md) —
they are deliberately absent rather than stubbed. See [`ROADMAP.md`](ROADMAP.md)
for the phase plan.

## Repository layout

| Path | Purpose |
| --- | --- |
| `apps/desktop/` | The single Tauri desktop application |
| `apps/desktop/src/` | React frontend (features, stores, services, UI primitives) |
| `apps/desktop/src-tauri/` | Rust backend (commands, platform, filesystem, settings, errors) |
| `docs/` | Architecture, design, development, and product documentation |
| `scripts/` | Development and release scripts |
| `tests/` | Cross-cutting testing notes |

## Platform support

| Platform | Status |
| --- | --- |
| **Windows 10 1607+ / Windows 11, 64-bit** | **Supported.** The only packaged and tested target. |
| Linux | Not built, not tested, not supported — application foundation only. |
| macOS | Not built, not tested, not supported — application foundation only. |

CrossPort is an application, not a filesystem driver: it moves files across the
volumes your operating system has already mounted. It does **not** provide
native NTFS write access on macOS, does not mount, format, or repair volumes,
and loads no kernel or system extension. The full position is in
[`docs/product/PLATFORM_SUPPORT.md`](docs/product/PLATFORM_SUPPORT.md) and
[`docs/product/PARAGON_CAPABILITY_GAP.md`](docs/product/PARAGON_CAPABILITY_GAP.md).

## Requirements

### To run CrossPort

- Windows 10 1607+ or Windows 11, 64-bit
- Microsoft Edge WebView2 runtime. Windows 11 and current Windows 10 include
  it; the installer downloads and installs it silently when it is missing
- Paths longer than 260 characters work when Windows has long paths enabled
  (`LongPathsEnabled`); without it, Windows itself refuses them and CrossPort
  reports the failure. Transfers below that limit are unaffected

### To build CrossPort

- Node.js >= 22 and pnpm 10
- Rust (stable) with Cargo
- Platform prerequisites for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

## Getting started

```bash
pnpm install            # install workspace dependencies
pnpm dev                # frontend dev server (http://localhost:5173)
```

To run inside the Tauri shell (required for drive enumeration, the native
folder picker, and backend-persisted settings):

```bash
pnpm --filter desktop exec tauri dev
```

## Installing

The release artifacts are produced locally (see [Building a release](#building-a-release));
there is no published download and no auto-updater.

1. Run `CrossPort_<version>_x64-setup.exe` (NSIS, installs for the current user
   into `%LOCALAPPDATA%\CrossPort`) or `CrossPort_<version>_x64_en-US.msi`
   (WiX). The NSIS installer also adds a Start-menu entry and a desktop
   shortcut.
2. Start CrossPort from the Start menu. The window opens with no setup step and
   no account.
3. To remove it, use **Apps → Installed apps → CrossPort → Uninstall** the same
   way as any other Windows application.

Uninstalling removes the program and its shortcuts. Settings, transfer history,
interrupted-transfer state, and logs live under `%APPDATA%\com.crossport.app`
and `%LOCALAPPDATA%\com.crossport.app`; remove those folders by hand if you want
them gone as well. CrossPort never deletes them for you.

## Verification

Every change must keep the full suite green:

```bash
pnpm lint               # ESLint
pnpm check              # Biome
pnpm --filter desktop build   # typecheck + production build
pnpm test               # Vitest (frontend, includes the performance budgets)
cd apps/desktop/src-tauri
cargo fmt --check       # Rust formatting
cargo check --all-targets
cargo clippy --all-targets
cargo test              # Rust tests, real files included
bash scripts/check-versions.sh   # version sync across manifests
```

`scripts/lint.sh`, `scripts/test.sh`, and the [CI workflow](.github/workflows/ci.yml)
orchestrate the same commands on Linux, and the workflow also runs the Rust
suite on Windows so the Windows-only behaviour is exercised where it ships.

Two further checks are run when preparing a release:

```bash
cargo test --test artifact_smoke -- --nocapture        # starts the built app
cargo test --lib measure -- --ignored --nocapture --test-threads=1   # performance numbers
bash scripts/release.sh                               # installers + checksums
```

## Building a release

```bash
bash scripts/release.sh
```

This checks that every manifest agrees on the version, builds the frontend,
builds the application with the committed lockfile (`cargo build --locked`),
produces the NSIS installer and the MSI, and writes
`apps/desktop/src-tauri/target/release/bundle/checksums.txt` with a SHA-256
digest for every artifact. Nothing is uploaded anywhere.

Artifacts are not code-signed: Windows SmartScreen will warn about an unknown
publisher until a signing certificate and `bundle > windows > certificateThumbprint`
are configured. `docs/development/RELEASE_PROCESS.md` has the details, including
how to verify a digest and what the installers do to user data.

## Running CrossPort

Open a volume on the left, walk into folders with the breadcrumb trail or by
double-clicking, tick the rows you want to transfer, and choose **Copy to…** or
**Move to…**. The composer shows what the backend planned — sources, counts,
free space, conflict behaviour, verification policy, and warnings — before
anything is queued. The queue page owns the job from there: live byte, speed,
and ETA progress; pause, resume, cancel; the verification verdict and what was
not preserved; and a link to the finished job's record in History. If a job is
interrupted (a crash, a close while it was running, a volume pulled out),
Recovery lists it with what a restart would do, and nothing is restarted on its
own. `Ctrl`/`Cmd`+`1`…`6` move between pages.

## Known limitations

- Windows is the only packaged target so far. The shell is cross-platform by
  construction, but no macOS or Linux artifact is built or tested here.
- Artifacts are not code-signed, so Windows shows an unknown-publisher warning
  on first install.
- There is no update checker: a new build is installed the same way as the
  first one. Installing an older version over a newer one is refused.
- One transfer runs at a time, by design. A second job waits in the queue.
- History is bounded (200 records by default, 2,000 at most) and pruned on
  write, so it is a record of recent work rather than an audit log.
- Verification proves only the claims it lists: `size` by default, SHA-256 when
  chosen, and `none` when asked for. Modification times, attributes, ownership,
  and alternate data streams are not reapplied to what is written.
- An interrupted transfer is never resumed at a byte offset: it is restarted or
discarded whole, and CrossPort says so before doing either.
- Paths longer than 260 characters need Windows long-path support enabled; the
  engine creates a directory 3,475 characters deep and copies a tree that deep
  on such a machine, and reports the Windows error verbatim on one where the
  limit still applies.
- Source/destination containment is decided lexically, not by resolving the
  filesystem: two spellings of the same location (an 8.3 short name, or a path
  through a pre-existing junction or reparse point) are not recognized as the
  same directory, and a destination path that traverses an existing junction is
  resolved by the operating system, so files land wherever that junction
  points. CrossPort never creates, plans through, copies, or removes reparse
  points itself.
- The installers are exercised as far as one version allows: a fresh install, a
  same-version reinstall, and an uninstall were all run against the built
  artifact, but a real version upgrade and a refused downgrade need a second
  version to exist and have not been.

## Architecture in one paragraph

The frontend is feature-folder based with shared stores (Zustand), services
that own all IPC (`src/services/`), and token-driven UI primitives. Every
backend call goes through one transport module that validates payloads and
normalizes failures into `IpcError` with a stable code. The Rust backend owns
user settings, persists them to the platform app-config directory, and exposes
commands for settings, platform facts, volume detection, directory listing,
path inspection, the native folder picker, transfers, history, and recovery.
Transfer documents (state, history) go through one atomic, versioned document
layer, and the archive is the only module that knows both the live engine and
the durable side. Filesystem and platform work stays in Rust, so the webview is
granted only Tauri core defaults.

See [`docs/architecture/SYSTEM_ARCHITECTURE.md`](docs/architecture/SYSTEM_ARCHITECTURE.md)
for details.

## License

MIT — see [LICENSE](LICENSE).
