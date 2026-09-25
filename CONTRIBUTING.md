# Contributing to CrossPort

Thanks for helping build CrossPort.

## Code of conduct

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). Participation
in the project is subject to it.

## Development setup

1. Install [pnpm](https://pnpm.io) and a stable Rust toolchain.
2. `pnpm install` (or `./scripts/bootstrap.sh`).
3. `pnpm dev` for the frontend; `pnpm --filter desktop exec tauri dev` for the
   full desktop app.

## Before opening a pull request

- Run the full verification suite (see [README](README.md#verification)):
  `pnpm lint`, `pnpm check`, `pnpm --filter desktop build`, `pnpm test`,
  `cargo fmt --check`, `cargo check`, `cargo test`.
- Keep versions in sync (`bash scripts/check-versions.sh`).
- Write tests for new logic. The suite must run real tests — CI fails if it
  doesn't.
- Prefer the simplest correct change. Readable over clever, reliable over
  feature-rich.

## Engineering principles

See [`docs/development/CODING_RULES.md`](docs/development/CODING_RULES.md) and
[`docs/product/PRODUCT_CONSTITUTION.md`](docs/product/PRODUCT_CONSTITUTION.md).
