// Structural schema for architecture figures. A #Page describes what a figure
// shows: zones, product cards, facts and the pipes between them. It carries no
// coordinates; the renderer decides geometry.
package stencil

import (
	"list"
	"strings"
)

// Values that read as plausible and are wrong often enough that no string in
// a figure may carry them:
//   64512           example ASN in the Dedicated Interconnect doc, not a requirement
//   130.211.0.0/22  health-check probe range, not used by serverless NEG backends
//   35.191.0.0/16   health-check probe range, same
//   10.8.0.0/28     Serverless VPC Access connector example range, absent on Direct VPC egress
#NoRememberedConstant: !~"\\b64512\\b" &
	!~"\\b130\\.211\\.0\\.0/22\\b" &
	!~"\\b35\\.191\\.0\\.0/16\\b" &
	!~"\\b10\\.8\\.0\\.0/28\\b"

// Every text field in the vocabulary. The bounds match the Rust model
// (crates/stencil-model/src/document.rs): 1 to 400 characters.
#Text: string & !="" & strings.MaxRunes(400) & #NoRememberedConstant

// A page body and the children of a Row, Col or Zone: 1 to 256 nodes, as in
// the Rust model.
#Children: [...#Node] & list.MinItems(1) & list.MaxItems(256)

#Canvas:   "customer" | "internal"
#PipeKind: "gray" | "blue" | "pink" | "dash" | "deny"
#ZoneKind: "gcp" | "vpc" | "region-a" | "region-b" | "subnet" | "onprem-a" | "onprem-b" |
		"project" | "optional" | "k8s" | "perimeter"
#NoteKind:    "kicker" | "h1" | "lede" | "legend" | "foot"
#Justify:     "start" | "center" | "end" | "space-between"
#Theme:       "center" | "dusk" | "wire"
#Arrow:       "none" | "end" | "start" | "both"
#Side:        "top" | "right" | "bottom" | "left"
#ListKind:    "plain" | "numbered" | "bulleted"
#CalloutKind: "note" | "risk" | "decision" | "open"

// A node id, the pattern of the Rust model's `id` field. Links name nodes by it.
#Id: =~"^[a-z0-9][a-z0-9-]{0,63}$"

// Filename stems under drafting-diagrams/stencil/icons/.
#Icon: "agents" | "ai-ml" | "bigquery" | "cloud-run-flat" | "cloud-run" | "cloud-sql" |
	"cloud-storage" | "compute-engine" | "compute" | "containers" | "data-analytics" |
	"databases" | "devops" | "gke" | "hybrid" | "integration" | "networking" |
	"observability" | "scc" | "security-identity" | "serverless" | "storage" | "vertex-ai"

// Tint pairing. Metro 1 and Region A are blue, Metro 2 and Region B are pink,
// end to end. The map is closed and covers every zone kind and pipe kind: a
// lookup of a key missing from an open struct is an incomplete value, and a
// hidden check built on one passes vet without having run.
#TintOf: {
	"region-a": "a"
	"onprem-a": "a"
	blue:       "a"
	"region-b": "b"
	"onprem-b": "b"
	pink:       "b"
	gcp:        "none"
	vpc:        "none"
	subnet:     "none"
	project:    "none"
	optional:   "none"
	k8s:        "none"
	perimeter:  "none"
	gray:       "none"
	dash:       "none"
	deny:       "none"
}

