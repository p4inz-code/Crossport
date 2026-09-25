# CI/CD

## Continuous integration (`.github/workflows/ci.yml`)

Runs on every push to `main` and every pull request:

| Job | Commands |
| --- | --- |
| Frontend | `pnpm lint`, `pnpm check`, `pnpm --filter desktop build`, `pnpm test` |
| Backend | `cargo fmt --check`, `cargo check`, `cargo test` |
| Versions | `bash scripts/check-versions.sh` |

The frontend job uses pnpm with cached dependencies; the backend job uses
`rust-cache` scoped to the `src-tauri` workspace.

## Release (`.github/workflows/release.yml`)

Tag pushes (`v*`) trigger a matrix build (ubuntu / windows / macos) that runs
`tauri build` via `tauri-action`, producing a draft GitHub release with the
platform bundles. Manual `workflow_dispatch` runs are also supported.

## Local equivalents

`./scripts/lint.sh` and `./scripts/test.sh` mirror the CI commands, so what
passes locally passes in CI.
