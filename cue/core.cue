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
#PipeForm:    "tube" | "band" | "rail"

// The flat outline of an Item on a design sheet: the card, a store cylinder
// with compartments, a person's head and shoulders, or a stack of three cards.
#Outline: "card" | "store" | "person" | "stack"

// A step badge's number, unique across the links and pipes of a sheet.
#LineStep: int & >=1 & <=64
#ListKind:    "plain" | "numbered" | "bulleted"
#CalloutKind: "note" | "risk" | "decision" | "open"

// Every surface valid on a design sheet only; a page without sheet that draws
// one fails _needsSheet. The Rust twin is SheetSurface::as_str.
#SheetSurface: "sheet" | "actors" | "view" | "phases" | "step" | "actor" | "legend-by-actor" | "marks" | "parts-list" | "general-notes" | "item-outline" | "compartments" | "planned" | "sheet-look" | "rail-form" | "link-step" | "pipe-step" | "valve" | "ladder" | "how-to"

// A title block cell caption: 1 to 60 characters.
#Caption: #Text & strings.MaxRunes(60)

// A design sheet's title block. The first cell is the page head; the other
// three are the problem, the answer and what the reader attends to.
#Sheet: {
	wordmark?: #Text & strings.MaxRunes(24)
	problem:   #Problem
	answer:    #Answer
	attention: #Attention
	// What each mark kind means, printed in the legend strip.
	marks?: #MarkLegend
}

// A mark names a key by id: a balloon a part of the parts list, a flag a
// general note, a diamond an attention item. Its numeral is the key's place in
// its list.
#Mark: {part: #Id} | {note: #Id} | {attention: #Id}

// The marks of one carrier, line or compartment: at most 4.
#Marks: [...#Mark] & list.MaxItems(4)

// The kind and id of a mark, read from whichever arm it is.
#MarkKey: {
	#mark: #Mark
	kind: [if #mark.part != _|_ {"part"}, if #mark.note != _|_ {"note"}, "attention"][0]
	id: [if #mark.part != _|_ {#mark.part}, if #mark.note != _|_ {#mark.note}, if #mark.attention != _|_ {#mark.attention}, ""][0]
}

// A line of text and the marks keyed to it: a string, or text with marks.
#MarkedText: #Text | {text: #Text, marks?: #Marks}

#MarkLegend: {
	part?:      #Text
	note?:      #Text
	attention?: #Text
	// What a step badge means, printed beside the sample badge.
	step?: #Text
}

// The keys of a carrier's own marks, its corner mark and its marked lines, as
// "<kind>.<id>"; one key twice is _marksRepeated. A carrier sets _ownMarks.
#CarrierMarks: {
	_ownMarks: [...#Mark]
	_ownKeys: [for m in _ownMarks {
		let k = #MarkKey & {#mark: m}
		"\(k.kind).\(k.id)"
	}]
	_marksRepeated: {
		for i, a in _ownKeys for j, b in _ownKeys if i < j && a == b {(b): "mark" & "once per carrier"}
	}
}

// The parts list: the keys of the balloons, numbered in list order. A part
// with node is that drawn Item, Box or Pipe and takes its name, what, actor
// and planned from it.
#PartEntry: {
	id:       #Id
	node?:    #Id
	name?:    #Text
	what?:    #Text
	why:      #Text
	actor?:   #Id
	planned?: bool
	if node != _|_ {
		if name != _|_ || what != _|_ || actor != _|_ {
			_partFieldFromNode: "part \(id)" & "takes its fields from node \(node)"
		}
		if planned != _|_ if planned {
			_partFieldFromNode: "part \(id)" & "takes its fields from node \(node)"
		}
	}
	if node == _|_ if name == _|_ {
		_partNameMissing: "part \(id)" & "names a node or writes its name"
	}
}

#GeneralNote: {
	id:      #Id
	text:    #Text
	detail?: #Text
}

#Problem: {
	caption?: #Caption
	lead?:    #Text
	points: [...#ProblemPoint] & list.MinItems(1) & list.MaxItems(8)
}

