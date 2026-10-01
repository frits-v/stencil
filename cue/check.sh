#!/usr/bin/env bash
# Vets the core package, both grammar packages and the g7 figure; exports each
# grammar and compares it byte for byte with crates/stencil-model/grammars/;
# compares the g7 customer export with examples/g7.json; vets every example
# against the gcp #Page; then runs the negative cases: a copy of the cue tree
# with one edit to figures/g7.cue that vet must reject with the named error.
# Exits non-zero if the cue binary is missing, if any positive step fails, or
# if any negative case passes or fails for a different reason.
#
# Every cue command runs from inside cue/, where cue.mod/ is: from the
# repository root cue cannot find the module.
set -euo pipefail

CUE="${CUE:-cue}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
golden="$repo/examples/g7.json"
committed_grammars="$repo/crates/stencil-model/grammars"
out="$here/out"
mkdir -p "$out"

if ! command -v "$CUE" >/dev/null 2>&1; then
	echo "FAIL cue binary not found: $CUE" >&2
	exit 1
fi

cd "$here"
"$CUE" vet -c .
"$CUE" vet -c ./grammars:gcp
"$CUE" vet -c ./grammars:plain
"$CUE" vet -c ./figures:gcp
echo "ok   vet: core, grammars gcp and plain, figures"

# The committed grammar JSON is what the Rust side embeds; it must be exactly
# this export, so the comparison is byte for byte. To regenerate, from cue/:
#   cue export ./grammars:<name> -e grammar --out json -o ../crates/stencil-model/grammars/<name>.json --force
for name in gcp plain; do
	"$CUE" export "./grammars:$name" -e grammar --out json -o "$out/grammar-$name.json" --force
	if ! cmp -s "$committed_grammars/$name.json" "$out/grammar-$name.json"; then
		diff -u "$committed_grammars/$name.json" "$out/grammar-$name.json" >&2 || true
		echo "FAIL grammar $name: export differs from crates/stencil-model/grammars/$name.json" >&2
		exit 1
	fi
	python3 - "$name" "$out/grammar-$name.json" <<'PY'
import json, sys

name, path = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as handle:
    grammar = json.load(handle)
containers, items = len(grammar["containers"]), len(grammar["items"])
if containers == 0 or items == 0:
    print(f"FAIL grammar {name}: export has no container or item kinds", file=sys.stderr)
    sys.exit(1)
products = sum(len(item["products"]) for item in grammar["items"])
print(f"ok   grammar {name}: equals crates/stencil-model/grammars/{name}.json, "
      f"{containers} container kinds, {items} item kinds, {products} product rows, "
      f"{len(grammar['remembered'])} remembered")
PY
done

# Key order may differ between the export and the hand-maintained file;
# values and list order may not.
"$CUE" export ./figures:gcp -e customer --out json -o "$out/g7-customer.json" --force
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

# Every example is a JSON document authored without CUE; each vets against
# the gcp #Page. An example listed here is a known defect, reported on every
# run: it must fail, and every error vet reports must name the listed rule.
# An entry that starts passing is stale and fails the script.
#   hero-iso: four items carry no subtitle and no fact, so the hop-fact rule
#   rejects them.
known_failures=(
	'hero-iso|_itemsWithoutSubtitleOrFact'
)
examples_vetted=0
examples_clean=0
examples_known=0
examples_failed=0
for example in "$repo"/examples/*.json; do
	stem="$(basename "$example" .json)"
	examples_vetted=$((examples_vetted + 1))
	known_rule=""
	for entry in "${known_failures[@]}"; do
		if [[ "${entry%%|*}" == "$stem" ]]; then
			known_rule="${entry#*|}"
		fi
	done
	if report="$("$CUE" vet -c -d '#Page' ./grammars:gcp "$example" 2>&1)"; then
		if [[ -n "$known_rule" ]]; then
			echo "FAIL example $stem: listed as failing $known_rule but vets clean; remove the entry" >&2
			examples_failed=$((examples_failed + 1))
		else
			echo "ok   example $stem: vets against the gcp #Page"
			examples_clean=$((examples_clean + 1))
		fi
		continue
	fi
	errors="$(grep -v '^[[:space:]]' <<<"$report" || true)"
	if [[ -n "$known_rule" && -n "$errors" ]] && ! grep -vqF -- "$known_rule" <<<"$errors"; then
		offenders="$(sed -n 's/^[^ ]*\."\{0,1\}\([^":]*\)"\{0,1\}: conflicting values.*/\1/p' <<<"$errors" | paste -sd ',' - | sed 's/,/, /g')"
		echo "KNOWN example $stem: fails $known_rule ($offenders)"
		examples_known=$((examples_known + 1))
	else
		echo "FAIL example $stem: vet rejected it:" >&2
		echo "$report" >&2
		examples_failed=$((examples_failed + 1))
	fi
