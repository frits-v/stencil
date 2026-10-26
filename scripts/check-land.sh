#!/usr/bin/env bash
# Runs scripts/land.sh against a stub gh for each mergeable, draft and signature state it
# reads, and checks its exit status and the gh calls it made. Run from the repository root. Fails
# when a case does not hold, and when no case ran.
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat >"$work/gh" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >>"$STUB_LOG"
case "$*" in
  "pr view 7 --json headRefName --jq .headRefName") echo feature ;;
  "pr view 7 --json headRefOid --jq .headRefOid") echo 0123456789abcdef ;;
  "pr view 7 --json baseRefName --jq .baseRefName") echo main ;;
  "api repos/{owner}/{repo}/compare/main...0123456789abcdef --jq .behind_by")
    # The first answer is read before the watch, later ones before the merge.
    if [[ -s $STUB_LOG.compared ]]; then
      echo "${STUB_BEHIND_AFTER_WATCH:-${STUB_BEHIND:-0}}"
    else
      echo "${STUB_BEHIND:-0}"
      echo compared >"$STUB_LOG.compared"
    fi
    ;;
  "pr view 7 --json mergeable --jq .mergeable") echo "$STUB_MERGEABLE" ;;
  "pr view 7 --json isDraft --jq .isDraft") echo "$STUB_DRAFT" ;;
  "pr view 7 --json commits --jq .commits[].oid") echo 0123456789abcdef ;;
  "api repos/{owner}/{repo}/commits/0123456789abcdef --jq .commit.verification.verified") echo "${STUB_VERIFIED:-true}" ;;
  "run list "*) echo 42 ;;
  "run watch 42 --exit-status") ;;
  "pr ready 7") ;;
  "pr merge 7 --squash") echo "merged 7" ;;
  *)
    echo "stub gh: unexpected call: $*" >&2
    exit 2
    ;;
esac
STUB
chmod +x "$work/gh"

cases=0
failures=0

# expect <mergeable> <draft> <exit status> <calls that must appear> <calls that must not>
expect() {
  local mergeable=$1 draft=$2 want_status=$3 present=$4 absent=$5
  local log="$work/calls-$cases"
  : >"$log"
  rm -f "$log.compared"
  local status=0
  PATH="$work:$PATH" STUB_LOG="$log" STUB_MERGEABLE="$mergeable" STUB_DRAFT="$draft" \
    scripts/land.sh 7 >"$work/out" 2>&1 || status=$?
  cases=$((cases + 1))
  local verdict=ok
  if (( status != want_status )); then
    verdict="exit $status, wanted $want_status"
  fi
  local call
  while IFS= read -r call; do
    if [[ -n $call ]] && ! grep -qxF "$call" "$log"; then
      verdict="no call '$call'"
    fi
  done <<<"$present"
  while IFS= read -r call; do
    if [[ -n $call ]] && grep -qxF "$call" "$log"; then
      verdict="unwanted call '$call'"
    fi
  done <<<"$absent"
  if [[ $verdict != ok ]]; then
    failures=$((failures + 1))
    echo "check-land: mergeable $mergeable, draft $draft: $verdict" >&2
    sed 's/^/  /' "$work/out" >&2
  fi
}

expect MERGEABLE true 0 $'pr ready 7\npr merge 7 --squash' ''
expect MERGEABLE false 0 'pr merge 7 --squash' 'pr ready 7'
expect UNKNOWN false 0 'pr merge 7 --squash' ''
expect CONFLICTING false 1 '' $'run watch 42 --exit-status\npr merge 7 --squash'
expect BLOCKED false 1 '' 'pr merge 7 --squash'
expect MERGEABLE maybe 1 '' $'pr ready 7\npr merge 7 --squash'
STUB_VERIFIED=false expect MERGEABLE false 1 '' $'run watch 42 --exit-status\npr ready 7\npr merge 7 --squash'
STUB_BEHIND=2 expect MERGEABLE false 1 '' $'run watch 42 --exit-status\npr ready 7\npr merge 7 --squash'
STUB_BEHIND_AFTER_WATCH=1 expect MERGEABLE true 1 'run watch 42 --exit-status' $'pr ready 7\npr merge 7 --squash'
STUB_BEHIND=unknown expect MERGEABLE false 1 '' $'run watch 42 --exit-status\npr merge 7 --squash'

if (( cases == 0 )); then
  echo "check-land: no case ran" >&2
  exit 1
fi
if (( failures != 0 )); then
  echo "check-land: $failures of $cases cases failed" >&2
  exit 1
fi
echo "check-land: $cases cases passed"
