#!/usr/bin/env bash
# Build the publishable `tpt-nrg-wasm` npm package.
#
# The `npm/` directory next to the crate holds the hand-written package
# metadata that `wasm-pack` does not generate (name, license, repository,
# keywords, file list). This script runs `wasm-pack`, copies that metadata and
# the repository licences over the generated package, and refreshes the
# checked-in TypeScript declarations so a drift check in CI has something to
# compare against.
#
# Usage:
#   tools/build-npm-package.sh [OUT_DIR]
#
# OUT_DIR defaults to `dist/npm` and is created if missing. Pass `--check` to
# build into a temporary directory and fail if the checked-in `.d.ts` differs
# from the freshly generated one, without touching the working tree. Pass
# `--no-opt` (or set `WASM_PACK_NO_OPT=1`) to skip the `wasm-opt` pass in
# environments where that binary cannot run.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/crates/core/tpt-nrg-wasm"
NPM_DIR="$CRATE_DIR/npm"
PKG_NAME="tpt_nrg_wasm"
DECLARATIONS="$NPM_DIR/$PKG_NAME.d.ts"

# On Windows the binary is `wasm-pack.exe`, which `command -v wasm-pack` does
# not always resolve under Git Bash / WSL.
WASM_PACK="${WASM_PACK:-wasm-pack}"
if ! command -v "$WASM_PACK" >/dev/null 2>&1 && command -v "$WASM_PACK.exe" >/dev/null 2>&1; then
  WASM_PACK="$WASM_PACK.exe"
fi

MODE="build"
NO_OPT="${WASM_PACK_NO_OPT:-0}"
OUT_DIR="$REPO_ROOT/dist/npm"
for arg in "$@"; do
  case "$arg" in
    --check)
      MODE="check"
      ;;
    --no-opt)
      NO_OPT=1
      ;;
    -*)
      echo "unknown option: $arg" >&2
      echo "usage: $(basename "$0") [--check] [--no-opt] [OUT_DIR]" >&2
      exit 2
      ;;
    *)
      OUT_DIR="$arg"
      ;;
  esac
done
if [ "$MODE" = "check" ]; then
  OUT_DIR="$(mktemp -d)"
  trap 'rm -rf "$OUT_DIR"' EXIT
fi

# A Windows `wasm-pack.exe` cannot read MSYS/WSL paths such as `/mnt/d/...`,
# so hand it a Windows path when this shell and that binary meet. `cygpath`
# comes with Git Bash, `wslpath` with WSL.
native_path() {
  case "$1" in
    /*)
      if command -v cygpath >/dev/null 2>&1; then
        cygpath -m "$1"
        return
      fi
      if command -v wslpath >/dev/null 2>&1; then
        wslpath -m "$1"
        return
      fi
      ;;
  esac
  printf '%s' "$1"
}

if ! command -v "$WASM_PACK" >/dev/null 2>&1; then
  echo "wasm-pack is not installed: cargo install wasm-pack" >&2
  exit 1
fi

# wasm-pack's own flags must come *before* the crate path; anything after the
# path is forwarded verbatim to `cargo build`, which is how `--features wasm`
# reaches cargo.
#
# `--no-opt` skips the `wasm-opt` pass, for environments where that binary
# cannot run (some Windows sandboxes). The published artefact is slightly
# larger; the JavaScript and TypeScript surface is identical.
WASM_OPT_FLAGS=()
if [ "$NO_OPT" = "1" ]; then
  WASM_OPT_FLAGS=(--no-opt)
fi

"$WASM_PACK" build \
  --target web \
  --release \
  "${WASM_OPT_FLAGS[@]}" \
  --out-dir "$(native_path "$OUT_DIR")" \
  --out-name "$PKG_NAME" \
  "$(native_path "$CRATE_DIR")" \
  --features wasm

# Replace the generated metadata with the published one.
cp "$NPM_DIR/package.json" "$OUT_DIR/package.json"
cp "$NPM_DIR/README.md" "$OUT_DIR/README.md"
cp "$REPO_ROOT/LICENSE-MIT" "$OUT_DIR/LICENSE-MIT"
cp "$REPO_ROOT/LICENSE-APACHE" "$OUT_DIR/LICENSE-APACHE"
rm -f "$OUT_DIR/.gitignore"

if [ "$MODE" = "check" ]; then
  if ! diff -u "$DECLARATIONS" "$OUT_DIR/$PKG_NAME.d.ts"; then
    echo "error: the checked-in TypeScript declarations are stale." >&2
    echo "       run tools/build-npm-package.sh and commit the result." >&2
    exit 1
  fi
  echo "TypeScript declarations are up to date."
else
  cp "$OUT_DIR/$PKG_NAME.d.ts" "$DECLARATIONS"
  echo "npm package built in $OUT_DIR"
  echo "refreshed $DECLARATIONS"
  echo
  echo "publish with:"
  echo "  cd $OUT_DIR && npm publish --access public"
fi
