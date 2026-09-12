#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
workflow="$repo_root/.github/workflows/release.yml"
macos_job=$(sed -n '/^  macos:/,/^  publish-release:/p' "$workflow")

if ! grep -Fq '    runs-on: macos-14' <<< "$macos_job"; then
  echo 'release workflow must build macOS artifacts on GitHub-hosted macos-14' >&2
  exit 1
fi

if grep -Eq 'oablab-macos|macmini runner|no gh CLI on this runner' "$workflow"; then
  echo 'release workflow still contains the retired self-hosted runner contract' >&2
  exit 1
fi
