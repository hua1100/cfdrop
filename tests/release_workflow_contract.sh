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

has_exact_sequence() {
  local content=$1
  shift
  local expected=("$@")
  local expected_count=${#expected[@]}
  local matched=0
  local line

  while IFS= read -r line; do
    if [[ "$line" == "${expected[$matched]}" ]]; then
      ((matched += 1))
      if ((matched == expected_count)); then
        return 0
      fi
    elif [[ "$line" == "${expected[0]}" ]]; then
      matched=1
    else
      matched=0
    fi
  done <<< "$content"

  return 1
}

has_macos_draft_check() {
  has_exact_sequence "$macos_job" \
    '          release_is_draft=$(gh release view "$GITHUB_REF_NAME" \' \
    '            --repo "$GITHUB_REPOSITORY" \' \
    '            --json isDraft \' \
    "            --jq '.isDraft')"
}

has_macos_fail_closed_guard() {
  has_exact_sequence "$macos_job" \
    '          if [ "$release_is_draft" != "true" ]; then' \
    '            echo "::error::refusing to upload assets to a published release"' \
    '            exit 1' \
    '          fi'
}

has_macos_upload() {
  has_exact_sequence "$macos_job" \
    '          gh release upload "$GITHUB_REF_NAME" \' \
    '            cfdrop-macos-arm64.tar.gz \' \
    '            --repo "$GITHUB_REPOSITORY" \' \
    '            --clobber'
}

macos_line_number() {
  local exact_line=$1
  local line
  local line_number=0

  while IFS= read -r line; do
    ((line_number += 1))
    if [[ "$line" == "$exact_line" ]]; then
      printf '%s\n' "$line_number"
      return 0
    fi
  done <<< "$active_macos_job"
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

if ! has_macos_draft_check || ! has_macos_fail_closed_guard; then
  echo 'macOS release upload must verify the draft with gh before uploading' >&2
  exit 1
fi

if ! has_macos_upload; then
  echo 'macOS release upload must use the guarded gh upload contract' >&2
  exit 1
fi

draft_check_line=$(macos_line_number '          release_is_draft=$(gh release view "$GITHUB_REF_NAME" \')
guard_line=$(macos_line_number '          if [ "$release_is_draft" != "true" ]; then')
guard_end_line=$(macos_line_number '          fi')
archive_line=$(macos_line_number '          tar -czf cfdrop-macos-arm64.tar.gz -C target/release cfdrop')
upload_line=$(macos_line_number '          gh release upload "$GITHUB_REF_NAME" \')

if [[ -z "$draft_check_line" || -z "$guard_line" || -z "$guard_end_line" ||
  -z "$archive_line" || -z "$upload_line" ]] ||
  (( draft_check_line >= guard_line || guard_end_line >= archive_line || archive_line >= upload_line )); then
  echo 'macOS release archive and upload must run only after the complete draft guard' >&2
  exit 1
fi
