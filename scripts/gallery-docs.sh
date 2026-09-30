#!/usr/bin/env bash
# Writes the PNGs the README embeds into <out-dir>: every example in the center theme and
# the one-pager in dusk and wire, all at scale 1, named <stem>.png and onepager-<theme>.png.
# Run from the repository root. The second argument is the stencil binary.
set -euo pipefail

out_dir=${1:?usage: scripts/gallery-docs.sh <out-dir> [stencil binary]}
stencil=${2:-target/release/stencil}

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

mkdir -p "$out_dir"
rm -f "$out_dir"/*.png

render() {
  local example=$1 theme=$2 name=$3
  local stem
  stem=$(basename "$example" .json)
  "$stencil" render "$example" --out-dir "$scratch/$name" --scale 1 --theme "$theme" >/dev/null
  cp "$scratch/$name/$stem.png" "$out_dir/$name.png"
}

count=0
for example in examples/*.json; do
  render "$example" center "$(basename "$example" .json)"
  count=$((count + 1))
done
for theme in dusk wire; do
  render examples/onepager.json "$theme" "onepager-$theme"
  count=$((count + 1))
done
echo "gallery-docs: wrote $count PNGs to $out_dir"
