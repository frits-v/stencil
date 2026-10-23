# Plain-language event triggers, evaluated over video clips `A1-A2` `/title`

Internal · Premise · trigger evaluation `A1-A2` `/kicker`

An operator describes an event in their own words. The service turns the description into an evaluation plan, runs the plan over each new clip, and stores a verdict with the frames that support it. `A1-A2` `/lede`

## Sheet

Drawn elements: 141 (63 nodes, 18 links, 24 entries, 36 marks). Canvas 1600 x 2581.1 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in. References: a reference such as B3 or B2-C5 names the grid cells under an element's top-left and bottom-right corners, columns 1 to 8 left to right, rows A to F top to bottom. The reference before a pointer is where that element is drawn; a Zones cell lists the cells of every balloon, flag or diamond keyed to its entry.

## Title block (view 1)

Wordmark trig. `A1-A8` `/sheet`

### Problem · rules written as code `A2-A5` `/sheet/problem`

Operators review long stretches of video to find a few events.

| Point | Text | Marks | Pointer |
|---|---|---|---|
| P1 | A rule written as code needs an engineer for every change. | balloon 1 Operator console `A4` `/sheet/problem/points/0/marks/0`; balloon 3 Trigger API `A4` `/sheet/problem/points/0/marks/1` | `A2-A4` `/sheet/problem/points/0` |
| P2 | A reviewer watches hours of footage to confirm a few seconds of it. | balloon 7 Clip evaluator `A4` `/sheet/problem/points/1/marks/0` | `A2-A4` `/sheet/problem/points/1` |
| P3 | A verdict without its frames cannot be checked a month later. | balloon 8 Verdict store `A4` `/sheet/problem/points/2/marks/0`; flag 2 Evidence frames are stored as references into the clip, never copied. `A4` `/sheet/problem/points/2/marks/1` | `A2-A4` `/sheet/problem/points/2` |
| P4 | A model update can quietly change what a trigger means. | diamond 3 Model updates. `A4` `/sheet/problem/points/3/marks/0`; flag 1 A plan change never reaches a running trigger until an operator approves it. `A4` `/sheet/problem/points/3/marks/1` | `A2-A4` `/sheet/problem/points/3` |

### Answer · triggers in plain language `A5-A7` `/sheet/answer`

The operator writes the event; the service plans it, runs it on every clip, and keeps the frames that decide it.

Versioned plans. Each trigger compiles to an evaluation plan with a version. A verdict records that version, so a changed plan shows in the history and is never applied silently.

Asynchronous evaluation. The API returns once a job is queued, and a slow clip never holds an operator request open.

### Your attention goes here, and only here `A7-A8` `/sheet/attention`

| Mark | Item | Actor | Zones | Pointer |
|---|---|---|---|---|
| diamond 1 | Plan approval. You confirm each new or changed plan before it runs. | Operator | B4 | `A7-A8` `/sheet/attention/items/0` |
| diamond 2 | Disputed verdicts. A verdict you mark wrong comes back with its frames and plan version. | Operator | B8 C2 | `A7-A8` `/sheet/attention/items/1` |
| diamond 3 | Model updates. A new evaluator version re-plans every active trigger and waits for your approval. | Operator | A4 B8 | `A7-A8` `/sheet/attention/items/2` |

## Who acts

| Actor | Id | Colour | Text | Pointer |
|---|---|---|---|---|
| Operator | operator | tint 4; amber in center | writes triggers, reviews verdicts | `A1-A3` `/actors/0` |
| Trigger API | api | tint 1; blue in center | plans, versions, queues | `A3-A4` `/actors/1` |
| Clip worker | worker | tint 6; green in center | one job per clip | `A4-A5` `/actors/2` |
| Evaluator | model | tint 5; violet in center | reads frames against a plan | `A5-A6` `/actors/3` |

