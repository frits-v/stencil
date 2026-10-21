# Plain-language event triggers, evaluated over video clips `/title`

Internal · Design one-pager · trigger evaluation service `/kicker`

An operator describes an event in their own words. The service turns the description into an evaluation plan, runs the plan over each new clip, and stores a verdict with the frames that support it. `/lede`

## Figure

Drawn elements: 29 (22 nodes, 7 links). Canvas 1480 x 1042.8 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Text Problem. `/body/0/children/0/children/0`
      - Operators review long stretches of video to find a few events. Rules written as code need an engineer for every change.
      - Operators need to describe an event in plain language and get a verdict per clip, with the frames that support it.
    - Text Goals. `/body/0/children/0/children/1`
      1. Accept a trigger written in plain language and show its evaluation plan before it runs.
      2. Evaluate every new clip against each active trigger.
      3. Store each verdict with its evidence frames and the plan version that produced it.
      4. Let an operator replay a trigger over earlier clips.
    - Text Non-goals. `/body/0/children/0/children/2`
      - Live stream analysis: the service works on finished clips.
      - Identifying people.
      - Training or tuning a model per customer.
    - Text Interfaces. `/body/0/children/0/children/3`
      - Create trigger: takes the description, returns the trigger and its plan.
      - Replay trigger: queues an evaluation over a range of stored clips.
      - List verdicts: filters by trigger, clip and capture time.
      - Clip arrival event: carries the clip location and capture time.
    - Callout, risk: Risk: plan drift. A model update can change how a description is read. Each verdict records the plan version, and a changed plan waits for operator approval before it runs. `/body/0/children/0/children/4`
    - Callout, decision: Decision: queue between API and worker. Evaluation runs asynchronously. The API returns once the plan is queued, so a slow clip never holds an operator request open. `/body/0/children/0/children/5`
  - Col, top to bottom. `/body/0/children/1`
    - Row, left to right. `/body/0/children/1/children/0`
      - Item Operator console: web client; kind product, icon devops. `/body/0/children/1/children/0/children/0`
    - Box Google Cloud; kind gcp. `/body/0/children/1/children/1`
      - Col, top to bottom. `/body/0/children/1/children/1/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/0`
          - Item Gateway: API Gateway; kind product, icon networking, product API Gateway. `/body/0/children/1/children/1/children/0/children/0/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/1`
          - Item Trigger API: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/1/children/1/children/0/children/1/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/2`
          - Item Evaluation queue: Pub/Sub; kind product, icon integration, product Pub/Sub. `/body/0/children/1/children/1/children/0/children/2/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/3`
          - Item Clip worker: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/1/children/1/children/0/children/3/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/4`
          - Item Clip evaluator: Vertex AI; kind product, icon vertex-ai, product Vertex AI. `/body/0/children/1/children/1/children/0/children/4/children/0`
          - Item Verdict store: Cloud SQL; kind product, icon cloud-sql, product Cloud SQL. `/body/0/children/1/children/1/children/0/children/4/children/1`
- Row, left to right. `/body/1`
  - Frame Trigger editor: description, generated plan, approve: 200 px placeholder. `/body/1/children/0`
  - Frame Verdict review: clip, evidence frames, verdict: 200 px placeholder. `/body/1/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| 1 submit trigger | link | Operator console (console) | Gateway (gateway) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/0` |
| 2 authorized call | link | Gateway (gateway) | Trigger API (api) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/1` |
| 3 queue plan | link | Trigger API (api) | Evaluation queue (queue) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/2` |
| 4 deliver job | link | Evaluation queue (queue) | Clip worker (worker) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/3` |
| 5 evaluate clip | link | Clip worker (worker) | Clip evaluator (evaluator) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/4` |
| 6 write verdict | link | Clip worker (worker) | Verdict store (database) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/5` |
| no direct write | link | Clip evaluator (evaluator) | Verdict store (database) | deny; red in center | arrow end | blocked path | `/links/6` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | request path, numbered in order | 6 | `/legend/0` |
| deny | Dashed red | blocked path | 1 | `/legend/1` |

Foot: The numbered links follow one request from the console to a stored verdict. `/foot`
