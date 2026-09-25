#!/usr/bin/env bash
# Runs all test suites (frontend + backend).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pnpm test
(cd apps/desktop/src-tauri && cargo test)
