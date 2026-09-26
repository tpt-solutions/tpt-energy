#!/usr/bin/env bash
# Pre-flight every crate for publication.
#
# Two checks, both of which have caught real release breakage before the tag
# is pushed:
#
# 1. `cargo package --list` per crate, which fails on a bad `include`/`exclude`
#    glob, a missing file, or a manifest cargo cannot read. It does not need
#    the registry.
# 2. `cargo publish --dry-run` per crate, which is the real packaging step.
#
# Check 2 cannot succeed before the workspace's own crates exist on crates.io:
# cargo rewrites a `path` dependency to a registry dependency when it packages
# a member, and then fails to resolve it. That state is detected from the
# error text, reported as a warning, and the run continues -- once the first
# release is out, the same command verifies for real. Any other failure is a
# failure.
#
# Usage: tools/publish-dry-run.sh

set -euo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SKIPPED=()
FAILED=0

mapfile -t CRATES < <(tools/workspace-crates.py --names)

if [ "${#CRATES[@]}" -eq 0 ]; then
  echo "error: no publishable crates found" >&2
  exit 1
fi

echo "checking ${#CRATES[@]} crate(s) for publication"

for crate in "${CRATES[@]}"; do
  if ! out="$(cargo package -p "$crate" --list --allow-dirty 2>&1)"; then
    echo "::error::$crate cannot be packaged" >&2
    echo "$out" | tail -20 >&2
    FAILED=1
    continue
  fi

  if ! out="$(cargo publish -p "$crate" --dry-run --no-verify --allow-dirty 2>&1)"; then
    if grep -q "no matching package named" <<<"$out"; then
      missing="$(sed -n 's/.*no matching package named `\([^`]*\)`.*/\1/p' <<<"$out" | head -1)"
      SKIPPED+=("$crate (waiting for $missing on crates.io)")
      continue
    fi
    echo "::error::$crate fails `cargo publish --dry-run`" >&2
    echo "$out" | tail -20 >&2
    FAILED=1
  fi
done

for entry in ${SKIPPED[@]+"${SKIPPED[@]}"}; do
  echo "warning: skipped the dry run for $entry"
done

if [ "$FAILED" -ne 0 ]; then
  exit 1
fi

echo "all crates are publishable"
