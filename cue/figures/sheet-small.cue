// small: the smallest design sheet, one View holding two items joined by a Tee
// and a link. Its link's via lies in the sheet's wider margin, which only a
// sheet's canvas holds. Negative cases in check.sh edit it.
package plain

import (
	"github.com/frits-v/stencil/cue/grammars:plain"
)

small: plain.#Page & {
	title:   "A request and its reply"
	kicker:  "Request flow"
	lede:    "A client calls a service and a store answers both."
	grammar: "plain"
	sheet: {
		wordmark: "relay"
		problem: {
			caption: "Problem"
			points: [{text: "Calls wait on a slow store."}]
		}
		answer: {
			statement: "The service answers from a cache."
			paragraphs: [{lead: "Cached reads.", text: "A read never waits on the store."}]
		}
		attention: {
			items: [{id: "staleness", lead: "Staleness.", text: "You decide how old a cached answer may be."}]
		}
	}
	body: [{
		tag:   "View"
		title: "System"
		children: [{
			tag: "Row"
			children: [
				{tag: "Item", id: "client", kind: "service", title: "Client", subtitle: "web front end"},
				{tag: "Tee", line: "solid", hub: "fan out", arms: [{tag: "Pipe", dir: "h", line: "solid", label: "read"}, {tag: "Pipe", dir: "h", line: "solid", label: "write"}]},
				{tag: "Item", id: "store", kind: "store", title: "Store", subtitle: "keeps every answer"},
			]
		}]
	}]
	legend: [{line: "solid", text: "call"}]
	links: [{from: "client", to: "store", line: "solid", label: "call", via: [{x: 1375, y: 40}]}]
}
