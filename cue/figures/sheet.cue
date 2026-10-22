// onepager: the design sheet of the trigger evaluation service: a title block,
// its actors, a Workflow view of who does what in which phase, and a System
// view. examples/onepager.json is this figure exported.
package gcp

import (
	"github.com/frits-v/stencil/cue/grammars:gcp"
)

onepager: gcp.#Page & {
	title:   "Plain-language event triggers, evaluated over video clips"
	kicker:  "Premise · trigger evaluation"
	lede:    "An operator describes an event in their own words. The service turns the description into an evaluation plan, runs the plan over each new clip, and stores a verdict with the frames that support it."
	foot:    "Every verdict keeps the frames that support it and the plan version that produced it."
	width:   1500
	badge:   "Internal"
	grammar: "gcp"
	sheet: {
		wordmark: "trig"
		problem: {
			caption: "Problem · rules written as code"
			lead:    "Operators review long stretches of video to find a few events."
			points: [
				{text: "A rule written as code needs an engineer for every change."},
				{text: "A reviewer watches hours of footage to confirm a few seconds of it."},
				{text: "A verdict without its frames cannot be checked a month later."},
				{text: "A model update can quietly change what a trigger means."},
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
						},
						{
							tag:   "Step"
							id:    "s-approve"
							lane:  "operator"
							phase: "plan"
							col:   2
							title: "Approve the plan"
							lines: ["reads the plan, not the prompt"]
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
						},
						{
							tag:   "Step"
							id:    "s-arrive"
							lane:  "worker"
							phase: "evaluate"
							title: "Clip arrives"
							lines: ["the arrival event names a finished clip"]
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
						},
						{
							tag:   "Step"
							id:    "s-judge"
							lane:  "model"
							phase: "evaluate"
							col:   2
							title: "Judge the frames"
							lines: ["against the approved plan"]
						},
						{
							tag:   "Step"
							id:    "s-write"
							lane:  "worker"
							phase: "evaluate"
							col:   3
							title: "Write the verdict"
							lines: ["verdict, frames, plan version"]
						},
						{
							tag:   "Step"
							id:    "s-review"
							lane:  "operator"
							phase: "review"
							title: "Review verdicts"
							lines: ["filter by trigger and time"]
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
			note:     "one request, from the console to a stored verdict"
			children: [
				{
					tag: "Row"
					gap: 32
					grow: [1, 4]
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
								{
									tag:    "Frame"
									id:     "screen-review"
									label:  "Verdict review in the console: clip, evidence frames, verdict and plan version"
									height: 135
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
									gap: 48
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
											grow: [1, 1, 1]
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
										{
											tag:     "Row"
											justify: "center"
											grow: [0]
											children: [
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
									]
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
		{from: "console", to: "gateway", line: "solid", label: "submit trigger"},
		{from: "gateway", to: "api", line: "solid", label: "authorized call"},
		{
			from:      "api"
			to:        "queue"
			line:      "solid"
			label:     "queue job"
			from_side: "bottom"
			to_side:   "top"
		},
		{from: "queue", to: "worker", line: "solid", label: "deliver job"},
		{from: "worker", to: "evaluator", line: "solid", label: "evaluate clip"},
		{from: "worker", to: "verdict-store", line: "dash", label: "write verdict"},
		{
			from:      "evaluator"
			to:        "verdict-store"
			line:      "deny"
			label:     "no direct write"
			from_side: "bottom"
			to_side:   "right"
		},
	]
}
