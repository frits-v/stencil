// The core vocabulary shared by every grammar. A #Page describes what a figure
// shows: boxes, items, facts and the pipes and links between them. It carries
// no coordinates; the renderer decides geometry. Box and Item kinds are any
// #KindName here; a grammar package (grammars/) narrows them to its own kinds
// and adds its domain rules.
package stencil

import (
	"list"
	"strings"
)

// Values that read as plausible and are wrong often enough that no string in
// a figure may carry them. They stay in the core under every grammar, because
// #Text applies them inside every node and a grammar package cannot reach in.
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

// A page body and the children of a Row, Col or Box: 1 to 256 nodes, as in
// the Rust model.
#Children: [...#Node] & list.MinItems(1) & list.MaxItems(256)

#Canvas:      "customer" | "internal"
#Line:        "gray" | "solid" | "dash" | "deny"
#TintSlot:    int & >=1 & <=8
#FactSource:  "doc" | "built" | "ask"
#Chrome:      "full" | "none"
#NoteKind:    "kicker" | "h1" | "lede" | "legend" | "foot"
#Justify:     "start" | "center" | "end" | "space-between"
#ThemeName:   "center" | "dusk" | "wire"
#Projection:  "flat" | "iso"
#Arrow:       "none" | "end" | "start" | "both"
#Side:        "top" | "right" | "bottom" | "left"
#ListKind:    "plain" | "numbered" | "bulleted"
#CalloutKind: "note" | "risk" | "decision" | "open"

// A built-in grammar name or a path to an exported grammar file, the
// GRAMMAR_REFERENCE_PATTERN of the Rust model.
#GrammarReference: =~"^(gcp|plain|[^\\x00-\\x1F]{1,395}\\.json)$"

// A node id, the pattern of the Rust model's `id` field. Links name nodes by it.
#Id: =~"^[a-z0-9][a-z0-9-]{0,63}$"

// Filename stems under drafting-diagrams/stencil/icons/.
#Icon: "agents" | "ai-ml" | "bigquery" | "cloud-run-flat" | "cloud-run" | "cloud-sql" |
	"cloud-storage" | "compute-engine" | "compute" | "containers" | "data-analytics" |
	"databases" | "devops" | "gke" | "hybrid" | "integration" | "networking" |
	"observability" | "scc" | "security-identity" | "serverless" | "storage" | "vertex-ai"

// The legend key of a line: the line and its tint for solid and dash, slot 1
// when tint is absent, and the line alone for gray and deny, whose tint has
// no effect. Pipe (Tee arms included), Tee, Link and LegendEntry each carry
// one as _lineKey.
#LineKey: {
	#line:  #Line
	#tint?: #TintSlot
	key: [
		if #line == "gray" || #line == "deny" {#line},
		if #tint != _|_ {"\(#line)-\(#tint)"},
		"\(#line)-1",
	][0]
	if #tint != _|_ if #line == "gray" || #line == "deny" {
		_tintWithoutEffect: "tint \(#tint)" & "has no effect on a \(#line) line"
	}
}

