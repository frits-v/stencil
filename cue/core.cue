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

#Line:       "gray" | "solid" | "dash" | "deny"
#TintSlot:   int & >=1 & <=8
#FactSource: "doc" | "built" | "ask"
#Chrome:     "full" | "none"
#NoteKind:   "kicker" | "h1" | "lede" | "legend" | "foot"
#Justify:    "start" | "center" | "end" | "space-between"
#Projection: "flat" | "iso"
#Arrow:      "none" | "end" | "start" | "both"
#Side:       "top" | "right" | "bottom" | "left"

// How a routed link draws a bend: a quarter circle, a cubic with both control
// points on the corner, or a spline from leg middle to leg middle through the
// whole route.
#Bend: "arc" | "curve" | "spline"

// The radius of a routed link's bends, clamped by the renderer to half the
// shorter leg; 0 draws square corners.
#Corner: number & >=0 & <=16

// The reading axis of a tag under iso (section 12.4).
#Axis: "x" | "y"

// The solid an item stands as under iso (section 12.3).
#Shape:       "card" | "tile" | "tower" | "cylinder" | "stack" | "block" | "figure" | "laptop" | "phone"
#PipeForm:    "tube" | "band"
#ListKind:    "plain" | "numbered" | "bulleted"
#CalloutKind: "note" | "risk" | "decision" | "open"

// Every surface valid on a design sheet only; a page without sheet that draws
// one fails _needsSheet. The Rust twin is SheetSurface::as_str.
#SheetSurface: "sheet" | "actors" | "view" | "phases" | "step" | "actor" | "legend-by-actor"

// A title block cell caption: 1 to 60 characters.
#Caption: #Text & strings.MaxRunes(60)

// A design sheet's title block. The first cell is the page head; the other
// three are the problem, the answer and what the reader attends to.
#Sheet: {
	wordmark?: #Text & strings.MaxRunes(24)
	problem:   #Problem
	answer:    #Answer
	attention: #Attention
}

#Problem: {
	caption?: #Caption
	lead?:    #Text
	points: [...#ProblemPoint] & list.MinItems(1) & list.MaxItems(8)
}

#ProblemPoint: {
	text: #Text
}

// A paragraph is a string, or a bold lead with its text after it.
#AnswerParagraph: #Text | {lead: #Text, text: #Text}

#Answer: {
	caption?:  #Caption
	statement: #Text
	paragraphs?: [...#AnswerParagraph] & list.MaxItems(4)
}

#Attention: {
	caption?: #Caption
	items: [...#AttentionItem] & list.MinItems(1) & list.MaxItems(8)
}

#AttentionItem: {
	id:     #Id
	lead:   #Text
	text:   #Text
	actor?: #Id
}

// Who acts on a design sheet. Each actor owns one tint slot.
#Actor: {
	id:    #Id
	name:  #Text
	text?: #Text
	tint:  #TintSlot
}

// A phase of a Phases node: a column group left to right.
#Phase: {
	id:    #Id
	title: #Text
}

#StepLook: "box" | "chip" | "note" | "stack" | "planned" | "text"

// A built-in theme name or a path to a theme file, the THEME_REFERENCE_PATTERN
// of the Rust model.
#ThemeReference: =~"^(center|paper|dusk|clear|clear-dark|wire|tokyo-night|solarized-light|solarized-dark|material-dark|gruvbox-dark|dracula|nord|[^\\x00-\\x1F]{1,395}\\.json)$"

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
// when tint is absent, "<line>-by-actor" for a use with an effective actor or a
// by_actor entry, and the line alone for gray and deny, whose tint has no
// effect; a band form appends "-band", since a band and a tube of one
// line are two looks. Pipe (Tee arms included), Tee, Link and LegendEntry
// each carry one as _lineKey.
#LineKey: {
	#line:      #Line
	#tint?:     #TintSlot
	#form?:     #PipeForm
	#by_actor?: bool
	_lineTint: [
		if #line == "gray" || #line == "deny" {#line},
		if #by_actor != _|_ if #by_actor {"\(#line)-by-actor"},
		if #tint != _|_ {"\(#line)-\(#tint)"},
		"\(#line)-1",
	][0]
	key: [
		if #form != _|_ if #form == "band" {"\(_lineTint)-band"},
		_lineTint,
	][0]
	if #tint != _|_ if #line == "gray" || #line == "deny" {
		_tintWithoutEffect: "tint \(#tint)" & "has no effect on a \(#line) line"
	}
}