done
if [[ "$examples_vetted" -eq 0 ]]; then
	echo "FAIL examples: no examples/*.json found under $repo/examples" >&2
	exit 1
fi
echo "examples: $examples_vetted vetted, $examples_clean clean, $examples_known known failures, $examples_failed failed"
if [[ "$examples_failed" -ne 0 ]]; then
	exit 1
fi

# A captioned figure (chrome none) whose lines share one key may leave the
# legend empty. A solid pipe without tint is slot 1, so both pipes below
# share the key solid-1. The two items without a subtitle satisfy the
# hop-fact rule with a doc fact (source absent) and an ask.
chrome_none="$out/chrome-none-one-key.json"
cat >"$chrome_none" <<'JSON'
{
  "title": "Captioned",
  "kicker": "Captioned figure",
  "lede": "Drawn without chrome.",
  "canvas": "internal",
  "grammar": "gcp",
  "chrome": "none",
  "body": [
    {
      "tag": "Row",
      "children": [
        { "tag": "Item", "kind": "product", "title": "Source", "subtitle": "Cloud Run" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "label": "request" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "retry" },
        { "tag": "Item", "kind": "product", "title": "Sink", "subtitle": "Cloud SQL" },
        { "tag": "Item", "kind": "product", "title": "Queue", "facts": [{ "text": "ordering keys on" }] },
        { "tag": "Item", "kind": "product", "title": "Cache", "facts": [{ "text": "eviction policy?", "source": "ask" }] }
      ]
    }
  ],
  "legend": []
}
JSON
"$CUE" vet -c -d '#Page' ./grammars:gcp "$chrome_none"
echo "ok   chrome-none-one-key: an empty legend with one key in use vets clean, as do items with only a doc or ask fact"

# Fillers for the length bounds: 401 characters for a text field, 255 facts
# added beside the one in Region A for 257 children.
long_text="$(printf 'x%.0s' {1..401})"
many_facts="$(printf '{tag: "Fact", text: "filler"},%.0s' {1..255})"
stray_subnet='{tag: "Box", kind: "subnet", label: "stray subnet", children: [{tag: "Fact", text: "no region around it"}]},'

