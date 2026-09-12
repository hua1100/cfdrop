#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
workflow="$repo_root/.github/workflows/release.yml"
mutated=$(mktemp)

cleanup() {
  rm -f "$mutated"
}
trap cleanup EXIT

awk '
  /^  macos:$/ { in_macos = 1 }
  in_macos && /^    runs-on: macos-14$/ {
    print "    runs-on: ubuntu-latest"
    print "    #    runs-on: macos-14"
    changed = 1
    next
  }
  { print }
  END { if (!changed) exit 2 }
' "$workflow" > "$mutated"

if "$repo_root/tests/release_workflow_contract.sh" "$mutated" >/dev/null 2>&1; then
  echo 'release workflow contract accepted a commented runner decoy' >&2
  exit 1
fi
