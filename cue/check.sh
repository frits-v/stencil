#!/usr/bin/env bash
# Vets the core package, both grammar packages and the g7, sequence, sheet and
# sheet-small figures; exports each grammar and compares it byte for byte with
# crates/stencil-model/grammars/; compares the g7 customer export with
# examples/g7.json, the sequence export with examples/sequence.json and the
# sheet export with examples/onepager.json; vets every example against the
# #Page of the grammar it names; vets every built-in theme file against #Theme
# and rejects the theme negative cases; then runs the negative cases: a copy of
# the cue tree with one edit to figures/g7.cue, figures/sequence.cue,
# figures/sheet.cue or figures/sheet-small.cue that vet must reject with the
# named error.
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
sequence_golden="$repo/examples/sequence.json"
onepager_golden="$repo/examples/onepager.json"
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
"$CUE" vet -c ./figures:plain
echo "ok   vet: core, grammars gcp and plain, figures gcp and plain"

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

# sequence.cue is the authored source of examples/sequence.json. As for g7,
# key order may differ; values and list order may not.
"$CUE" export ./figures:plain -e figure --out json -o "$out/sequence.json" --force
python3 - "$sequence_golden" "$out/sequence.json" <<'PY'
import difflib, json, sys

def canonical(path):
    with open(path, encoding="utf-8") as handle:
        return json.dumps(json.load(handle), indent=2, sort_keys=True, ensure_ascii=False).splitlines()

golden_path, export_path = sys.argv[1], sys.argv[2]
golden, exported = canonical(golden_path), canonical(export_path)
if golden != exported:
    sys.stderr.writelines(line + "\n" for line in difflib.unified_diff(golden, exported, golden_path, export_path, lineterm=""))
    print("FAIL sequence: export differs from examples/sequence.json", file=sys.stderr)
    sys.exit(1)
with open(export_path, encoding="utf-8") as handle:
    links = json.load(handle).get("links", [])
messages = sum(1 for link in links if "order" in link)
if messages == 0:
    print("FAIL sequence: export has no ordered links", file=sys.stderr)
    sys.exit(1)
print(f"ok   sequence: equals examples/sequence.json, {messages} ordered links, {len(golden)} canonical lines")
PY

# sheet.cue is the authored source of examples/onepager.json, compared as g7 is.
"$CUE" export ./figures:gcp -e onepager --out json -o "$out/onepager.json" --force
python3 - "$onepager_golden" "$out/onepager.json" <<'PY'
import difflib, json, sys

def canonical(path):
    with open(path, encoding="utf-8") as handle:
        return json.dumps(json.load(handle), indent=2, sort_keys=True, ensure_ascii=False).splitlines()

golden_path, export_path = sys.argv[1], sys.argv[2]
golden, exported = canonical(golden_path), canonical(export_path)
if golden != exported:
    sys.stderr.writelines(line + "\n" for line in difflib.unified_diff(golden, exported, golden_path, export_path, lineterm=""))
    print("FAIL onepager: export differs from examples/onepager.json", file=sys.stderr)
    sys.exit(1)
with open(export_path, encoding="utf-8") as handle:
    views = [node for node in json.load(handle)["body"] if node.get("tag") == "View"]
if not views:
    print("FAIL onepager: export has no View", file=sys.stderr)
    sys.exit(1)
print(f"ok   onepager: equals examples/onepager.json, {len(views)} views, {len(golden)} canonical lines")
PY

