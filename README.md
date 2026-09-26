# CrossPort

A fast, reliable, cross-platform file transfer utility for Windows, macOS, and Linux.

Free forever. Offline first. No accounts, no ads, no telemetry.

## Status

Phase 2 (volumes and filesystem browsing) is complete. The repository currently
provides:

- A Tauri 2 desktop shell (React 19 + TypeScript + Vite frontend, Rust backend)
- A design-token-driven UI system with light/dark/system themes
- A typed IPC layer with structured `{ code, message }` errors end to end
- Backend-owned, validated, persisted settings
- Volume detection with real metadata: kind (fixed, removable, network, optical,
  RAM disk, or unknown), volume name, filesystem type, total/free/used capacity,
  read-only flag, and mounted status. Windows volumes are probed through the
  Win32 volume APIs; anything a platform cannot report stays unknown
- A storage browser: pick a volume, open its folders, navigate back and up,
  refresh, and see every entry with its size, modification time, and kind
- Safe navigation: every path is validated in Rust before it is read, listings
  never recurse, and symlinks/reparse points are reported but never followed
- A filesystem foundation (path normalization, directory validation, metadata,
  single-directory listing)
- A platform abstraction (OS identity, app directories, volumes)
- A native folder picker hosted in Rust
- Windows-safe production logging (stdout + rotating per-app log file)
- A restrictive CSP and a minimal Tauri capability set
- Real test suites (Vitest + `cargo test`) and CI

Transfers and history are **not** implemented yet — they are Phase 3 work and
are deliberately absent rather than stubbed. See
[`ROADMAP.md`](ROADMAP.md) and [`docs/product/FEATURE_SPECIFICATION.md`](docs/product/FEATURE_SPECIFICATION.md).

## Repository layout

| Path | Purpose |
| --- | --- |
| `apps/desktop/` | The single Tauri desktop application |
| `apps/desktop/src/` | React frontend (features, stores, services, UI primitives) |
| `apps/desktop/src-tauri/` | Rust backend (commands, platform, filesystem, settings, errors) |
| `docs/` | Architecture, design, development, and product documentation |
| `scripts/` | Development and release scripts |
| `tests/` | Cross-cutting testing notes |

## Requirements

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

## Verification

Every change must keep the full suite green:

```bash
pnpm lint               # ESLint
pnpm check              # Biome
pnpm --filter desktop build   # typecheck + production build
pnpm test               # Vitest (frontend)
cd apps/desktop/src-tauri
cargo fmt --check       # Rust formatting
cargo check             # Rust typecheck
cargo test              # Rust tests
bash scripts/check-versions.sh   # version sync across manifests
```

`scripts/lint.sh`, `scripts/test.sh`, and the [CI workflow](.github/workflows/ci.yml)
orchestrate the same commands.

## Architecture in one paragraph

The frontend is feature-folder based with shared stores (Zustand), services
that own all IPC (`src/services/`), and token-driven UI primitives. Every
backend call goes through one transport module that validates payloads and
normalizes failures into `IpcError` with a stable code. The Rust backend owns
user settings, persists them to the platform app-config directory, and exposes
commands for settings, platform facts, volume detection, directory listing,
path inspection, and the native folder picker. Filesystem and platform work stays in Rust, so the
webview is granted only Tauri core defaults.

See [`docs/architecture/SYSTEM_ARCHITECTURE.md`](docs/architecture/SYSTEM_ARCHITECTURE.md)
for details.

## License

MIT — see [LICENSE](LICENSE).
