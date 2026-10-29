#!/usr/bin/env bash
# Regenerates the committed fixtures of the given example stems (every example when none
# is named): the center SVG, measured JSON and description under
# crates/stencil-render/tests/fixtures, the corner-0 SVG for the stems that have one, and
# the check and vet lines under crates/stencil-cli/tests/fixtures, and for g7 the theme
# bundle golden (center, clear-dark and wire, the @font-face rules dropped). With no stem
# named, the check and vet lines are rewritten only for the stems that already have them; a
# stem named on the command line always gets both, so a new example gets its fixtures.
# The SVG fixture is the document render_svg returns, so scripts/resvg-input.py removes the
# style block and the browser-only elements the written file carries; the measured JSON
# fixture leaves out the document key.
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
  python3 scripts/resvg-input.py "$out/$stem.svg" "$render_fixtures/$stem.center.svg"
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
    python3 scripts/resvg-input.py "$out/square/$stem.square.svg" "$render_fixtures/$stem.square.svg"
  fi
  if (( named > 0 )) || [[ -e "$cli_fixtures/$stem.check.txt" ]]; then
    "$stencil" check "examples/$stem.json" >| "$cli_fixtures/$stem.check.txt" || true
    "$stencil" vet "examples/$stem.json" >| "$cli_fixtures/$stem.vet.txt" || true
  fi
  if [[ "$stem" == g7 ]]; then
    "$stencil" render "examples/$stem.json" --out-dir "$out/themes" --themes center,clear-dark,wire >/dev/null
    grep -v '@font-face' "$out/themes/$stem.svg" >| "$cli_fixtures/$stem.themes.svg"
  fi
  count=$((count + 1))
done
echo "regen-fixtures: regenerated the fixtures of $count examples"
