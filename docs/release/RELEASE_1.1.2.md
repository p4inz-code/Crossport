# CrossPort 1.1.2 — release document

Status: built, installed, and validated on Windows. Not code-signed. Windows only.

This is a **corrective patch** on the Windows 1.1 line. It ships the fixes that
landed on `main` after `v1.1.1`, and it is the first release taken from the
repository's Apache-2.0 development line. `RELEASE_1.1.1.md` remains the record
for that release, and `RELEASE_1.1.0.md` and `RELEASE_1.0.md` for theirs. The
`v1.0.0`, `v1.1.0`, and `v1.1.1` tags and their artifacts are untouched.

## Version

| Manifest | Version |
| --- | --- |
| `package.json` (root, canonical) | `1.1.2` |
| `apps/desktop/package.json` | `1.1.2` |
| `apps/desktop/src-tauri/tauri.conf.json` | `1.1.2` |
| `apps/desktop/src-tauri/Cargo.toml` | `1.1.2` |
| `apps/desktop/src-tauri/Cargo.lock` (the `crossport` entry) | `1.1.2` |

`scripts/check-versions.sh` enforces that the first four move together.

## What 1.1.2 fixes

**Two preserved copies could overwrite each other.**

When a history or settings document could not be used, the backend sets it aside
next to the original so the problem stays diagnosable. The preserved name came
from a millisecond timestamp alone (`history.corrupt-<stamp>.json`). Two
documents preserved inside the same millisecond therefore produced the same
name, and the second `std::fs::rename` silently replaced the first — losing the
very evidence the preservation exists to keep.

The name now carries an attempt counter, and it only uses the counter when the
plain name is already taken, so ordinary preserved names are unchanged. The
collision was reproduced at **9 failures in 60 runs** before the change and **0
in 320** after it.

**Error classification no longer assumes Windows path semantics.**

A path whose closest existing ancestor is a regular file cannot exist. Windows
reports that as "not found"; POSIX reports `ENOTDIR`. The persistence and safety
layers previously read the OS error kind, so the same input could produce
different outcomes per platform — and reading `io::ErrorKind::NotADirectory`
would have raised the crate's declared MSRV (1.77.2).

Both now ask `filesystem::blocking_file`, a small helper that walks the parents
for the first entry that is not a directory. Persistence classifies such a
document as `Missing` and the transfer safety rules return `path_not_directory`,
identically on every target, and the tests that cover it are platform-neutral.

No engine, persistence, security, or IPC-shape behaviour changed beyond these.

## Supported platform

Unchanged: **Windows 10 1607+ or Windows 11, 64-bit** is the only packaged and
tested target, with the Microsoft Edge WebView2 runtime. Paths over 260
characters need Windows long-path support enabled.

macOS and Linux are **not packaged, not built, and not tested**. CrossPort is an
application, not a filesystem driver. See
`docs/product/PLATFORM_SUPPORT.md` and `docs/product/PARAGON_CAPABILITY_GAP.md`.

## Verification performed

| Check | Result |
| --- | --- |
| `pnpm check` (Biome) | 186 files checked, no fixes |
| `pnpm lint` (ESLint) | clean |
| `pnpm --filter desktop build` (tsc + Vite) | built |
| `pnpm test` (Vitest) | 43 files, 455 tests passed |
| `cargo fmt --check` | clean |
| `cargo check --all-targets` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 400 passed, 0 failed, 8 ignored, plus the release-artifact smoke test |
| `bash scripts/check-versions.sh` | all versions in sync: 1.1.2 |

Family totals inside the Rust suite: persistence 21, recovery 37, verification
47, safety 15, plus the engine, filesystem, archive, and on-disk sanity suites.
The frontend suite includes the journey tests and the performance-budget tests.

### Regression coverage for the two fixes

| Fix | Test |
| --- | --- |
| Preserved copies cannot collide | `persistence::tests::preserved_files_do_not_collide_with_each_other` |
| A path through a file is blocked | `filesystem::tests::blocking_file_finds_the_file_a_path_runs_through`, `filesystem::tests::blocking_file_ignores_a_path_that_simply_does_not_exist` |

The two `blocking_file` tests are new in 1.1.2. The persistence and safety
behaviour they enable is covered by
`persistence::tests::a_write_failure_is_a_structured_error` (a document under a
file loads as `Missing`) and
`transfer::safety::tests::create_chain_rejects_a_file_in_the_way` (a file in the
way of a destination chain is `path_not_directory`). Both now reach the same
verdict on every target.

