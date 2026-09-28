# CI/CD

## Continuous integration (`.github/workflows/ci.yml`)

Runs on every push to `main` and every pull request.

| Job | Runner | Commands |
| --- | --- | --- |
| Frontend | ubuntu-latest | `pnpm lint`, `pnpm check`, `pnpm --filter desktop build`, `pnpm test` |
| Backend | ubuntu-latest | `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test` |
| Backend on Windows | windows-latest | `cargo fmt --check`, `cargo test` |
| Version sync | ubuntu-latest | `bash scripts/check-versions.sh` |

The frontend job uses pnpm with cached dependencies; the backend jobs use
`rust-cache` scoped to the `src-tauri` workspace.

The Windows job exists because the application ships on Windows: volume
probing, cross-volume transfers, long-path handling, and the log directory are
all platform behaviour, and before this job they were only ever exercised on the
machine that happened to build a release.

Nothing in CI builds installers: `scripts/release.sh` does that locally, and the
artifact smoke test that launches the built application is run as part of the
release pass rather than on every push (see
`docs/development/RELEASE_PROCESS.md`).

## Continuous delivery

There is none. CrossPort has no update server, no publishing automation, and no
code signing in this repository, so nothing is uploaded from CI. Releases are
built and distributed manually; `docs/development/RELEASE_PROCESS.md` is the
process.

## Local equivalents

`./scripts/lint.sh` and `./scripts/test.sh` mirror the CI commands, so what
passes locally passes in CI.