# Every example is a JSON document authored without CUE; each vets against
# the #Page of the grammar it names, gcp when it names none. An example listed here is a known defect, reported on every
# run: it must fail, and every error vet reports must name the listed rule.
# An entry that starts passing is stale and fails the script. The list is
# empty: every example vets clean.
known_failures=()
examples_vetted=0
examples_clean=0
examples_known=0
examples_failed=0
for example in "$repo"/examples/*.json; do
	stem="$(basename "$example" .json)"
	examples_vetted=$((examples_vetted + 1))
	known_rule=""
	for entry in ${known_failures[@]+"${known_failures[@]}"}; do
		if [[ "${entry%%|*}" == "$stem" ]]; then
			known_rule="${entry#*|}"
		fi
	done
	grammar="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8")).get("grammar", "gcp"))' "$example")"
	if [[ "$grammar" != "gcp" && "$grammar" != "plain" ]]; then
		echo "FAIL example $stem: names grammar $grammar, which has no package under grammars/" >&2
		examples_failed=$((examples_failed + 1))
		continue
	fi
	if report="$("$CUE" vet -c -d '#Page' "./grammars:$grammar" "$example" 2>&1)"; then
		if [[ -n "$known_rule" ]]; then
			echo "FAIL example $stem: listed as failing $known_rule but vets clean; remove the entry" >&2
			examples_failed=$((examples_failed + 1))
		else
			echo "ok   example $stem: vets against the $grammar #Page"
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

# Every built-in theme file vets against the closed #Theme, the definition the
# Rust loader mirrors. Each negative case is a copy of center with one fault
# that vet must reject with the named error; the Rust tests reject the same
# faults (crates/stencil-model/tests/theme.rs).
themes_dir="$repo/crates/stencil-render/themes"
themes_vetted=0
for theme in "$themes_dir"/*.json "$themes_dir"/imported/*.json; do
	[[ -e "$theme" ]] || continue
	"$CUE" vet -c -d '#Theme' . "$theme"
	themes_vetted=$((themes_vetted + 1))
done
# Six designed themes and seven imported ones (BUILTIN_THEMES).
if [[ "$themes_vetted" -ne 13 ]]; then
	echo "FAIL themes: found $themes_vetted theme files under $themes_dir, expected 13" >&2
	exit 1
fi
echo "ok   themes: $themes_vetted built-in theme files vet against #Theme"

theme_work="$(mktemp -d)"
theme_failed=0
# name | python expression applied to the parsed center theme `t` | text the
# vet error must contain
theme_cases=(
	'seven-tints|t["tints"].pop()|tints: invalid value'
	'unknown-role|t.__setitem__("glow", "#FFFFFF")|glow: field not allowed'
	'unknown-iso-role|t["iso"].__setitem__("glow", 1)|iso.glow: field not allowed'
	'lowercase-hex|t.__setitem__("page", "#ffffff")|page: invalid value "#ffffff"'
	'width-above-4|t["card"]["border"].__setitem__("width", 5)|card.border.width: invalid value 5'
	'strong-tone-filled|t["tones"]["strong"].__setitem__("fill", "#000000")|tones.strong.fill: field not allowed'
	'sheet-unknown-leaf|t["sheet"].__setitem__("glow", "#FFFFFF")|sheet.glow: field not allowed'
)
for entry in "${theme_cases[@]}"; do
	IFS='|' read -r name expression expected <<<"$entry"
	case_file="$theme_work/$name.json"
	python3 - "$themes_dir/center.json" "$case_file" "$expression" <<'PY'
import json, sys
source, target, expression = sys.argv[1], sys.argv[2], sys.argv[3]
with open(source, encoding="utf-8") as handle:
    t = json.load(handle)
eval(expression)
with open(target, "w", encoding="utf-8") as handle:
    json.dump(t, handle)
PY
	if report="$("$CUE" vet -c -d '#Theme' . "$case_file" 2>&1)"; then
		echo "FAIL theme $name: vet passed" >&2
		theme_failed=$((theme_failed + 1))
	elif grep -qF -- "$expected" <<<"$report"; then
		echo "ok   theme $name: rejected ($(grep -F -- "$expected" <<<"$report" | head -1 | cut -c1-120))"
	else
		echo "FAIL theme $name: rejected for another reason:" >&2
		echo "$report" >&2
		theme_failed=$((theme_failed + 1))
	fi
done
rm -rf "$theme_work"
echo "${#theme_cases[@]} theme negative cases, $((${#theme_cases[@]} - theme_failed)) rejected as expected"
if [[ "$theme_failed" -ne 0 ]]; then
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
  "badge": "Internal",
  "grammar": "gcp",
  "chrome": "none",
  "body": [
    {
      "tag": "Row",
      "children": [
        { "tag": "Item", "id": "source", "kind": "product", "title": "Source", "subtitle": "Cloud Run" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "label": "request", "from": "source", "to": "sink" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "retry" },
        { "tag": "Item", "id": "sink", "kind": "product", "title": "Sink", "subtitle": "Cloud SQL" },
        { "tag": "Item", "kind": "product", "title": "Queue", "facts": [{ "text": "ordering keys on" }] },
        { "tag": "Item", "kind": "product", "title": "Cache", "facts": [{ "text": "eviction policy?", "source": "ask" }] }
      ]
    }
  ],
  "legend": []
}
JSON
"$CUE" vet -c -d '#Page' ./grammars:gcp "$chrome_none"
echo "ok   chrome-none-one-key: an empty legend with one key in use vets clean, as do items with only a doc or ask fact and a pipe naming its targets"

# Fillers for the length bounds: 401 characters for a text field, 255 facts
# added beside the one in Region A for 257 children.
long_text="$(printf 'x%.0s' {1..401})"
many_facts="$(printf '{tag: "Fact", text: "filler"},%.0s' {1..255})"
stray_subnet='{tag: "Box", kind: "subnet", label: "stray subnet", children: [{tag: "Fact", text: "no region around it"}]},'
stray_apis='{tag: "Box", kind: "project", label: "Data project", children: [{tag: "Box", kind: "apis", label: "Stray APIs", children: [{tag: "Fact", text: "no gcp frame around it"}]}]},'
# Fillers for the sheet list bounds: nine actors, lanes and phases, 65 Steps and
# seven Step lines, each one past its list's maximum.
nine_actors="$(printf '{id: "actor-%s", name: "Actor", tint: 1},' 1 2 3 4 5 6 7 8 9)"
nine_lanes="$(printf '"lane-%s",' 1 2 3 4 5 6 7 8 9)"
nine_phases="$(printf '{id: "phase-%s", title: "Phase"},' 1 2 3 4 5 6 7 8 9)"
many_steps="$(printf '{tag: "Step", lane: "operator", phase: "describe", title: "Filler step"},%.0s' {1..65})"
seven_lines="$(printf '"line %s",' 1 2 3 4 5 6 7)"
# Fillers for the key list bounds: 48 parts beside sheet-small's two, and 16
# notes beside its one.
many_parts="$(printf '{id: "part-%s", name: "Part", why: "Filler."},' $(seq 1 48))"
many_notes="$(printf '{id: "note-%s", text: "Filler."},' $(seq 1 16))"

# figure | name | sed expression applied to figures/<figure>.cue | text the
# vet error must contain [| second text the error must also contain]. A g7 or
# sheet case vets ./figures:gcp and a sequence or sheet-small case
# ./figures:plain.
g7_cases=(
	'asn-64512|s/subtitle: "private ASN · RFC 6996"/subtitle: "private ASN 64512"/|.subtitle: invalid value "private ASN 64512"'
	'legend-unused-line|s/{line: "dash", text: "region failover, not a fifth line"},/&\n\t\t{line: "deny", text: "prohibited path"},/|_legendKeysUnusedInBody.deny'
	'pipe-line-missing-from-legend|/{line: "dash", text: "region failover, not a fifth line"},/d|_pipeKeysMissingFromLegend."dash-1"'
	'band-pipe-needs-its-own-legend-entry|s/label: "VLAN 1", sub: "EAD 1 · BGP"/form: "band", &/|_pipeKeysMissingFromLegend."solid-1-band"'
	'item-without-subtitle-or-fact|s/title: "Cloud Router A", subtitle: "private ASN · RFC 6996"/title: "Cloud Router A"/|_itemsWithoutSubtitleOrFact."Cloud Router A"'
	'empty-ask|s/subtitle: "same private ASN as Region A"/facts: [{text: "", source: "ask"}]/|.facts.0.text: invalid value ""'
	'solid-tint-2-pipe-beside-region-tint-1|s/line:  "dash"/line:  "solid", tint: 2/|_pipeBesideZoneOfOtherTint'
	'solid-tint-2-tee-arm-inside-region-tint-1|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Tee", line: "solid", tint: 1, hub: "hub", arms: [{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 1"}, {tag: "Pipe", dir: "h", line: "solid", tint: 2, label: "arm 2"}]},/|_otherTintInsideZone."solid-2"'
	'solid-tint-2-tee-spine-inside-region-tint-1|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Tee", line: "solid", tint: 2, hub: "hub", arms: [{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 1"}, {tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 2"}]},/|_otherTintInsideZone."solid-2"'
	'onprem-tint-2-inside-region-default-tint|/kind:  "region"/,/label: "Region A"/{/tint:  1/d;}; s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Box", kind: "optional", label: "standby", children: [{tag: "Box", kind: "onprem", tint: 2, label: "colo", children: [{tag: "Fact", text: "rack"}]}]},/|_otherTintInsideZone."onprem-2"|inside region tint 1'
	'workshop-label-on-customer-canvas|s/_audience: "internal"$/_audience: "customer"/|internal._workshop'
	'grow-length-mismatch|s/grow: \[0, 0, 1\]/grow: [0, 1]/|customer.body.0._growLengthMatchesChildren'
	'grow-weight-above-100|/halves share the column height/,/grow:/s/grow: \[1, 1\]/grow: [1, 101]/|.grow.1: invalid value 101'
	'justify-unknown|s/justify: "center"/justify: "middle"/|customer.body.0.children.1.children.0.justify:|"middle"'
	'gap-above-64|s/gap: 8$/gap: 65/|customer.body.0.gap: invalid value 65'
	'gap-negative|s/gap: 8$/gap: -1/|customer.body.0.gap: invalid value -1'
	'grow-weight-negative|/halves share the column height/,/grow:/s/grow: \[1, 1\]/grow: [1, -1]/|.grow.1: invalid value -1'
	'width-below-640|s/^\ttitle: /\twidth: 600\n&/|customer.width: invalid value 600'
	'width-above-2560|s/^\ttitle: /\twidth: 2600\n&/|customer.width: invalid value 2600'
	'corner-above-16|s/^\ttitle: /\tcorner: 17\n&/|customer.corner: invalid value 17'
	'corner-negative|s/^\ttitle: /\tcorner: -1\n&/|customer.corner: invalid value -1'
	'bend-unknown|s/^\ttitle: /\tbend: "zigzag"\n&/|customer.bend:|"zigzag"'
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
	'bigquery-product-inside-vpc|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Item", kind: "product", icon: "bigquery", title: "Warehouse", subtitle: "BigQuery"},/|_productInsideVpc.Warehouse|sits inside a vpc; draw it in an apis box'
	'cloud-storage-icon-inside-vpc|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Item", kind: "product", icon: "cloud-storage", title: "Landing bucket", facts: [{text: "dual-region"}]},/|_productInsideVpc."Landing bucket"'
	'cloud-run-product-inside-vpc|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Item", kind: "product", icon: "cloud-run", title: "Gateway", subtitle: "Cloud Run"},/|_serverlessInsideVpc.Gateway|reached over Private Service Connect'
	'cloud-sql-product-inside-vpc|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Item", kind: "product", icon: "cloud-sql", title: "Database", subtitle: "Cloud SQL"},/|_managedDataInsideVpc.Database|name the attachment'
	'vertex-ai-icon-inside-vpc|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Item", kind: "product", icon: "vertex-ai", title: "Model", facts: [{text: "endpoint"}]},/|_serverlessInsideVpc.Model'
	'ncc-hub-inside-region|s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Item", kind: "product", icon: "networking", title: "Hub", subtitle: "Network Connectivity Center"},/|_globalInsideRegion.Hub|sits outside every region'
	"apis-box-outside-gcp|s/^\tbody: \[{\$/\tbody: [$stray_apis {/|_apisOutsideGcp.\"Stray APIs\"|sits outside the gcp frame"
	'pipe-target-unknown|s/label: "VLAN 1", sub: "EAD 1 · BGP"/&, to: "nowhere"/|_pipeTargetsUnknown.nowhere'
	'pipe-targets-equal|s/title: "On-prem router 1"/id: "r1", &/; s/label: "VLAN 1", sub: "EAD 1 · BGP"/&, from: "r1", to: "r1"/|_pipeTargetsEqual|the same node as pipe to'
	'pipe-target-on-tee-arm|s/title: "On-prem router 1"/id: "r1", &/; s/{tag: "Fact", text: "BGP peering[^}]*},/&\n{tag: "Tee", line: "solid", tint: 1, hub: "hub", arms: [{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 1", to: "r1"}, {tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "arm 2"}]},/|_targetOnTeeArm."0"'
	'chrome-none-three-keys-empty-legend|s/^\ttitle: /\tchrome: "none"\n&/; /^\tlegend: \[$/,/^\t\]$/d; s/^}$/\tlegend: []\n}/|_pipeKeysMissingFromLegend'
)
sequence_cases=(
	'ordered-link-to-a-node-that-is-not-a-lane-head|s/label: "Job service"/id: "job-service", &/; s/to: "console", line: "solid", label: "submit job"/to: "job-service", line: "solid", label: "submit job"/|_orderedLinkNotBetweenLaneHeads."0"'
	'two-ordered-links-with-one-order|s/label: "POST \/jobs", order: 2/label: "POST \/jobs", order: 1/|_linkOrderUsedTwice."1"'
	'line-kind-corner-above-16|s/^figure: plain.#Page \& {/_house: plain.grammar \& {lines: [{line: "solid", corner: 17}]}\n&/|_house.lines.0.corner: invalid value 17'
	'needs-sheet-view|s/^\tbody: \[{$/\tbody: [{tag: "View", title: "Extra", children: [{tag: "Item", kind: "service", title: "Aside", subtitle: "in a view"}]}, {/|_needsSheet.view'
	'needs-sheet-actors|s/^\tgrammar: "plain"$/&\n\tactors: [{id: "crew", name: "Crew", tint: 2}]/|_needsSheet.actors'
	'needs-sheet-actor|s/title: "Operator", subtitle: "submits and cancels jobs"/actor: "crew", &/|_needsSheet.actor'
	'needs-sheet-legend-by-actor|s/{line: "solid", text: "call"}/{line: "solid", by_actor: true, text: "call"}/|_needsSheet."legend-by-actor"'
	'needs-sheet-phases|s/^\tbody: \[{$/\tbody: [{tag: "Phases", lanes: ["crew"], phases: [{id: "run", title: "Run"}], children: [{tag: "Step", lane: "crew", phase: "run", title: "Work"}]}, {/|_needsSheet.phases'
	'needs-sheet-marks|s/title: "Operator", subtitle: "submits and cancels jobs"/marks: [{part: "console"}], &/|_needsSheet.marks'
	'needs-sheet-parts-list|s/^\tbody: \[{$/\tbody: [{tag: "PartsList", title: "Parts", parts: [{id: "console", name: "Console", why: "Submits jobs."}]}, {/|_needsSheet."parts-list"'
	'needs-sheet-general-notes|s/^\tbody: \[{$/\tbody: [{tag: "GeneralNotes", title: "Notes", notes: [{id: "retry", text: "Jobs retry once."}]}, {/|_needsSheet."general-notes"'
	'needs-sheet-step|s/^\tbody: \[{$/\tbody: [{tag: "Phases", lanes: ["crew"], phases: [{id: "run", title: "Run"}], children: [{tag: "Step", lane: "crew", phase: "run", title: "Work"}]}, {/|_needsSheet.step'
	'line-kind-listed-twice|s/^figure: plain.#Page \& {/_house: plain.grammar \& {lines: [{line: "dash", corner: 4}, {line: "dash", bend: "curve"}]}\n&/|_house._linesAreUnique'
)
# A subnet directly in a View sits in the page, since a View is transparent for
# kind nesting as Row, Col and Lanes are.
sheet_cases=(
	'kind-parent-through-view|s/^\t\t\tchildren: \[$/\t\t\tchildren: [{tag: "Box", kind: "subnet", label: "stray subnet", children: [{tag: "Fact", text: "no region around it"}]},/|onepager._kindParentNotAllowed.subnet|cannot sit in page'
	'actor-tint-duplicate|s/text: "one job per clip", tint: 6/text: "one job per clip", tint: 4/|_actorTintsTwice."4"'
	'actor-unknown|s/actor:    "api"/actor:    "apis"/|_actorsUnknown.apis'
	'tint-on-actor-page|s/{from: "s-describe", to: "s-compile", line: "solid"}/{from: "s-describe", to: "s-compile", line: "solid", tint: 2}/|_tintOnActorPage'
	'kind-tint-without-actor|s/^\t\t\tchildren: \[$/\t\t\tchildren: [{tag: "Box", kind: "gcp", label: "Edge", children: [{tag: "Box", kind: "region", label: "Edge region", children: [{tag: "Fact", text: "no actor"}]}]},/|_kindTintWithoutActor."Edge region"'
	'line-without-actor|s/^\tlinks: \[$/&\n\t\t{from: "verdict-store", to: "worker", line: "solid"},/|_lineWithoutActor'
	'actor-on-neutral-line|s/line:      "deny"/&\n\t\t\tactor:     "api"/|_actorOnNeutralLine'
	'actor-on-untintable-kind|s/label: "Google Cloud"/&\n\t\t\t\t\t\t\tactor: "api"/|_actorOnUntintableKind'
	'legend-by-actor-misused|s/{line: "deny", text: "blocked path"}/{line: "deny", by_actor: true, text: "blocked path"}/|_legendByActorMisused'
	'key-duplicate-actor|s/{id: "model", name: "Evaluator"/{id: "worker", name: "Evaluator"/|_keysUsedTwice."actor.worker"'
	'key-duplicate-phase|s/{id: "review", title: "Review"}/{id: "plan", title: "Review"}/|_phaseIdsUsedTwice.plan'
	'phases-child-not-step|s/^\t\t\t\t\tchildren: \[$/&\n\t\t\t\t\t\t{tag: "Fact", text: "loose"},/|children.0.tag: conflicting values'
	'step-outside-phases|s/^\t\t\tchildren: \[$/&\n\t\t\t\t{tag: "Step", lane: "api", phase: "plan", title: "Loose step"},/|_stepOutsidePhases."Loose step"'
	'phase-lane-twice|s/lanes: \["operator", "api", "worker", "model"\]/lanes: ["operator", "api", "worker", "model", "api"]/|_lanesUsedTwice.api'
	'step-lane-unknown|s/lanes: \["operator", "api", "worker", "model"\]/lanes: ["operator", "api", "worker"]/|_stepLanesUnknown'
	'step-phase-unknown|s/{id: "review", title: "Review"},//|_stepPhasesUnknown'
	'step-span-out-of-range|s/phase: "describe"/&, col: 5, span: 3/|_stepSpanOutOfRange'
	'step-text-fill|s/look:  "text"/&, fill: "tint"/|_stepTextFill'
	'step-fill-unknown|s/look:  "text"/look: "box", fill: "paper"/|fill: 3 errors in empty disjunction'
	'band-unknown|s/lanes: \["operator", "api", "worker", "model"\]/&, band: "solid"/|band: 2 errors in empty disjunction'
	'list-empty-lanes|s/lanes: \["operator", "api", "worker", "model"\]/lanes: []/|lanes: invalid value []|list.MinItems(1)'
	'list-empty-phases|/^\t\t\t\t\tphases: \[$/,/^\t\t\t\t\t\]$/d; s/^\t\t\t\t\tlanes: .*/&\n\t\t\t\t\tphases: []/|phases: invalid value []|list.MinItems(1)'
	'keys-used-actor|s/^\tactors: \[$/&\n\t\t{id: "auditor", name: "Auditor", tint: 7},/|_actorsUnused.auditor'
	"list-too-long-actors|s/^\tactors: \[$/&\n\t\t$nine_actors/|actors: invalid value|list.MaxItems(8)"
	"list-too-long-lanes|s/lanes: \[\"operator\"/lanes: [$nine_lanes \"operator\"/|lanes: invalid value|list.MaxItems(8)"
	"list-too-long-phases|s/^\t\t\t\t\tphases: \[$/&\n\t\t\t\t\t\t$nine_phases/|phases: invalid value|list.MaxItems(8)"
	"list-too-long-steps|s/^\t\t\t\t\tchildren: \[$/&\n\t\t\t\t\t\t$many_steps/|children: invalid value|list.MaxItems(64)"
	'mark-unknown-part|s/{part: "worker"}, {part: "queue"}/{part: "worker"}, {part: "scheduler"}/|_partMarksUnknown.scheduler'
	'mark-unknown-note|s/marks: \[{note: "no-store-credentials"}\]/marks: [{note: "no-credentials"}]/|_noteMarksUnknown."no-credentials"'
	'mark-unknown-attention|s/marks: \[{attention: "model-updates"}\]$/marks: [{attention: "model-update"}]/|_attentionMarksUnknown."model-update"'
	'link-marks-without-tag|/label:     "no direct write"/d|_linkMarksWithoutTag'
	'part-node-unknown|s/{id: "queue", node: "queue",/{id: "queue", node: "s-evaluate",/|_partNodesUnknown."s-evaluate"'
	'part-node-twice|s/{id: "queue", node: "queue",/{id: "queue", node: "worker",/|_partNodesTwice.worker'
	'part-field-from-node|s/{id: "gateway", node: "gateway",/& name: "Gateway",/|_partFieldFromNode'
	'keys-used-note|s/, {note: "no-store-credentials"}//; /marks: \[{note: "no-store-credentials"}\]/d|_notesUnflagged."no-store-credentials"'
	"list-too-long-step-lines|s/lines: \[\"in plain language\", \"camera and hours\"\]/lines: [$seven_lines]/|lines: invalid value|list.MaxItems(6)"
)
sheet_small_cases=(
	'sheet-in-iso|s/^\tgrammar: "plain"$/&\n\tprojection: "iso"/|_sheetInIso'
	'sheet-chrome-none|s/^\tgrammar: "plain"$/&\n\tchrome: "none"/|_sheetChromeNone'
	'sheet-body-not-view|s/^\t}]$/\t}, {tag: "Row", children: [{tag: "Item", kind: "service", title: "Loose", subtitle: "outside every view"}]}]/|_sheetBodyNotView."1"'
	'view-not-top-level|s/^\t\t}]$/\t\t}, {tag: "View", title: "Inner", children: [{tag: "Item", kind: "service", title: "Nested", subtitle: "inside another view"}]}]/|_viewNotTopLevel.Inner'
	'sheet-without-lines|/^\tlinks:/d; /tag: "Tee"/d; s/^\tlegend: .*/\tlegend: []/|_sheetWithoutLines'
	'list-empty-problem-points|s/points: \[{text: "Calls wait on a slow store.", marks: \[{attention: "staleness"}\]}\]/points: []/|sheet.problem.points: invalid value []|list.MinItems(1)'
	'list-empty-attention-items|s/items: \[{id: "staleness".*}\]$/items: []/|sheet.attention.items: invalid value []|list.MinItems(1)'
	'list-too-long-problem-points|s/points: \[{text: "Calls wait on a slow store.", marks: \[{attention: "staleness"}\]}\]/points: [{text: "point 1", marks: [{attention: "staleness"}]}, {text: "point 2"}, {text: "point 3"}, {text: "point 4"}, {text: "point 5"}, {text: "point 6"}, {text: "point 7"}, {text: "point 8"}, {text: "point 9"}]/|list.MaxItems(8)'
	'list-too-long-answer-paragraphs|s/paragraphs: \[{lead: "Cached reads.", text: "A read never waits on the store."}\]/paragraphs: ["paragraph 1", "paragraph 2", "paragraph 3", "paragraph 4", "paragraph 5"]/|list.MaxItems(4)'
	'list-too-long-attention-items|s/items: \[{id: "staleness".*}\]$/items: [{id: "item-1", lead: "Item 1.", text: "text"}, {id: "item-2", lead: "Item 2.", text: "text"}, {id: "item-3", lead: "Item 3.", text: "text"}, {id: "item-4", lead: "Item 4.", text: "text"}, {id: "item-5", lead: "Item 5.", text: "text"}, {id: "item-6", lead: "Item 6.", text: "text"}, {id: "item-7", lead: "Item 7.", text: "text"}, {id: "item-8", lead: "Item 8.", text: "text"}, {id: "item-9", lead: "Item 9.", text: "text"}]/|list.MaxItems(8)'
	'sheet-text-too-long-wordmark|s/wordmark: "relay"/wordmark: "wwwwwwwwwwwwwwwwwwwwwwwww"/|sheet.wordmark: invalid value|strings.MaxRunes(24)'
	'sheet-text-too-long-caption|s/caption: "Problem"/caption: "ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"/|sheet.problem.caption: invalid value|strings.MaxRunes(60)'
	'tee-arm-actor-differs|s/^\tgrammar: "plain"$/&\n\tactors: [{id: "crew", name: "Crew", tint: 2}, {id: "ops", name: "Ops", tint: 3}]/; s/{tag: "Tee", line: "solid", hub: "fan out", arms: \[{tag: "Pipe", dir: "h", line: "solid", label: "read"}/{tag: "Tee", line: "solid", actor: "crew", hub: "fan out", arms: [{tag: "Pipe", dir: "h", line: "solid", actor: "ops", label: "read"}/; s/id: "client", kind: "service"/&, actor: "crew"/|_teeArmActorDiffers."0"'
	'key-duplicate-attention|s/items: \[{id: "staleness", \(.*\)}\]$/items: [{id: "staleness", \1}, {id: "staleness", \1}]/|_keysUsedTwice."attention.staleness"'
	'link-via-outside-sheet|s/x: 1375/x: 1381/|_viaOutsidePage'
	'mark-repeated-carrier|s/marks: \[{part: "cache"}\]/marks: [{part: "cache"}, {part: "cache"}]/|_marksRepeated."part.cache"'
	'mark-repeated-balloon|s/subtitle: "web front end"/&, marks: [{part: "client"}]/|_partBalloonRepeated.client'
	'mark-legend-unused|s/tag:   "GeneralNotes"/tag: "Fact", text: "no notes"/; /title: "General notes"/d; /notes: \[{id: "bounded"/d; s/, marks: \[{note: "bounded"}\]//; s/^\tsheet: {$/\tsheet: {marks: {note: "a rule"}/|_markLegendUnused.note'
	'keyed-list-twice|s/notes: \[{id: "bounded", text: "A cached answer is never older than a minute."}\]/&\n\t\t}, {\n\t\t\ttag: "GeneralNotes", title: "More notes", notes: [{id: "second", text: "Another note."}]/; s/marks: \[{note: "bounded"}\]/marks: [{note: "bounded"}, {note: "second"}]/|_keyedListTwice.GeneralNotes'
	'part-name-missing|s/{id: "cache", name: "Cache", /{id: "cache", /|_partNameMissing'
	'key-duplicate-part|s/{id: "cache", name/{id: "client", name/; s/marks: \[{part: "cache"}\]/marks: [{part: "client"}]/|_keysUsedTwice."part.client"'
	'key-duplicate-note|s/notes: \[{id: "bounded", text: "\([^"]*\)"}\]/notes: [{id: "bounded", text: "\1"}, {id: "bounded", text: "Again."}]/|_keysUsedTwice."note.bounded"'
	'list-too-long-marks|s/marks: \[{attention: "staleness"}\]/marks: [{attention: "staleness"}, {part: "client"}, {part: "cache"}, {note: "bounded"}, {part: "client"}]/|marks: invalid value|list.MaxItems(4)'
	"list-too-long-parts|s/^\t\t\tparts: \[$/&\n\t\t\t\t$many_parts/|parts: invalid value|list.MaxItems(48)"
	"list-too-long-general-notes|s/notes: \[{id: \"bounded\"/notes: [$many_notes {id: \"bounded\"/|notes: invalid value|list.MaxItems(16)"
	'list-empty-parts|/^\t\t\tparts: \[$/,/^\t\t\t\]$/d; s/^\t\t\ttag:   "PartsList"$/&\n\t\t\tparts: []/; s/, marks: \[{part: "cache"}\]//|parts: invalid value []|list.MinItems(1)'
	'list-empty-general-notes|s/notes: \[{id: "bounded".*\]$/notes: []/; s/, marks: \[{note: "bounded"}\]//|notes: invalid value []|list.MinItems(1)'
	'keys-used-part|s/{id: "cache", name: "Cache"/{id: "spare", name: "Spare", why: "Kept for later."},\n\t\t\t\t&/|_partsUnmarked.spare'
	'keys-used-attention|s/items: \[{id: "staleness", \(.*\)}\]$/items: [{id: "staleness", \1}, {id: "later", lead: "Later.", text: "Not marked anywhere."}]/|_attentionUnmarked.later'
)
cases=()
for entry in "${g7_cases[@]}"; do
	cases+=("g7|$entry")
