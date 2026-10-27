// onepager: the design sheet of a library's hold-and-pickup service: a title block,
// its actors, a Workflow view of who does what in which phase, and a System
// view of the parts as built (an assembly, a rail, a store and planned work)
// with its parts list and general notes. examples/onepager.json is this figure
// exported.
package plain

import (
	"github.com/frits-v/stencil/cue/grammars:plain"
)

onepager: plain.#Page & {
	title:   "Library holds, from the catalogue to the pickup shelf"
	kicker:  "Library · hold and pickup"
	lede:    "A patron places a hold in the catalogue app. The service keeps a line of holds for each title, sets a returned copy aside at the patron's branch, and tells the patron it is ready to collect."
	foot:    "Balloons key the drawings to the parts list, flags to the general notes, diamonds to the attention items in the title block."
	width:   1500
	badge:   "Internal"
	grammar: "plain"
	sheet: {
		wordmark: "holds"
		problem: {
			caption: "Problem · holds kept on paper"
			lead:    "Branch staff track holds on paper slips and by phone."
			points: [
				{text: "A patron must call or visit a branch to place a hold.", marks: [{part: "console"}, {part: "trigger-api"}]},
				{text: "Staff phone each patron when a held copy comes in.", marks: [{part: "evaluator"}]},
				{text: "A lost slip takes the patron's place in line with it.", marks: [{part: "verdict-store"}, {note: "evidence-by-reference"}]},
				{text: "A hold with no branch or an old phone number strands the copy.", marks: [{attention: "model-updates"}, {note: "approved-plans-only"}]},
			]
		}
		answer: {
			caption:   "Answer · holds placed online"
			statement: "The patron places the hold in the app; the service keeps the line, sets the copy aside and sends the notice."
			paragraphs: [
				{
					lead: "One line per title."
					text: "Each hold takes a place in its title's line. A returned copy goes to the first confirmed hold, and the app shows each patron their place."
				},
				{
					lead: "Queued work."
					text: "The API returns once a hold is queued, and a busy branch never holds a patron's request open."
				},
			]
		}
		attention: {
			caption: "The patron's part, and only this"
			items: [
				{
					id:    "plan-approval"
					lead:  "Pickup branch."
					text:  "The patron confirms the branch before a copy is set aside."
					actor: "operator"
				},
				{
					id:    "disputed-verdicts"
					lead:  "Collection."
					text:  "A ready copy waits seven days on the hold shelf at that branch."
					actor: "operator"
				},
				{
					id:    "model-updates"
					lead:  "Contact details."
					text:  "Notices go to the email or phone on the card, so the patron keeps them current."
					actor: "operator"
				},
			]
		}
		marks: {
			part:      "a piece of the service"
			note:      "asked of a patron"
			attention: "the patron's part"
			step:      "a numbered request step"
		}
	}
	actors: [
		{id: "operator", name: "Patron", text: "places and collects holds", tint: 4},
		{id: "api", name: "Holds API", text: "keeps lines, queues jobs", tint: 1},
		{id: "worker", name: "Pickup worker", text: "one job per hold", tint: 6},
		{id: "model", name: "Notifier", text: "sends ready notices", tint: 5},
	]
	body: [
		{
			tag:      "View"
			id:       "workflow"
			title:    "Workflow"
			subtitle: "one hold, from placing it to pickup"
			note:     "a lane for each actor; time runs left to right"
			children: [
				{
					tag: "Phases"
					id:  "trigger-life"
					lanes: ["operator", "api", "worker", "model"]
					phases: [
						{id: "describe", title: "Place"},
						{id: "plan", title: "Wait in line"},
						{id: "evaluate", title: "Set aside and notify"},
						{id: "review", title: "Collect"},
					]
					step_fill: "cell"
					children: [
						{
							tag:   "Step"
							id:    "s-describe"
							lane:  "operator"
							phase: "describe"
							title: "Place a hold"
							lines: ["from the catalogue app", "pickup branch"]
							marks: [{part: "console"}]
						},
						{
							tag:   "Step"
							id:    "s-idle"
							lane:  "api"
							phase: "describe"
							title: "no copy is free yet"
							look:  "text"
						},
						{
							tag:   "Step"
							id:    "s-compile"
							lane:  "api"
							phase: "plan"
							title: "Join the title's line"
							lines: ["one line per title", "place 3 of 5"]
							marks: [{part: "trigger-api"}]
						},
						{
							tag:   "Step"
							id:    "s-approve"
							lane:  "operator"
							phase: "plan"
							col:   2
							title: "Confirm the branch"
							lines: ["or suspend while away"]
							marks: [{attention: "plan-approval"}, {note: "approved-plans-only"}]
						},
						{
							tag:   "Step"
							id:    "s-store-plan"
							lane:  "api"
							phase: "plan"
							col:   2
							title: "Store the hold"
							look:  "chip"
							fill:  "tint"
							marks: [{part: "verdict-store"}]
						},
						{
							tag:   "Step"
							id:    "s-arrive"
							lane:  "worker"
							phase: "evaluate"
							title: "Copy checked in"
							lines: [{text: "the check-in event names a returned copy", marks: [{note: "out-of-scope"}]}]
							marks: [{part: "clip-bucket"}]
						},
						{
							tag:   "Step"
							id:    "s-evaluate"
							lane:  "worker"
							phase: "evaluate"
							col:   2
							title: "One job per hold"
							look:  "stack"
							lines: ["set the copy aside", "call the notifier"]
							marks: [{part: "worker"}, {part: "queue"}]
						},
						{
							tag:   "Step"
							id:    "s-judge"
							lane:  "model"
							phase: "evaluate"
							col:   2
							span:  2
							title: "Send the ready notice"
							lines: ["by email or text message"]
							marks: [{part: "evaluator"}, {note: "no-store-credentials"}]
						},
						{
							tag:   "Step"
							id:    "s-write"
							lane:  "worker"
							phase: "evaluate"
							col:   3
							title: "Mark the hold ready"
							lines: ["shelf slot, collect-by date"]
							marks: [{part: "verdict-store"}, {note: "evidence-by-reference"}]
						},
						{
							tag:   "Step"
							id:    "s-review"
							lane:  "operator"
							phase: "review"
							title: "Collect the copy"
							lines: ["at the confirmed branch"]
							marks: [{attention: "disputed-verdicts"}]
						},
						{
							tag:   "Step"
							id:    "s-replay"
							lane:  "api"
							phase: "review"
							title: "Expire uncollected holds"
							look:  "planned"
							marks: [{part: "replay"}]
						},
						{
							tag:   "Step"
							id:    "s-model-update"
							lane:  "model"
							phase: "review"
							title: "A bounced notice flags the card for the desk"
							look:  "note"
							marks: [{attention: "model-updates"}]
						},
					]
				},
				{
					tag:   "Ladder"
					id:    "failure-ladder"
					title: "When a pick fails or a hold goes unclaimed"
					note:  "a failed pick never blocks the queue"
					around: ["s-evaluate", "s-judge", "s-write"]
					rung_fill: "none"
					rungs: [
						{
							title: "Retry"
							who:   "the pickup worker"
							text:  "A failed copy lookup or notifier call retries the hold job with backoff, three attempts in all."
							actor: "worker"
						},
						{
							title: "Dead letter"
							who:   "the Holds API"
							text:  "After the third failure the job moves to a dead-letter subscription and the desk lists the hold as stuck."
							actor: "api"
						},
						{
							title: "Ask the patron"
							who:   "the patron"
							text:  "An unclaimed copy sends a reminder on day five; the patron can extend the hold once or cancel it."
							actor: "operator"
							marks: [{attention: "disputed-verdicts"}]
						},
						{
							title:   "Page the owner"
							who:     "the service owner"
							text:    "A branch with more than ten stuck holds in an hour pages the service owner."
							planned: true
						},
					]
				},
			]
		},
		{
			tag:      "View"
			id:       "system"
			title:    "System"
			subtitle: "the pieces, and why each exists"
			note:     "balloons key the drawing to the parts list"
			children: [
				{
					tag: "Row"
					gap: 20
					grow: [5, 2]
					children: [
						{
							tag: "Col"
							gap: 20
							children: [
								{
									tag: "Row"
									gap: 64
									grow: [0, 1]
									children: [
										{
											tag:     "Col"
											gap:     64
											justify: "center"
											children: [
												{
													tag:     "Row"
													justify: "center"
													grow: [0]
													children: [
														{
															tag:      "Item"
															id:       "operator-person"
															kind:     "person"
															outline:  "person"
															title:    "Patron"
															subtitle: "one library card"
															actor:    "operator"
														},
													]
												},
												{
													tag:     "Row"
													justify: "center"
													grow: [0]
													children: [
														{
															tag:      "Item"
															id:       "console"
															kind:     "service"
															title:    "Catalogue app"
															subtitle: "web and mobile"
															actor:    "operator"
														},
													]
												},
											]
										},
										{
											tag:   "Box"
											id:    "cloud"
											kind:  "system"
											label: "Hold and pickup service"
											children: [
												{
													tag: "Col"
													gap: 64
													children: [
														{
															tag: "Row"
															grow: [3, 2]
															children: [
																{
																	tag:     "Row"
																	justify: "center"
																	grow: [0]
																	children: [
																		{
																			tag:      "Item"
																			id:       "gateway"
																			kind:     "network"
																			title:    "Gateway"
																			subtitle: "checks library cards"
																			actor:    "api"
																		},
																	]
																},
																{
																	tag:     "Row"
																	justify: "center"
																	grow: [0]
																	children: [
																		{
																			tag:      "Item"
																			id:       "api"
																			kind:     "service"
																			title:    "Holds API"
																			subtitle: "keeps the lines"
																			actor:    "api"
																		},
																	]
																},
															]
														},
														{
															tag:     "Row"
															gap:     32
															justify: "center"
															grow: [0, 1]
															children: [
																{
																	tag:   "Pipe"
																	id:    "queue"
																	dir:   "v"
																	form:  "rail"
																	line:  "solid"
																	actor: "api"
																	label: "Pick queue"
																	sub:   "at-least-once delivery"
																},
																{
																	tag:     "Row"
																	justify: "center"
																	grow: [0]
																	children: [
																		{
																			tag:   "Box"
																			id:    "evaluation-path"
																			kind:  "assembly"
																			label: "Pickup path · one job per hold"
																			actor: "worker"
																			children: [
																				{
																					tag: "Col"
																					gap: 64
																					children: [
																						{
																							tag:     "Row"
																							justify: "center"
																							grow: [0]
																							children: [
																								{
																									tag:      "Item"
																									id:       "worker"
																									kind:     "service"
																									outline:  "stack"
																									title:    "Pickup worker"
																									subtitle: "one job per hold"
																									actor:    "worker"
																								},
																							]
																						},
																						{
																							tag:     "Row"
																							justify: "center"
																							grow: [0]
																							children: [
																								{
																									tag:      "Item"
																									id:       "evaluator"
																									kind:     "managed"
																									title:    "Notifier"
																									subtitle: "hosted messaging service"
																									actor:    "model"
																								},
																							]
																						},
																					]
																				},
																			]
																		},
																	]
																},
															]
														},
														{
															tag: "Row"
															grow: [2, 3, 2]
															children: [
																{
																	tag:     "Col"
																	justify: "center"
																	children: [
																		{
																			tag:     "Row"
																			justify: "center"
																			grow: [0]
																			children: [
																				{
																					tag:      "Item"
																					id:       "bucket"
																					kind:     "store"
																					title:    "Copy catalogue"
																					subtitle: "circulation records"
																				},
																			]
																		},
																	]
																},
																{
																	tag:     "Row"
																	justify: "center"
																	grow: [0]
																	children: [
																		{
																			tag:      "Item"
																			id:       "verdict-store"
																			kind:     "store"
																			outline:  "store"
																			title:    "Holds store"
																			subtitle: "relational database"
																			compartments: [
																				"Holds",
																				"Title lines",
																				"Notices",
																				{text: "Copy refs", marks: [{note: "evidence-by-reference"}]},
																			]
																			actor: "worker"
																		},
																	]
																},
																{
																	tag:     "Col"
																	justify: "center"
																	children: [
																		{
																			tag:     "Row"
																			justify: "center"
																			grow: [0]
																			children: [
																				{
																					tag:      "Item"
																					id:       "replay"
																					kind:     "managed"
																					title:    "Expiry scheduler"
																					subtitle: "hosted scheduler"
																					planned:  true
																					actor:    "api"
																				},
																			]
																		},
																	]
																},
															]
														},
													]
												},
											]
										},
									]
								},
								{
									tag:   "GeneralNotes"
									id:    "notes"
									title: "General notes"
									note:  "what the service asks of its patrons"
									notes: [
										{
											id:     "approved-plans-only"
											text:   "A hold reaches the pick queue only once its patron has confirmed the branch."
											detail: "The worker never sets a copy aside for an unconfirmed hold."
										},
										{
											id:     "evidence-by-reference"
											text:   "A hold stores a reference to the copy record, never a copy of it."
											detail: "Copy details follow the catalogue."
										},
										{
											id:     "no-store-credentials"
											text:   "The notifier holds no credentials for the holds store."
											detail: "The blocked line in view 3 is a database grant, not a convention."
										},
										{
											id:     "out-of-scope"
											text:   "Out of scope: interlibrary loans, fines, and holds on e-books."
											detail: "The service fills holds with copies this library owns."
										},
									]
									foot: "Each flag on the drawings marks where its note applies."
								},
							]
						},
						{
							tag: "Col"
							gap: 20
							children: [
								{
									tag:   "PartsList"
									id:    "parts"
									title: "Parts list"
									note:  "what the service is made of, and why each part exists"
									parts: [
										{id: "console", node: "console", why: "Patrons place and track holds without a call to the branch."},
										{id: "gateway", node: "gateway", why: "One signed-in entry point; the API never sees a call without a card."},
										{
											id:   "trigger-api"
											node: "api"
											why:  "Keeps one line per title and refuses to queue a hold nobody has confirmed."
										},
										{id: "queue", node: "queue", why: "The API returns once a job is queued, so a busy branch never holds a request open."},
										{
											id:   "clip-bucket"
											node: "bucket"
											why:  "A hold is filled only from a copy on the shelf, and the check-in event names it."
										},
										{id: "worker", node: "worker", why: "Holds are independent, so a failed pick retries alone."},
										{id: "evaluator", node: "evaluator", why: "Sends the ready notice, and has no write path to the store."},
										{
											id:   "verdict-store"
											node: "verdict-store"
											why:  "Every hold keeps its place in line, its branch and the notices sent for it."
										},
										{
											id:   "replay"
											node: "replay"
											why:  "Releases a copy left on the shelf past its date, so the next patron in line gets it."
										},
									]
								},
								{
									tag:    "Frame"
									id:     "screen-review"
									label:  "Hold shelf at the desk: title, library card, slot and collect-by date"
									height: 240
								},
							]
						},
					]
				},
			]
		},
		{
			tag:      "View"
			id:       "how-tos"
			title:    "How-tos"
			subtitle: "what desk staff type"
			note:     "real output, trimmed"
			children: [
				{
					tag: "HowTo"
					id:  "operator-commands"
					items: [
						{
							title: "Place a hold"
							commands: [
								"hold place",
								{words: "then"},
								"hold confirm",
							]
							text: "Place the hold for the patron, check the branch it names, then confirm it. No copy is set aside before that."
							terminal: [
								{kind: "command", text: "hold place --card 2104-8831 --branch east \"Rivers of the North\""},
								{kind: "output", text: "hold h-5120 placed, 3 of 5 in line"},
								{kind: "output", text: "title: Rivers of the North; branch: East; notice: text message"},
								{kind: "comment", text: "the hold waits for confirmation"},
								{kind: "command", text: "hold confirm h-5120"},
								{kind: "ok", text: "h-5120 confirmed for East branch"},
							]
						},
						{
							title: "Extend a hold"
							commands: ["hold extend"]
							text: "Adds seven days to a ready hold; the patron gets the new collect-by date by text."
							terminal: [
								{kind: "command", text: "hold extend h-5120 --days 7"},
								{kind: "output", text: "h-5120 collect-by date moved to 2026-11-09"},
								{kind: "ok", text: "notice sent by text message"},
							]
						},
						{
							title: "Cancel a hold"
							commands: ["hold cancel"]
							text: "Releases the hold and its place in line. A copy already set aside goes to the next hold."
							terminal: [
								{kind: "command", text: "hold cancel h-5141 --reason \"bought a copy\""},
								{kind: "output", text: "hold h-5141 cancelled, 4 holds left in line"},
								{kind: "comment", text: "the next hold in line moves up one place"},
							]
						},
						{
							title: "List holds"
							commands: ["hold list"]
							text:   "Filters by card, title and branch. A ready hold stays listed until it is collected or expires."
							beside: true
							terminal: [
								{kind: "command", text: "hold list --card 2104-8831"},
								{kind: "output", text: "3 holds on card 2104-8831"},
								{kind: "output", indent: 1, text: "h-5120 ready at East, collect by 2026-11-02"},
								{kind: "output", indent: 1, text: "h-5133 in line, 2 of 4"},
								{kind: "output", indent: 1, text: "h-5141 waiting for a free copy"},
							]
						},
					]
				},
			]
		},
	]
	legend: [
		{line: "solid", by_actor: true, text: "work moves, in the colour of the actor that sends it"},
		{line: "dash", by_actor: true, text: "reads or writes the holds store"},
		{line: "solid", by_actor: true, form: "rail", text: "one topic that every job passes through"},
		{line: "solid", by_actor: true, planned: true, text: "planned, not built yet"},
		{line: "gray", text: "event from the library system"},
		{line: "deny", text: "blocked path"},
	]
	links: [
		{from: "s-describe", to: "s-compile", line: "solid"},
		{from: "s-compile", to: "s-approve", line: "solid", label: "in line", from_side: "top", to_side: "left"},
		{from: "s-approve", to: "s-store-plan", line: "solid"},
		{from: "s-store-plan", to: "s-evaluate", line: "dash", actor: "worker", label: "reads confirmed holds", to_side: "top"},
		{from: "s-arrive", to: "s-evaluate", line: "solid"},
		{from: "s-evaluate", to: "s-judge", line: "solid"},
		{from: "s-judge", to: "s-write", line: "solid"},
		{from: "s-write", to: "s-review", line: "solid"},
		{from: "operator-person", to: "console", line: "solid", label: "places, tracks"},
		{from: "console", to: "gateway", line: "solid", step: 1, label: "place hold"},
		{from: "gateway", to: "api", line: "solid", step: 2, label: "authorized call"},
		{
			from:  "api"
			to:    "queue"
			line:  "solid"
			step:  3
			label: "queue job"
			sub:   "confirmed holds only"
			valve: true
			marks: [{note: "approved-plans-only"}]
			from_side: "bottom"
			to_side:   "right"
		},
		{from: "queue", to: "worker", line: "solid", step: 4, label: "deliver job"},
		{from: "worker", to: "evaluator", line: "solid", step: 5, label: "send notice"},
		{from: "worker", to: "verdict-store", line: "dash", step: 6, label: "mark ready"},
		{from: "api", to: "replay", line: "solid", planned: true, label: "schedule expiry", from_side: "bottom", to_side: "top"},
		{from: "bucket", to: "queue", line: "gray", label: "copy returned"},
		{
			from:  "evaluator"
			to:    "verdict-store"
			line:  "deny"
			label: "no direct write"
			marks: [{note: "no-store-credentials"}]
		},
	]
}
