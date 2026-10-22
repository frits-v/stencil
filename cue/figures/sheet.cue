// onepager: the design sheet of the trigger evaluation service: a title block,
// its actors, a Workflow view of who does what in which phase, and a System
// view with its parts list, and general notes. examples/onepager.json is this figure exported.
package gcp

import (
	"github.com/frits-v/stencil/cue/grammars:gcp"
)

onepager: gcp.#Page & {
	title:   "Plain-language event triggers, evaluated over video clips"
	kicker:  "Premise · trigger evaluation"
	lede:    "An operator describes an event in their own words. The service turns the description into an evaluation plan, runs the plan over each new clip, and stores a verdict with the frames that support it."
	foot:    "Balloons key the drawings to the parts list, flags to the general notes, diamonds to the attention items in the title block."
	width:   1500
	badge:   "Internal"
	grammar: "gcp"
	sheet: {
		wordmark: "trig"
		problem: {
			caption: "Problem · rules written as code"
			lead:    "Operators review long stretches of video to find a few events."
			points: [
				{text: "A rule written as code needs an engineer for every change.", marks: [{part: "console"}, {part: "trigger-api"}]},
				{text: "A reviewer watches hours of footage to confirm a few seconds of it.", marks: [{part: "evaluator"}]},
				{text: "A verdict without its frames cannot be checked a month later.", marks: [{part: "verdict-store"}, {note: "evidence-by-reference"}]},
				{text: "A model update can quietly change what a trigger means.", marks: [{attention: "model-updates"}, {note: "approved-plans-only"}]},
			]
		}
		answer: {
			caption:   "Answer · triggers in plain language"
			statement: "The operator writes the event; the service plans it, runs it on every clip, and keeps the frames that decide it."
			paragraphs: [
				{
					lead: "Versioned plans."
					text: "Each trigger compiles to an evaluation plan with a version. A verdict records that version, so a changed plan shows in the history and is never applied silently."
				},
				{
					lead: "Asynchronous evaluation."
					text: "The API returns once a job is queued, and a slow clip never holds an operator request open."
				},
			]
		}
		attention: {
			caption: "Your attention goes here, and only here"
			items: [
				{
					id:    "plan-approval"
					lead:  "Plan approval."
					text:  "You confirm each new or changed plan before it runs."
					actor: "operator"
				},
				{
					id:    "disputed-verdicts"
					lead:  "Disputed verdicts."
					text:  "A verdict you mark wrong comes back with its frames and plan version."
					actor: "operator"
				},
				{
					id:    "model-updates"
					lead:  "Model updates."
					text:  "A new evaluator version re-plans every active trigger and waits for your approval."
					actor: "operator"
				},
			]
		}
		marks: {
			part:      "a piece of the service"
			note:      "asked of an operator"
			attention: "your attention"
		}
	}
	actors: [
		{id: "operator", name: "Operator", text: "writes triggers, reviews verdicts", tint: 4},
		{id: "api", name: "Trigger API", text: "plans, versions, queues", tint: 1},
		{id: "worker", name: "Clip worker", text: "one job per clip", tint: 6},
		{id: "model", name: "Evaluator", text: "reads frames against a plan", tint: 5},
	]
	body: [
		{
			tag:      "View"
			id:       "workflow"
			title:    "Workflow"
			subtitle: "one trigger, from description to stored verdict"
			note:     "a lane for each actor; time runs left to right"
			children: [
				{
					tag: "Phases"
					id:  "trigger-life"
					lanes: ["operator", "api", "worker", "model"]
					phases: [
						{id: "describe", title: "Describe"},
						{id: "plan", title: "Plan and approve"},
						{id: "evaluate", title: "Evaluate each clip"},
						{id: "review", title: "Review"},
					]
					step_fill: "none"
					children: [
						{
							tag:   "Step"
							id:    "s-describe"
							lane:  "operator"
							phase: "describe"
							title: "Describe the event"
							lines: ["in plain language", "camera and hours"]
							marks: [{part: "console"}]
						},
						{
							tag:   "Step"
							id:    "s-idle"
							lane:  "api"
							phase: "describe"
							title: "nothing runs yet"
							look:  "text"
						},
						{
							tag:   "Step"
							id:    "s-compile"
							lane:  "api"
							phase: "plan"
							title: "Compile a plan"
							lines: ["objects, conditions, frame rate", "plan version 1"]
							marks: [{part: "trigger-api"}]
						},
						{
							tag:   "Step"
							id:    "s-approve"
							lane:  "operator"
							phase: "plan"
							col:   2
							title: "Approve the plan"
							lines: ["reads the plan, not the prompt"]
							marks: [{attention: "plan-approval"}, {note: "approved-plans-only"}]
						},
						{
							tag:   "Step"
							id:    "s-store-plan"
							lane:  "api"
							phase: "plan"
							col:   2
							title: "Store plan version"
							look:  "chip"
							fill:  "tint"
							marks: [{part: "verdict-store"}]
						},
						{
							tag:   "Step"
							id:    "s-arrive"
							lane:  "worker"
							phase: "evaluate"
							title: "Clip arrives"
							lines: [{text: "the arrival event names a finished clip", marks: [{note: "out-of-scope"}]}]
							marks: [{part: "clip-bucket"}]
						},
						{
							tag:   "Step"
							id:    "s-evaluate"
							lane:  "worker"
							phase: "evaluate"
							col:   2
							title: "One job per clip"
							look:  "stack"
							lines: ["fetch frames", "call the evaluator"]
							marks: [{part: "worker"}, {part: "queue"}]
						},
						{
							tag:   "Step"
							id:    "s-judge"
							lane:  "model"
							phase: "evaluate"
							col:   2
							title: "Judge the frames"
							lines: ["against the approved plan"]
							marks: [{part: "evaluator"}, {note: "no-store-credentials"}]
						},
						{
							tag:   "Step"
							id:    "s-write"
							lane:  "worker"
							phase: "evaluate"
							col:   3
							title: "Write the verdict"
							lines: ["verdict, frames, plan version"]
							marks: [{part: "verdict-store"}, {note: "evidence-by-reference"}]
						},
						{
							tag:   "Step"
							id:    "s-review"
							lane:  "operator"
							phase: "review"
							title: "Review verdicts"
							lines: ["filter by trigger and time"]
							marks: [{attention: "disputed-verdicts"}]
						},
						{
							tag:   "Step"
							id:    "s-replay"
							lane:  "api"
							phase: "review"
							title: "Replay over earlier clips"
							look:  "planned"
						},
						{
							tag:   "Step"
							id:    "s-model-update"
							lane:  "model"
							phase: "review"
							title: "A new model version re-plans every trigger"
							look:  "note"
							marks: [{attention: "model-updates"}]
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
					grow: [2, 1]
					children: [
						{
							tag: "Col"
							gap: 20
							children: [
								{
									tag: "Row"
									gap: 24
									grow: [0, 1]
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
															id:       "console"
															kind:     "product"
															icon:     "devops"
															title:    "Operator console"
															subtitle: "web client"
															actor:    "operator"
														},
													]
												},
											]
											gap: 24
										},
										{
											tag:   "Box"
											id:    "cloud"
											kind:  "gcp"
											label: "Google Cloud"
											children: [
												{
													tag: "Col"
													gap: 56
													children: [
														{
															tag: "Row"
															grow: [1, 1]
															children: [
																{
																	tag:     "Row"
																	justify: "center"
																	grow: [0]
																	children: [
																		{
																			tag:      "Item"
																			id:       "gateway"
																			kind:     "product"
																			icon:     "networking"
																			title:    "Gateway"
																			subtitle: "API Gateway"
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
																			kind:     "product"
																			icon:     "cloud-run"
																			title:    "Trigger API"
																			subtitle: "Cloud Run"
																			actor:    "api"
																		},
																	]
																},
															]
														},
														{
															tag: "Row"
															grow: [1, 1]
															children: [
																{
																	tag:     "Row"
																	justify: "center"
																	grow: [0]
																	children: [
																		{
																			tag:      "Item"
																			id:       "queue"
																			kind:     "product"
																			icon:     "integration"
																			title:    "Evaluation queue"
																			subtitle: "Pub/Sub"
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
																			id:       "worker"
																			kind:     "product"
																			icon:     "cloud-run"
																			title:    "Clip worker"
																			subtitle: "Cloud Run job"
																			actor:    "worker"
																		},
																	]
																},
															]
														},
														{
															tag: "Row"
															grow: [1, 1]
															children: [
																{
																	tag:     "Row"
																	gap:     32
																	justify: "start"
																	grow: [0, 0]
																	children: [
																		{
																			tag:     "Row"
																			justify: "center"
																			grow: [0]
																			children: [
																				{
																					tag:      "Item"
																					id:       "bucket"
																					kind:     "product"
																					icon:     "cloud-storage"
																					title:    "Clip bucket"
																					subtitle: "Cloud Storage"
																				},
																			]
																		},
																		{
																			tag:      "Item"
																			id:       "verdict-store"
																			kind:     "product"
																			icon:     "cloud-sql"
																			title:    "Verdict store"
																			subtitle: "Cloud SQL"
																		},
																	]
																},
																{
																	tag:     "Row"
																	justify: "end"
																	grow: [0]
																	children: [
																		{
																			tag:     "Row"
																			justify: "center"
																			grow: [0]
																			children: [
																				{
																					tag:      "Item"
																					id:       "evaluator"
																					kind:     "product"
																					icon:     "vertex-ai"
																					title:    "Clip evaluator"
																					subtitle: "Vertex AI endpoint"
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
									]
								},
								{
									tag:   "GeneralNotes"
									id:    "notes"
									title: "General notes"
									note:  "what the service asks of its operators"
									notes: [
										{
											id:     "approved-plans-only"
											text:   "A plan change never reaches a running trigger until an operator approves it."
											detail: "The worker reads the approved version only."
										},
										{
											id:     "evidence-by-reference"
											text:   "Evidence frames are stored as references into the clip, never copied."
											detail: "Retention follows the clip bucket."
										},
										{
											id:     "no-store-credentials"
											text:   "The evaluator holds no credentials for the verdict store."
											detail: "The blocked line in view 3 is an IAM binding, not a convention."
										},
										{
											id:     "out-of-scope"
											text:   "Out of scope: live streams, identifying people, and training a model per customer."
											detail: "The service evaluates finished clips with one shared model."
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
										{id: "console", node: "console", why: "Operators write triggers and judge verdicts without an engineer in the loop."},
										{id: "gateway", node: "gateway", why: "One authenticated entry point; the API never sees an anonymous call."},
										{
											id:   "trigger-api"
											node: "api"
											why:  "Turns a description into a versioned plan and refuses to queue a plan nobody approved."
										},
										{id: "queue", node: "queue", why: "The API returns once a job is queued, so a slow clip never holds a request open."},
										{
											id:   "clip-bucket"
											node: "bucket"
											why:  "A clip is evaluated only once it is complete, and the arrival event names it."
										},
										{id: "worker", node: "worker", why: "Clips are independent, so a failed clip retries alone."},
										{id: "evaluator", node: "evaluator", why: "Reads frames against the plan, and has no write path to the store."},
										{
											id:   "verdict-store"
											node: "verdict-store"
											why:  "Every verdict keeps its evidence frames and the plan version that produced it."
										},
									]
								},
								{
									tag:    "Frame"
									id:     "screen-review"
									label:  "Verdict review in the console: clip, evidence frames, verdict and plan version"
									height: 151
								},
							]
						},
					]
				},
			]
		},
	]
	legend: [
		{line: "solid", by_actor: true, text: "work moves, in the colour of the actor that sends it"},
		{line: "dash", by_actor: true, text: "reads or writes the verdict store"},
		{line: "gray", text: "event from the platform"},
		{line: "deny", text: "blocked path"},
	]
	links: [
		{from: "s-describe", to: "s-compile", line: "solid"},
		{
			from:      "s-compile"
			to:        "s-approve"
			line:      "solid"
			label:     "plan"
			from_side: "top"
			to_side:   "left"
		},
		{from: "s-approve", to: "s-store-plan", line: "solid"},
		{
			from:    "s-store-plan"
			to:      "s-evaluate"
			line:    "dash"
			actor:   "worker"
			label:   "reads approved version"
			to_side: "top"
		},
		{from: "s-arrive", to: "s-evaluate", line: "solid"},
		{from: "s-evaluate", to: "s-judge", line: "solid"},
		{from: "s-judge", to: "s-write", line: "solid"},
		{from: "s-write", to: "s-review", line: "solid"},
		{from: "console", to: "gateway", line: "solid", label: "submit trigger", from_side: "top"},
		{from: "gateway", to: "api", line: "solid", label: "authorized call"},
		{
			from:      "api"
			to:        "queue"
			line:      "solid"
			label:     "queue job"
			from_side: "bottom"
			to_side:   "top"
			sub:       "approved plans only"
			marks: [{note: "approved-plans-only"}]
		},
		{from: "queue", to: "worker", line: "solid", label: "deliver job"},
		{
			from:      "worker"
			to:        "evaluator"
			line:      "solid"
			label:     "evaluate clip"
			from_side: "bottom"
			to_side:   "top"
		},
		{
			from:      "worker"
			to:        "verdict-store"
			line:      "dash"
			label:     "write verdict"
			from_side: "bottom"
			to_side:   "top"
		},
		{from: "bucket", to: "queue", line: "gray", label: "clip finished"},
		{
			from:      "evaluator"
			to:        "verdict-store"
			line:      "deny"
			label:     "no direct write"
			from_side: "left"
			to_side:   "right"
			marks: [{note: "no-store-credentials"}]
		},
	]
}