# name | sed expression applied to figures/g7.cue | text the vet error must
# contain [| second text the error must also contain]
cases=(
	'asn-64512|s/subtitle: "private ASN · RFC 6996"/subtitle: "private ASN 64512"/|.subtitle: invalid value "private ASN 64512"'
	'legend-unused-line|s/{line: "dash", text: "region failover, not a fifth line"},/&\n\t\t{line: "deny", text: "prohibited path"},/|_legendKeysUnusedInBody.deny'
	'pipe-line-missing-from-legend|/{line: "dash", text: "region failover, not a fifth line"},/d|_pipeKeysMissingFromLegend."dash-1"'
	'item-without-subtitle-or-fact|s/title: "Cloud Router A", subtitle: "private ASN · RFC 6996"/title: "Cloud Router A"/|_itemsWithoutSubtitleOrFact."Cloud Router A"'
	'empty-ask|s/subtitle: "same private ASN as Region A"/facts: [{text: "", source: "ask"}]/|.facts.0.text: invalid value ""'
	'solid-tint-2-pipe-beside-region-tint-1|s/line:  "dash"/line:  "solid", tint: 2/|_pipeBesideZoneOfOtherTint'
	'solid-tint-2-tee-arm-inside-region-tint-1|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Tee", line: "solid", tint: 1, hub: "hub", arms: [{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 1"}, {tag: "Pipe", dir: "h", line: "solid", tint: 2, label: "arm 2"}]},/|_otherTintInsideZone."solid-2"'
	'solid-tint-2-tee-spine-inside-region-tint-1|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Tee", line: "solid", tint: 2, hub: "hub", arms: [{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 1"}, {tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 2"}]},/|_otherTintInsideZone."solid-2"'
	'onprem-tint-2-inside-region-default-tint|/kind:  "region"/,/label: "Region A"/{/tint:  1/d;}; s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Box", kind: "optional", label: "standby", children: [{tag: "Box", kind: "onprem", tint: 2, label: "colo", children: [{tag: "Fact", text: "rack"}]}]},/|_otherTintInsideZone."onprem-2"|inside region tint 1'
	'workshop-label-on-customer-canvas|s/canvas: "internal"$/canvas: "customer"/|internal._workshop'
	'grow-length-mismatch|s/grow: \[0, 0, 1\]/grow: [0, 1]/|customer.body.0._growLengthMatchesChildren'
	'grow-weight-above-100|/halves share the column height/,/grow:/s/grow: \[1, 1\]/grow: [1, 101]/|.grow.1: invalid value 101'
	'justify-unknown|s/justify: "center"/justify: "middle"/|customer.body.0.children.1.children.0.justify:|"middle"'
	'gap-above-64|s/gap: 8$/gap: 65/|customer.body.0.gap: invalid value 65'
	'gap-negative|s/gap: 8$/gap: -1/|customer.body.0.gap: invalid value -1'
	'grow-weight-negative|/halves share the column height/,/grow:/s/grow: \[1, 1\]/grow: [1, -1]/|.grow.1: invalid value -1'
	'width-below-640|s/^\ttitle: /\twidth: 600\n&/|customer.width: invalid value 600'
	'width-above-2560|s/^\ttitle: /\twidth: 2600\n&/|customer.width: invalid value 2600'
	'solid-tint-2-pipe-between-metros|/sits level with metro i/,/grow:/s/grow: \[1, 1\]/grow: [1, 0, 1]/; /title: "On-prem router 2"/,/^\t\t\t\t\t},$/s/^\t\t\t\t\t},$/&\n\t\t\t\t\t{tag: "Pipe", dir: "h", line: "solid", tint: 2, label: "stray"},/|customer.body.0.children.0._pipeBesideZoneOfOtherTint.stray'
	'empty-gutter-half|/halves share the column height/,/grow:/s/grow: \[1, 1\]/grow: [1, 1, 0]/; s/{tag: "Pipe", dir: "h", line: "solid", tint: 2, label: "VLAN 4"[^}]*},/&\n]}, {tag: "Col", children: [/|customer.body.0.children.1.children.2.children: invalid value []|list.MinItems(1)'
	"text-above-400|s/title: \"Cloud Router A\"/title: \"$long_text\"/|customer.body.0.children.2.children.0.children.0.children.0.title: invalid value|strings.MaxRunes(400)"
	"children-above-256|s/{tag: \"Fact\", text: \"BGP peering[^}]*},/&$many_facts/|customer.body.0.children.2.children.0.children.0.children: invalid value|list.MaxItems(256)"
	'duplicate-id|s/title: "On-prem router 1"/id: "r1", &/; s/title: "On-prem router 3"/id: "r1", &/|customer._idsUsedTwice.r1'
	'link-unknown-endpoint|s/title: "On-prem router 1"/id: "r1", &/; s/^\ttitle: /\tlinks: [{from: "r1", to: "nowhere", line: "solid", tint: 1}]\n&/|customer._linkEndpointsUnknown.nowhere'
	'link-to-itself|s/title: "On-prem router 1"/id: "r1", &/; s/^\ttitle: /\tlinks: [{from: "r1", to: "r1", line: "solid", tint: 1}]\n&/|customer._linkToItself.r1'
	'link-line-missing-from-legend|s/title: "On-prem router 1"/id: "r1", &/; s/title: "On-prem router 3"/id: "r3", &/; s/^\ttitle: /\tlinks: [{from: "r1", to: "r3", line: "gray"}]\n&/|customer._linkKeysMissingFromLegend.gray'
	'id-malformed|s/title: "On-prem router 1"/id: "Router-1", &/|.id: invalid value "Router-1"'
	'tint-above-8|s/tint: 1, label: "VLAN 1"/tint: 9, label: "VLAN 1"/|.tint: invalid value 9|<=8'
	'tint-on-deny-pipe|s/line:  "dash"/line:  "deny", tint: 2/; s/{line: "dash", text:/{line: "deny", text:/|_lineKey._tintWithoutEffect|has no effect on a deny line'
	'box-kind-zone|s/kind:  "vpc"/kind:  "zone"/|customer.body.0.children.2.children.0.kind: 10 errors in empty disjunction|"zone"'
	"subnet-at-top-level|s/^\tbody: \[{\$/\tbody: [$stray_subnet {/|customer._kindParentNotAllowed.subnet|cannot sit in page"
	'legend-solid-tint-3-unused|s/{line: "dash", text: "region failover, not a fifth line"},/&\n\t\t{line: "solid", tint: 3, text: "unused slot"},/|_legendKeysUnusedInBody."solid-3"'
	'tint-on-untintable-vpc|s/kind:  "vpc"/kind:  "vpc", tint: 1/|customer.body.0.children.2.children.0._tintWithoutEffect|has no effect on a vpc box'
	"built-fact-without-subtitle|s/subtitle: \"private ASN · RFC 6996\"/facts: [{text: \"cr-region-a\", source: \"built\"}]/|_itemsWithoutSubtitleOrFact.\"Cloud Router A\""
	'chrome-none-three-keys-empty-legend|s/^\ttitle: /\tchrome: "none"\n&/; /^\tlegend: \[$/,/^\t\]$/d; s/^}$/\tlegend: []\n}/|_pipeKeysMissingFromLegend'
)

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
failed=0
for entry in "${cases[@]}"; do
	also=""
	IFS='|' read -r name expression expected also <<<"$entry"
	dir="$work/$name"
	mkdir -p "$dir"
	cp -R "$here/cue.mod" "$here/core.cue" "$here/grammar.cue" "$here/grammars" "$here/figures" "$dir/"
	sed -e "$expression" "$here/figures/g7.cue" >"$dir/figures/g7.cue"
	if cmp -s "$here/figures/g7.cue" "$dir/figures/g7.cue"; then
		echo "FAIL $name: edit did not change figures/g7.cue" >&2
		failed=$((failed + 1))
		continue
	fi
	if report="$(cd "$dir" && "$CUE" vet -c ./figures:gcp 2>&1)"; then
		echo "FAIL $name: vet passed" >&2
		failed=$((failed + 1))
	elif grep -qF -- "$expected" <<<"$report" && { [[ -z "$also" ]] || grep -qF -- "$also" <<<"$report"; }; then
		echo "ok   $name: rejected ($(grep -F -- "$expected" <<<"$report" | head -1 | cut -c1-160))"
	else
		echo "FAIL $name: rejected for another reason:" >&2
		echo "$report" >&2
		failed=$((failed + 1))
	fi
done

echo "${#cases[@]} negative cases, $((${#cases[@]} - failed)) rejected as expected"
[[ "$failed" -eq 0 ]]
