#!/usr/bin/env bash
# Builds the desktop frontend for production.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pnpm --filter desktop build
