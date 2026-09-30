// Structural schema for architecture figures. A #Page describes what a figure
// shows: zones, product cards, facts and the pipes between them. It carries no
// coordinates; renderers and derived views (eraser.cue) decide geometry.
package stencil

import "list"

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

// Every text field in the vocabulary.
#Text: string & !="" & #NoRememberedConstant

#Canvas:   "customer" | "internal"
#PipeKind: "gray" | "blue" | "pink" | "dash" | "deny"
#ZoneKind: "gcp" | "vpc" | "region-a" | "region-b" | "subnet" | "onprem-a" | "onprem-b" |
		"project" | "optional" | "k8s" | "perimeter"
#NoteKind: "kicker" | "h1" | "lede" | "legend" | "foot"
#Justify:  "start" | "center" | "end" | "space-between"

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
	body: [...#Node]
	legend: [...#LegendEntry]

	// Each check below is a struct keyed by the offending item, so a failure
	// names the item in its path: _legendKindsUnusedInBody.deny: conflicting values ...
	_used: {for n in body {n._kinds}}
	_usedKinds: [for k, _ in _used {k}]
	_legendKinds: [for e in legend {e.kind}]
	_nameless: {for n in body {n._nameless}}
	_legendKindsAreUnique: _legendKinds & list.UniqueItems()
	_pipeKindsMissingFromLegend: {
		for k in _usedKinds if !list.Contains(_legendKinds, k) {(k): "pipe kind in body" & "missing from legend"}
	}
	_legendKindsUnusedInBody: {
		for k in _legendKinds if !list.Contains(_usedKinds, k) {(k): "legend entry" & "no pipe of this kind in body"}
	}
	_pcardsWithoutPnFactOrAsk: {
		for k, _ in _nameless {(k): "product card" & "needs pn, fact or ask"}
	}
}

#LegendEntry: {
	kind: #PipeKind
	text: #Text
}

// A node is one of eight tags. Switching on tag, rather than a disjunction of
// definitions, keeps vet errors pointed at the offending field.
#Node: {
	tag: "Row" | "Col" | "Zone" | "Pcard" | "Fact" | "Note" | "Pipe" | "Tee"
	if tag == "Row" {#Row}
	if tag == "Col" {#Col}
	if tag == "Zone" {#Zone}
	if tag == "Pcard" {#Pcard}
	if tag == "Fact" {#Fact}
	if tag == "Note" {#Note}
	if tag == "Pipe" {#Pipe}
	if tag == "Tee" {#Tee}
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
	gap?: int & >=0 & <=64
	grow?: [...int & >=0 & <=100]
	justify?: #Justify
	children: [...#Node]
	if grow != _|_ {
		_growLengthMatchesChildren: len(grow) & len(children)
	}
	_kinds: {for c in children {c._kinds}}
	_zoneKinds: {for c in children {c._zoneKinds}}
	_nameless: {for c in children {c._nameless}}
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
	tag:   "Zone"
	kind:  #ZoneKind
	label: #Text
	children: [...#Node]
	_kinds: {for c in children {c._kinds}}
	_below: {for c in children {c._zoneKinds}}
	_zoneKinds: {(kind): true, _below}
	_nameless: {for c in children {c._nameless}}
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
	tag:   "Pcard"
	icon?: #Icon
	fn:    #Text
	pn?:   #Text
	fact?: #Text
	ask?:  #Text
	_kinds: {}
	_zoneKinds: {}
	_nameless: {if pn == _|_ && fact == _|_ && ask == _|_ {(fn): true}}
}

#Fact: {
	tag:  "Fact"
	text: #Text
	_kinds: {}
	_zoneKinds: {}
	_nameless: {}
}

#Note: {
	tag:  "Note"
	kind: #NoteKind
	text: #Text
	_kinds: {}
	_zoneKinds: {}
	_nameless: {}
}

#Pipe: {
	tag:   "Pipe"
	dir:   "h" | "v"
	kind:  #PipeKind
	label: #Text
	sub?:  #Text
	_kinds: {(kind): true}
	_zoneKinds: {}
	_nameless: {}
}

// One source fanned to two destinations. kind colours the spine; each arm is
// a full pipe with its own kind and label.
#Tee: {
	tag:  "Tee"
	kind: #PipeKind
	hub:  #Text
	arms: [#Pipe, #Pipe]
	_kinds: {(kind): true, arms[0]._kinds, arms[1]._kinds}
	_zoneKinds: {}
	_nameless: {}
}
