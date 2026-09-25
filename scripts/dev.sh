#!/usr/bin/env bash
# Starts the frontend dev server.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pnpm dev
