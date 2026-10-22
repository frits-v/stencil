# Plain-language event triggers, evaluated over video clips `A1-A2` `/title`

Internal · Premise · trigger evaluation `A1-A2` `/kicker`

An operator describes an event in their own words. The service turns the description into an evaluation plan, runs the plan over each new clip, and stores a verdict with the frames that support it. `A1-B2` `/lede`

## Sheet

Drawn elements: 70 (44 nodes, 15 links, 11 entries). Canvas 1600 x 1248.8 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in. References: a reference such as B3 or B2-C5 names the grid cells under an element's top-left and bottom-right corners, columns 1 to 8 left to right, rows A to F top to bottom. The reference before a pointer is where that element is drawn.

## Title block (view 1)

Wordmark trig. `A1-B8` `/sheet`

### Problem · rules written as code `A2-B5` `/sheet/problem`

Operators review long stretches of video to find a few events.

| Point | Text | Pointer |
|---|---|---|
| P1 | A rule written as code needs an engineer for every change. | `A2-A4` `/sheet/problem/points/0` |
| P2 | A reviewer watches hours of footage to confirm a few seconds of it. | `A2-A4` `/sheet/problem/points/1` |
| P3 | A verdict without its frames cannot be checked a month later. | `A2-A4` `/sheet/problem/points/2` |
| P4 | A model update can quietly change what a trigger means. | `A2-A4` `/sheet/problem/points/3` |

### Answer · triggers in plain language `A5-B7` `/sheet/answer`

The operator writes the event; the service plans it, runs it on every clip, and keeps the frames that decide it.

Versioned plans. Each trigger compiles to an evaluation plan with a version. A verdict records that version, so a changed plan shows in the history and is never applied silently.

Asynchronous evaluation. The API returns once a job is queued, and a slow clip never holds an operator request open.

### Your attention goes here, and only here `A7-B8` `/sheet/attention`

| Item | Pointer |
|---|---|
| Plan approval. You confirm each new or changed plan before it runs. | `A7-A8` `/sheet/attention/items/0` |
| Disputed verdicts. A verdict you mark wrong comes back with its frames and plan version. | `A7-A8` `/sheet/attention/items/1` |
| Model updates. A new evaluator version re-plans every active trigger and waits for your approval. | `A7-A8` `/sheet/attention/items/2` |

## Who acts

| Actor | Id | Colour | Text | Pointer |
|---|---|---|---|---|
| Operator | operator | tint 4; amber in center | writes triggers, reviews verdicts | `B1-B3` `/actors/0` |
| Trigger API | api | tint 1; blue in center | plans, versions, queues | `B3-B4` `/actors/1` |
| Clip worker | worker | tint 6; green in center | one job per clip | `B4-B5` `/actors/2` |
| Evaluator | model | tint 5; violet in center | reads frames against a plan | `B5-B6` `/actors/3` |

## Legend `B1-B8` `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| solid-by-actor | Solid, actor colour | work moves, in the colour of the actor that sends it | 12 | `B1-B3` `/legend/0` |
| dash-by-actor | Dashed, actor colour | reads or writes the verdict store | 2 | `B3-B5` `/legend/1` |
| deny | Dashed red | blocked path | 1 | `B5-B6` `/legend/2` |

## View 2: Workflow `B1-E8` `/body/0`

Subtitle: one trigger, from description to stored verdict. Note: a lane for each actor; time runs left to right.