## Legend `A1-A8` `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| solid-by-actor | Solid, actor colour | work moves, in the colour of the actor that sends it | 13 | `A1-A3` `/legend/0` |
| dash-by-actor | Dashed, actor colour | reads or writes the verdict store | 2 | `A3-A5` `/legend/1` |
| solid-by-actor-rail | Solid, actor colour, rail | one topic that every job passes through | 1 | `A5-A7` `/legend/2` |
| solid-by-actor-planned | Hollow, actor colour | planned, not built yet | 1 | `A1-A2` `/legend/3` |
| gray | Solid gray | event from the platform | 1 | `A2-A3` `/legend/4` |
| deny | Dashed red | blocked path | 1 | `A3-A4` `/legend/5` |

Mark kinds: a piece of the service (balloon, parts list); asked of an operator (flag, general notes); your attention (diamond, attention items); a numbered request step (step badge, numbered lines).

## View 2: Workflow `A1-C8` `/body/0`

Subtitle: one trigger, from description to stored verdict. Note: a lane for each actor; time runs left to right.

- Phases: lanes Operator, Trigger API, Clip worker, Evaluator; phases 1 Describe, 2 Plan and approve, 3 Evaluate each clip, 4 Review; step fill cell; detail A frames One job per clip, Judge the frames, Write the verdict. `A1-C8` `/body/0/children/0`
  - Step Describe the event. Lane Operator, phase 1 Describe, column 1. `B1-B2` `/body/0/children/0/children/0`
    - Mark balloon 1 Operator console `B2` `/body/0/children/0/children/0/marks/0`
    - Line in plain language.
    - Line camera and hours.
  - Step nothing runs yet. Lane Trigger API, phase 1 Describe, column 1, look text. `B1-B2` `/body/0/children/0/children/1`
  - Step Compile a plan. Lane Trigger API, phase 2 Plan and approve, column 1. `B2-B3` `/body/0/children/0/children/2`
    - Mark balloon 3 Trigger API `B3` `/body/0/children/0/children/2/marks/0`
    - Line objects, conditions, frame rate.
    - Line plan version 1.
  - Step Approve the plan. Lane Operator, phase 2 Plan and approve, column 2. `B3-B4` `/body/0/children/0/children/3`
    - Mark diamond 1 Plan approval. `B4` `/body/0/children/0/children/3/marks/0`
    - Mark flag 1 A plan change never reaches a running trigger until an operator approves it. `B4` `/body/0/children/0/children/3/marks/1`
    - Line reads the plan, not the prompt.
  - Step Store plan version. Lane Trigger API, phase 2 Plan and approve, column 2, look chip, fill tint. `B3-B4` `/body/0/children/0/children/4`
    - Mark balloon 8 Verdict store `B4` `/body/0/children/0/children/4/marks/0`
  - Step Clip arrives. Lane Clip worker, phase 3 Evaluate each clip, column 1. `B4-B5` `/body/0/children/0/children/5`
    - Mark balloon 5 Clip bucket `B5` `/body/0/children/0/children/5/marks/0`
    - Line the arrival event names a finished clip.
      - Mark flag 4 Out of scope: live streams, identifying people, and training a model per customer. `B5` `/body/0/children/0/children/5/lines/0/marks/0`
  - Step One job per clip. Lane Clip worker, phase 3 Evaluate each clip, column 2, look stack. `B6` `/body/0/children/0/children/6`
    - Mark balloon 6 Clip worker `B6` `/body/0/children/0/children/6/marks/0`
    - Mark balloon 4 Evaluation queue `B6` `/body/0/children/0/children/6/marks/1`
    - Line fetch frames.
    - Line call the evaluator.
  - Step Judge the frames. Lane Evaluator, phase 3 Evaluate each clip, column 2 to 3. `B6-C7` `/body/0/children/0/children/7`
    - Mark balloon 7 Clip evaluator `B7` `/body/0/children/0/children/7/marks/0`
    - Mark flag 3 The evaluator holds no credentials for the verdict store. `B7` `/body/0/children/0/children/7/marks/1`
    - Line against the approved plan.
  - Step Write the verdict. Lane Clip worker, phase 3 Evaluate each clip, column 3. `B6-B7` `/body/0/children/0/children/8`
    - Mark balloon 8 Verdict store `B7` `/body/0/children/0/children/8/marks/0`
    - Mark flag 2 Evidence frames are stored as references into the clip, never copied. `B7` `/body/0/children/0/children/8/marks/1`
    - Line verdict, frames, plan version.
  - Step Review verdicts. Lane Operator, phase 4 Review, column 1. `B7-B8` `/body/0/children/0/children/9`
    - Mark diamond 2 Disputed verdicts. `B8` `/body/0/children/0/children/9/marks/0`
    - Line filter by trigger and time.
  - Step Replay over earlier clips. Lane Trigger API, phase 4 Review, column 1, look planned, fill cell. `B7-B8` `/body/0/children/0/children/10`
    - Mark balloon 9 Replay scheduler `B8` `/body/0/children/0/children/10/marks/0`
  - Step A new model version re-plans every trigger. Lane Evaluator, phase 4 Review, column 1, look note, fill cell. `B7-C8` `/body/0/children/0/children/11`
    - Mark diamond 3 Model updates. `B8` `/body/0/children/0/children/11/marks/0`
