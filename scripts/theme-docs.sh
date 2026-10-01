#!/usr/bin/env bash
# Writes the preview of every built-in theme into <out-dir> as <name>.png at scale 1, the
# images the README's theme table embeds. The theme names come from the binary, so the set
# follows BUILTIN_THEMES. Run from the repository root. The second argument is the stencil
# binary.
set -euo pipefail

out_dir=${1:?usage: scripts/theme-docs.sh <out-dir> [stencil binary]}
stencil=${2:-target/release/stencil}

# `theme show` with an unknown name lists the built-ins on stderr and exits 2.
listing=$("$stencil" theme show -- --list-themes 2>&1 >/dev/null || true)
names=${listing##*themes: }
if [[ "$names" == "$listing" || -z "$names" ]]; then
  echo "theme-docs: could not read the built-in theme names from $stencil" >&2
  exit 2
fi

mkdir -p "$out_dir"
rm -f "$out_dir"/*.png

count=0
IFS=', ' read -r -a themes <<<"$names"
for theme in "${themes[@]}"; do
  [[ -n "$theme" ]] || continue
  "$stencil" theme preview "$theme" -o "$out_dir/$theme.png" --scale 1 >/dev/null
  count=$((count + 1))
done
if [[ "$count" -eq 0 ]]; then
  echo "theme-docs: wrote 0 PNGs, FAILED: nothing rendered" >&2
  exit 1
fi
echo "theme-docs: wrote $count PNGs to $out_dir"
