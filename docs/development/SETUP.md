# Setup

## Requirements

- Node.js >= 22
- pnpm 10
- A stable Rust toolchain (`rustc 1.97.1` is the version the current release
  was built and tested with)
- Platform prerequisites for Tauri 2
  (https://v2.tauri.app/start/prerequisites/)

Running a built application needs Windows 10 1607+ or Windows 11 and the
WebView2 runtime; the installers bring the runtime with them when it is
missing. See `README.md` for the end-user requirements.

## Install

```bash
pnpm install
```

or `./scripts/bootstrap.sh`.

## Run

| Task | Command |
| --- | --- |
| Frontend dev server | `pnpm dev` (http://localhost:5173, browser mode) |
| Full desktop app | `pnpm --filter desktop exec tauri dev` |
| Production frontend build | `./scripts/build.sh` |
| Release build + installers | `./scripts/release.sh` |
| Tests | `./scripts/test.sh` |
| Static checks | `./scripts/lint.sh` |
| Performance numbers | `cargo test --lib measure -- --ignored --nocapture` (in `apps/desktop/src-tauri`) |
| Built-artifact smoke test | `cargo test --test artifact_smoke -- --nocapture` (after a release build) |

Builds land in `apps/desktop/src-tauri/target/release/`; installers and their
checksums land in `target/release/bundle/`.

## Where things live

- Frontend source: `apps/desktop/src`
- Backend source: `apps/desktop/src-tauri/src`
- Design tokens: `apps/desktop/src/styles/tokens`

## Browser dev mode

The frontend also runs in a plain browser, but the backend commands do not
exist there. Settings fall back to validated localStorage; drive enumeration,
platform facts, path inspection, and the native folder picker report
`unavailable` instead of pretending to work. Use `tauri dev` for those.

## Runtime data locations (Tauri)

- Settings: the platform app-config directory, file `settings.json`
  (e.g. Windows `%APPDATA%\{identifier}\settings.json`).
- History and interrupted-transfer state: the same config directory, as
  `transfer-history.json` and `transfer-state.json`.
- Logs: the platform app-log directory; the plugin writes and rotates a
  `crossport`-named file there — 5 MiB per file, three files kept
  (e.g. Windows `%LOCALAPPDATA%\{identifier}\logs`).
- WebView2 profile: the platform local-app-data directory
  (e.g. Windows `%LOCALAPPDATA%\{identifier}\EBWebView`).
- Startup logs print the resolved config and log directories, which is the
  quickest way to locate them on a given machine.

## Toolchain note

The release profile pins `tauri-utils` to `opt-level = 0`: rustc 1.97.1 on
Windows segfaults while optimizing that crate, reproducibly, at every higher
level. The override is documented next to the profile in
`apps/desktop/src-tauri/Cargo.toml` and can be deleted once the toolchain no
longer crashes.