#Page: {
	title:  #Text
	kicker: #Text
	lede:   #Text
	foot?:  #Text
	width?: int & >=640 & <=2560
	canvas: #Canvas
	theme?: #Theme
	body:   #Children
	// Unique kinds bound the legend at five entries, inside the Rust
	// model's 16.
	legend: [...#LegendEntry]
	links?: [...#Link] & list.MaxItems(256)

	// The canvas is the page width plus 20 px padding on each side; a via
	// point must lie inside it. Its height is known only after layout.
	_canvasWidth: [if width != _|_ {width + 40}, 1320][0]
	_links: [if links != _|_ for l in links {l}]

	// Each check below is a struct keyed by the offending item, so a failure
	// names the item in its path: _legendKindsUnusedInBody.deny: conflicting values ...
	_used: {for n in body {n._kinds}}
	_linkKinds: {for l in _links {(l.kind): true}}
	_usedKinds: [for k, _ in {_used, _linkKinds} {k}]
	_ids: list.Concat([for n in body {n._ids}])
	_sortedIds: list.SortStrings(_ids)
	_idsUsedTwice: {
		for i, a in _sortedIds if i > 0 if _sortedIds[i-1] == a {(a): "id" & "used by two nodes"}
	}
	_linkEndpointsUnknown: {
		for l in _links for e in [l.from, l.to] if !list.Contains(_ids, e) {(e): "link endpoint" & "names no node id"}
	}
	_linkToItself: {
		for l in _links if l.from == l.to {(l.from): "link from" & "the same node as link to"}
	}
	_linkKindsMissingFromLegend: {
		for k, _ in _linkKinds if !list.Contains(_legendKinds, k) {(k): "link kind" & "missing from legend"}
	}
	_viaOutsidePage: {
		for i, l in _links if l.via != _|_ for j, p in l.via if p.x > _canvasWidth {"\(i).\(j)": "via x" & "beyond the canvas width"}
	}
	_legendKinds: [for e in legend {e.kind}]
	_nameless: {for n in body {n._nameless}}
	_legendKindsAreUnique: _legendKinds & list.UniqueItems()
	_pipeKindsMissingFromLegend: {
		for k, _ in _used if !list.Contains(_legendKinds, k) {(k): "pipe kind in body" & "missing from legend"}
	}
	_legendKindsUnusedInBody: {
		for k in _legendKinds if !list.Contains(_usedKinds, k) {(k): "legend entry" & "no pipe or link of this kind"}
	}
	_pcardsWithoutPnFactOrAsk: {
		for k, _ in _nameless {(k): "product card" & "needs pn, fact or ask"}
	}
}

#LegendEntry: {
	kind: #PipeKind
	text: #Text
}

// A routed line between two nodes, named by id. It takes no layout space.
#Link: {
	from:       #Id
	to:         #Id
	kind:       #PipeKind
	label?:     #Text
	sub?:       #Text
	arrow?:     #Arrow
	from_side?: #Side
	to_side?:   #Side
	via?: [...#PagePoint] & list.MaxItems(8)
}

#PagePoint: {
	x: number & >=0
	y: number & >=0
}