- Ladder, detail A: When a clip fails or a verdict is disputed; a failed clip never blocks the queue; rung fill none. Frames One job per clip, Judge the frames, Write the verdict. `C1-C8` `/body/0/children/1`
  - Rung 1 (bottom) Retry, the clip worker; actor Clip worker: A failed frame fetch or evaluator call retries the clip job with backoff, three attempts in all. `C1-C8` `/body/0/children/1/rungs/0`
  - Rung 2 Dead letter, the Trigger API; actor Trigger API: After the third failure the job moves to a dead-letter subscription and the trigger lists the clip as unevaluated. `C1-C8` `/body/0/children/1/rungs/1`
  - Rung 3 Ask the operator, you; actor Operator: A disputed verdict returns with its frames and plan version; a confirmed dispute opens a plan change for approval. `C1-C8` `/body/0/children/1/rungs/2`
    - Mark diamond 2 Disputed verdicts. `C2` `/body/0/children/1/rungs/2/marks/0`
  - Rung 4 (top) Page the owner, the service owner; planned: A trigger with more than ten dead-lettered clips in an hour pages the service owner. `C1-C8` `/body/0/children/1/rungs/3`

| Lane | 1 Describe `A1-A2` `/body/0/children/0/phases/0` | 2 Plan and approve `A2` `/body/0/children/0/phases/1` | 3 Evaluate each clip `A4` `/body/0/children/0/phases/2` | 4 Review `A7-A8` `/body/0/children/0/phases/3` |
|---|---|---|---|---|
| Operator `A1-B8` `/body/0/children/0/lanes/0` | Describe the event | Approve the plan | none | Review verdicts |
| Trigger API `B1-B8` `/body/0/children/0/lanes/1` | nothing runs yet | Compile a plan; Store plan version | none | Replay over earlier clips |
| Clip worker `B1-B8` `/body/0/children/0/lanes/2` | none | none | Clip arrives; One job per clip; Write the verdict | none |
| Evaluator `B1-C8` `/body/0/children/0/lanes/3` | none | none | Judge the frames | A new model version re-plans every trigger |

### Lines in view 2

| Line | Kind | From | To | Style | Ends | Means | Actor | Marks | Pointer |
|---|---|---|---|---|---|---|---|---|---|
| unlabelled | link | Describe the event (s-describe) | Compile a plan (s-compile) | solid, tint 4, Operator; amber in center | arrow end | work moves, in the colour of the actor that sends it | Operator | none | `B2` `/links/0` |
| plan | link | Compile a plan (s-compile) | Approve the plan (s-approve) | solid, tint 1, Trigger API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Trigger API | none | `B3` `/links/1` |
| unlabelled | link | Approve the plan (s-approve) | Store plan version (s-store-plan) | solid, tint 4, Operator; amber in center | arrow end | work moves, in the colour of the actor that sends it | Operator | none | `B4` `/links/2` |
| reads approved version | link | Store plan version (s-store-plan) | One job per clip (s-evaluate) | dash, tint 6, Clip worker; green in center | arrow end | reads or writes the verdict store | Clip worker | none | `B5` `/links/3` |
| unlabelled | link | Clip arrives (s-arrive) | One job per clip (s-evaluate) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | none | `B5` `/links/4` |
| unlabelled | link | One job per clip (s-evaluate) | Judge the frames (s-judge) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | none | `B6` `/links/5` |
| unlabelled | link | Judge the frames (s-judge) | Write the verdict (s-write) | solid, tint 5, Evaluator; violet in center | arrow end | work moves, in the colour of the actor that sends it | Evaluator | none | `B7` `/links/6` |
| unlabelled | link | Write the verdict (s-write) | Review verdicts (s-review) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | none | `B7` `/links/7` |

