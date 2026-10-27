#!/usr/bin/env python3
"""Writes the SVG render_svg returns from the file `stencil render` writes.

Usage: scripts/resvg-input.py <written svg> <output svg>

The written file adds what only a browser reads: the <defs><style> block after the opening
tag, each rule source's <g data-own=""> wrapper and the <a> runs inside it, the unpin floor
and every hit area (data-hit). Each sits on its own line, so dropping those lines, and the
</g> that closes a wrapper, gives back the resvg input byte for byte; the center fixture
tests compare the result with render_svg.
"""
import sys

written, output = sys.argv[1], sys.argv[2]
lines = open(written, encoding="utf-8").read().split("\n")
if len(lines) > 2 and lines[1] == "  <defs>" and lines[2] == "    <style>":
    lines = lines[:1] + lines[lines.index("  </defs>") + 1:]
kept = []
# One entry per open <g> on its own line: True for a wrapper the written file added.
groups = []
for line in lines:
    text = line.strip()
    if text.startswith("<a ") or text == "</a>" or ' data-hit=""' in text:
        continue
    if text.startswith("<g") and text.endswith(">") and not text.endswith("/>") and "</g>" not in text:
        wrapper = text == '<g data-own="">'
        groups.append(wrapper)
        if wrapper:
            continue
    elif text == "</g>":
        if not groups:
            sys.exit(f"resvg-input: unbalanced </g> in {written}")
        if groups.pop():
            continue
    kept.append(line)
if groups:
    sys.exit(f"resvg-input: {len(groups)} unclosed <g> in {written}")
open(output, "w", encoding="utf-8").write("\n".join(kept))
