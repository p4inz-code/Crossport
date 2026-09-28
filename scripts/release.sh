#!/usr/bin/env bash
# Produces the release artifacts: checks version sync, builds the frontend, then
# builds and bundles the desktop application. Finishes by writing a checksums
# file next to the bundles so an artifact can be verified after it is copied.
#
# Nothing is published anywhere: this script only writes into the repository's
# own build directories.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/check-versions.sh
pnpm --filter desktop build

# `--locked` makes the release build fail rather than quietly resolve a
# dependency the lockfile does not pin; the pass-through arguments after `--`
# go to cargo.
pnpm --filter desktop exec tauri build -- --locked

BUNDLE_DIR="apps/desktop/src-tauri/target/release/bundle"
if [ ! -d "$BUNDLE_DIR" ]; then
  echo "no bundle directory at $BUNDLE_DIR: nothing to checksum" >&2
  exit 1
fi

CHECKSUMS="$BUNDLE_DIR/checksums.txt"
: > "$CHECKSUMS"

# Every installer and installer-adjacent artifact, deepest last, so the file is
# stable between runs on the same numbers of files.
while IFS= read -r artifact; do
  relative="${artifact#"$BUNDLE_DIR"/}"
  digest="$(sha256sum "$artifact" | cut -d' ' -f1)"
  printf '%s  %s\n' "$digest" "$relative" | tee -a "$CHECKSUMS"
done < <(find "$BUNDLE_DIR" -type f \
  \( -name '*.msi' -o -name '*.exe' -o -name '*.zip' -o -name '*.sig' \) \
  | sort)

echo "release artifacts written under $BUNDLE_DIR (checksums: $CHECKSUMS)"