#Page: {
	title:  #Text
	kicker: #Text
	lede:   #Text
	foot?:  #Text
	width?: int & >=640 & <=2560
	// The pill before the kicker, printed uppercase; absent draws none.
	badge?:   #Text
	grammar?: #GrammarReference
	theme?:   #ThemeReference
	// A partial theme merged onto the chosen one. Open here: the Rust loader
	// checks it against #Theme, since a partial of a closed definition would
	// repeat #Theme field by field.
	theme_overrides?: {...}
	projection?: #Projection
	chrome?:     #Chrome
	// Absent is 6 and arc; a line kind of the grammar that sets its own wins.
	corner?: #Corner
	bend?:   #Bend
	// Draws the page as a design sheet: a frame, a title block and numbered
	// Views. Flat only.
	sheet?: #Sheet
	// Who acts on a design sheet; everything an actor does takes its slot.
	actors?: [...#Actor] & list.MaxItems(8)
	body:    #Children
	legend: [...#LegendEntry] & list.MaxItems(16)
	links?: [...#Link] & list.MaxItems(256)

	// The canvas is the page width plus the root padding on each side, 20 px
	// and 50 on a design sheet; a via point must lie inside it. Its height is
	// known only after layout.
	_rootPadding: [if sheet != _|_ {50}, 20][0]
	_canvasWidth: [if width != _|_ {width + 2*_rootPadding}, 1280 + 2*_rootPadding][0]
	_chrome: [if chrome != _|_ {chrome}, "full"][0]
	_links: [if links != _|_ for l in links {l}]
	_actors: [if actors != _|_ for a in actors {a}]
	_actorIds: [for a in _actors {a.id}]
	_hasActors: len(_actors) > 0

	// What each node with an id acts for: an Item's, Box's, Pipe's or Tee's
	// actor, a Step's lane. A line's effective actor is its own, else the actor
	// of the node it leaves; gray and deny lines act for no one.
	_nodes: list.Concat([for n in body {n._nodes}])
	_effectiveActor: {
		for i, l in _links {
			"\(i)": [
				if l.line == "gray" || l.line == "deny" {""},
				if l.actor != _|_ {l.actor},
				for n in _nodes if n.id == l.from if n.actor != _|_ {n.actor},
				"",
			][0]
		}
	}
	_lineUses: list.Concat([for n in body {n._lineUses}])
	_lineUseActor: [for u in _lineUses {[
		if u.line == "gray" || u.line == "deny" {""},
		if u.actor != _|_ {u.actor},
		for n in _nodes if u.from != _|_ if n.id == u.from if n.actor != _|_ {n.actor},
		"",
	][0]}]

	// Each check below is a struct keyed by the offending item, so a failure
	// names the item in its path: _legendKeysUnusedInBody.deny: conflicting values ...
	_pipeKeys: {
		if !_hasActors for n in body {n._keys}
		if _hasActors for i, u in _lineUses {
			let actor = _lineUseActor[i]
			let form = [if u.form == "band" {"-band"}, ""][0]
			if actor == "" {(u.key): true}
			if actor != "" {"\(u.line)-by-actor\(form)": true}
		}
	}
	_linkKeys: {
		for i, l in _links {
			if !_hasActors || _effectiveActor["\(i)"] == "" {(l._lineKey.key): true}
			if _hasActors && _effectiveActor["\(i)"] != "" {"\(l.line)-by-actor": true}
		}
	}
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
	_pipeTargets: list.Concat([for n in body {n._pipeTargets}])
	_pipeTargetsUnknown: {
		for t in _pipeTargets if !list.Contains(_ids, t) {(t): "pipe target" & "names no node id"}
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

	_surfaces: list.Concat([
		[if _hasActors {"actors"}],
		for n in body {n._surfaces},
		[for l in _links if l.actor != _|_ {"actor"}],
		[for e in legend if e.by_actor != _|_ if e.by_actor {"legend-by-actor"}],
	])
	if sheet == _|_ {
		_needsSheet: {
			for s in _surfaces {(s): "\(s) is drawn on a design sheet only" & "add sheet or remove it"}
		}
	}
	if sheet != _|_ {
		if projection != _|_ if projection == "iso" {
			_sheetInIso: "a design sheet" & "is drawn flat only"
		}
		if _chrome == "none" {
			_sheetChromeNone: "a design sheet draws its head in the title block" & "chrome cannot be none"
		}
		_sheetBodyNotView: {
			for i, n in body if n.tag != "View" {"\(i)": "\(n.tag) in the body of a design sheet" & "wrap it in a View"}
		}
		_viewNotTopLevel: {
			for n in body for t in n._viewTitles {(t): "View" & "a body child of a sheet"}
		}
		if len(_usedKeys) == 0 {
			_sheetWithoutLines: "a design sheet" & "draws at least one Pipe, Tee or Link"
		}
		_attentionIds: list.SortStrings([for item in sheet.attention.items {item.id}])
		_sortedActorIds: list.SortStrings(_actorIds)
		_keysUsedTwice: {
			for i, a in _attentionIds if i > 0 if _attentionIds[i-1] == a {"attention.\(a)": "key" & "used by two entries of one list"}
			for i, a in _sortedActorIds if i > 0 if _sortedActorIds[i-1] == a {"actor.\(a)": "key" & "used by two entries of one list"}
		}
		_actorTints: list.SortStrings([for a in _actors {"\(a.tint)"}])
		_actorTintsTwice: {
			for i, t in _actorTints if i > 0 if _actorTints[i-1] == t {(t): "tint" & "used by two actors"}
		}
		_namedActors: list.Concat([
			[for n in _nodes if n.actor != _|_ {n.actor}],
			[for u in _lineUses if u.actor != _|_ {u.actor}],
			[for l in _links if l.actor != _|_ {l.actor}],
			[for item in sheet.attention.items if item.actor != _|_ {item.actor}],
			list.Concat([for n in body {n._lanes}]),
		])
		_actorsUnknown: {
			for a in _namedActors if !list.Contains(_actorIds, a) {(a): "actor" & "is not declared"}
		}
		_actorsUnused: {
			for a in _actorIds if !list.Contains(_namedActors, a) {(a): "actor" & "named by no lane, node, line or attention item"}
		}
		if _hasActors {
			_tintOnActorPage: {
				for i, l in _links if l.tint != _|_ {"link.\(i)": "tint" & "a page with actors colours by actor"}
				for i, e in legend if e.tint != _|_ {"legend.\(i)": "tint" & "a page with actors colours by actor"}
				for i, u in _lineUses if u.tint != _|_ {"line.\(i)": "tint" & "a page with actors colours by actor"}
			}
			_lineWithoutActor: {
				for i, l in _links if (l.line == "solid" || l.line == "dash") && _effectiveActor["\(i)"] == "" {"link.\(i)": "\(l.line) line from \(l.from)" & "has an actor"}
				for i, u in _lineUses if (u.line == "solid" || u.line == "dash") && _lineUseActor[i] == "" {"line.\(i)": "\(u.line) line" & "has an actor"}
			}
		}
		_actorOnNeutralLine: {
			for i, l in _links if l.actor != _|_ if l.line == "gray" || l.line == "deny" {"link.\(i)": "a gray or deny line" & "takes no actor"}
			for i, u in _lineUses if u.ownActor != _|_ if u.line == "gray" || u.line == "deny" {"line.\(i)": "a gray or deny line" & "takes no actor"}
		}
		_legendByActorMisused: {
			for i, e in legend if e.by_actor != _|_ if e.by_actor if !_hasActors || e.line == "gray" || e.line == "deny" {"\(i)": "by_actor" & "names solid or dashed lines on a page with actors"}
		}
		_looseSteps: list.Concat([for n in body {n._looseSteps}])
		_stepOutsidePhases: {
			for t in _looseSteps {(t): "Step" & "sits directly in a Phases node"}
		}
	}
}

#LegendEntry: {
	line:      #Line
	tint?:     #TintSlot
	form?:     #PipeForm
	by_actor?: bool
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}, if form != _|_ {#form: form}, if by_actor != _|_ {#by_actor: by_actor}}
	text: #Text
}

// A routed line between two nodes, named by id. It takes no layout space.
#Link: {
	from:  #Id
	to:    #Id
	line:   #Line
	tint?:  #TintSlot
	actor?: #Id
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}}
	label?: #Text
	sub?:   #Text
	arrow?: #Arrow
	// Under iso, the axis the tag reads along; absent follows the leg it sits on.
	axis?:      #Axis
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

