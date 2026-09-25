#!/usr/bin/env bash
# Installs all workspace dependencies.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pnpm install
