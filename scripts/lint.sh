#!/usr/bin/env bash
# Runs all static checks: ESLint, Biome, and Rust formatting.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pnpm lint
pnpm check
(cd apps/desktop/src-tauri && cargo fmt --check)
