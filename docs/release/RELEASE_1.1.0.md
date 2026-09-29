# CrossPort 1.1.0 — release document

Status: built and validated on Windows. Not code-signed. Windows only.

> **Superseded by [1.1.1](RELEASE_1.1.1.md).** The 1.1.0 artifacts contain a
> wire-contract defect that makes the transfer queue and the start-transfer
> flow fail in the interface. This document is kept as the record of that
> release and is not updated further.

This is the current development release line, built on top of the frozen 1.0.0
baseline. `docs/release/RELEASE_1.0.md` remains the document for 1.0.0; this
file states what 1.1.0 adds, what was verified for it, and what it still does
not claim. Everything here describes code in this repository.

## Version

| Manifest | Version |
| --- | --- |
| `package.json` (root, canonical) | `1.1.0` |
| `apps/desktop/package.json` | `1.1.0` |
| `apps/desktop/src-tauri/tauri.conf.json` | `1.1.0` |
| `apps/desktop/src-tauri/Cargo.toml` | `1.1.0` |

`scripts/check-versions.sh` enforces that all four move together.

The tagged **`v1.0.0` baseline is unchanged and frozen**. It is not moved, not
rebuilt, and not re-pointed by this release.

## Supported platform

Unchanged from 1.0.0: **Windows 10 1607+ or Windows 11, 64-bit** is the only
packaged and tested target, with the Microsoft Edge WebView2 runtime. Paths
over 260 characters need Windows long-path support enabled.

macOS and Linux are **not** packaged, not built, and not tested. CrossPort is an
application, not a filesystem driver: it does not provide native NTFS write
access on macOS, does not mount, format, or repair volumes, and loads no kernel
or system extension. See `docs/product/PLATFORM_SUPPORT.md` and
`docs/product/PARAGON_CAPABILITY_GAP.md`.

## What 1.1.0 changes

- **Interface consistency.** Every colour, spacing, radius, and type value now
  comes from the shared token set. The History and Recovery surfaces — which
  previously carried hard-coded pixel values — were rebuilt on the design
  system, and the volume cards, transfer cards, composer, sidebar, and card
  primitive were given a consistent hierarchy and complete hover / focus /
  active / disabled states.
- **Defect fixes.** Four design variables that components referenced were never
  defined (`--color-surface-muted`, `--color-accent`, `--font-normal`,
  `--space-3-5`); a header-only card drew a stray rule beneath its header; and
  the measurement harness let its scenarios contaminate one another.
- **Responsive behaviour** was re-verified end to end (below).
- No engine, persistence, security, or IPC behaviour changed.

## Verification performed

The full gate was run and is green:

| Check | Result |
| --- | --- |
| `pnpm check` (Biome) | 186 files, no findings |
| `pnpm lint` (ESLint) | clean |
| `pnpm --filter desktop build` (tsc + Vite) | built |
| `pnpm test` (Vitest) | 43 files, 455 tests passed, including the performance budgets |
| `cargo fmt --check` | clean |
| `cargo check --all-targets` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 398 unit tests passed (8 opt-in measurement tests ignored), 1 release-artifact smoke test passed |
| `cargo test --lib measure -- --ignored --nocapture` | 8 measurement scenarios passed |
| `bash scripts/check-versions.sh` | all versions in sync: 1.1.0 |

### Layout and theme (real rendering)

The built frontend was served and driven by headless Chrome over the DevTools
Protocol. At **1024×720, 1280×720, 1366×768, 1440×900, 1920×1080, 800×600
(narrow)**, and **1100×720 (restored default)**, each of the six routes
(Home, Drives, Transfers, History, Recovery, Settings) was measured for
horizontal overflow and clipped controls:

- document `scrollWidth` never exceeded `clientWidth` (0 px overflow at every
  size and route);
- no control was clipped outside the viewport;
- the shell, sidebar, and top bar resized correctly (sidebar 240 px wide at
  ≥1366, tightening to 184 px at 800).

Theme resolution was checked with the OS preference emulated both ways: light
resolves to the light tokens (`body` `#f6f7f9`, white sidebar) and dark to the
dark tokens (`body` `#0f1216`, `#171b21` sidebar), each with a matching
`color-scheme` and no overflow.

This validates layout and theming in the built bundle. It is not a
pixel-by-pixel design review, and it exercises the browser build's degraded
states for the pages that need the Rust backend.

### Core workflow

The primary user journey is covered by the existing real tests: the frontend
journey suite (`src/app/journeys.test.tsx`) walks drives → browse → select →
composer → start → queue → completion → history with only the Rust boundary
mocked, and the Rust suite exercises copy, move, every conflict strategy,
pause / resume / cancel cleanliness, verification verdicts, move-source
retention, and recovery classification against real files on disk. The
release-artifact smoke test launches the packaged binary.