## View 3: System `C1-E8` `/body/1`

Subtitle: the pieces, and why each exists. Note: balloons key the drawing to the parts list.

- Row, left to right. `/body/1/children/0`
  - Col, top to bottom. `/body/1/children/0/children/0`
    - Row, left to right. `/body/1/children/0/children/0/children/0`
      - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/0`
        - Row, left to right. `/body/1/children/0/children/0/children/0/children/0/children/0`
          - Item Operator: one per site; kind person, outline person, actor Operator. `D1` `/body/1/children/0/children/0/children/0/children/0/children/0/children/0`
        - Row, left to right. `/body/1/children/0/children/0/children/0/children/0/children/1`
          - Item Operator console: web client; kind product, icon devops, actor Operator. `D1-D2` `/body/1/children/0/children/0/children/0/children/0/children/1/children/0`
            - Mark balloon 1 Operator console `D1` `/body/1/children/0/children/1/children/0/parts/0/node`
      - Box Google Cloud; kind gcp. `C2-E6` `/body/1/children/0/children/0/children/0/children/1`
        - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0`
          - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/0`
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/0`
              - Item Gateway: API Gateway; kind product, icon integration, product API Gateway, actor Trigger API. `C3-D4` `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/0/children/0`
                - Mark balloon 2 Gateway `C3` `/body/1/children/0/children/1/children/0/parts/1/node`
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/1`
              - Item Trigger API: Cloud Run; kind product, icon cloud-run, product Cloud Run, actor Trigger API. `C5-D6` `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/1/children/0`
                - Mark balloon 3 Trigger API `C6` `/body/1/children/0/children/1/children/0/parts/2/node`
          - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1`
            - Pipe Evaluation queue · Pub/Sub.
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1`
              - Box Evaluation path · one job per clip; kind assembly, tint 6, actor Clip worker. `D4-D5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0`
                - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0`
                  - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/0`
                    - Item Clip worker: Cloud Run job; kind product, icon cloud-run, product Cloud Run, outline stack, actor Clip worker. `D4-D5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/0/children/0`
                      - Mark balloon 6 Clip worker `D5` `/body/1/children/0/children/1/children/0/parts/5/node`
                  - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/1`
                    - Item Clip evaluator: Vertex AI endpoint; kind product, icon vertex-ai, product Vertex AI, actor Evaluator. `D4-D5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/1/children/0`
                      - Mark balloon 7 Clip evaluator `D5` `/body/1/children/0/children/1/children/0/parts/6/node`
          - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2`
            - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/0`
              - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/0/children/0`
                - Item Clip bucket: Cloud Storage; kind product, icon cloud-storage, product Cloud Storage. `D2-E3` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/0/children/0/children/0`
                  - Mark balloon 5 Clip bucket `E3` `/body/1/children/0/children/1/children/0/parts/4/node`
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/1`
              - Item Verdict store: Cloud SQL; kind product, icon cloud-sql, product Cloud SQL, outline store, actor Clip worker. `D3-E5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/1/children/0`
                - Mark balloon 8 Verdict store `D4` `/body/1/children/0/children/1/children/0/parts/7/node`
                - Compartment Triggers.
                - Compartment Plan versions.
                - Compartment Verdicts.
                - Compartment Evidence refs.
                  - Mark flag 2 Evidence frames are stored as references into the clip, never copied. `E5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/1/children/0/compartments/3/marks/0`
            - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/2`
              - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/2/children/0`
                - Item Replay scheduler: Cloud Scheduler; kind product, icon integration, product Cloud Scheduler, actor Trigger API, planned. `D5-E6` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/2/children/0/children/0`
                  - Mark balloon 9 Replay scheduler `E6` `/body/1/children/0/children/1/children/0/parts/8/node`
    - General notes: what the service asks of its operators. `E1-E6` `/body/1/children/0/children/0/children/1`
      - Note 1 (approved-plans-only): A plan change never reaches a running trigger until an operator approves it. The worker reads the approved version only. Zones A4 B4 D5. `E1-E6` `/body/1/children/0/children/0/children/1/notes/0`
      - Note 2 (evidence-by-reference): Evidence frames are stored as references into the clip, never copied. Retention follows the clip bucket. Zones A4 B7 E5. `E1-E6` `/body/1/children/0/children/0/children/1/notes/1`
      - Note 3 (no-store-credentials): The evaluator holds no credentials for the verdict store. The blocked line in view 3 is an IAM binding, not a convention. Zones B7 D5. `E1-E6` `/body/1/children/0/children/0/children/1/notes/2`
      - Note 4 (out-of-scope): Out of scope: live streams, identifying people, and training a model per customer. The service evaluates finished clips with one shared model. Zones B5. `E1-E6` `/body/1/children/0/children/0/children/1/notes/3`
      - Foot: Each flag on the drawings marks where its note applies.
  - Col, top to bottom. `/body/1/children/0/children/1`
    - Parts list: what the service is made of, and why each part exists. `C6-E8` `/body/1/children/0/children/1/children/0`
    - Frame Verdict review in the console: clip, evidence frames, verdict and plan version: 240 px placeholder. `E6-E8` `/body/1/children/0/children/1/children/1`

