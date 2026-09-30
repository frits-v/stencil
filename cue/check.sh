#!/usr/bin/env bash
# Vets the package, exports both eraser views, compares the customer export
# with examples/g7.json, then runs each negative case: a copy of the package
# with one edit to g7.cue that vet must reject with the named error. Exits
# non-zero if the cue binary is missing, if any positive step fails, or if any
# negative case passes or fails for a different reason.
set -euo pipefail

CUE="${CUE:-cue}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
golden="$here/../examples/g7.json"
out="$here/out"
mkdir -p "$out"

if ! command -v "$CUE" >/dev/null 2>&1; then
	echo "FAIL cue binary not found: $CUE" >&2
	exit 1
fi

# cue reads an absolute directory argument as an import path; run from inside.
cd "$here"
"$CUE" vet -c .
"$CUE" export . -e eraserCustomer --out json -o "$out/g7-eraser.json" --force
"$CUE" export . -e eraserInternal --out json -o "$out/g7-eraser-internal.json" --force
for view in g7-eraser g7-eraser-internal; do
	counts="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(len(d["entities"]), len(d["connections"]))' "$out/$view.json")"
	read -r entities connections <<<"$counts"
	if [[ "$entities" -eq 0 || "$connections" -eq 0 ]]; then
		echo "FAIL $view: $entities entities, $connections connections" >&2
		exit 1
	fi
	echo "ok   $view: $entities entities, $connections connections"
done

# Key order may differ between the export and the hand-maintained file;
# values and list order may not.
"$CUE" export . -e customer --out json -o "$out/g7-customer.json" --force
python3 - "$golden" "$out/g7-customer.json" <<'PY'
import difflib, json, sys

def canonical(path):
    with open(path, encoding="utf-8") as handle:
        return json.dumps(json.load(handle), indent=2, sort_keys=True, ensure_ascii=False).splitlines()

def count_nodes(nodes, depth):
    assert depth <= 24, "depth above 24"
    total = 0
    for node in nodes:
        total += 1 + count_nodes(node.get("children", []), depth + 1) + len(node.get("arms", []))
    return total

golden_path, export_path = sys.argv[1], sys.argv[2]
golden, exported = canonical(golden_path), canonical(export_path)
if golden != exported:
    sys.stderr.writelines(line + "\n" for line in difflib.unified_diff(golden, exported, golden_path, export_path, lineterm=""))
    print("FAIL g7-customer: export differs from examples/g7.json", file=sys.stderr)
    sys.exit(1)
with open(export_path, encoding="utf-8") as handle:
    nodes = count_nodes(json.load(handle)["body"], 1)
if nodes == 0:
    print("FAIL g7-customer: export has no body nodes", file=sys.stderr)
    sys.exit(1)
print(f"ok   g7-customer: equals examples/g7.json, {nodes} nodes, {len(golden)} canonical lines")
PY

# name | sed expression applied to g7.cue | text the vet error must contain
cases=(
	'asn-64512|s/pn: "private ASN · RFC 6996"/pn: "private ASN 64512"/|.pn: invalid value "private ASN 64512"'
	'legend-unused-kind|s/{kind: "dash", text: "region failover, not a fifth line"},/&\n\t\t{kind: "deny", text: "prohibited path"},/|_legendKindsUnusedInBody.deny'
	'pipe-kind-missing-from-legend|/{kind: "dash", text: "region failover, not a fifth line"},/d|_pipeKindsMissingFromLegend.dash'
	'card-without-fact|s/{tag: "Pcard", icon: "networking", fn: "Cloud Router A", pn: "private ASN · RFC 6996"}/{tag: "Pcard", icon: "networking", fn: "Cloud Router A"}/|_pcardsWithoutPnFactOrAsk'
	'empty-ask|s/pn: "same private ASN as Region A"/ask: ""/|.ask: invalid value ""'
	'pink-pipe-beside-region-a|s/kind:  "dash"/kind:  "pink"/|_pipeBesideZoneOfOtherTint'
	'blue-vlan-from-metro-2|s/kind: "pink", label: "VLAN 3"/kind: "blue", label: "VLAN 3"/|_leavesCardInItsMetro'
	'pink-tee-arm-inside-region-a|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Tee", kind: "blue", hub: "hub", arms: [{tag: "Pipe", dir: "h", kind: "blue", label: "arm 1"}, {tag: "Pipe", dir: "h", kind: "pink", label: "arm 2"}]},/|_otherTintInsideZone.pink'
	'gray-pipe-in-gutter|s/kind: "blue", label: "VLAN 1"/kind: "gray", label: "VLAN 1"/|_leavesCardInItsMetro: undefined field: gray'
	'workshop-label-on-customer-canvas|s/canvas: "internal"$/canvas: "customer"/|internal._workshop'
	'grow-length-mismatch|s/grow: \[0, 0, 1\]/grow: [0, 1]/|customer.body.0._growLengthMatchesChildren'
	'grow-weight-above-100|s/grow: \[1, 1\]/grow: [1, 101]/|.grow.1: invalid value 101'
	'justify-unknown|s/justify: "center"/justify: "middle"/|.justify: conflicting values "start" and "middle"'
	'gap-above-64|s/gap: 8$/gap: 65/|customer.body.0.gap: invalid value 65'
)

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
failed=0
for entry in "${cases[@]}"; do
	IFS='|' read -r name expression expected <<<"$entry"
	dir="$work/$name"
	mkdir -p "$dir"
	cp "$here"/*.cue "$dir/"
	sed -e "$expression" "$here/g7.cue" >"$dir/g7.cue"
	if cmp -s "$here/g7.cue" "$dir/g7.cue"; then
		echo "FAIL $name: edit did not change g7.cue" >&2
		failed=$((failed + 1))
		continue
	fi
	if report="$(cd "$dir" && "$CUE" vet -c . 2>&1)"; then
		echo "FAIL $name: vet passed" >&2
		failed=$((failed + 1))
	elif grep -qF -- "$expected" <<<"$report"; then
		echo "ok   $name: rejected ($(grep -F -- "$expected" <<<"$report" | head -1))"
	else
		echo "FAIL $name: rejected for another reason:" >&2
		echo "$report" >&2
		failed=$((failed + 1))
	fi
done

echo "${#cases[@]} negative cases, $((${#cases[@]} - failed)) rejected as expected"
[[ "$failed" -eq 0 ]]