#ProblemPoint: {
	text:   #Text
	marks?: #Marks
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
#BandLook: "wash" | "none"
#StepFill: "tint" | "cell" | "none"

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
// effect; a band or rail form appends "-band" or "-rail", since a band and a
// tube of one line are two looks, and a planned line appends "-planned".
// Pipe (Tee arms included), Tee, Link and LegendEntry each carry one as
// _lineKey.
#LineKey: {
	#line:      #Line
	#tint?:     #TintSlot
	#form?:     #PipeForm
	#by_actor?: bool
	#planned?:  bool
	_lineTint: [
		if #line == "gray" || #line == "deny" {#line},
		if #by_actor != _|_ if #by_actor {"\(#line)-by-actor"},
		if #tint != _|_ {"\(#line)-\(#tint)"},
		"\(#line)-1",
	][0]
	_formSuffix: [
		if #form != _|_ if #form == "band" {"-band"},
		if #form != _|_ if #form == "rail" {"-rail"},
		"",
	][0]
	_plannedSuffix: [if #planned != _|_ if #planned {"-planned"}, ""][0]
	key: "\(_lineTint)\(_formSuffix)\(_plannedSuffix)"
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
			let form = [if u.form == "band" {"-band"}, if u.form == "rail" {"-rail"}, ""][0]
			let planned = [if u.planned {"-planned"}, ""][0]
			if actor == "" {(u.key): true}
			if actor != "" {"\(u.line)-by-actor\(form)\(planned)": true}
		}
	}
	_linkKeys: {
		for i, l in _links {
			if !_hasActors || _effectiveActor["\(i)"] == "" {(l._lineKey.key): true}
			if _hasActors && _effectiveActor["\(i)"] != "" {"\(l.line)-by-actor\(l._plannedSuffix)": true}
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
		[for l in _links if l.marks != _|_ if len(l.marks) > 0 {"marks"}],
		[for e in legend if e.by_actor != _|_ if e.by_actor {"legend-by-actor"}],
		[for l in _links if l.step != _|_ {"link-step"}],
		[for l in _links if l._valve {"valve"}],
		[for l in _links if l._planned {"planned"}],
		[for e in legend if e.form != _|_ if e.form == "rail" {"rail-form"}],
		[for e in legend if e.planned != _|_ if e.planned {"planned"}],
	])
	// The ids of planned work (an Item or Box with planned, a Step with look
	// planned) and the step badge of every pipe and link.
	_plannedIds: list.Concat([for n in body {n._plannedIds}])
	_lineSteps: list.Concat([
		[for u in _lineUses if u.step != _|_ {u.step}],
		[for l in _links if l.step != _|_ {l.step}],
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
			for i, a in _sortedPartIds if i > 0 if _sortedPartIds[i-1] == a {"part.\(a)": "key" & "used by two entries of one list"}
			for i, a in _sortedNoteIds if i > 0 if _sortedNoteIds[i-1] == a {"note.\(a)": "key" & "used by two entries of one list"}
		}
		_sortedPartIds: list.SortStrings(_partIds)
		_sortedNoteIds: list.SortStrings(_noteIds)
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
			list.Concat([for l in _keyedLists {l.actors}]),
			list.Concat([for l in _ladders {l.actors}]),
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
		_ladders: list.Concat([for n in body {n._ladders}])
		_stepSets: list.Concat([for n in body {n._stepSets}])
		_stepIds: list.Concat(_stepSets)
		if len(_ladders) > 8 {
			_laddersTooMany: "\(len(_ladders)) ladders, above 8" & "details are lettered A to H"
		}
		_ladderAroundUnknown: {
			for l in _ladders for a in l.around if !list.Contains(_stepIds, a) {(a): "\"\(a)\"" & "names a Step"}
		}
		_ladderAroundTwoPhases: {
			for l in _ladders
			let framing = [for i, set in _stepSets if len([for a in l.around if list.Contains(set, a) {a}]) > 0 {i}]
			if len(framing) > 1 {(l.title): "a detail frame" & "lies in one Phases node"}
		}
		_stepOutsidePhases: {
			for t in _looseSteps {(t): "Step" & "sits directly in a Phases node"}
		}

		// Marks and the keys they name. A part with node puts its balloon on
		// the Item, Box or Pipe with that id.
		_keyedLists: list.Concat([for n in body {n._keyedLists}])
		_keyedListTwice: {
			for t in ["PartsList", "GeneralNotes"] if len([for l in _keyedLists if l.tag == t {l}]) > 1 {(t): "the sheet" & "has one \(t)"}
		}
		_partIds: list.Concat([for l in _keyedLists if l.tag == "PartsList" {l.ids}])
		_noteIds: list.Concat([for l in _keyedLists if l.tag == "GeneralNotes" {l.ids}])
		_attentionItemIds: [for item in sheet.attention.items {item.id}]
		_partNodes: list.Concat([for l in _keyedLists if l.tag == "PartsList" {l.nodes}])
		_kinded: list.Concat([for n in body {n._kinded}])
		_kindedIds: [for k in _kinded {k.id}]
		_pointMarks: list.Concat([for p in sheet.problem.points if p.marks != _|_ {[for m in p.marks {
			let k = #MarkKey & {#mark: m}
			{kind: k.kind, id: k.id, carrier: p.text}
		}]}])
		_linkMarks: list.Concat([for l in _links if l.marks != _|_ {[for m in l.marks {
			let k = #MarkKey & {#mark: m}
			{kind: k.kind, id: k.id, carrier: "\(l.from) to \(l.to)"}
		}]}])
		_allMarks: list.Concat([_pointMarks, list.Concat([for n in body {n._marks}]), _linkMarks])
		_partMarksUnknown: {
			for m in _allMarks if m.kind == "part" if !list.Contains(_partIds, m.id) {(m.id): "part \(m.id)" & "in the parts list"}
		}
		_noteMarksUnknown: {
			for m in _allMarks if m.kind == "note" if !list.Contains(_noteIds, m.id) {(m.id): "note \(m.id)" & "in the general notes"}
		}
		_attentionMarksUnknown: {
			for m in _allMarks if m.kind == "attention" if !list.Contains(_attentionItemIds, m.id) {(m.id): "attention \(m.id)" & "in the attention items"}
		}
		// A node that a part names carries that part's balloon; marking the same
		// part on it again repeats the key.
		_partBalloonRepeated: {
			for m in _allMarks if m.kind == "part" if m.node != _|_
			for p in _partNodes if p.node == m.node && p.part == m.id {(m.node): "part \(m.id)" & "marked once on its node"}
		}
		_pointMarksRepeated: {
			for i, p in sheet.problem.points if p.marks != _|_
			let keys = [for m in p.marks {let k = #MarkKey & {#mark: m}, "\(k.kind).\(k.id)"}]
			for a, x in keys for b, y in keys if a < b && x == y {"\(i).\(y)": "mark" & "once per carrier"}
		}
		_linkMarksRepeated: {
			for i, l in _links if l.marks != _|_
			let keys = [for m in l.marks {let k = #MarkKey & {#mark: m}, "\(k.kind).\(k.id)"}]
			for a, x in keys for b, y in keys if a < b && x == y {"\(i).\(y)": "mark" & "once per carrier"}
		}
		_partNodesUnknown: {
			for p in _partNodes if !list.Contains(_kindedIds, p.node) {(p.node): "part node \(p.node)" & "names an Item, Box or Pipe"}
		}
		if sheet.marks != _|_ {
			_markLegendUnused: {
				if sheet.marks.part != _|_ if len([for l in _keyedLists if l.tag == "PartsList" {l}]) == 0 {part: "no part key" & "on this sheet"}
				if sheet.marks.note != _|_ if len([for l in _keyedLists if l.tag == "GeneralNotes" {l}]) == 0 {note: "no note key" & "on this sheet"}
				if sheet.marks.step != _|_ if len(_lineSteps) == 0 {step: "no step key" & "on this sheet"}
			}
		}
		// A built solid or dash line that ends at planned work: a solid one is
		// planned too, and a dash one cannot be.
		_lineIntoPlanned: {
			for i, l in _links if !l._planned if l.line == "solid" || l.line == "dash"
			for end in [l.to, l.from] if list.Contains(_plannedIds, end) {"link.\(i).\(end)": "\(l.line) line to planned \(end)" & "planned too"}
			for i, u in _lineUses if !u.planned if u.line == "solid" || u.line == "dash"
			for end in [if u.to != _|_ {u.to}, if u.from != _|_ {u.from}] if list.Contains(_plannedIds, end) {"line.\(i).\(end)": "\(u.line) line to planned \(end)" & "planned too"}
		}
		_sortedSteps: list.Sort(_lineSteps, list.Ascending)
		_linkStepUsedTwice: {
			for i, a in _sortedSteps if i > 0 if _sortedSteps[i-1] == a {"\(a)": "step \(a)" & "used by one line"}
		}
		// keys-used: a part never ballooned, a note never flagged, an attention
		// item never marked.
		_balloonedParts: list.Concat([
			[for m in _allMarks if m.kind == "part" {m.id}],
			[for n in _partNodes if list.Contains(_kindedIds, n.node) {n.part}],
		])
		_partsUnmarked: {
			for p in _partIds if !list.Contains(_balloonedParts, p) {(p): "part \(p)" & "ballooned on the drawing"}
		}
		_notesUnflagged: {
			for n in _noteIds if !list.Contains([for m in _allMarks if m.kind == "note" {m.id}], n) {(n): "general note \(n)" & "flagged on the drawing"}
		}
		_attentionUnmarked: {
			for a in _attentionItemIds if !list.Contains([for m in _allMarks if m.kind == "attention" {m.id}], a) {(a): "attention item \(a)" & "marked on the drawing"}
		}
	}
}

#LegendEntry: {
	line:      #Line
	tint?:     #TintSlot
	form?:     #PipeForm
	by_actor?: bool
	// On a design sheet, keys a planned line, drawn hollow.
	planned?: bool
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}, if form != _|_ {#form: form}, if by_actor != _|_ {#by_actor: by_actor}, if planned != _|_ {#planned: planned}}
	text: #Text
	_planned: [if planned != _|_ {planned}, false][0]
	if _planned if line != "solid" {
		_plannedNotSolid: "a planned \(line) line" & "is solid"
	}
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
	// On a design sheet, marks in the tag after the label and sub, or under the
	// valve when the link has neither label nor step.
	marks?: #Marks
	// On a design sheet: a numbered badge at the left of the tag, a valve before
	// the arrowhead, and a planned line drawn hollow.
	step?:  #LineStep
	valve?: bool
	_valve: [if valve != _|_ {valve}, false][0]
	planned?: bool
	_planned: [if planned != _|_ {planned}, false][0]
	if _planned if line != "solid" {
		_plannedNotSolid: "a planned \(line) line" & "is solid"
	}
	_plannedSuffix: [if _planned {"-planned"}, ""][0]
	_lineKey: {#planned: _planned}
	if marks != _|_ if len(marks) > 0 if label == _|_ if step == _|_ if !_valve {
		_linkMarksWithoutTag: "a link's marks sit in its tag or under its valve" & "give it a label"
	}
	if _valve if arrow != _|_ if arrow == "none" {
		_valveWithoutArrow: "a valve" & "sits before an arrowhead"
	}
}

#PagePoint: {
	x: number & >=0
	y: number & >=0
}

// A node is one of nineteen tags. Switching on tag, rather than a disjunction of
// definitions, keeps vet errors pointed at the offending field. Every node may
// carry an id; _ids lists the ids in its subtree and _keys the legend keys of
// the pipes and tees in it.
#Node: {
	tag: "Row" | "Col" | "Lanes" | "Box" | "Item" | "Fact" | "Note" | "Pipe" | "Tee" | "Text" | "Callout" | "Frame" | "View" | "Phases" | "Step" | "PartsList" | "GeneralNotes" | "Ladder" | "HowTo"
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
	if tag == "PartsList" {#PartsList}
	if tag == "GeneralNotes" {#GeneralNotes}
	if tag == "Ladder" {#Ladder}
	if tag == "HowTo" {#HowTo}

	// The ids a Pipe names in from and to, gathered up the tree. A Tee's arms
	// cannot name one, so a Tee gathers none.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "Pipe", "View", "Phases"], tag) {
		_pipeTargets: []
	}

	// The sheet surfaces in the subtree and the titles of the Views strictly
	// below the node.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Item", "Pipe", "Tee", "Phases", "Step", "PartsList", "GeneralNotes", "Ladder", "HowTo"], tag) {
		_surfaces: []
	}

	// _marks: every mark on a carrier in the subtree. _kinded: the ids of the
	// Items, Boxes and Pipes a part may name. _keyedLists: each PartsList and
	// GeneralNotes node with its keys.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Item", "Pipe", "Tee", "Phases", "Step", "Ladder"], tag) {
		_marks: []
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Item", "Pipe", "Tee"], tag) {
		_kinded: []
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "PartsList", "GeneralNotes"], tag) {
		_keyedLists: []
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

	// _plannedIds: the ids of planned Items, Boxes and Steps.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Item", "Phases", "Step"], tag) {
		_plannedIds: []
	}

	// _stepSets: the Step ids of each Phases node. _ladders: each Ladder with
	// its around ids and the actors its rungs name.
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Phases"], tag) {
		_stepSets: []
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "View", "Ladder"], tag) {
		_ladders: []
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
	_marks: list.Concat([for c in children {c._marks}])
	_kinded: list.Concat([for c in children {c._kinded}])
	_keyedLists: list.Concat([for c in children {c._keyedLists}])
	_plannedIds: list.Concat([for c in children {c._plannedIds}])
	_stepSets: list.Concat([for c in children {c._stepSets}])
	_ladders: list.Concat([for c in children {c._ladders}])
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
	_railOutsideRow: {
		for i, c in children if c.tag == "Pipe" if c.form != _|_ if c.form == "rail" {"\(i)": "a rail" & "a column of a Row"}
	}
}