## Cross-platform validation

| Target | What was done | Result |
| --- | --- | --- |
| Windows | Built the release binary, NSIS installer, and MSI; ran the artifact smoke test; ran the full Rust suite (real files) | **Validated** |
| Linux | Source audit of every `#[cfg]` arm; frontend POSIX-path coverage in the existing tests; attempted `cargo check --target x86_64-unknown-linux-gnu` | **Not build-validated** |
| macOS | Source audit only | **Not build-validated** |

The Linux cross-check could not run on this host: it fails inside third-party
build scripts that need a Linux sysroot and `pkg-config`
(`libdbus-sys` first, and the GTK/WebKit stack behind it), not in CrossPort's
own code:

```
pkg_config failed: pkg-config has not been configured to support cross-compilation.
```

macOS cannot be cross-built from Windows at all (it needs the Apple SDK). The
audit found no platform-specific defect in the application layer: the frontend
handles both path separators and POSIX mount points (`/`, `/Volumes/...`,
`/media/...`, `/run/media/...`) in its tests, no Windows-specific wording
appears in user-facing text, every `windows-sys` call sits inside a
`#[cfg(windows)]` function with a working non-Windows arm, and the frontend
reads the host platform from the backend rather than assuming one.

**Linux and macOS remain foundation only — not supported, not claimed.**

## Artifacts and checksums

`bash scripts/release.sh` wrote, under
`apps/desktop/src-tauri/target/release/bundle/`:

| Artifact | Path |
| --- | --- |
| NSIS installer | `nsis/CrossPort_1.1.0_x64-setup.exe` |
| WiX MSI | `msi/CrossPort_1.1.0_x64_en-US.msi` |
| Executable | `release/CrossPort.exe` |
| Checksums | `bundle/checksums.txt` (SHA-256 per artifact) |

Verify with:

```bash
cd apps/desktop/src-tauri/target/release/bundle
sha256sum -c checksums.txt
```

Both digests verified `OK` for this build, and the built `CrossPort.exe` reports
version `1.1.0`. The digest values are not recorded here on purpose: they change
with every build, and `checksums.txt` is the authoritative list that ships
beside the artifacts.

### Artifact smoke test

`cargo test --test artifact_smoke -- --nocapture` copied `CrossPort.exe` alone
into a temporary directory and launched it with Node, pnpm, Cargo, and the
repository scrubbed from its environment:

```
ARTIFACT startup line: 1319ms, window: 1319ms
ARTIFACT clean close: 100ms, exit code Some(0)
```

The packaged application starts outside the repository, writes its startup line
to its own log, shows a window, and exits cleanly when that window is closed.

## Known limitations

All limitations from 1.0.0 still apply (see `RELEASE_1.0.md`): Windows only, not
code-signed, one transfer at a time, bounded history, verification proves only
the claims it lists, no byte-offset resume, lexical (not canonical) path
containment, and long paths needing Windows support. In addition, for this
cycle:

- **The MSI was built but not installed** (it requires elevation), and the
  NSIS install / reinstall / uninstall cycle was not re-run against the 1.1.0
  installers. The installer smoke evidence for 1.1.0 is the artifact smoke test
  above.
- **Linux and macOS were not compiled or run.** Only the source audit above was
  performed.
- **A downgrade is not refused.** `allowDowngrades` is `false`, but the NSIS
  template only enforces it on the interactive reinstall page, where a
  downgrade requires uninstalling the newer version first; a silent install
  replaces the newer build outright. The upgrade from 1.0.0 to 1.1.0 was
  exercised and works, and a downgraded build refuses a newer history document
  rather than corrupting it.

## Release procedure

1. `bash scripts/check-versions.sh`
2. The full gate (see the table above).
3. `bash scripts/release.sh` — builds the frontend, builds with `--locked`,
   and writes `checksums.txt`.
4. `cargo test --test artifact_smoke -- --nocapture`
5. `sha256sum -c checksums.txt` from the bundle directory.
6. Tag `v1.1.0` at the validated commit and attach the installers plus
   `checksums.txt` by hand. No workflow builds or publishes anything.

## Not supported (deliberately out of scope)

Unchanged from 1.0.0: folder synchronization, scheduling, cloud storage and
accounts, telemetry, update checking, remote filesystem access, macOS/Linux
packaging, and code signing. See `NON_GOALS.md` and
`docs/product/FEATURE_SPECIFICATION.md`.
