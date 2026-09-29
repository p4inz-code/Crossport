# CrossPort 1.0 — production release document

Status: released, security hardened, and frozen. Windows only.

This is the single document to read before shipping a build. It states what the
release contains, what it proves, how to build and verify it, and what it
deliberately does not do. Everything here describes the code in this
repository; nothing in it is aspirational.

## Version

| Manifest | Version |
| --- | --- |
| `package.json` (root, canonical) | `1.0.0` |
| `apps/desktop/package.json` | `1.0.0` |
| `apps/desktop/src-tauri/tauri.conf.json` | `1.0.0` |
| `apps/desktop/src-tauri/Cargo.toml` | `1.0.0` |

This is the released 1.0 baseline. `scripts/check-versions.sh` enforces that all
four manifests move together, and the annotated tag `v1.0.0` records it.

## Supported platform

- Windows 10 1607+ or Windows 11, 64-bit. This is the only packaged and tested
  target.
- Microsoft Edge WebView2 runtime. Windows 11 and current Windows 10 include it;
  the installer downloads and installs it silently when it is missing.
- Paths longer than 260 characters need Windows long-path support enabled
  (`LongPathsEnabled`). Without it, Windows refuses the path and CrossPort
  reports the failure verbatim; shorter transfers are unaffected.

macOS and Linux are **not** packaged, not built, and not tested. The code is
written to be portable, and the platform layer has no Windows-only logic outside
guarded blocks, but that is a design property, not a supported platform.

## Installation

1. Run `CrossPort_1.0.0_x64-setup.exe` (NSIS, current user, installs into
   `%LOCALAPPDATA%\CrossPort`) or `CrossPort_1.0.0_x64_en-US.msi` (WiX,
   per-machine, needs elevation).
2. Start CrossPort from the Start menu. No account, no setup step, no network
   call.
3. Uninstall through **Apps → Installed apps → CrossPort**, or use the
   uninstaller.

The installers never touch `%APPDATA%\com.crossport.app` or
`%LOCALAPPDATA%\com.crossport.app` (settings, history, interrupted-transfer
state, logs, WebView2 profile). Uninstalling leaves those directories in place;
deleting them is the user's decision.

Installing an older version over a newer one is refused
(`bundle > windows > allowDowngrades` is `false`).

## Core functionality

- Browse volumes and directories. Every path is validated and resolved in Rust;
  listings never recurse and never follow symlinks or reparse points.
- Copy or move files and folders between volumes, with a queue that runs jobs in
  order (one at a time, by design).
- Three conflict strategies: `replace`, `skip` (the default), `rename`
  (`report (2).txt`).
- A dry run before anything is queued: what moves, where it lands, item counts,
  collisions, and free space at the destination.
- Pause, resume, and cancel. Cancelling discards only CrossPort's own partial
  output.
- Move semantics: a same-volume move of a clean root is one atomic rename; every
  other move copies first, checks per-file byte counts, and deletes the source
  only after the whole root succeeded. Anything failed or skipped keeps the
  source and reports why.
- Durable history of finished transfers, with filters, retention by count
  (default 200, range 20–2000), and a per-record verdict.
- Rotating logs (5 MiB × 3) in the platform app-log directory plus stdout. Logs
  record startup context, app directories, job identifiers, and document paths —
  not the contents of the files transferred.

## Verification behavior

Three policies, chosen in Settings and folded into a job when it is
planned/started, so changing the setting never alters queued work:

| Policy | What it proves |
| --- | --- |
| `size` (default) | The committed file's byte count equals the bytes streamed out of the source. |
| `checksum` | The SHA-256 computed **while copying** matches the SHA-256 recomputed from the file on disk, so a source that changed mid-copy cannot pass. |
| `none` | Nothing is checked, and the record says so (`status: skipped`). |

A mismatch fails its item and the job, keeps the file it wrote for the user to
inspect, and reports both sides of the discrepancy. Modification times,
attributes, ownership, and alternate data streams are **not** reapplied, and the
verification summary states that explicitly, so a passing verdict never implies
metadata was preserved.

## Recovery behavior

- Live job state is journaled while a job runs; the history record is written
  **before** the live state is forgotten, so a crash can never leave a finished
  job looking unfinished.
- On startup, interrupted jobs are classified into explicit outcomes:
  `completed_before_crash`, `restart_required`, `source_missing`,
  `destination_unavailable`, `unsupported`.