- Phases: lanes Operator, Trigger API, Clip worker, Evaluator; phases 1 Describe, 2 Plan and approve, 3 Evaluate each clip, 4 Review; step fill none. `B1-E8` `/body/0/children/0`
  - Step Describe the event. Lane Operator, phase 1 Describe, column 1, fill none. `B1-C2` `/body/0/children/0/children/0`
    - Line in plain language.
    - Line camera and hours.
  - Step nothing runs yet. Lane Trigger API, phase 1 Describe, column 1, look text. `C1-C2` `/body/0/children/0/children/1`
  - Step Compile a plan. Lane Trigger API, phase 2 Plan and approve, column 1, fill none. `C2-C3` `/body/0/children/0/children/2`
    - Line objects, conditions, frame rate.
    - Line plan version 1.
  - Step Approve the plan. Lane Operator, phase 2 Plan and approve, column 2, fill none. `B3-C4` `/body/0/children/0/children/3`
    - Line reads the plan, not the prompt.
  - Step Store plan version. Lane Trigger API, phase 2 Plan and approve, column 2, look chip, fill tint. `C3-C4` `/body/0/children/0/children/4`
  - Step Clip arrives. Lane Clip worker, phase 3 Evaluate each clip, column 1, fill none. `D4-D5` `/body/0/children/0/children/5`
    - Line the arrival event names a finished clip.
  - Step One job per clip. Lane Clip worker, phase 3 Evaluate each clip, column 2, look stack, fill none. `D6` `/body/0/children/0/children/6`
    - Line fetch frames.
    - Line call the evaluator.
  - Step Judge the frames. Lane Evaluator, phase 3 Evaluate each clip, column 2, fill none. `D6` `/body/0/children/0/children/7`
    - Line against the approved plan.
  - Step Write the verdict. Lane Clip worker, phase 3 Evaluate each clip, column 3, fill none. `D6-D7` `/body/0/children/0/children/8`
    - Line verdict, frames, plan version.
  - Step Review verdicts. Lane Operator, phase 4 Review, column 1, fill none. `B7-C8` `/body/0/children/0/children/9`
    - Line filter by trigger and time.
  - Step Replay over earlier clips. Lane Trigger API, phase 4 Review, column 1, look planned. `C7-C8` `/body/0/children/0/children/10`
  - Step A new model version re-plans every trigger. Lane Evaluator, phase 4 Review, column 1, look note. `D7-D8` `/body/0/children/0/children/11`

| Lane | 1 Describe `B1-B2` `/body/0/children/0/phases/0` | 2 Plan and approve `B2` `/body/0/children/0/phases/1` | 3 Evaluate each clip `B4` `/body/0/children/0/phases/2` | 4 Review `B7` `/body/0/children/0/phases/3` |
|---|---|---|---|---|
| Operator `B1-C8` `/body/0/children/0/lanes/0` | Describe the event | Approve the plan | none | Review verdicts |
| Trigger API `C1-D8` `/body/0/children/0/lanes/1` | nothing runs yet | Compile a plan; Store plan version | none | Replay over earlier clips |
| Clip worker `D1-D8` `/body/0/children/0/lanes/2` | none | none | Clip arrives; One job per clip; Write the verdict | none |
| Evaluator `D1-E8` `/body/0/children/0/lanes/3` | none | none | Judge the frames | A new model version re-plans every trigger |

### Lines in view 2

| Line | Kind | From | To | Style | Ends | Means | Actor | Pointer |
|---|---|---|---|---|---|---|---|---|
| unlabelled | link | Describe the event (s-describe) | Compile a plan (s-compile) | solid, tint 4, Operator; amber in center | arrow end | work moves, in the colour of the actor that sends it | Operator | `C2` `/links/0` |
| plan | link | Compile a plan (s-compile) | Approve the plan (s-approve) | solid, tint 1, Trigger API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Trigger API | `C3` `/links/1` |
| unlabelled | link | Approve the plan (s-approve) | Store plan version (s-store-plan) | solid, tint 4, Operator; amber in center | arrow end | work moves, in the colour of the actor that sends it | Operator | `C4` `/links/2` |
| reads approved version | link | Store plan version (s-store-plan) | One job per clip (s-evaluate) | dash, tint 6, Clip worker; green in center | arrow end | reads or writes the verdict store | Clip worker | `C5` `/links/3` |
| unlabelled | link | Clip arrives (s-arrive) | One job per clip (s-evaluate) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | `D5` `/links/4` |
| unlabelled | link | One job per clip (s-evaluate) | Judge the frames (s-judge) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | `D6` `/links/5` |
| unlabelled | link | Judge the frames (s-judge) | Write the verdict (s-write) | solid, tint 5, Evaluator; violet in center | arrow end | work moves, in the colour of the actor that sends it | Evaluator | `D6` `/links/6` |
| unlabelled | link | Write the verdict (s-write) | Review verdicts (s-review) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | `C7` `/links/7` |

