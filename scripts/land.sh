#!/usr/bin/env bash
# Merge a pull request once its CI run passes and the base branch rules clear it:
# scripts/land.sh <pr-number>. Watches the ci.yml run for the head commit, then polls the
# merge state, so neither an optional check that never reports nor a workflow GitHub
# injects can stall or fail the merge.
set -euo pipefail

pr="${1:?usage: scripts/land.sh <pr-number>}"
head=$(gh pr view "$pr" --json headRefName --jq .headRefName)
sha=$(gh pr view "$pr" --json headRefOid --jq .headRefOid)
run=$(gh run list --workflow ci.yml --branch "$head" --commit "$sha" --limit 1 --json databaseId --jq '.[0].databaseId')
if [ -z "$run" ]; then
  echo "land: no ci.yml run for $head at $sha" >&2
  exit 1
fi
echo "land: watching ci run $run for $head at ${sha:0:7}"
gh run watch "$run" --exit-status

# Code scanning and required checks settle after CI; wait up to 20 minutes for the rules.
for attempt in $(seq 1 60); do
  state=$(gh pr view "$pr" --json mergeStateStatus --jq .mergeStateStatus)
  case "$state" in
    CLEAN|UNSTABLE|HAS_HOOKS) break ;;
    DIRTY) echo "land: $pr conflicts with its base; rebase it" >&2; exit 1 ;;
    *) echo "land: merge state $state, attempt $attempt of 60"; sleep 20 ;;
  esac
done
gh pr merge "$pr" --squash
