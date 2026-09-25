#!/usr/bin/env bash
# Verifies version sync, builds the frontend, and produces the desktop bundle.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

./scripts/check-versions.sh
pnpm --filter desktop build
pnpm --filter desktop exec tauri build