## View 3: System `E1-F8` `/body/1`

Subtitle: the pieces, and why each exists. Note: one request, from the console to a stored verdict.

- Row, left to right. `/body/1/children/0`
  - Col, top to bottom. `/body/1/children/0/children/0`
    - Row, left to right. `/body/1/children/0/children/0/children/0`
      - Item Operator console: web client; kind product, icon devops, actor Operator. `E1-E2` `/body/1/children/0/children/0/children/0/children/0`
    - Frame Verdict review in the console: clip, evidence frames, verdict and plan version: 135 px placeholder. `E1-F2` `/body/1/children/0/children/0/children/1`
  - Box Google Cloud; kind gcp. `E2-F8` `/body/1/children/0/children/1`
    - Col, top to bottom. `/body/1/children/0/children/1/children/0`
      - Row, left to right. `/body/1/children/0/children/1/children/0/children/0`
        - Row, left to right. `/body/1/children/0/children/1/children/0/children/0/children/0`
          - Item Gateway: API Gateway; kind product, icon networking, product API Gateway, actor Trigger API. `E4` `/body/1/children/0/children/1/children/0/children/0/children/0/children/0`
        - Row, left to right. `/body/1/children/0/children/1/children/0/children/0/children/1`
          - Item Trigger API: Cloud Run; kind product, icon cloud-run, product Cloud Run, actor Trigger API. `E7` `/body/1/children/0/children/1/children/0/children/0/children/1/children/0`
      - Row, left to right. `/body/1/children/0/children/1/children/0/children/1`
        - Row, left to right. `/body/1/children/0/children/1/children/0/children/1/children/0`
          - Item Evaluation queue: Pub/Sub; kind product, icon integration, product Pub/Sub, actor Trigger API. `E3-F4` `/body/1/children/0/children/1/children/0/children/1/children/0/children/0`
        - Row, left to right. `/body/1/children/0/children/1/children/0/children/1/children/1`
          - Item Clip worker: Cloud Run job; kind product, icon cloud-run, product Cloud Run, actor Clip worker. `E5-F6` `/body/1/children/0/children/1/children/0/children/1/children/1/children/0`
        - Row, left to right. `/body/1/children/0/children/1/children/0/children/1/children/2`
          - Item Clip evaluator: Vertex AI endpoint; kind product, icon vertex-ai, product Vertex AI, actor Evaluator. `E7-F8` `/body/1/children/0/children/1/children/0/children/1/children/2/children/0`
      - Row, left to right. `/body/1/children/0/children/1/children/0/children/2`
        - Item Verdict store: Cloud SQL; kind product, icon cloud-sql, product Cloud SQL. `F5-F6` `/body/1/children/0/children/1/children/0/children/2/children/0`

### Lines in view 3

| Line | Kind | From | To | Style | Ends | Means | Actor | Pointer |
|---|---|---|---|---|---|---|---|---|
| submit trigger | link | Operator console (console) | Gateway (gateway) | solid, tint 4, Operator; amber in center | arrow end | work moves, in the colour of the actor that sends it | Operator | `E3` `/links/8` |
| authorized call | link | Gateway (gateway) | Trigger API (api) | solid, tint 1, Trigger API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Trigger API | `E5` `/links/9` |
| queue job | link | Trigger API (api) | Evaluation queue (queue) | solid, tint 1, Trigger API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Trigger API | `E5` `/links/10` |
| deliver job | link | Evaluation queue (queue) | Clip worker (worker) | solid, tint 1, Trigger API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Trigger API | `F4` `/links/11` |
| evaluate clip | link | Clip worker (worker) | Clip evaluator (evaluator) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | `F6` `/links/12` |
| write verdict | link | Clip worker (worker) | Verdict store (verdict-store) | dash, tint 6, Clip worker; green in center | arrow end | reads or writes the verdict store | Clip worker | `F5` `/links/13` |
| no direct write | link | Clip evaluator (evaluator) | Verdict store (verdict-store) | deny; red in center | arrow end | blocked path | none | `F7` `/links/14` |

Foot: Every verdict keeps the frames that support it and the plan version that produced it. `F1-F8` `/foot`