### Parts list

| Part | Name | What | Why | Actor | Zones | Pointer |
|---|---|---|---|---|---|---|
| 1 | Operator console | web client | Operators write triggers and judge verdicts without an engineer in the loop. | Operator | A4 B2 D1 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/0` |
| 2 | Gateway | API Gateway | One authenticated entry point; the API never sees an anonymous call. | Trigger API | C3 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/1` |
| 3 | Trigger API | Cloud Run | Turns a description into a versioned plan and refuses to queue a plan nobody approved. | Trigger API | A4 B3 C6 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/2` |
| 4 | Evaluation queue | Pub/Sub | The API returns once a job is queued, so a slow clip never holds a request open. | Trigger API | B6 D3 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/3` |
| 5 | Clip bucket | Cloud Storage | A clip is evaluated only once it is complete, and the arrival event names it. | none | B5 E3 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/4` |
| 6 | Clip worker | Cloud Run job | Clips are independent, so a failed clip retries alone. | Clip worker | B6 D5 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/5` |
| 7 | Clip evaluator | Vertex AI endpoint | Reads frames against the plan, and has no write path to the store. | Evaluator | A4 B7 D5 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/6` |
| 8 | Verdict store | Cloud SQL | Every verdict keeps its evidence frames and the plan version that produced it. | Clip worker | A4 B4 B7 D4 | `E6-E8` `/body/1/children/0/children/1/children/0/parts/7` |
| 9 | Replay scheduler, planned | Cloud Scheduler | Runs a trigger over earlier clips on a schedule, so the operator does not wait on a replay. | Trigger API | B8 E6 | `E6-E8` `/body/1/children/0/children/1/children/0/parts/8` |

### Lines in view 3

| Line | Kind | From | To | Style | Ends | Means | Actor | Marks | Pointer |
|---|---|---|---|---|---|---|---|---|---|
| Evaluation queue · Pub/Sub | pipe v, rail | rail; in from Trigger API (api); in from Clip bucket (bucket) | rail; out to Clip worker (worker) | solid, tint 1, Trigger API; blue in center | no arrowheads | one topic that every job passes through | Trigger API | balloon 4 Evaluation queue `D3` `/body/1/children/0/children/1/children/0/parts/3/node` | `D2-D3` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/0` |
| writes, reviews | link | Operator (operator-person) | Operator console (console) | solid, tint 4, Operator; amber in center | arrow end | work moves, in the colour of the actor that sends it | Operator | none | `D1` `/links/8` |
| step 1: submit trigger | link | Operator console (console) | Gateway (gateway) | solid, tint 4, Operator; amber in center | arrow end | work moves, in the colour of the actor that sends it | Operator | none | `C2` `/links/9` |
| step 2: authorized call | link | Gateway (gateway) | Trigger API (api) | solid, tint 1, Trigger API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Trigger API | none | `C4` `/links/10` |
| step 3: queue job · approved plans only | link | Trigger API (api) | Evaluation queue (queue) | solid, tint 1, Trigger API; blue in center | dot end, valve before the end | work moves, in the colour of the actor that sends it | Trigger API | flag 1 A plan change never reaches a running trigger until an operator approves it. `D5` `/links/11/marks/0` | `D4` `/links/11` |
| step 4: deliver job | link | Evaluation queue (queue) | Clip worker (worker) | solid, tint 1, Trigger API; blue in center | dot start, arrow end | work moves, in the colour of the actor that sends it | Trigger API | none | `D3` `/links/12` |
| step 5: evaluate clip | link | Clip worker (worker) | Clip evaluator (evaluator) | solid, tint 6, Clip worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Clip worker | none | `D5` `/links/13` |
| step 6: write verdict | link | Clip worker (worker) | Verdict store (verdict-store) | dash, tint 6, Clip worker; green in center | arrow end | reads or writes the verdict store | Clip worker | none | `D4` `/links/14` |
| schedule replays | link | Trigger API (api) | Replay scheduler (replay) | solid, tint 1, Trigger API, planned; blue in center | arrow end | planned, not built yet | Trigger API | none | `D5` `/links/15` |
| clip finished | link | Clip bucket (bucket) | Evaluation queue (queue) | gray; gray in center | dot end | event from the platform | none | none | `D3` `/links/16` |
| no direct write | link | Clip evaluator (evaluator) | Verdict store (verdict-store) | deny; red in center | stop plate end | blocked path | none | flag 3 The evaluator holds no credentials for the verdict store. `D5` `/links/17/marks/0` | `D5` `/links/17` |

