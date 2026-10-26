#!/usr/bin/env bash
# Merge a pull request once its CI run passes and the base branch rules clear it:
# scripts/land.sh <pr-number>. Watches the ci.yml run for the head commit, then polls the
# merge state, so neither an optional check that never reports nor a workflow GitHub
# injects can stall or fail the merge.
set -euo pipefail

pr="${1:?usage: scripts/land.sh <pr-number>}"
head=$(gh pr view "$pr" --json headRefName --jq .headRefName)
sha=$(gh pr view "$pr" --json headRefOid --jq .headRefOid)
# GitHub starts no pull_request run for a head that conflicts with its base, so the watch
# below would find nothing. UNKNOWN is a mergeability GitHub has not computed yet.
mergeable=$(gh pr view "$pr" --json mergeable --jq .mergeable)
case "$mergeable" in
  MERGEABLE | UNKNOWN) ;;
  CONFLICTING)
    echo "land: $head conflicts with its base and gets no ci run; rebase it and push" >&2
    exit 1
    ;;
  *)
    echo "land: unrecognised mergeable state '$mergeable' for $pr" >&2
    exit 1
    ;;
esac
# A ci run tests the head as pushed. A head behind its base was never tested against what
# the squash merge produces, so a test over every example can pass on the branch and fail on
# main once another branch has added an example.
base=$(gh pr view "$pr" --json baseRefName --jq .baseRefName)
require_current_base() {
  local behind
  behind=$(gh api "repos/{owner}/{repo}/compare/$base...$sha" --jq .behind_by)
  case "$behind" in
    0) ;;
    '' | *[!0-9]*)
      echo "land: unrecognised behind count '$behind' for $head" >&2
      exit 1
      ;;
    *)
      echo "land: $head is $behind commit(s) behind $base; merge $base into it and push" >&2
      exit 1
      ;;
  esac
}
require_current_base
# The branch filter combined with the workflow filter can list nothing for a run that exists,
# so the run is found by commit and its branch is checked from the result.
run=$(gh run list --workflow ci.yml --commit "$sha" --limit 20 --json databaseId,headBranch \
  --jq "[.[] | select(.headBranch == \"$head\")][0].databaseId // empty")
if [[ -z $run ]]; then
  echo "land: no ci.yml run for $head at $sha" >&2
  exit 1
fi
# The base branch rules require signed commits, and an unsigned one blocks the squash merge
# behind "not mergeable" on every retry; refuse it before the watch.
unsigned=$(gh pr view "$pr" --json commits --jq '.commits[].oid' | while read -r commit; do
  if [[ "$(gh api "repos/{owner}/{repo}/commits/$commit" --jq .commit.verification.verified)" != "true" ]]; then
    echo "${commit:0:7}"
  fi
done)
if [[ -n $unsigned ]]; then
  echo "land: unsigned commit(s) on $head, which main refuses: $(echo "$unsigned" | paste -sd ' ' -); sign and push again" >&2
  exit 1
fi
echo "land: watching ci run $run for $head at ${sha:0:7}"
# gh polls every 3 seconds by default, about 1200 calls an hour from one watch against the
# 5000 an hour every session of this account shares; a ci run takes minutes, so 30 seconds loses
# little and leaves the budget for the other sessions.
gh run watch "$run" --exit-status --interval 30
# The base can move while CI runs.
require_current_base

# A draft cannot merge; it is marked ready once its CI run has passed.
draft=$(gh pr view "$pr" --json isDraft --jq .isDraft)
case "$draft" in
  true) gh pr ready "$pr" ;;
  false) ;;
  *)
    echo "land: unrecognised draft state '$draft' for $pr" >&2
    exit 1
    ;;
esac

# Code scanning and required checks settle after CI; retry the merge for up to 20 minutes.
for attempt in $(seq 1 60); do
  if output=$(gh pr merge "$pr" --squash 2>&1); then
    echo "$output"
    exit 0
  fi
  case "$output" in
    *"not mergeable"*) echo "land: not mergeable yet, attempt $attempt of 60"; sleep 20 ;;
    *) echo "$output" >&2; exit 1 ;;
  esac
done
echo "land: gave up on $pr after 60 attempts" >&2
exit 1
