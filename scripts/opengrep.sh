#!/usr/bin/env bash
# Test the opengrep rules against their fixtures, then scan the repository with them:
# scripts/opengrep.sh. Fails when a rule misses or over-fires on its fixture, when a rule
# has no fixture lines, when the scan examines no files, or on any finding.
set -euo pipefail

rules=opengrep/rust.yml
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

rule_ids=$(sed -n 's/^  - id: //p' "$rules" | sort)
rule_count=$(printf '%s\n' "$rule_ids" | grep -c .)
if [ "$rule_count" -eq 0 ]; then
  echo "opengrep: $rules defines no rules" >&2
  exit 1
fi

status=0
opengrep test --json opengrep/ >"$work/test.json" 2>"$work/test.err" || status=$?
case "$status" in
  0 | 1) ;;
  *) cat "$work/test.err" >&2; echo "opengrep: test exited $status" >&2; exit 1 ;;
esac
if [ "$(jq '(.config_with_errors | length) + (.config_missing_tests | length)' "$work/test.json")" -ne 0 ]; then
  jq '{config_with_errors, config_missing_tests}' "$work/test.json" >&2
  echo "opengrep: a rule file has errors or no fixture" >&2
  exit 1
fi
jq -r '.results[].checks | to_entries[]
  | "\(.key) \(.value.passed) \([.value.matches[].expected_lines[]] | length) \([.value.matches[].reported_lines[]] | length)"' \
  "$work/test.json" | sort >"$work/checks"
tested_ids=$(cut -d' ' -f1 "$work/checks")
if [ "$tested_ids" != "$rule_ids" ]; then
  printf 'opengrep: rules %s, tested %s\n' "$(tr '\n' ' ' <<<"$rule_ids")" "$(tr '\n' ' ' <<<"$tested_ids")" >&2
  exit 1
fi
while read -r id passed expected reported; do
  if [ "$passed" != true ] || [ "$expected" -eq 0 ]; then
    echo "opengrep: $id expected $expected fixture findings, reported $reported" >&2
    exit 1
  fi
done <"$work/checks"
echo "opengrep: $rule_count rules pass their fixtures"

status=0
opengrep scan --config "$rules" --error --disable-version-check --quiet --exclude opengrep \
  --json . >"$work/scan.json" 2>"$work/scan.err" || status=$?
case "$status" in
  0 | 1) ;;
  *) cat "$work/scan.err" >&2; echo "opengrep: scan exited $status" >&2; exit 1 ;;
esac
if [ "$(jq '(.errors | length) + (.skipped_rules | length)' "$work/scan.json")" -ne 0 ]; then
  jq '{errors, skipped_rules}' "$work/scan.json" >&2
  echo "opengrep: scan reported errors or skipped rules" >&2
  exit 1
fi
scanned=$(jq '.paths.scanned | length' "$work/scan.json")
findings=$(jq '.results | length' "$work/scan.json")
if [ "$scanned" -eq 0 ]; then
  echo "opengrep: scan examined no files" >&2
  exit 1
fi
if [ "$findings" -ne 0 ] || [ "$status" -ne 0 ]; then
  jq -r '.results[] | "\(.path):\(.start.line): \(.check_id | split(".") | last): \(.extra.message)"' \
    "$work/scan.json" >&2
  echo "opengrep: $findings findings in $scanned files" >&2
  exit 1
fi
echo "opengrep: $rule_count rules, $scanned files, no findings"
