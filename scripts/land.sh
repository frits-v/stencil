#!/usr/bin/env bash
# Merge a pull request once every workflow run for its head commit passes:
# scripts/land.sh <pr-number>. Watches the runs (CI and CodeQL) rather than every check
# on the pull request, so an optional check that never reports cannot stall the merge.
set -euo pipefail

pr="${1:?usage: scripts/land.sh <pr-number>}"
head=$(gh pr view "$pr" --json headRefName --jq .headRefName)
sha=$(gh pr view "$pr" --json headRefOid --jq .headRefOid)
runs=$(gh run list --branch "$head" --commit "$sha" --limit 20 --json databaseId,workflowName --jq '.[] | "\(.databaseId) \(.workflowName)"')
if [ -z "$runs" ]; then
  echo "land: no workflow run for $head at $sha" >&2
  exit 1
fi
while read -r run name; do
  echo "land: watching $name run $run for $head at ${sha:0:7}"
  gh run watch "$run" --exit-status
done <<< "$runs"
gh pr merge "$pr" --squash