## View 4: How-tos `F1-F8` `/body/2`

Subtitle: what an operator types. Note: real output, trimmed.

- How-to column. `/body/2/children/0`

### How-to 4.1: Write a trigger `F1-F4` `/body/2/children/0/items/0`

- Command trig create.
- Words then.
- Command trig approve.

Describe the event, read the plan the service returns, then approve it. Nothing runs before the approval.

```text
$ trig create --camera dock-4 "a forklift enters while the bay door is open"
  trigger t-31 created, plan version 1
  objects: forklift, bay door; condition: door open while a forklift is in the bay
# the plan waits for approval
$ trig approve t-31 --version 1
✓ t-31 live from plan version 1
```

### How-to 4.2: Replay a trigger `F5-F8` `/body/2/children/0/items/1`

- Command trig replay.

Queues every stored clip in the range; verdicts arrive as the jobs finish.

```text
$ trig replay t-31 --from 2026-10-01 --to 2026-10-07
  412 clips queued under plan version 1
✓ verdicts: 409 written, 3 retrying
```

### How-to 4.3: Dispute a verdict `F1-F4` `/body/2/children/0/items/2`

- Command trig dispute.

Sends the verdict back with its frames. A confirmed dispute opens a plan change.

```text
$ trig dispute v-88201 --reason "the door was closed"
  verdict v-88201 returned to t-31 with 6 frames
# a confirmed dispute opens plan version 2 for approval
```

### How-to 4.4: List verdicts `F5-F8` `/body/2/children/0/items/3`

- Command trig verdicts.

Text beside the terminal.

Filters by trigger, clip and capture time. A disputed verdict stays listed until its dispute closes.

```text
$ trig verdicts t-31 --since 2026-10-07
  3 verdicts for t-31 under plan version 1
    v-88201 match, 6 frames, disputed
    v-88202 no match
    v-88203 match, 4 frames
```

Foot: Balloons key the drawings to the parts list, flags to the general notes, diamonds to the attention items in the title block. `F1-F8` `/foot`