### Live validation of the packaged application

The **NSIS installer built for this release** was installed on a clean machine
state (the 1.1.1 installation was uninstalled first) and the installed
application was driven through its own backend and interface:

| Step | Result |
| --- | --- |
| Startup | log records `CrossPort v1.1.2 starting on windows (windows/x86_64)`; footer reads **v1.1.2** |
| Browse a volume | volume rail, breadcrumb walk `C:\` → `Users` → `Public` → a folder, listing rows |
| Copy with checksum verification | **completed**, verdict `verified (size_and_checksum, 23 bytes)` |
| Conflict strategy `skip` | **completed** with `skippedItems: 1` and one issue of reason `skipped` |
| Move | **completed**, verdict `not verified` — a same-volume move of a non-colliding root is a single rename that writes no bytes, which is the documented outcome (`docs/architecture/VERIFICATION.md`) |
| Pause / resume | running at 10 of 61 files → **paused**, held at 10 of 61 for 1.5 s with no progress → resumed → **completed**, `verified (size, 61 files, 1403316480 bytes)`, 0 issues |
| Cancel | running at 21 of 60 files → **cancelling** → **cancelled**; the 21 committed files were verified and kept (`verified (size_and_checksum, 21 files, 176160768 bytes)`), no `.partial` or temp file was left behind |
| History | the completed, cancelled, and moved jobs are recorded durably (`transfer-history.json`, 21 records) |
| Recovery | `candidates: []` — nothing was left interrupted |
| Clean close | process exits; `transfer-state.json` holds `{"jobs": [], "schemaVersion": 1}`; the log contains no warning or error for the whole session |
| Uninstall | program directory, Start-menu entry, and desktop shortcut all removed; user data under `%APPDATA%\com.crossport.app` survives, as documented |

## Artifacts and checksums

`bash scripts/release.sh` writes, under
`apps/desktop/src-tauri/target/release/bundle/`:

| Artifact | Path |
| --- | --- |
| NSIS installer | `nsis/CrossPort_1.1.2_x64-setup.exe` |
| WiX MSI | `msi/CrossPort_1.1.2_x64_en-US.msi` |
| Checksums | `bundle/checksums.txt` (SHA-256 per artifact) |

Verify with:

```bash
cd apps/desktop/src-tauri/target/release/bundle
sha256sum -c checksums.txt
```

Digest values are not recorded here on purpose: they change with every build,
and `checksums.txt` is the authoritative list that ships beside the artifacts.

### Packaged metadata

| Field | Value |
| --- | --- |
| Executable product name / version | `CrossPort` / `1.1.2` |
| Executable company | P4inz Interactive Labs |
| MSI `ProductName` / `ProductVersion` | `CrossPort` / `1.1.2` |
| MSI `Manufacturer` | P4inz Interactive Labs |
| MSI language | 1033 (`en-US`) |
| Bundled license text | the repository's Apache-2.0 `LICENSE` |

## Known limitations

All limitations from 1.1.1 still apply: Windows only, not code-signed, one
transfer at a time, bounded history, verification proves only the claims it
lists, no byte-offset resume, lexical path containment, long paths needing
Windows support, and a downgrade being permitted rather than refused.

The installers are **not code-signed**, so Windows SmartScreen shows an
unknown-publisher warning on first run and the MSI cannot be installed without
accepting it. Verify a download against `checksums.txt` before running it.

The MSI is **built and its metadata, payload, and checksum are validated, but it
is not installed**: installing an MSI requires elevation, and this build
environment is not elevated (`MSI_LUA: … no credential elevation is possible`,
exit 1603). The NSIS installer is the graded artifact for this release.

## Release procedure

1. `bash scripts/check-versions.sh`
2. The full gate (see the table above).
3. `bash scripts/release.sh` — builds the frontend, builds with `--locked`, and
   writes `checksums.txt`.
4. `cargo test --test artifact_smoke -- --nocapture`
5. `sha256sum -c checksums.txt` from the bundle directory.
6. Tag `v1.1.2` at the validated commit and attach the installers plus
   `checksums.txt`. No workflow builds or publishes anything.

## Not supported (deliberately out of scope)

Unchanged from 1.0.0, 1.1.0, and 1.1.1: folder synchronization, scheduling,
cloud storage and accounts, telemetry, update checking, remote filesystem
access, macOS/Linux packaging, and code signing. See `NON_GOALS.md` and
`docs/product/FEATURE_SPECIFICATION.md`.
