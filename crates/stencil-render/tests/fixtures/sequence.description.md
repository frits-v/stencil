# A batch job from submission to completion `/title`

Internal · Request flow · job service `/kicker`

An operator submits a job in the console. The API queues it and answers at once; a worker pulls the job, reports progress and acknowledges it once the result is stored. A cancel that arrives while the job runs is refused. `/lede`

## Figure

Drawn elements: 23 (13 nodes, 10 links). Canvas 1320 x 718 px, projection flat, grammar plain. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Job service; kind system. `/body/0`
  - Lanes, heads left to right. `/body/0/children/0`
    - Item Operator: submits and cancels jobs; kind person. `/body/0/children/0/children/0`
    - Item Console: web front end; kind service. `/body/0/children/0/children/1`
    - Item Jobs API: owns job state; kind service. `/body/0/children/0/children/2`
    - Item Queue: at-least-once delivery; kind queue. `/body/0/children/0/children/3`
    - Item Worker: runs one job at a time; kind service. `/body/0/children/0/children/4`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| submit job | message, order 1 | Operator (operator) | Console (console) | solid, tint 1; blue in center | arrow end | call | `/links/0` |
| POST /jobs | message, order 2 | Console (console) | Jobs API (api) | solid, tint 1; blue in center | arrow end | call | `/links/1` |
| enqueue · job id as the key | message, order 3 | Jobs API (api) | Queue (queue) | solid, tint 1; blue in center | arrow end | call | `/links/2` |
| 202 accepted | message, order 4 | Jobs API (api) | Console (console) | dash, tint 1; blue in center | arrow end | reply | `/links/3` |
| pull | message, order 5 | Worker (worker) | Queue (queue) | solid, tint 1; blue in center | arrow end | call | `/links/4` |
| deliver job | message, order 6 | Queue (queue) | Worker (worker) | solid, tint 1; blue in center | arrow end | call | `/links/5` |
| report running | message, order 7 | Worker (worker) | Jobs API (api) | solid, tint 1; blue in center | arrow end | call | `/links/6` |
| cancel job | message, order 8 | Operator (operator) | Console (console) | solid, tint 1; blue in center | arrow end | call | `/links/7` |
| DELETE /jobs/{id} · 409: job is running | message, order 9 | Console (console) | Jobs API (api) | deny; red in center | stop plate end | rejected call | `/links/8` |
| ack · after the result is stored | message, order 10 | Worker (worker) | Queue (queue) | solid, tint 1; blue in center | arrow end | call | `/links/9` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | call | 8 | `/legend/0` |
| dash | Dashed blue | reply | 1 | `/legend/1` |
| deny | Dashed red | rejected call | 1 | `/legend/2` |
