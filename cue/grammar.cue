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
	// A house style per line for routed links, winning over the page's.
	lines?: [...#LineKind] & list.MaxItems(4)
	_lineNames: [if lines != _|_ for l in lines {l.line}]
	_linesAreUnique: _lineNames & list.UniqueItems()
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
		pattern: "solid" | "dashed" | "dotted" | "dash-dot" | "none"
		// Layout reserves this width; 0 exactly when pattern is none.
		width: number & >=0 & <=4
	}
	padding: number & >=0 & <=32
	radius:  number & >=0 & <=16
	// plain: zone_label; accent: perimeter_label; bar: gcp_bar, frame only; tab: a
	// tab straddling the top border, on a design sheet only.
	label: "plain" | "accent" | "bar" | "tab"
	// Container kinds a Box of this kind may sit in, or "page" for the top level.
	parents: [...#KindName] & list.MinItems(1)
}

#LineKind: {
	line:    #Line
	corner?: #Corner
	bend?:   #Bend
}

// A bundled icon pack, in alphabetical order, or none.
#IconPack: "gcp" | "none"

// The stems of one pack, selected by comprehension: a lookup by struct key of
// a pack with no entry would be incomplete, and the check built on it would
// pass without having run. none has no stems.
#PackStems: {
	#pack: #IconPack
	stems: list.Concat([
		if #pack == "gcp" {#IconGcpStems},
	])
}

#ItemKind: {
	name: #KindName
	// The bundled icon pack the kind draws from; none takes no icon.
	icons: #IconPack
	// The icon-to-name table of icon-matches-product; empty when icons is none.
	// Each row's icon is a stem of the kind's pack.
	products: [...#IconProducts]
	_packStems: (#PackStems & {#pack: icons}).stems
	_iconsOutsidePack: {
		for row in products if !list.Contains(_packStems, row.icon) {
			(row.icon): "icon \(row.icon)" & "is not in icon pack \(icons), which item kind \(name) draws from"
		}
	}
	parents: [...#KindName] & list.MinItems(1)
	// Under iso, the solid an item of this kind stands as when neither the item
	// nor its icon's row names one; absent is card.
	shape?: #Shape
}

#IconProducts: {
	icon:  #Icon
	class: "product" | "category"
	names: [...string & !="" & strings.MaxRunes(64)] & list.MinItems(1)
	// Under iso, the solid an item with this icon stands as; absent is card.
	shape?: #Shape
}

#Remembered: {
	literal: string & !="" & strings.MaxRunes(64)
	reason:  string & !="" & strings.MaxRunes(400)
}