// A node is one of fifteen tags. Switching on tag, rather than a disjunction of
// definitions, keeps vet errors pointed at the offending field. Every node may
// carry an id; _ids lists the ids in its subtree and _keys the legend keys of
// the pipes and tees in it.
#Node: {
	tag: "Row" | "Col" | "Lanes" | "Box" | "Item" | "Fact" | "Note" | "Pipe" | "Tee" | "Text" | "Callout" | "Frame" | "View" | "Phases" | "Step"
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
	if tag == "View" {#View}
	if tag == "Phases" {#Phases}
	if tag == "Step" {#StepNode}

	// The ids a Pipe names in from and to, gathered up the tree. A Tee's arms
	// cannot name one, so a Tee gathers none.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "Pipe", "View", "Phases"], tag) {
		_pipeTargets: []
	}

	// The sheet surfaces in the subtree and the titles of the Views strictly
	// below the node.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Item", "Pipe", "Tee", "Phases", "Step"], tag) {
		_surfaces: []
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View"], tag) {
		_viewTitles: []
	}
	// _nodes: what each node with an id acts for. _lineUses: every Pipe, Tee
	// spine and Tee arm with its line, form, own actor and from. _lanes: the
	// actors of each Phases node's lanes. _looseSteps: the titles of Steps not
	// directly in a Phases node.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Item", "Pipe", "Tee", "Phases", "Step"], tag) {
		_nodes: []
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Pipe", "Tee"], tag) {
		_lineUses: []
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Phases"], tag) {
		_lanes: []
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Step"], tag) {
		_looseSteps: []
	}
}

