#!/usr/bin/env bash
# Merge a pull request once its CI workflow run passes: scripts/land.sh <pr-number>.
# Watches the ci.yml run for the pull request's head commit rather than every check on the
# pull request, so an optional check that never reports cannot stall the merge.
set -euo pipefail

pr="${1:?usage: scripts/land.sh <pr-number>}"
head=$(gh pr view "$pr" --json headRefName --jq .headRefName)
sha=$(gh pr view "$pr" --json headRefOid --jq .headRefOid)
run=$(gh run list --workflow ci.yml --branch "$head" --commit "$sha" --limit 1 --json databaseId --jq '.[0].databaseId')
if [ -z "$run" ]; then
  echo "land: no ci.yml run for $head at $sha" >&2
  exit 1
fi
echo "land: watching ci.yml run $run for $head at ${sha:0:7}"
gh run watch "$run" --exit-status
gh pr merge "$pr" --squash
