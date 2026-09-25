#!/usr/bin/env bash
# Asserts that every declared version in the repository agrees with the root
# package.json. Run in CI so a release bump can never drift across the
# package manifest, the Tauri config, and the Rust crate.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Resolve paths relative to the repo root so this works in git-bash on
# Windows, where node cannot load absolute /c/... paths.
ROOT_VERSION="$(cd "$ROOT" && node -p "require('./package.json').version")"
DESKTOP_VERSION="$(cd "$ROOT" && node -p "require('./apps/desktop/package.json').version")"
TAURI_VERSION="$(cd "$ROOT" && node -p "JSON.parse(require('fs').readFileSync('./apps/desktop/src-tauri/tauri.conf.json', 'utf8')).version")"
CARGO_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/apps/desktop/src-tauri/Cargo.toml" | head -n 1)"

FAILED=0
for pair in "desktop:$DESKTOP_VERSION" "tauri:$TAURI_VERSION" "cargo:$CARGO_VERSION"; do
  label="${pair%%:*}"
  value="${pair#*:}"
  if [ "$value" != "$ROOT_VERSION" ]; then
    echo "version mismatch: root=$ROOT_VERSION but $label=$value" >&2
    FAILED=1
  fi
done

if [ "$FAILED" -ne 0 ]; then
  exit 1
fi

echo "all versions in sync: $ROOT_VERSION"
