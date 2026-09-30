#!/usr/bin/env bash
# Regenerates the README PNGs into a temporary directory and compares them byte for byte
# with docs/gallery. Fails on a differing, missing or extra file, and when nothing was
# compared. Run from the repository root. The first argument is the stencil binary; the
# second, optional, is where the regenerated PNGs are written and kept.
set -euo pipefail

stencil=${1:-target/release/stencil}
regenerated=${2:-$(mktemp -d)}

scripts/gallery-docs.sh "$regenerated" "$stencil"

if ! diff <(ls docs/gallery) <(ls "$regenerated"); then
  echo "check-gallery-docs: docs/gallery and the regenerated set list different files; run mise run gallery-docs" >&2
  exit 1
fi

compared=0
failed=0
for file in "$regenerated"/*.png; do
  name=$(basename "$file")
  if ! cmp "$file" "docs/gallery/$name"; then
    failed=$((failed + 1))
  fi
  compared=$((compared + 1))
done

if [ "$compared" -eq 0 ]; then
  echo "check-gallery-docs: compared 0 files, FAILED: nothing examined" >&2
  exit 1
fi
if [ "$failed" -gt 0 ]; then
  echo "check-gallery-docs: compared $compared files, $failed differ; run mise run gallery-docs" >&2
  exit 1
fi
echo "check-gallery-docs: compared $compared files, 0 differ"