- Nothing restarts automatically. The user chooses **Restart**, **Discard**, or
  **Confirm** (only when the archive's own record proves the job finished).
- Partial output is matched exactly (`.crossport-<job>-<index>.partial`) and is
  never treated as a rename candidate, so recovery cannot claim or delete
  another job's files.
- Byte-offset resume of a partial file is deliberately refused: nothing
  persisted proves which prefix is valid, so a restart removes the leftovers and
  writes the affected files from zero.

## Security model

- **Rust is authoritative.** Every security-relevant decision — path validation,
  containment, overwrite, deletion, persistence — happens in the Rust backend.
  The webview is treated as untrusted input.
- **Minimal webview surface.** The only capability granted is `core:default`.
  No filesystem, dialog, shell, or HTTP plugin permission reaches the frontend,
  and unused plugin commands are stripped at build time
  (`removeUnusedCommands`). The folder picker and the dialog plugin run in Rust.
- **Restrictive CSP** on the production build, including `object-src 'none'`,
  `base-uri 'self'`, `form-action 'none'`, and `frame-ancestors 'none'`. The
  looser dev CSP exists only for Vite HMR.
- **No network.** The application makes no network calls. No telemetry, no
  update checker, no accounts, no secrets. (`reqwest`/`hyper` appear in
  `Cargo.lock` for other targets of the Tauri toolchain; neither is compiled
  into the Windows build.)
- **Conservative filesystem behavior.** A destination file only ever appears
  complete: bytes are written to a sibling `.partial` file and renamed into
  place. An interrupted, cancelled, or failed transfer never overwrites an
  existing file with a half-written one.
- **Containment is lexical.** Paths are normalized (absolute, no null bytes, no
  `..` escape) and compared component-wise, case-insensitively on Windows.
  Containment is not resolved through the filesystem — see the limitation below.
- **Persistence is defensive.** Documents are written to a unique temp file,
  fsynced, and renamed; they carry a `schemaVersion`; an older schema is
  migrated, a newer one is refused and left untouched, and an unusable document
  is preserved beside the original (`*.corrupt-<ms>.*`) rather than silently
  becoming an empty one.
- **No secrets.** The application collects no credentials and stores none.

## Known limitations

- **Windows only.** No macOS or Linux artifact exists.
- **Not code-signed.** Windows shows an unknown-publisher warning on first
  install. There is no auto-updater; a new version is installed by hand.
- **One transfer at a time,** by design.
- **History is bounded,** so it is a record of recent work, not an audit log.
- **Verification only proves what it lists** (size, or size + SHA-256). Metadata
  is not preserved.
- **Interrupted transfers are not resumed at a byte offset** — restart or
  discard, whole.
- **Path containment is lexical, not canonical.** Two spellings of the same
  location (an 8.3 short name such as `PROGRA~1`, or a path that passes through
  a pre-existing junction) are not recognized as the same directory, and a
  destination that traverses an existing junction is resolved by the operating
  system, so files land at the junction's target. CrossPort never creates,
  follows for planning, or removes reparse points.
- **Paths over 260 characters** need Windows long-path support.
- **A real version upgrade and a refused downgrade** have not been exercised,
  because only one shipped version exists. Re-check both at the first version
  after 1.0.0.
- **The MSI is built but not installed** in the recorded verification pass,
  because it requires elevation. The NSIS install / same-version reinstall /
  uninstall cycle was exercised against the built installer during the Phase 6
  pass (`docs/development/RELEASE_PROCESS.md`); it was **not** re-run for the
  1.0.0 artifacts. For the 1.0.0 build, verification was the artifact smoke test:
  the built `CrossPort.exe` starting on an isolated desktop with no development
  tools in its environment, writing its startup line, showing a window, and
  exiting cleanly. Re-run the full installer cycle before publishing.

## Artifact names and checksums

`bash scripts/release.sh` writes, under
`apps/desktop/src-tauri/target/release/bundle/`:

| Artifact | Path |
| --- | --- |
| NSIS installer | `nsis/CrossPort_1.0.0_x64-setup.exe` |
| WiX MSI | `msi/CrossPort_1.0.0_x64_en-US.msi` |
| Executable | `release/CrossPort.exe` |
| Checksums | `bundle/checksums.txt` (SHA-256 per artifact) |

Verify with:

```bash
cd apps/desktop/src-tauri/target/release/bundle
sha256sum -c checksums.txt
```

The exact digest values are not recorded here on purpose: they change with every
build, and `checksums.txt` is the authoritative list that ships beside the
artifacts.

## Release procedure

1. `bash scripts/check-versions.sh` — every manifest agrees.
2. The full gate: `pnpm check`, `pnpm lint`, `pnpm test`,
   `pnpm --filter desktop build`, then in `apps/desktop/src-tauri`:
   `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets`,
   `cargo test`.
3. `bash scripts/release.sh` — checks versions, builds the frontend, builds with
   `--locked`, and writes `checksums.txt`.
4. `cargo test --test artifact_smoke -- --nocapture` — the built executable
   starts from an isolated directory with no development tools in its
   environment, writes its startup line, shows a window, and exits cleanly.
5. Install, launch, reinstall, and uninstall pass on Windows
   (`docs/release/RELEASE_CHECKLIST.md`).
6. Tag `vX.Y.Z` and attach the installers plus `checksums.txt` to the release by
   hand. No workflow builds or publishes anything.

## Not supported (deliberately out of scope)

These are not stubbed and not hidden behind a flag; they do not exist:

- folder synchronization and watched folders
- batch or scheduled operations, and advanced transfer rules
- cloud storage, accounts, and sign-in of any kind
- telemetry, analytics, and crash reporting
- an update checker or auto-updater
- remote filesystem access and background services
- macOS and Linux packaging
- code signing

They belong to a later version. See `NON_GOALS.md`, `ROADMAP.md`, and
`docs/product/FEATURE_SPECIFICATION.md`.
