# CrossPort Desktop

The Tauri 2 desktop application for CrossPort — a fast, reliable,
cross-platform file transfer utility.

## Stack

- Tauri 2 + Rust backend (`src-tauri/`)
- React 19 + TypeScript + Vite
- Zustand (state), Zod (validation), Lucide (icons), React Router (routing)

## Architecture

The frontend is organized into:

| Path             | Purpose                                              |
| ---------------- | ---------------------------------------------------- |
| `src/app/`       | Application shell, providers, root routing           |
| `src/components/`| Reusable UI primitives (Button, Card, EmptyState…)   |
| `src/features/`  | Feature folders (drives, transfer, history, settings)|
| `src/hooks/`     | Shared hooks (e.g. `useThemeMode`)                   |
| `src/layouts/`   | AppShell, TopBar, Sidebar, StatusBar, PageContainer  |
| `src/lib/`       | Framework-agnostic utilities and constants          |
| `src/services/`  | Persistence and backend invocation services         |
| `src/stores/`    | Zustand stores (app, settings, transfer, drive, history) |
| `src/styles/`    | Design tokens and base styles                       |
| `src/types/`     | Shared domain types                                 |

The Rust backend (`src-tauri/src/`) mirrors the same domains with
`commands/`, `filesystem/`, `transfer/`, `history/`, `platform/`,
`settings/`, `models/`, `errors/`, and `state/` modules, all wired through a
single managed `AppState`.

## Development

```bash
pnpm install
pnpm dev       # Vite dev server
pnpm build     # TypeScript + Vite production build
pnpm lint      # ESLint
```

To run inside Tauri:

```bash
pnpm --filter desktop exec tauri dev
```

## Design tokens

All visual decisions flow from the CSS custom properties in
`src/styles/tokens/`. Never hardcode colors, spacing, radius, typography,
elevation, or transitions in components — use the tokens.
