#!/usr/bin/env bash
# Audit the GitHub workflows with zizmor and jactionlint at their pedantic levels:
# scripts/workflows.sh. Fails on any finding, when jactionlint examined no files, and when
# it cannot run shellcheck, since it then drops the run: script rule and still passes.
set -euo pipefail

zizmor --persona=pedantic .github

log=$(mktemp)
trap 'rm -f "$log"' EXIT
status=0
jactionlint --profile pedantic --verbose 2>"$log" || status=$?
grep -v '^verbose: ' "$log" >&2 || true
if grep -q 'Rule "shellcheck" was disabled' "$log"; then
  echo "workflows: jactionlint could not run shellcheck over the run: scripts" >&2
  exit 1
fi
summary=$(sed -n 's/^verbose: \(Found [0-9]* errors in \([0-9]*\) files\)$/\1/p' "$log")
files=$(sed -n 's/^verbose: Found [0-9]* errors in \([0-9]*\) files$/\1/p' "$log")
if [ -z "$files" ] || [ "$files" -eq 0 ]; then
  echo "workflows: jactionlint examined no workflow files" >&2
  exit 1
fi
echo "jactionlint: $summary"
exit "$status"