// The surfaces, nested View titles, actors, line uses, lanes and loose Steps a
// container passes up from children.
#SheetCollectors: {
	children: [...]
	_surfaces: list.Concat([for c in children {c._surfaces}])
	#ActorCollectors
}

#ActorCollectors: {
	children: [...]
	_viewTitles: list.Concat([for c in children if c.tag == "View" {[c.title]}, for c in children {c._viewTitles}])
	_nodes: list.Concat([for c in children {c._nodes}])
	_lineUses: list.Concat([for c in children {c._lineUses}])
	_lanes: list.Concat([for c in children {c._lanes}])
	_looseSteps: list.Concat([for c in children {c._looseSteps}])
}

// A numbered view of a design sheet: a heading, then its children in a
// column. Transparent for kind nesting, as Row, Col and Lanes are.
#View: {
	id?:       #Id
	tag:       "View"
	title:     #Text
	subtitle?: #Text
	note?:     #Text
	gap?:      int & >=0 & <=64
	children:  #Children
	_keys: {for c in children {c._keys}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	_laneSets: list.Concat([for c in children {c._laneSets}])
	_pipeTargets: list.Concat([for c in children {c._pipeTargets}])
	_surfaces: list.Concat([["view"], for c in children {c._surfaces}])
	#ActorCollectors
}

