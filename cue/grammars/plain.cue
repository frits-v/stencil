// The default grammar: the container and item kinds of a system figure that
// names no vendor, and the one rule that follows from a kind's definition.
// A page that names no grammar is a plain page.
// `grammar` is exported to crates/stencil-model/grammars/plain.json.
package plain

import (
	"list"

	"github.com/frits-v/stencil/cue:stencil"
)

grammar: stencil.#Grammar & {
	name: "plain"
	containers: [
		{
			name:     "system"
			role:     "frame"
			tintable: false
			border: {pattern: "solid", width: 3}
			padding: 0
			radius:  10
			label:   "bar"
			parents: ["page"]
		},
		{
			name:     "boundary"
			role:     "boundary"
			tone:     "strong"
			tintable: false
			border: {pattern: "dashed", width: 2}
			padding: 10
			radius:  8
			label:   "plain"
			parents: ["page", "system", "group", "tile"]
		},
		{
			name:     "group"
			role:     "group"
			tone:     "neutral"
			tintable: true
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "system", "boundary", "group", "tile"]
		},
		{
			name:     "tile"
			role:     "tile"
			tone:     "highlight"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "system", "boundary", "tile"]
		},
		{
			// The parts one actor runs, on a design sheet only: a dash-dot outline
			// with its name on a tab.
			name:     "assembly"
			role:     "group"
			tone:     "neutral"
			tintable: true
			border: {pattern: "dash-dot", width: 1.1}
			padding: 14
			radius:  0
			label:   "tab"
			parents: ["page", "system", "boundary", "group", "tile"]
		},
	]
	// One row per item kind, with the solid it stands as under iso: a kind with
	// no icon needs its own shape, or every item is a block.
	items: [for entry in _itemKinds {
		name:  entry.name
		icons: "none"
		products: []
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		shape: entry.shape
	}]
	remembered: []
}

// service: a running service the figure's owner builds and operates.
// managed: a service the owner uses but does not run (a vendor's database
// engine, model endpoint or scheduler). store: a database, bucket or file
// share. queue: a queue, topic or stream. network: a router, switch, firewall,
// load balancer or gateway. external: a system outside the figure's scope.
// person: a human actor. device: an end-user device.
_itemKinds: [
	{name: "service", shape: "block"},
	{name: "managed", shape: "block"},
	{name: "store", shape: "cylinder"},
	{name: "queue", shape: "stack"},
	{name: "network", shape: "tile"},
	{name: "external", shape: "block"},
	{name: "person", shape: "figure"},
	{name: "device", shape: "laptop"},
]

_nesting: stencil.#GrammarNesting & {#grammar: grammar}

// A plain figure: the core page with the plain kinds, nesting and tint pairing,
// plus one rule of its own. An external system is outside the figure's scope,
// so it sits outside every system frame; the check names the item in its path.
#Page: _nesting.#KindedPage & {
	body: [...#Node]
}

// The plain walk over the body: each node carries _externals, the titles of
// the external items below it, and a system Box reports every one it holds.
#Node: {
	tag: string
	if tag == "Row" || tag == "Col" || tag == "Lanes" || tag == "View" {
		children: [...#Node]
		_externals: {for c in children {c._externals}}
	}
	if tag == "Box" {
		kind: string
		children: [...#Node]
		_externals: {for c in children {c._externals}}
		if kind == "system" {
			_externalInsideSystem: {
				for k, _ in _externals {(k): "external item" & "sits inside the system frame; a system outside the figure's scope sits beside it"}
			}
		}
	}
	if tag == "Item" {
		kind:  string
		title: string
		_externals: {if kind == "external" {(title): true}}
	}
	if !list.Contains(["Row", "Col", "Lanes", "View", "Box", "Item"], tag) {
		_externals: {}
	}
	...
}
