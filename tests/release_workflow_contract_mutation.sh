#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
workflow="$repo_root/.github/workflows/release.yml"
mutated=$(mktemp)
system_awk=$(command -v awk)

cleanup() {
  rm -f "$mutated"
}
trap cleanup EXIT

if ! CFDROP_SYSTEM_AWK="$system_awk" \
  PATH="$repo_root/tests/fixtures/mawk-bin:$PATH" \
  "$repo_root/tests/release_workflow_contract.sh" "$workflow"; then
  echo 'release workflow contract must pass with mawk-compatible -v escaping' >&2
  exit 1
fi

expect_rejected() {
  local description=$1
  if "$repo_root/tests/release_workflow_contract.sh" "$mutated" >/dev/null 2>&1; then
    echo "release workflow contract accepted mutation: $description" >&2
    exit 1
  fi
}

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

expect_rejected 'commented runner decoy'

awk '
  /^  macos:$/ { in_macos = 1 }
  in_macos && /release_is_draft=\$\(gh release view/ {
    print "          release_is_draft=true"
    print "          # release_is_draft=$(gh release view \"$GITHUB_REF_NAME\" \\"
    changed = 1
    next
  }
  { print }
  END { if (!changed) exit 2 }
' "$workflow" > "$mutated"

expect_rejected 'commented draft-check decoy'

awk '
  /^  macos:$/ { in_macos = 1 }
  in_macos && /^  [A-Za-z0-9_-]+:$/ && $0 != "  macos:" { in_macos = 0 }
  in_macos && /^            exit 1$/ {
    print "            # exit 1"
    changed = 1
    next
  }
  { print }
  END { if (!changed) exit 2 }
' "$workflow" > "$mutated"

expect_rejected 'commented fail-closed exit decoy'

awk '
  /^  macos:$/ { in_macos = 1 }
  in_macos && /^  [A-Za-z0-9_-]+:$/ && $0 != "  macos:" { in_macos = 0 }
  in_macos && /release_is_draft=\$\(gh release view/ && !inserted {
    print "          gh release upload \"$GITHUB_REF_NAME\" \\"
    print "            cfdrop-macos-arm64.tar.gz \\"
    print "            --repo \"$GITHUB_REPOSITORY\" \\"
    print "            --clobber"
    inserted = 1
  }
  in_macos && /^          gh release upload \"\$GITHUB_REF_NAME\" \\$/ {
    getline
    getline
    getline
    removed = 1
    next
  }
  { print }
  END { if (!inserted || !removed) exit 2 }
' "$workflow" > "$mutated"

expect_rejected 'upload before the draft guard'

awk '
  /^  macos:$/ { in_macos = 1 }
  in_macos && /gh release upload \"\$GITHUB_REF_NAME\"/ {
    print "          echo upload skipped"
    print "          # gh release upload \"$GITHUB_REF_NAME\" \\"
    changed = 1
    next
  }
  { print }
  END { if (!changed) exit 2 }
' "$workflow" > "$mutated"

expect_rejected 'commented upload decoy'

awk '
  /^  macos:$/ { in_macos = 1 }
  in_macos && /cargo build --release/ {
    print
    print "          curl -sf \"https://api.github.com/repos/$GITHUB_REPOSITORY/releases/tags/$GITHUB_REF_NAME\""
    changed = 1
    next
  }
  { print }
  END { if (!changed) exit 2 }
' "$workflow" > "$mutated"

expect_rejected 'draft-incompatible curl metadata lookup'

awk '
  /^  macos:$/ { in_macos = 1 }
  in_macos && /^            --clobber$/ {
    print "            # --clobber"
    changed = 1
    next
  }
  { print }
  END { if (!changed) exit 2 }
' "$workflow" > "$mutated"

expect_rejected 'commented clobber decoy'