// The rules every grammar shares, read from its data: Box and Item kinds are
// the grammar's names, each Box and Item sits where its kind's parents allow,
// a tint on a kind that is not tintable is reported, and tint pairing holds: a
// solid pipe of one slot sits beside or inside no tinted Box of another slot
// (_pipeBesideZoneOfOtherTint, _otherTintInsideZone), while dash, gray and
// deny lines and untinted Boxes are outside it. A grammar package
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
	// Col, Lanes and View are transparent, so they pass their children's up.
	#KindedNode: {
		tag: string
		if tag == "Row" || tag == "Col" || tag == "Lanes" || tag == "View" {
			children: [...#KindedNode]
			_tops: {for c in children {c._tops}}
			_tinted: {for c in children {c._tinted}}
			_tintedWithoutActor: list.Concat([for c in children {c._tintedWithoutActor}])
			_sheetLooks: list.Concat([for c in children {c._sheetLooks}])
			#SiblingTint
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
			// On a design sheet an actor colours a Box, so its kind must take a tint;
			// a kind that paints a default tint reads as an actor unless it names one.
			actor?: string
			label:  string
			// The Box's effective tint as a list of zero or one slot: its own, else its
			// kind's default. An actor's colour says who acts, not which path, so a Box
			// that names an actor takes no part in tint pairing.
			_tint: list.Take([
				if actor == _|_ if _tintable if tint != _|_ {tint},
				if actor == _|_ for c in #grammar.containers if c.name == kind && c.tintable && c.default_tint != _|_ {c.default_tint},
			], 1)
			_below: {for c in children {c._tinted}}
			_tinted: {_below, for t in _tint {"\(kind)-\(t)": t}}
			// A tinted Box holds no solid line, tee arm or Box of another tint at any
			// depth.
			for boxTint in _tint {
				_otherTintInsideZone: {
					for k, t in _below if t != boxTint {(k): "\(k)" & "inside \(kind) tint \(boxTint)"}
				}
			}
			#SiblingTint
			if actor != _|_ if !_tintable {
				_actorOnUntintableKind: "\(kind) takes no colour" & "so it cannot name an actor"
			}
			_tintedWithoutActor: list.Concat([
				[if actor == _|_ for c in #grammar.containers if c.name == kind && c.tintable && c.default_tint != _|_ {label}],
				for c in children {c._tintedWithoutActor},
			])
			// A kind with a dash-dot border or a tab label draws its look on a
			// design sheet only.
			_sheetLooks: list.Concat([
				[for c in #grammar.containers if c.name == kind && (c.border.pattern == "dash-dot" || c.label == "tab") {label}],
				for c in children {c._sheetLooks},
			])
		}
		if tag == "Item" {
			kind: or(_itemNames)
			_tops: {(kind): true}
			_tinted: {}
			// An icon is a stem of the pack the item's kind draws from.
			icon?: string
			if icon != _|_ for i in #grammar.items if i.name == kind
			let stems = (#PackStems & {#pack: i.icons}).stems
			if !list.Contains(stems, icon) {
				_iconOutsidePack: "icon \(icon)" & "is not in icon pack \(i.icons), which item kind \(kind) draws from"
			}
		}
		// A solid pipe or tee leg without a tint is slot 1; dash, gray and deny lines
		// are outside tint pairing.
		if tag == "Pipe" {
			line:  string
			tint?: int
			_solidTint: [if line == "solid" {[if tint != _|_ {tint}, 1][0]}]
			_tinted: {for t in _solidTint {"solid-\(t)": t}}
		}
		if tag == "Tee" {
			line:  string
			tint?: int
			arms: [...]
			let spineTint = [if tint != _|_ {tint}, 1][0]
			_tinted: {
				if line == "solid" {"solid-\(spineTint)": spineTint}
				for arm in arms if arm.line == "solid"
				let armTint = [if arm.tint != _|_ {arm.tint}, 1][0] {"solid-\(armTint)": armTint}
			}
		}
		if !list.Contains(["Row", "Col", "Lanes", "View", "Box", "Item"], tag) {
			_tops: {}
		}
		if !list.Contains(["Row", "Col", "Lanes", "View", "Box", "Item", "Pipe", "Tee"], tag) {
			_tinted: {}
		}
		if !list.Contains(["Row", "Col", "Lanes", "View", "Box"], tag) {
			_tintedWithoutActor: []
			_sheetLooks: []
		}
		...
	}

	// Tint pairing, the convention every grammar shares: a solid pipe between two
	// sibling Boxes must not touch a tinted Box of another slot. Row, Col, Lanes,
	// View and Box embed this, so the check holds wherever siblings sit.
	#SiblingTint: {
		children: [...]
		_pipeBesideZoneOfOtherTint: {
			for i, c in children if c.tag == "Pipe" for pipeTint in c._solidTint {
				for j in [i - 1, i + 1] if j >= 0 && j < len(children) {
					let neighbor = children[j]
					if neighbor.tag == "Box" for boxTint in neighbor._tint if boxTint != pipeTint {
						(c.label): "solid tint \(pipeTint) pipe" & "beside \(neighbor.kind) tint \(boxTint)"
					}
				}
			}
		}
	}

	#KindedPage: #Page & {
		grammar?: #grammar.name
		sheet?:   _
		body: [...#KindedNode]
		actors?: [...]
		if actors != _|_ if len(actors) > 0 {
			_kindTintWithoutActor: {
				for n in body for l in n._tintedWithoutActor {(l): "a Box whose kind paints a tint" & "names its actor on a page with actors"}
			}
		}
		_kindParentNotAllowed: {
			for n in body for k, _ in n._tops
			let parents = list.FlattenN([for d in _kinds if d.name == k {d.parents}], 1)
			if !list.Contains(parents, "page") {(k): "\(k)" & "cannot sit in page"}
		}
		if sheet == _|_ {
			_needsSheet: {
				for n in body for l in n._sheetLooks {"sheet-look": "sheet-look is drawn on a design sheet only" & "add sheet or remove it"}
			}
		}
	}
}