#Page: {
	title:       #Text
	kicker:      #Text
	lede:        #Text
	foot?:       #Text
	width?:      int & >=640 & <=2560
	canvas:      #Canvas
	grammar?:    #GrammarReference
	theme?:      #ThemeName
	projection?: #Projection
	chrome?:     #Chrome
	body:        #Children
	legend: [...#LegendEntry] & list.MaxItems(16)
	links?: [...#Link] & list.MaxItems(256)

	// The canvas is the page width plus 20 px padding on each side; a via
	// point must lie inside it. Its height is known only after layout.
	_canvasWidth: [if width != _|_ {width + 40}, 1320][0]
	_chrome: [if chrome != _|_ {chrome}, "full"][0]
	_links: [if links != _|_ for l in links {l}]

	// Each check below is a struct keyed by the offending item, so a failure
	// names the item in its path: _legendKeysUnusedInBody.deny: conflicting values ...
	_pipeKeys: {for n in body {n._keys}}
	_linkKeys: {for l in _links {(l._lineKey.key): true}}
	_usedKeys: [for k, _ in {_pipeKeys, _linkKeys} {k}]
	_legendKeys: [for e in legend {e._lineKey.key}]
	_legendKeysAreUnique: _legendKeys & list.UniqueItems()
	// A figure captioned by its document (chrome none) whose lines all share
	// one key may leave the legend empty.
	_legendMayBeEmpty: _chrome == "none" && len(legend) == 0 && len(_usedKeys) == 1
	if !_legendMayBeEmpty {
		_pipeKeysMissingFromLegend: {
			for k, _ in _pipeKeys if !list.Contains(_legendKeys, k) {(k): "pipe line in body" & "missing from legend"}
		}
		_linkKeysMissingFromLegend: {
			for k, _ in _linkKeys if !list.Contains(_legendKeys, k) {(k): "link line" & "missing from legend"}
		}
	}
	_legendKeysUnusedInBody: {
		for k in _legendKeys if !list.Contains(_usedKeys, k) {(k): "legend entry" & "no pipe or link with this line and tint"}
	}

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
	// Each Lanes node's head ids, one list per node. An ordered link is a message
	// of exactly the Lanes node whose heads it joins, and one node uses each order
	// once.
	_laneSets: list.Concat([for n in body {n._laneSets}])
	_orderedLinks: [for i, l in _links if l.order != _|_ {
		index: i
		order: l.order
		lanes: [for s, heads in _laneSets if l.from != l.to && list.Contains(heads, l.from) && list.Contains(heads, l.to) {s}]
	}]
	_orderedLinkNotBetweenLaneHeads: {
		for m in _orderedLinks if len(m.lanes) == 0 {"\(m.index)": "ordered link" & "joins two lane heads of one Lanes node"}
	}
	_linkOrderUsedTwice: {
		for a in _orderedLinks for b in _orderedLinks if a.index < b.index && a.order == b.order && len(a.lanes) > 0 && a.lanes == b.lanes {"\(b.order)": "order" & "used by two links of one Lanes node"}
	}
	_viaOutsidePage: {
		for i, l in _links if l.via != _|_ for j, p in l.via if p.x > _canvasWidth {"\(i).\(j)": "via x" & "beyond the canvas width"}
	}
}

#LegendEntry: {
	line:  #Line
	tint?: #TintSlot
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}}
	text: #Text
}

// A routed line between two nodes, named by id. It takes no layout space.
#Link: {
	from:  #Id
	to:    #Id
	line:  #Line
	tint?: #TintSlot
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}}
	label?:     #Text
	sub?:       #Text
	arrow?:     #Arrow
	from_side?: #Side
	to_side?:   #Side
	via?: [...#PagePoint] & list.MaxItems(8)
	// A message of a Lanes node: its place along the time axis.
	order?: int & >=1 & <=256
}

#PagePoint: {
	x: number & >=0
	y: number & >=0
}

