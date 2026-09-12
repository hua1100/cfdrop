#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
workflow=${1:-"$repo_root/.github/workflows/release.yml"}

job_block() {
  local job_name=$1
  awk -v header="  ${job_name}:" '
    $0 == header { in_job = 1 }
    in_job && $0 ~ /^  [A-Za-z0-9_-]+:$/ && $0 != header { exit }
    in_job { print }
  ' "$workflow"
}

macos_job=$(job_block macos)
create_release_job=$(job_block create-release)

active_macos_job=$(sed '/^[[:space:]]*#/d' <<< "$macos_job")

has_macos_draft_check() {
  awk \
    -v first='          release_is_draft=$(gh release view "$GITHUB_REF_NAME" \' \
    -v second='            --repo "$GITHUB_REPOSITORY" \' \
    -v third='            --json isDraft \' \
    -v fourth="            --jq '.isDraft')" '
      $0 == first {
        getline
        if ($0 != second) next
        getline
        if ($0 != third) next
        getline
        if ($0 == fourth) found = 1
      }
      END { exit(found ? 0 : 1) }
    ' <<< "$macos_job"
}

has_macos_upload() {
  awk \
    -v first='          gh release upload "$GITHUB_REF_NAME" \' \
    -v second='            cfdrop-macos-arm64.tar.gz \' \
    -v third='            --repo "$GITHUB_REPOSITORY" \' \
    -v fourth='            --clobber' '
      $0 == first {
        getline
        if ($0 != second) next
        getline
        if ($0 != third) next
        getline
        if ($0 == fourth) found = 1
      }
      END { exit(found ? 0 : 1) }
    ' <<< "$macos_job"
}

if ! grep -Eq '^      - uses: actions/checkout@v4$' <<< "$create_release_job" ||
  ! grep -Eq '^          tests/release_workflow_contract\.sh$' <<< "$create_release_job" ||
  ! grep -Eq '^          tests/release_workflow_contract_mutation\.sh$' <<< "$create_release_job"; then
  echo 'create-release must run the release workflow contract tests' >&2
  exit 1
fi

contract_line=$(grep -nE '^          tests/release_workflow_contract\.sh$' <<< "$create_release_job" | cut -d: -f1)
mutation_line=$(grep -nE '^          tests/release_workflow_contract_mutation\.sh$' <<< "$create_release_job" | cut -d: -f1)
checkout_line=$(grep -nE '^      - uses: actions/checkout@v4$' <<< "$create_release_job" | cut -d: -f1)
draft_line=$(grep -nF '      - name: Create draft release' <<< "$create_release_job" | cut -d: -f1)
if (( checkout_line >= draft_line || contract_line >= draft_line || mutation_line >= draft_line )); then
  echo 'release workflow contract tests must run before draft creation' >&2
  exit 1
fi

if ! grep -Eq '^    runs-on: macos-14$' <<< "$macos_job"; then
  echo 'release workflow must build macOS artifacts on GitHub-hosted macos-14' >&2
  exit 1
fi

if grep -Eq 'oablab-macos|macmini runner|no gh CLI on this runner' "$workflow"; then
  echo 'release workflow still contains the retired self-hosted runner contract' >&2
  exit 1
fi

if grep -Eq 'releases/tags/|uploads\.github\.com|upload_url|\|[[:space:]]*python3' <<< "$active_macos_job"; then
  echo 'macOS release upload must not use the draft-incompatible REST metadata path' >&2
  exit 1
fi

if ! has_macos_draft_check ||
  ! grep -Fqx '          if [ "$release_is_draft" != "true" ]; then' <<< "$macos_job" ||
  ! grep -Fqx '            echo "::error::refusing to upload assets to a published release"' <<< "$macos_job"; then
  echo 'macOS release upload must verify the draft with gh before uploading' >&2
  exit 1
fi

if ! has_macos_upload; then
  echo 'macOS release upload must use the guarded gh upload contract' >&2
  exit 1
fi
