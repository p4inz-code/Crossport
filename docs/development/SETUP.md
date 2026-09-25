# Setup

## Requirements

- Node.js >= 22
- pnpm 10
- A stable Rust toolchain
- Platform prerequisites for Tauri 2
  (https://v2.tauri.app/start/prerequisites/)

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
| Production build | `./scripts/build.sh` |
| Tests | `./scripts/test.sh` |
| Static checks | `./scripts/lint.sh` |

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
- Logs: the platform app-log directory; the plugin writes and rotates a
  `crossport`-named file there — 5 MiB per file, three files kept
  (e.g. Windows `%LOCALAPPDATA%\{identifier}\logs`).
- Startup logs print both resolved directories, which is the quickest way to
  locate them on a given machine.