// A swimlane of a design sheet: a lane per actor top to bottom, phases left to
// right, and Steps placed by lane, phase and column.
#Phases: {
	id?: #Id
	tag: "Phases"
	lanes: [...#Id] & list.MinItems(1) & list.MaxItems(8)
	phases: [...#Phase] & list.MinItems(1) & list.MaxItems(8)
	children: [...#Node & {tag: "Step"}] & list.MinItems(1) & list.MaxItems(64)
	_keys: {}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	_laneSets: []
	_pipeTargets: []
	_surfaces: list.Concat([["phases"], for c in children {c._surfaces}])
	_viewTitles: []
	_nodes: list.Concat([for c in children {c._nodes}])
	_lineUses: []
	_lanes: lanes
	_looseSteps: []
	_phaseIds: list.SortStrings([for p in phases {p.id}])
	_phaseIdsUsedTwice: {
		for i, a in _phaseIds if i > 0 if _phaseIds[i-1] == a {(a): "phase id" & "used by two phases of one Phases node"}
	}
	_sortedLanes: list.SortStrings(lanes)
	_lanesUsedTwice: {
		for i, a in _sortedLanes if i > 0 if _sortedLanes[i-1] == a {(a): "actor" & "has one lane"}
	}
	_stepLanesUnknown: {
		for c in children if !list.Contains(lanes, c.lane) {(c.title): "lane \(c.lane)" & "a lane of this Phases node"}
	}
	_stepPhasesUnknown: {
		for c in children if !list.Contains([for p in phases {p.id}], c.phase) {(c.title): "phase \(c.phase)" & "a phase of this Phases node"}
	}
}

// One piece of work in a Phases node: who does it, in which phase and columns.
// #Step is the theme's face lightness step, so the node is #StepNode.
#StepNode: {
	id?:   #Id
	tag:   "Step"
	lane:  #Id
	phase: #Id
	col?:  int & >=1 & <=6
	span?: int & >=1 & <=6
	title: #Text
	lines?: [...#Text] & list.MaxItems(6)
	look?: #StepLook
	_first: [if col != _|_ {col}, 1][0]
	_count: [if span != _|_ {span}, 1][0]
	if _first+_count-1 > 6 {
		_stepSpanOutOfRange: "columns \(_first) to \(_first+_count-1)" & "inside column 6"
	}
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_surfaces: ["step"]
	_nodes: [if id != _|_ {{"id": id, actor: lane}}]
	_lanes: []
	_looseSteps: [title]
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
	_pipeTargets: list.Concat([for c in children {c._pipeTargets}])
	#SheetCollectors
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
	_pipeTargets: list.Concat([for c in children {c._pipeTargets}])
	#SheetCollectors
}

// Any container. The grammar's container kind decides how it is drawn and
// where it may sit; tint picks a slot on a tintable kind.
#Box: {
	id?:      #Id
	tag:      "Box"
	kind:     #KindName
	tint?:    #TintSlot
	actor?:   #Id
	label:    #Text
	children: #Children
	_keys: {for c in children {c._keys}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	_laneSets: list.Concat([for c in children {c._laneSets}])
	_pipeTargets: list.Concat([for c in children {c._pipeTargets}])
	_surfaces: list.Concat([[if actor != _|_ {"actor"}], for c in children {c._surfaces}])
	_viewTitles: list.Concat([for c in children if c.tag == "View" {[c.title]}, for c in children {c._viewTitles}])
	_nodes: list.Concat([[if id != _|_ if actor != _|_ {{"id": id, "actor": actor}}], for c in children {c._nodes}])
	_lineUses: list.Concat([for c in children {c._lineUses}])
	_lanes: list.Concat([for c in children {c._lanes}])
	_looseSteps: list.Concat([for c in children {c._looseSteps}])
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
	// Under iso, the solid the item stands as; absent follows its icon's row in
	// the kind's product table, or card.
	shape?: #Shape
	actor?: #Id
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_surfaces: [if actor != _|_ {"actor"}]
	_nodes: [if id != _|_ if actor != _|_ {{"id": id, "actor": actor}}]
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
	id?:    #Id
	tag:    "Pipe"
	dir:    "h" | "v"
	line:   #Line
	tint?:  #TintSlot
	actor?: #Id
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}, if form != _|_ {#form: form}}
	label:  #Text
	sub?:   #Text
	arrow?: #Arrow
	// Under iso, the axis the tag reads along; absent follows dir.
	axis?: #Axis
	// Under iso, a round tube (the default) or a band, a flat wide arrow on
	// the floor.
	form?: #PipeForm
	// The nodes the pipe joins: from on its left (h) or upper (v) end, to on
	// the other. Layout centers the pipe's slot on them.
	from?: #Id
	to?:   #Id
	if from != _|_ if to != _|_ if from == to {
		_pipeTargetsEqual: "pipe from \(from)" & "the same node as pipe to"
	}
	_keys: {(_lineKey.key): true}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_pipeTargets: [if from != _|_ {from}, if to != _|_ {to}]
	_surfaces: [if actor != _|_ {"actor"}]
	_nodes: [if id != _|_ if actor != _|_ {{"id": id, "actor": actor}}]
	// A Tee passes its own actor to an arm that names none.
	_inherited?: #Id
	_lineUses: [{
		key: _lineKey.key
		"line": line
		"form": [if form != _|_ {form}, "tube"][0]
		if tint != _|_ {"tint": tint}
		if actor != _|_ {ownActor: actor, "actor": actor}
		if actor == _|_ if _inherited != _|_ {"actor": _inherited}
		if from != _|_ {"from": from}
	}]
}

// One source fanned to two destinations. line colours the spine; each arm is
// a full pipe with its own line and label.
#Tee: {
	id?:    #Id
	tag:    "Tee"
	line:   #Line
	tint?:  #TintSlot
	actor?: #Id
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}}
	hub: #Text
	arms: [#Pipe, #Pipe]
	if actor != _|_ {
		arms: [{_inherited: actor}, {_inherited: actor}]
	}
	_teeArmActorDiffers: {
		for i, a in arms if a.actor != _|_ if actor == _|_ || a.actor != actor {"\(i)": "a Tee arm" & "acts for its Tee's actor"}
	}
	// The Tee's grid places its arms, so an arm names no target.
	_targetOnTeeArm: {
		for i, a in arms if a.from != _|_ || a.to != _|_ {"\(i)": "tee arm" & "cannot name a target"}
	}
	_keys: {(_lineKey.key): true, arms[0]._keys, arms[1]._keys}
	_ids: list.Concat([[if id != _|_ {id}], arms[0]._ids, arms[1]._ids])
	_laneSets: []
	_surfaces: list.Concat([[if actor != _|_ {"actor"}], arms[0]._surfaces, arms[1]._surfaces])
	_nodes: [if id != _|_ if actor != _|_ {{"id": id, "actor": actor}}]
	_lineUses: list.Concat([[{
		key: _lineKey.key
		"line": line
		form: "tube"
		if tint != _|_ {"tint": tint}
		if actor != _|_ {ownActor: actor, "actor": actor}
	}], arms[0]._lineUses, arms[1]._lineUses])
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
