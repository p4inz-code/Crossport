# CrossPort 1.1.1

A Windows file-transfer utility that moves files between your drives — with a
review of what will happen before it starts, and proof of what landed when it
finishes.

---

## What's new

- **Fixed: transfers now work from the interface.** In 1.1.0 a value the
  interface needs was missing from the data the backend sent back about a
  transfer. Starting a copy or move reported an "unexpected payload" error, and
  the Transfers page reported the queue as unavailable. 1.1.1 sends that value
  again, so transfers can be started, watched, paused, and reviewed in the app.
  The transfer engine itself was never affected — it moved files correctly the
  whole time.
- **Fixed: the queue shows real jobs again**, with live progress and the final
  verification verdict.
- **Documentation correction:** a downgrade is *permitted*, not refused. The
  interactive installer reports that a newer version is installed and replaces
  it only after uninstalling it, and a silent install replaces it outright.
- Includes the 1.1.0 interface work: a responsive layout that uses the whole
  window, one finalized logo, and a consistent design-token pass over the
  interface.

## Core capabilities

- Browse mounted volumes and directories, with real volume metadata (kind,
  filesystem, capacity, read-only state).
- Copy or move files and folders recursively.
- Resolve conflicts with Replace, Skip, or Rename.
- Review a **dry run** before anything is written: where each source lands,
  item and byte counts, free space, conflict behaviour, and warnings.
- Verify what was written: **size** (default) or **SHA-256**, with an honest
  verdict that states what was and was not checked.
- Live progress with pause, resume, and cancel.
- Durable transfer history and crash recovery with an explicit decision.

The full matrix is in
[`docs/product/CAPABILITY_MATRIX.md`](https://github.com/p4inz-code/Crossport/blob/main/docs/product/CAPABILITY_MATRIX.md).

## Validation

- Frontend: Biome, ESLint, TypeScript + Vite production build, and the Vitest
  suite (including performance budgets) all pass.
- Rust: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy
  --all-targets -- -D warnings`, and `cargo test` — 398 tests passed, including
  real-file copy, move, conflict, pause/resume/cancel, verification, and
  recovery tests — all clean.
- A regression test now asserts that the transfer data sent to the interface
  includes its verification verdict.
- Real-file workflow: a copy was executed through the application's own backend
  and the interface confirmed the job completed with a SHA-256 verification
  verdict.
- Artifact smoke test: the packaged executable starts outside the repository
  with no development tools in its environment, shows a window, and exits
  cleanly.

## Platform status

| Platform | Status |
| --- | --- |
| **Windows 10 1607+ / Windows 11, 64-bit** | **Supported.** The only packaged and tested target. |
| Linux | Not built, not tested, not supported — application foundation only. |
| macOS | Not built, not tested, not supported — application foundation only. |

CrossPort is an application, not a filesystem driver. It does not mount, format,
or repair volumes, and it does not add native NTFS write access on macOS.

## Known limitations

- Windows only; no macOS or Linux build.
- The installers are **not code-signed**, so Windows shows an unknown-publisher
  warning on first install.
- A downgrade is permitted rather than refused; there is no auto-updater.
- One transfer runs at a time.
- History is bounded (200 records by default, 2,000 at most).
- Verification proves only the claims it lists; modified times, attributes, and
  alternate data streams are not reapplied.
- An interrupted transfer is restarted or discarded whole — there is no
  byte-offset resume.
- Paths longer than 260 characters need Windows long-path support enabled.

## Installation

1. Download **`CrossPort_1.1.1_x64-setup.exe`** (recommended) or
   **`CrossPort_1.1.1_x64_en-US.msi`**.
2. Run it. The NSIS installer installs for the current user into
   `%LOCALAPPDATA%\CrossPort` and adds a Start-menu entry and a desktop
   shortcut. No account is required.
3. Start CrossPort from the Start menu.

Verify the download against `checksums.txt`:

```bash
sha256sum -c checksums.txt
```

To uninstall, use **Apps → Installed apps → CrossPort → Uninstall**. Your
settings, history, and logs are kept under `%APPDATA%\com.crossport.app` and
`%LOCALAPPDATA%\com.crossport.app` and are never removed for you.

---

CrossPort is free and offline: no account, no telemetry, no background
synchronization. MIT licensed.
