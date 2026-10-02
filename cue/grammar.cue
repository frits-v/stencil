package stencil

import (
	"list"
	"strings"
)

#KindName: =~"^[a-z][a-z0-9-]{0,31}$"
#Tone:     "neutral" | "warm" | "cool" | "soft" | "strong" | "highlight" | "emphasis" | "accent"

#Grammar: {
	name: #KindName
	containers: [...#ContainerKind] & list.MinItems(1) & list.MaxItems(32)
	items: [...#ItemKind] & list.MinItems(1) & list.MaxItems(32)
	// Literals no text field may carry, checked by remembered-constants.
	remembered: [...#Remembered] & list.MaxItems(64)
}

#ContainerKind: {
	name: #KindName
	role: "frame" | "boundary" | "group" | "tile"
	// Every role but frame names a tone; a frame paints from the theme's frame role.
	tone?:    #Tone
	tintable: bool
	// The slot a tintable kind takes when the Box sets none; absent leaves it untinted.
	default_tint?: int & >=1 & <=8
	border: {
		pattern: "solid" | "dashed" | "dotted" | "none"
		// Layout reserves this width; 0 exactly when pattern is none.
		width: number & >=0 & <=4
	}
	padding: number & >=0 & <=32
	radius:  number & >=0 & <=16
	// plain: zone_label; accent: perimeter_label; bar: gcp_bar, frame only.
	label: "plain" | "accent" | "bar"
	// Container kinds a Box of this kind may sit in, or "page" for the top level.
	parents: [...#KindName] & list.MinItems(1)
}

#ItemKind: {
	name: #KindName
	// The bundled icon pack the kind draws from; none takes no icon.
	icons: "gcp" | "none"
	// The icon-to-name table of icon-matches-product; empty when icons is none.
	products: [...#IconProducts]
	parents: [...#KindName] & list.MinItems(1)
}

#IconProducts: {
	icon:  #Icon
	class: "product" | "category"
	names: [...string & !="" & strings.MaxRunes(64)] & list.MinItems(1)
	// Under iso, the solid an item with this icon stands as; absent is card.
	shape?: "card" | "tile" | "tower" | "cylinder" | "stack"
}

#Remembered: {
	literal: string & !="" & strings.MaxRunes(64)
	reason:  string & !="" & strings.MaxRunes(400)
}

// The rules every grammar shares, read from its data: Box and Item kinds are
// the grammar's names, each Box and Item sits where its kind's parents allow,
// and a tint on a kind that is not tintable is reported. A grammar package
// instantiates it with its own data:
//
//	_nesting: stencil.#GrammarNesting & {#grammar: grammar}
//	#Page:    _nesting.#KindedPage & {...}
//
// Kinds are looked up by comprehension over the grammar's lists, never by
// struct key: a lookup of a missing key is an incomplete value and the check
// built on it would pass without having run.
#GrammarNesting: {
	#grammar: #Grammar

	_containerNames: [for c in #grammar.containers {c.name}]
	_itemNames: [for i in #grammar.items {i.name}]
	_kinds: [
		for c in #grammar.containers {name: c.name, parents: c.parents},
		for i in #grammar.items {name: i.name, parents: i.parents},
	]

	// _tops holds the kinds of the Boxes and Items nearest below a node: Row,
	// Col and Lanes are transparent, so they pass their children's up.
	#KindedNode: {
		tag: string
		if tag == "Row" || tag == "Col" || tag == "Lanes" {
			children: [...#KindedNode]
			_tops: {for c in children {c._tops}}
		}
		if tag == "Box" {
			kind:  or(_containerNames)
			tint?: int
			children: [...#KindedNode]
			_tops: {(kind): true}
			_kindParentNotAllowed: {
				for c in children for k, _ in c._tops
				let parents = list.FlattenN([for d in _kinds if d.name == k {d.parents}], 1)
				if !list.Contains(parents, kind) {(k): "\(k)" & "cannot sit in \(kind)"}
			}
			_tintable: [for c in #grammar.containers if c.name == kind && c.tintable {c}] != []
			if tint != _|_ if !_tintable {
				_tintWithoutEffect: "tint \(tint)" & "has no effect on a \(kind) box"
			}
		}
		if tag == "Item" {
			kind: or(_itemNames)
			_tops: {(kind): true}
		}
		if !list.Contains(["Row", "Col", "Lanes", "Box", "Item"], tag) {
			_tops: {}
		}
		...
	}

	#KindedPage: #Page & {
		grammar?: #grammar.name
		body: [...#KindedNode]
		_kindParentNotAllowed: {
			for n in body for k, _ in n._tops
			let parents = list.FlattenN([for d in _kinds if d.name == k {d.parents}], 1)
			if !list.Contains(parents, "page") {(k): "\(k)" & "cannot sit in page"}
		}
	}
}
