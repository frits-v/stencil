#!/usr/bin/env bash
# Re-imports every pinned base16 scheme under crates/stencil-render/themes/imported/ into
# its theme file and report (section 13.9). Run from the repository root. The argument is
# the stencil binary. The import exits 1 for every scheme of the tier, because none passes
# the quality thresholds, so only exit 2 stops the script.
set -euo pipefail

stencil=${1:-target/release/stencil}
imported=crates/stencil-render/themes/imported

count=0
for scheme in "$imported"/*.base16.yaml; do
  name=$(basename "$scheme" .base16.yaml)
  status=0
  "$stencil" theme import --base16 "$scheme" --name "$name" -o "$imported/$name.json" >/dev/null || status=$?
  if [[ "$status" -eq 2 ]]; then
    echo "import-themes: $scheme could not be imported" >&2
    exit 2
  fi
  count=$((count + 1))
done
if [[ "$count" -eq 0 ]]; then
  echo "import-themes: no schemes under $imported" >&2
  exit 1
fi
echo "import-themes: imported $count schemes into $imported"