// A swimlane of a design sheet: a lane per actor top to bottom, phases left to
// right, and Steps placed by lane, phase and column.
#Phases: {
	id?: #Id
	tag: "Phases"
	lanes: [...#Id] & list.MinItems(1) & list.MaxItems(8)
	phases: [...#Phase] & list.MinItems(1) & list.MaxItems(8)
	band?:      #BandLook
	step_fill?: #StepFill
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
	_marks: list.Concat([for c in children {c._marks}])
	_plannedIds: list.Concat([for c in children {c._plannedIds}])
	_stepSets: [[for c in children if c.id != _|_ {c.id}]]
	_ladders: []
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
	lines?: [...#MarkedText] & list.MaxItems(6)
	look?: #StepLook
	fill?: #StepFill
	if look != _|_ if fill != _|_ if look == "text" {
		_stepTextFill: "a text Step" & "draws no box, so it takes no fill"
	}
	_first: [if col != _|_ {col}, 1][0]
	_count: [if span != _|_ {span}, 1][0]
	if _first+_count-1 > 6 {
		_stepSpanOutOfRange: "columns \(_first) to \(_first+_count-1)" & "inside column 6"
	}
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	marks?:       #Marks
	corner_mark?: #Mark
	#CarrierMarks
	_ownMarks: list.Concat([[if marks != _|_ for m in marks {m}], [if corner_mark != _|_ {corner_mark}], list.Concat([if lines != _|_ for l in lines if l.marks != _|_ {l.marks}])])
	_hasMarks: len(_ownMarks) > 0
	_surfaces: ["step", if _hasMarks {"marks"}]
	_nodes: [if id != _|_ {{"id": id, actor: lane}}]
	_lanes: []
	_looseSteps: [title]
	_plannedIds: [if id != _|_ if look != _|_ if look == "planned" {id}]
	_marks: [for m in _ownMarks {
		let k = #MarkKey & {#mark: m}
		{kind: k.kind, id: k.id, carrier: title}
	}]
}

// The parts list of a design sheet: the keys of its balloons.
#PartsList: {
	id?:   #Id
	tag:   "PartsList"
	title: #Text
	note?: #Text
	parts: [...#PartEntry] & list.MinItems(1) & list.MaxItems(48)
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_surfaces: ["parts-list"]
	_nodeNames: list.SortStrings([for p in parts if p.node != _|_ {p.node}])
	_partNodesTwice: {
		for i, a in _nodeNames if i > 0 if _nodeNames[i-1] == a {(a): "node \(a)" & "is one part"}
	}
	_keyedLists: [{
		tag: "PartsList"
		ids: [for p in parts {p.id}]
		nodes: [for p in parts if p.node != _|_ {{node: p.node, part: p.id}}]
		actors: [for p in parts if p.actor != _|_ {p.actor}]
	}]
}

// The general notes of a design sheet: the keys of its flags.
#GeneralNotes: {
	id?:   #Id
	tag:   "GeneralNotes"
	title: #Text
	note?: #Text
	notes: [...#GeneralNote] & list.MinItems(1) & list.MaxItems(16)
	foot?: #Text
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_surfaces: ["general-notes"]
	_keyedLists: [{tag: "GeneralNotes", ids: [for n in notes {n.id}], actors: []}]
}

// A detail ladder of a design sheet: rungs of escalation, rung 1 at the bottom,
// and a detail frame around the around Steps, which lie in one Phases node.
#Ladder: {
	id?:   #Id
	tag:   "Ladder"
	title: #Text
	note?: #Text
	around: [...#Id] & list.MinItems(1) & list.MaxItems(16)
	// What every rung's box is filled with; absent is tint.
	rung_fill?: #StepFill
	rungs: [...#Rung] & list.MinItems(1) & list.MaxItems(6)
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_rungMarked: [for r in rungs if r.marks != _|_ if len(r.marks) > 0 {r}]
	_surfaces: ["ladder", if len(_rungMarked) > 0 {"marks"}]
	_title:  title
	_around: around
	_ladders: [{
		title:  _title
		around: _around
		actors: [for r in rungs if r.actor != _|_ {r.actor}]
	}]
	_marks: list.Concat([for r in rungs if r.marks != _|_ {[for m in r.marks {
		let k = #MarkKey & {#mark: m}
		{kind: k.kind, id: k.id, carrier: r.title}
	}]}])
	_rungMarksRepeated: {
		for i, r in rungs if r.marks != _|_
		let keys = [for m in r.marks {let k = #MarkKey & {#mark: m}, "\(k.kind).\(k.id)"}]
		for a, x in keys for b, y in keys if a < b && x == y {"\(i).\(y)": "mark" & "once per carrier"}
	}
}

#Rung: {
	title:    #Text
	who?:     #Text
	text:     #Text
	actor?:   #Id
	planned?: bool
	marks?:   #Marks
}

// What an operator types, as items numbered <view>.<n>. Each item says or shows
// something; beside sets its text beside its terminal and needs both.
#HowTo: {
	id?: #Id
	tag: "HowTo"
	items: [...#HowToItem] & list.MinItems(1) & list.MaxItems(6)
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_surfaces: ["how-to"]
}

#HowToItem: {
	title: #Text
	commands?: [...#CommandRun] & list.MaxItems(5)
	text?:   #Text
	beside?: bool
	terminal?: [...#TerminalLine] & list.MaxItems(12)
	_shows: [if terminal != _|_ if len(terminal) > 0 {true}]
	if text == _|_ if len(_shows) == 0 {
		_howToItemEmpty: "how-to \(title)" & "says or shows something"
	}
	if beside != _|_ if beside {
		if text == _|_ || len(_shows) == 0 {
			_besideAlone: "beside on how-to \(title)" & "the item needs both text and terminal"
		}
	}
}

// A command chip, or words between chips.
#CommandRun: #Text | {words: #Text}

#TerminalLine: {
	kind:    "command" | "output" | "comment" | "ok"
	indent?: int & >=0 & <=8
	text:    #Text
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

// A rail is a column of a Row: a Col, Lanes, View, Box or Tee that holds a
// Pipe with form rail reports it as _railOutsideRow.
#Col: {
	tag:      "Col"
	children: #Children
	#Container
	_railOutsideRow: {
		for i, c in children if c.tag == "Pipe" if c.form != _|_ if c.form == "rail" {"\(i)": "a rail" & "a column of a Row"}
	}
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
	_railOutsideRow: {
		for i, c in children if c.tag == "Pipe" if c.form != _|_ if c.form == "rail" {"\(i)": "a rail" & "a column of a Row"}
	}
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
	// On a design sheet, work not built yet: a dashed outline with no fill.
	planned?: bool
	_planned: [if planned != _|_ {planned}, false][0]
	marks?:       #Marks
	corner_mark?: #Mark
	#CarrierMarks
	_ownMarks: list.Concat([[if marks != _|_ for m in marks {m}], [if corner_mark != _|_ {corner_mark}]])
	_hasMarks: len(_ownMarks) > 0
	_plannedIds: list.Concat([[if id != _|_ if _planned {id}], for c in children {c._plannedIds}])
	_railOutsideRow: {
		for i, c in children if c.tag == "Pipe" if c.form != _|_ if c.form == "rail" {"\(i)": "a rail" & "a column of a Row"}
	}
	_keys: {for c in children {c._keys}}
	_ids: list.Concat([[if id != _|_ {id}], for c in children {c._ids}])
	_laneSets: list.Concat([for c in children {c._laneSets}])
	_pipeTargets: list.Concat([for c in children {c._pipeTargets}])
	_surfaces: list.Concat([[if actor != _|_ {"actor"}, if _hasMarks {"marks"}, if _planned {"planned"}], for c in children {c._surfaces}])
	_marks: list.Concat([[for m in _ownMarks {
		let k = #MarkKey & {#mark: m}
		{kind: k.kind, id: k.id, carrier: label, if id != _|_ {node: id}}
	}], for c in children {c._marks}])
	_kinded: list.Concat([[if id != _|_ {{"id": id, tag: "Box"}}], for c in children {c._kinded}])
	_keyedLists: list.Concat([for c in children {c._keyedLists}])
	_viewTitles: list.Concat([for c in children if c.tag == "View" {[c.title]}, for c in children {c._viewTitles}])
	_nodes: list.Concat([[if id != _|_ if actor != _|_ {{"id": id, "actor": actor}}], for c in children {c._nodes}])
	_lineUses: list.Concat([for c in children {c._lineUses}])
	_lanes: list.Concat([for c in children {c._lanes}])
	_looseSteps: list.Concat([for c in children {c._looseSteps}])
	_stepSets: list.Concat([for c in children {c._stepSets}])
	_ladders: list.Concat([for c in children {c._ladders}])
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
	// On a design sheet: the flat outline, the compartments of a store, and
	// planned work, drawn dashed with no fill.
	outline?: #Outline
	compartments?: [...#MarkedText] & list.MaxItems(6)
	planned?: bool
	_outline: [if outline != _|_ {outline}, "card"][0]
	_compartments: [if compartments != _|_ for c in compartments {c}]
	_planned: [if planned != _|_ {planned}, false][0]
	if len(_compartments) > 0 if _outline != "store" {
		_compartmentsWithoutStore: "compartments" & "are drawn in a store"
	}
	if _outline == "person" if icon != _|_ {
		_outlinePersonWithIcon: "a person outline" & "draws no icon"
	}
	marks?:       #Marks
	corner_mark?: #Mark
	#CarrierMarks
	_ownMarks: list.Concat([[if marks != _|_ for m in marks {m}], [if corner_mark != _|_ {corner_mark}], list.Concat([for c in _compartments if c.marks != _|_ {c.marks}])])
	_hasMarks: len(_ownMarks) > 0
	_keys: {}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_surfaces: [if actor != _|_ {"actor"}, if _hasMarks {"marks"}, if _outline != "card" {"item-outline"}, if len(_compartments) > 0 {"compartments"}, if _planned {"planned"}]
	_plannedIds: [if id != _|_ if _planned {id}]
	_nodes: [if id != _|_ if actor != _|_ {{"id": id, "actor": actor}}]
	_marks: [for m in _ownMarks {
		let k = #MarkKey & {#mark: m}
		{kind: k.kind, id: k.id, carrier: title, if id != _|_ {node: id}}
	}]
	_kinded: [if id != _|_ {{"id": id, tag: "Item"}}]
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
	_lineKey: #LineKey & {#line: line, if tint != _|_ {#tint: tint}, if form != _|_ {#form: form}, #planned: _planned}
	label:  #Text
	sub?:   #Text
	arrow?: #Arrow
	// Under iso, the axis the tag reads along; absent follows dir.
	axis?: #Axis
	// Under iso, a round tube (the default) or a band, a flat wide arrow on
	// the floor. On a design sheet, a rail: a heavy vertical bus links attach
	// to, a column of a Row with no targets.
	form?: #PipeForm
	_rail: [if form != _|_ if form == "rail" {true}, false][0]
	if _rail if dir != "v" {
		_railNotVertical: "a rail" & "stands vertical"
	}
	if _rail if from != _|_ || to != _|_ {
		_railWithTargets: "a rail" & "is attached by links, not targets"
	}
	// On a design sheet, a numbered badge at the left of the tag, and a planned
	// line drawn hollow.
	step?: #LineStep
	planned?: bool
	_planned: [if planned != _|_ {planned}, false][0]
	if _planned if line != "solid" {
		_plannedNotSolid: "a planned \(line) line" & "is solid"
	}
	// The nodes the pipe joins: from on its left (h) or upper (v) end, to on
	// the other. Layout centers the pipe's slot on them.
	from?: #Id
	to?:   #Id
	if from != _|_ if to != _|_ if from == to {
		_pipeTargetsEqual: "pipe from \(from)" & "the same node as pipe to"
	}
	// On a design sheet, marks in the tag after the label and sub.
	marks?: #Marks
	#CarrierMarks
	_ownMarks: [if marks != _|_ for m in marks {m}]
	_hasMarks: len(_ownMarks) > 0
	_keys: {(_lineKey.key): true}
	_ids: [if id != _|_ {id}]
	_laneSets: []
	_pipeTargets: [if from != _|_ {from}, if to != _|_ {to}]
	_surfaces: [if actor != _|_ {"actor"}, if _hasMarks {"marks"}, if _rail {"rail-form"}, if step != _|_ {"pipe-step"}, if _planned {"planned"}]
	_marks: [for m in _ownMarks {
		let k = #MarkKey & {#mark: m}
		{kind: k.kind, id: k.id, carrier: label, if id != _|_ {node: id}}
	}]
	_kinded: [if id != _|_ {{"id": id, tag: "Pipe"}}]
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
		if to != _|_ {"to": to}
		if step != _|_ {"step": step}
		"planned": _planned
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
	_marks: list.Concat([arms[0]._marks, arms[1]._marks])
	_kinded: list.Concat([arms[0]._kinded, arms[1]._kinded])
	_nodes: [if id != _|_ if actor != _|_ {{"id": id, "actor": actor}}]
	_lineUses: list.Concat([[{
		key: _lineKey.key
		"line": line
		form: "tube"
		if tint != _|_ {"tint": tint}
		if actor != _|_ {ownActor: actor, "actor": actor}
		planned: false
	}], arms[0]._lineUses, arms[1]._lineUses])
	_railOutsideRow: {
		for i, c in arms if c.tag == "Pipe" if c.form != _|_ if c.form == "rail" {"\(i)": "a rail" & "a column of a Row"}
	}
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
