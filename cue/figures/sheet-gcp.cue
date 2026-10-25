// cloud: the smallest design sheet under the gcp grammar, one View holding a
// person and a Google Cloud frame with one region and one product with an icon.
// Negative cases in check.sh edit it for the sheet rules that need a gcp kind: a
// container that cannot sit in the page, a kind with a default tint, a product
// with an icon.
package gcp

import (
	"github.com/frits-v/stencil/cue/grammars:gcp"
)

cloud: gcp.#Page & {
	title:   "A request into one region"
	kicker:  "Request flow"
	lede:    "A client calls a service that runs in one region."
	grammar: "gcp"
	sheet: {
		wordmark: "one"
		problem: {
			caption: "Problem"
			points: [{text: "Calls cross regions.", marks: [{attention: "latency"}]}]
		}
		answer: {
			statement: "One region answers every call."
			paragraphs: [{lead: "One region.", text: "The service runs where its callers are."}]
		}
		attention: {
			items: [{id: "latency", lead: "Latency.", text: "You choose the region.", actor: "crew"}]
		}
	}
	actors: [{id: "crew", name: "Crew", text: "runs the service", tint: 2}]
	body: [{
		tag:   "View"
		title: "System"
		children: [{
			tag: "Row"
			gap: 64
			children: [
				{tag: "Item", id: "client", kind: "person", title: "Client", subtitle: "calls the service"},
				{tag: "Box", kind: "gcp", label: "Google Cloud", children: [{
					tag:   "Box"
					kind:  "region"
					label: "Region"
					actor: "crew"
					children: [
						{tag: "Item", id: "service", kind: "product", icon: "cloud-run", title: "Service", subtitle: "Cloud Run", actor: "crew"},
					]
				}]},
			]
		}]
	}]
	legend: [{line: "solid", by_actor: true, text: "call"}]
	links: [{from: "client", to: "service", line: "solid", actor: "crew", label: "call"}]
}
