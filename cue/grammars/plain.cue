// A domain-neutral grammar for system diagrams: no icons, no remembered
// constants and no rules beyond the core and the kind nesting.
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
	]
	items: [for itemName in ["service", "store", "external", "person", "device"] {
		name:  itemName
		icons: "none"
		products: []
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		if itemName == "person" {shape: "figure"}
		if itemName == "device" {shape: "laptop"}
	}]
	remembered: []
}

_nesting: stencil.#GrammarNesting & {#grammar: grammar}

#Page: _nesting.#KindedPage
