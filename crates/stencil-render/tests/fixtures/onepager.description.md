# Plain-language event triggers, evaluated over video clips `A1-B2` `/title`

Internal · Premise · trigger evaluation `A1-A2` `/kicker`

An operator describes an event in their own words. The service turns the description into an evaluation plan, runs the plan over each new clip, and stores a verdict with the frames that support it. `B1-B2` `/lede`

## Sheet

Drawn elements: 38 (28 nodes, 7 links, 3 entries). Canvas 1600 x 806.4 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in. References: a reference such as B3 or B2-C5 names the grid cells under an element's top-left and bottom-right corners, columns 1 to 8 left to right, rows A to F top to bottom. The reference before a pointer is where that element is drawn.

## Title block (view 1)

Wordmark trig. `A1-C8` `/sheet`

### Problem · rules written as code `A2-B5` `/sheet/problem`

Operators review long stretches of video to find a few events.

| Point | Text | Pointer |
|---|---|---|
| P1 | A rule written as code needs an engineer for every change. | `A2-A4` `/sheet/problem/points/0` |
| P2 | A reviewer watches hours of footage to confirm a few seconds of it. | `A2-B4` `/sheet/problem/points/1` |
| P3 | A verdict without its frames cannot be checked a month later. | `B2-B4` `/sheet/problem/points/2` |
| P4 | A model update can quietly change what a trigger means. | `B2-B4` `/sheet/problem/points/3` |

### Answer · triggers in plain language `A5-B7` `/sheet/answer`

The operator writes the event; the service plans it, runs it on every clip, and keeps the frames that decide it.

Versioned plans. Each trigger compiles to an evaluation plan with a version. A verdict records that version, so a changed plan shows in the history and is never applied silently.

Asynchronous evaluation. The API returns once a job is queued, and a slow clip never holds an operator request open.

### Your attention goes here, and only here `A7-B8` `/sheet/attention`

| Item | Pointer |
|---|---|
| Plan approval. You confirm each new or changed plan before it runs. | `A7-A8` `/sheet/attention/items/0` |
| Disputed verdicts. A verdict you mark wrong comes back with its frames and plan version. | `A7-B8` `/sheet/attention/items/1` |
| Model updates. A new evaluator version re-plans every active trigger and waits for your approval. | `B7-B8` `/sheet/attention/items/2` |

## Legend `B1-C8` `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| solid-4 | Solid amber | the operator's request | 1 | `B1-C2` `/legend/0` |
| blue | Solid blue | the API plans and queues the job | 2 | `B2-C4` `/legend/1` |
| solid-6 | Solid green | the clip worker runs one job per clip | 2 | `B4-C5` `/legend/2` |
| dash-6 | Dashed green | writes the verdict store | 1 | `B5-C7` `/legend/3` |
| deny | Dashed red | blocked path | 1 | `B7-C8` `/legend/4` |

## View 2: System `C1-F8` `/body/0`

Subtitle: the pieces, and why each exists. Note: one request, from the console to a stored verdict.

- Row, left to right. `/body/0/children/0`
  - Col, top to bottom. `/body/0/children/0/children/0`
    - Row, left to right. `/body/0/children/0/children/0/children/0`
      - Item Operator console: web client; kind product, icon devops. `D1-D2` `/body/0/children/0/children/0/children/0/children/0`
  - Box Google Cloud; kind gcp. `C2-E8` `/body/0/children/0/children/1`
    - Col, top to bottom. `/body/0/children/0/children/1/children/0`
      - Row, left to right. `/body/0/children/0/children/1/children/0/children/0`
        - Row, left to right. `/body/0/children/0/children/1/children/0/children/0/children/0`
          - Item Gateway: API Gateway; kind product, icon networking, product API Gateway. `C4-D4` `/body/0/children/0/children/1/children/0/children/0/children/0/children/0`
        - Row, left to right. `/body/0/children/0/children/1/children/0/children/0/children/1`
          - Item Trigger API: Cloud Run; kind product, icon cloud-run, product Cloud Run. `C7-D7` `/body/0/children/0/children/1/children/0/children/0/children/1/children/0`
      - Row, left to right. `/body/0/children/0/children/1/children/0/children/1`
        - Row, left to right. `/body/0/children/0/children/1/children/0/children/1/children/0`
          - Item Evaluation queue: Pub/Sub; kind product, icon integration, product Pub/Sub. `D3-E4` `/body/0/children/0/children/1/children/0/children/1/children/0/children/0`
        - Row, left to right. `/body/0/children/0/children/1/children/0/children/1/children/1`
          - Item Clip worker: Cloud Run job; kind product, icon cloud-run, product Cloud Run. `D5-E6` `/body/0/children/0/children/1/children/0/children/1/children/1/children/0`
        - Row, left to right. `/body/0/children/0/children/1/children/0/children/1/children/2`
          - Item Clip evaluator: Vertex AI endpoint; kind product, icon vertex-ai, product Vertex AI. `D7-E8` `/body/0/children/0/children/1/children/0/children/1/children/2/children/0`
      - Row, left to right. `/body/0/children/0/children/1/children/0/children/2`
        - Item Verdict store: Cloud SQL; kind product, icon cloud-sql, product Cloud SQL. `E5-E6` `/body/0/children/0/children/1/children/0/children/2/children/0`
- Frame Verdict review in the console: clip, evidence frames, verdict and plan version: 72 px placeholder. `F1-F8` `/body/0/children/1`

### Lines in view 2

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| submit trigger | link | Operator console (console) | Gateway (gateway) | solid, tint 4; amber in center | arrow end | the operator's request | `D2` `/links/0` |
| authorized call | link | Gateway (gateway) | Trigger API (api) | solid, tint 1; blue in center | arrow end | the API plans and queues the job | `D5` `/links/1` |
| queue job | link | Trigger API (api) | Evaluation queue (queue) | solid, tint 1; blue in center | arrow end | the API plans and queues the job | `D5` `/links/2` |
| deliver job | link | Evaluation queue (queue) | Clip worker (worker) | solid, tint 6; green in center | arrow end | the clip worker runs one job per clip | `D4` `/links/3` |
| evaluate clip | link | Clip worker (worker) | Clip evaluator (evaluator) | solid, tint 6; green in center | arrow end | the clip worker runs one job per clip | `D6` `/links/4` |
| write verdict | link | Clip worker (worker) | Verdict store (verdict-store) | dash, tint 6; green in center | arrow end | writes the verdict store | `E5` `/links/5` |
| no direct write | link | Clip evaluator (evaluator) | Verdict store (verdict-store) | deny; red in center | arrow end | blocked path | `E7` `/links/6` |

Foot: Every verdict keeps the frames that support it and the plan version that produced it. `F1-F8` `/foot`