// A node is one of eleven tags. Switching on tag, rather than a disjunction of
// definitions, keeps vet errors pointed at the offending field. Every node may
// carry an id; _ids lists the ids in its subtree.
#Node: {
	tag: "Row" | "Col" | "Zone" | "Pcard" | "Fact" | "Note" | "Pipe" | "Tee" | "Text" | "Callout" | "Frame"
	if tag == "Row" {#Row}
	if tag == "Col" {#Col}
	if tag == "Zone" {#Zone}
	if tag == "Pcard" {#Pcard}
	if tag == "Fact" {#Fact}
	if tag == "Note" {#Note}
	if tag == "Pipe" {#Pipe}
	if tag == "Tee" {#Tee}
	if tag == "Text" {#TextBlock}
	if tag == "Callout" {#Callout}
	if tag == "Frame" {#Frame}
}

// A pipe that sits between two sibling zones must not touch a zone of the
// other tint: a blue pipe beside a region-b or onprem-b zone is a bug. Row,
// Col and Zone embed this, so the check holds wherever siblings sit.
#SiblingTint: {
	children: [...#Node]
	_pipeBesideZoneOfOtherTint: {
		for i, c in children if c.tag == "Pipe" {
			for j in [i - 1, i + 1] if j >= 0 && j < len(children) {
				if children[j].tag == "Zone" {
					let pipeTint = #TintOf[c.kind]
					let zoneTint = #TintOf[children[j].kind]
					if pipeTint != "none" && zoneTint != "none" && pipeTint != zoneTint {
						(c.label): "\(c.kind) pipe" & "beside \(children[j].kind) zone"
					}
				}
			}
		}
	}
}

// Shared by Row and Col. _kinds is the set of pipe kinds in the subtree and
// _zoneKinds the set of zone kinds; #Page and #Zone read them. grow carries
// one weight per child.
#Container: {
	id?:  #Id
	gap?: int & >=0 & <=64
	grow?: [...int & >=0 & <=100]
	justify?: #Justify
	children: #Children
	if grow != _|_ {
		_growLengthMatchesChildren: len(grow) & len(children)
	}
	_kinds: {for c in children {c._kinds}}
	_zoneKinds: {for c in children {c._zoneKinds}}
	_nameless: {for c in children {c._nameless}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	#SiblingTint
}

#Row: {
	tag: "Row"
	#Container
}

#Col: {
	tag: "Col"
	#Container
}

// A tinted zone (region-a, region-b, onprem-a, onprem-b) must not contain a
// pipe, tee arm or nested zone of the other tint anywhere below it.
#Zone: {
	id?:      #Id
	tag:      "Zone"
	kind:     #ZoneKind
	label:    #Text
	children: #Children
	_kinds: {for c in children {c._kinds}}
	_below: {for c in children {c._zoneKinds}}
	_zoneKinds: {(kind): true, _below}
	_nameless: {for c in children {c._nameless}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	#SiblingTint
	if #TintOf[kind] != "none" {
		_otherTintInsideZone: {
			for k, _ in _kinds
			let t = #TintOf[k]
			if t != "none" && t != #TintOf[kind] {(k): "pipe kind" & "inside \(kind) zone"}
			for z, _ in _below
			let t = #TintOf[z]
			if t != "none" && t != #TintOf[kind] {(z): "zone kind" & "inside \(kind) zone"}
		}
	}
}

// A product card names a looked-up fact (pn or fact) or an explicit ask. This
// holds for every card, which covers the cards on hops and in regions. The
// check runs in #Page (_pcardsWithoutPnFactOrAsk) because a check on an
// optional field inside #Pcard would fail the definition itself.
#Pcard: {
	id?:   #Id
	tag:   "Pcard"
	icon?: #Icon
	fn:    #Text
	pn?:   #Text
	fact?: #Text
	ask?:  #Text
	_kinds: {}
	_zoneKinds: {}
	_nameless: {if pn == _|_ && fact == _|_ && ask == _|_ {(fn): true}}
	_ids: [if id != _|_ {id}]
}

#Fact: {
	id?:  #Id
	tag:  "Fact"
	text: #Text
	_kinds: {}
	_zoneKinds: {}
	_nameless: {}
	_ids: [if id != _|_ {id}]
}

#Note: {
	id?:  #Id
	tag:  "Note"
	kind: #NoteKind
	text: #Text
	_kinds: {}
	_zoneKinds: {}
	_nameless: {}
	_ids: [if id != _|_ {id}]
}

#Pipe: {
	id?:    #Id
	tag:    "Pipe"
	dir:    "h" | "v"
	kind:   #PipeKind
	label:  #Text
	sub?:   #Text
	arrow?: #Arrow
	_kinds: {(kind): true}
	_zoneKinds: {}
	_nameless: {}
	_ids: [if id != _|_ {id}]
}

// One source fanned to two destinations. kind colours the spine; each arm is
// a full pipe with its own kind and label.
#Tee: {
	id?:  #Id
	tag:  "Tee"
	kind: #PipeKind
	hub:  #Text
	arms: [#Pipe, #Pipe]
	_kinds: {(kind): true, arms[0]._kinds, arms[1]._kinds}
	_zoneKinds: {}
	_nameless: {}
	_ids: list.Concat([[if id != _|_ {id}], arms[0]._ids, arms[1]._ids])
}

// The lines of a Text block, as in the Rust model: 1 to 64. Declared outside
// #TextBlock because its `list` field shadows the list package there.
#TextLines: [...#Text] & list.MinItems(1) & list.MaxItems(64)

// Document blocks for one-page design documents. They are leaves: no pipe
// kinds, zone kinds or product cards below them. #TextBlock is the Text tag;
// #Text is already the name of a text value.
#TextBlock: {
	id?:      #Id
	tag:      "Text"
	heading?: #Text
	body:     #TextLines
	list?:    #ListKind
	_kinds: {}
	_zoneKinds: {}
	_nameless: {}
	_ids: [if id != _|_ {id}]
}

#Callout: {
	id?:    #Id
	tag:    "Callout"
	kind:   #CalloutKind
	title?: #Text
	text:   #Text
	_kinds: {}
	_zoneKinds: {}
	_nameless: {}
	_ids: [if id != _|_ {id}]
}

// A wireframe placeholder. height is authored; the Rust default is 200.
#Frame: {
	id?:     #Id
	tag:     "Frame"
	label:   #Text
	height?: int & >=40 & <=1200
	_kinds: {}
	_zoneKinds: {}
	_nameless: {}
	_ids: [if id != _|_ {id}]
}
