#!/usr/bin/env bash
# Regenerates the committed fixtures of the given example stems (every example when none
# is named): the center SVG, measured JSON and description under
# crates/stencil-render/tests/fixtures, the corner-0 SVG for the stems that have one, and
# the check and vet lines under crates/stencil-cli/tests/fixtures. With no stem named, the
# check and vet lines are rewritten only for the stems that already have them; a stem named on
# the command line always gets both, so a new example gets its fixtures.
# The SVG fixture is the font-free document render_svg returns, so the @font-face block the
# written file carries is removed; the measured JSON fixture leaves out the document key.
# Run from the repository root. The first argument is the stencil binary.
set -euo pipefail

stencil=${1:?usage: scripts/regen-fixtures.sh <stencil binary> [stem...]}
shift
render_fixtures=crates/stencil-render/tests/fixtures
cli_fixtures=crates/stencil-cli/tests/fixtures

stems=("$@")
named=${#stems[@]}
if (( named == 0 )); then
  for example in examples/*.json; do
    stems+=("$(basename "$example" .json)")
  done
fi

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

count=0
for stem in "${stems[@]}"; do
  out=$scratch/$stem
  "$stencil" render "examples/$stem.json" --out-dir "$out" --scale 2 --theme center >/dev/null
  python3 - "$out/$stem.svg" "$render_fixtures/$stem.center.svg" <<'PY'
import sys
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
if lines[1] == "  <defs>" and lines[2] == "    <style>":
    lines = lines[:1] + lines[lines.index("  </defs>") + 1:]
open(sys.argv[2], "w", encoding="utf-8").write("\n".join(lines))
PY
  python3 - "$out/$stem.measured.json" "$render_fixtures/$stem.center.geometry.json" <<'PY'
import sys
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
start = lines.index('  "document": {')
end = next(index for index in range(start + 1, len(lines)) if lines[index] in ("  },", "  }"))
kept = lines[:start] + lines[end + 1:]
if lines[end] == "  }":
    kept[start - 1] = kept[start - 1].rstrip(",")
open(sys.argv[2], "w", encoding="utf-8").write("\n".join(kept))
PY
  cp "$out/$stem.description.md" "$render_fixtures/$stem.description.md"
  if [[ -e "$render_fixtures/$stem.square.svg" ]]; then
    python3 - "examples/$stem.json" "$out/$stem.square.json" <<'PY'
import json, sys
document = json.load(open(sys.argv[1], encoding="utf-8"))
document["corner"] = 0
json.dump(document, open(sys.argv[2], "w", encoding="utf-8"), ensure_ascii=False)
PY
    "$stencil" render "$out/$stem.square.json" --out-dir "$out/square" --scale 2 --theme center >/dev/null
    python3 - "$out/square/$stem.square.svg" "$render_fixtures/$stem.square.svg" <<'PY'
import sys
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
if lines[1] == "  <defs>" and lines[2] == "    <style>":
    lines = lines[:1] + lines[lines.index("  </defs>") + 1:]
open(sys.argv[2], "w", encoding="utf-8").write("\n".join(lines))
PY
  fi
  if (( named > 0 )) || [[ -e "$cli_fixtures/$stem.check.txt" ]]; then
    "$stencil" check "examples/$stem.json" >| "$cli_fixtures/$stem.check.txt" || true
    "$stencil" vet "examples/$stem.json" >| "$cli_fixtures/$stem.vet.txt" || true
  fi
  count=$((count + 1))
done
echo "regen-fixtures: regenerated the fixtures of $count examples"
