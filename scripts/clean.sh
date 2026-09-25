#!/usr/bin/env bash
# Removes build artifacts and installed dependencies.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

rm -rf node_modules apps/desktop/node_modules
rm -rf apps/desktop/dist
rm -rf apps/desktop/src-tauri/target