// A node is one of twelve tags. Switching on tag, rather than a disjunction of
// definitions, keeps vet errors pointed at the offending field. Every node may
// carry an id; _ids lists the ids in its subtree and _keys the legend keys of
// the pipes and tees in it.
#Node: {
	tag: "Row" | "Col" | "Lanes" | "Box" | "Item" | "Fact" | "Note" | "Pipe" | "Tee" | "Text" | "Callout" | "Frame"
	if tag == "Row" {#Row}
	if tag == "Col" {#Col}
	if tag == "Lanes" {#Lanes}
	if tag == "Box" {#Box}
	if tag == "Item" {#Item}
	if tag == "Fact" {#Fact}
	if tag == "Note" {#Note}
	if tag == "Pipe" {#Pipe}
	if tag == "Tee" {#Tee}
	if tag == "Text" {#TextBlock}
	if tag == "Callout" {#Callout}
	if tag == "Frame" {#Frame}
}

// Shared by Row and Col. grow carries one weight per child.
#Container: {
	id?:  #Id
	gap?: int & >=0 & <=64
	grow?: [...int & >=0 & <=100]
	justify?: #Justify
	children: #Children
	if grow != _|_ {
		_growLengthMatchesChildren: len(grow) & len(children)
	}
	_keys: {for c in children {c._keys}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	_laneSets: list.Concat([for c in children {c._laneSets}])
}

#Row: {
	tag: "Row"
	#Container
}

#Col: {
	tag: "Col"
	#Container
}

// Columns that share one vertical axis. Each child is a lane head: 1 to 32.
#Lanes: {
	id?:  #Id
	tag:  "Lanes"
	gap?: int & >=0 & <=64
	children: [...#Node] & list.MinItems(1) & list.MaxItems(32)
	_keys: {for c in children {c._keys}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	// The ids of this node's lane heads, then the sets of any Lanes below them.
	_laneSets: list.Concat([[[for c in children if c.id != _|_ {c.id}]], for c in children {c._laneSets}])
}

// Any container. The grammar's container kind decides how it is drawn and
// where it may sit; tint picks a slot on a tintable kind.
#Box: {
	id?:      #Id
	tag:      "Box"
	kind:     #KindName
	tint?:    #TintSlot
	label:    #Text
	children: #Children
	_keys: {for c in children {c._keys}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	_laneSets: list.Concat([for c in children {c._laneSets}])
}

// Any named leaf, drawn with its facts in list order under title and subtitle.
#Item: {
	id?:       #Id
	tag:       "Item"
	kind:      #KindName
	icon?:     #Icon
	title:     #Text
	subtitle?: #Text
	facts?: [...#FactEntry] & list.MaxItems(8)
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
}

// doc: read from the live documentation when the figure was authored (the
// default). built: an as-built name read off the running system. ask: an open
// question for the reader.
#FactEntry: {
	text:    #Text
	source?: #FactSource
}

#Fact: {
	id?:     #Id
	tag:     "Fact"
	text:    #Text
	source?: #FactSource
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
}

#Note: {
	id?:  #Id
	tag:  "Note"
	kind: #NoteKind
	text: #Text
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
}

#Pipe: {
	id?:   #Id
	tag:   "Pipe"
	dir:   "h" | "v"
	line:  #Line
	tint?: #TintSlot
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}}
	label:  #Text
	sub?:   #Text
	arrow?: #Arrow
	_keys: {(_lineKey.key): true}
	_ids: [if id != _|_ {id}]
	_laneSets: []
}

// One source fanned to two destinations. line colours the spine; each arm is
// a full pipe with its own line and label.
#Tee: {
	id?:   #Id
	tag:   "Tee"
	line:  #Line
	tint?: #TintSlot
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}}
	hub: #Text
	arms: [#Pipe, #Pipe]
	_keys: {(_lineKey.key): true, arms[0]._keys, arms[1]._keys}
	_ids: list.Concat([[if id != _|_ {id}], arms[0]._ids, arms[1]._ids])
	_laneSets: []
}

// The lines of a Text block, as in the Rust model: 1 to 64. Declared outside
// #TextBlock because its `list` field shadows the list package there.
#TextLines: [...#Text] & list.MinItems(1) & list.MaxItems(64)

// Document blocks for one-page design documents. They are leaves: no pipes,
// boxes or items below them. #TextBlock is the Text tag; #Text is already the
// name of a text value.
#TextBlock: {
	id?:      #Id
	tag:      "Text"
	heading?: #Text
	body:     #TextLines
	list?:    #ListKind
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
}

#Callout: {
	id?:    #Id
	tag:    "Callout"
	kind:   #CalloutKind
	title?: #Text
	text:   #Text
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
}

// A wireframe placeholder. height is authored; the Rust default is 200.
#Frame: {
	id?:     #Id
	tag:     "Frame"
	label:   #Text
	height?: int & >=40 & <=1200
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
}