done
for entry in "${sequence_cases[@]}"; do
	cases+=("sequence|$entry")
done
for entry in "${sheet_cases[@]}"; do
	cases+=("sheet|$entry")
done
for entry in "${sheet_small_cases[@]}"; do
	cases+=("sheet-small|$entry")
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
failed=0
for entry in "${cases[@]}"; do
	also=""
	IFS='|' read -r figure name expression expected also <<<"$entry"
	case "$figure" in
	g7) package=gcp ;;
	sequence) package=plain ;;
	sheet) package=gcp ;;
	sheet-small) package=plain ;;
	*)
		echo "FAIL $name: no package for figure $figure" >&2
		failed=$((failed + 1))
		continue
		;;
	esac
	dir="$work/$name"
	mkdir -p "$dir"
	cp -R "$here/cue.mod" "$here/core.cue" "$here/grammar.cue" "$here/theme.cue" "$here/grammars" "$here/figures" "$dir/"
	sed -e "$expression" "$here/figures/$figure.cue" >"$dir/figures/$figure.cue"
	if cmp -s "$here/figures/$figure.cue" "$dir/figures/$figure.cue"; then
		echo "FAIL $name: edit did not change figures/$figure.cue" >&2
		failed=$((failed + 1))
		continue
	fi
	if report="$(cd "$dir" && "$CUE" vet -c "./figures:$package" 2>&1)"; then
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
