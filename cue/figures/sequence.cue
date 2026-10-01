// sequence: one batch job from submission to completion, drawn as five lanes
// with ten ordered messages. examples/sequence.json is this figure exported.
package plain

import (
	"github.com/frits-v/stencil/cue/grammars:plain"
)

figure: plain.#Page & {
	title:   "A batch job from submission to completion"
	kicker:  "Request flow · job service"
	lede:    "An operator submits a job in the console. The API queues it and answers at once; a worker pulls the job, reports progress and acknowledges it once the result is stored. A cancel that arrives while the job runs is refused."
	canvas:  "internal"
	grammar: "plain"

	body: [{
		tag:   "Box"
		kind:  "system"
		label: "Job service"
		children: [{
			tag: "Lanes"
			children: [
				{tag: "Item", id: "operator", kind: "person", title: "Operator", subtitle: "submits and cancels jobs"},
				{tag: "Item", id: "console", kind: "service", title: "Console", subtitle: "web front end"},
				{tag: "Item", id: "api", kind: "service", title: "Jobs API", subtitle: "owns job state"},
				{tag: "Item", id: "queue", kind: "store", title: "Queue", subtitle: "at-least-once delivery"},
				{tag: "Item", id: "worker", kind: "service", title: "Worker", subtitle: "runs one job at a time"},
			]
		}]
	}]

	legend: [
		{line: "solid", text: "call"},
		{line: "dash", text: "reply"},
		{line: "deny", text: "rejected call"},
	]

	links: [
		{from: "operator", to: "console", line: "solid", label: "submit job", order: 1},
		{from: "console", to: "api", line: "solid", label: "POST /jobs", order: 2},
		{from: "api", to: "queue", line: "solid", label: "enqueue", sub: "job id as the key", order: 3},
		{from: "api", to: "console", line: "dash", label: "202 accepted", order: 4},
		{from: "worker", to: "queue", line: "solid", label: "pull", order: 5},
		{from: "queue", to: "worker", line: "solid", label: "deliver job", order: 6},
		{from: "worker", to: "api", line: "solid", label: "report running", order: 7},
		{from: "operator", to: "console", line: "solid", label: "cancel job", order: 8},
		{from: "console", to: "api", line: "deny", label: "DELETE /jobs/{id}", sub: "409: job is running", order: 9},
		{from: "worker", to: "queue", line: "solid", label: "ack", sub: "after the result is stored", order: 10},
	]
}
