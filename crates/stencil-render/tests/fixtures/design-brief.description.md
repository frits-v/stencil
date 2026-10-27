# Library holds, from the catalogue to the pickup shelf `/title`

Internal · Design one-pager · hold and pickup service `/kicker`

A patron places a hold in the catalogue app. The service keeps a line of holds for each title, sets a returned copy aside at the patron's branch, and tells the patron it is ready to collect. `/lede`

## Figure

Drawn elements: 29 (22 nodes, 7 links). Canvas 1480 x 1042.8 px, projection flat, grammar plain. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Text Problem. `/body/0/children/0/children/0`
      - Branch staff track holds on paper slips and by phone. A patron has to call or visit a branch to place one.
      - Patrons need to place a hold online, keep their place in line, and hear when a copy is ready to collect.
    - Text Goals. `/body/0/children/0/children/1`
      1. Accept a hold placed in the catalogue app and show the patron their place in line.
      2. Set each returned copy aside for the first confirmed hold on its title.
      3. Tell the patron by email or text when the copy is ready, with a collect-by date.
      4. Release a copy nobody collects in seven days to the next hold in line.
    - Text Non-goals. `/body/0/children/0/children/2`
      - Interlibrary loans: the service fills holds with copies this library owns.
      - Fines and fees.
      - Holds on e-books.
    - Text Interfaces. `/body/0/children/0/children/3`
      - Place hold: takes the card, the title and the branch, returns the hold.
      - Cancel hold: releases the hold and its place in line.
      - List holds: filters by card, title and branch.
      - Copy check-in event: carries the copy and the branch that received it.
    - Callout, risk: Risk: stale contact details. A notice sent to an old address or number leaves the copy unclaimed. A bounced notice flags the card for the desk, and the copy still expires after seven days. `/body/0/children/0/children/4`
    - Callout, decision: Decision: queue between API and worker. Pickup runs asynchronously. The API returns once the hold is queued, so a busy branch never holds a patron's request open. `/body/0/children/0/children/5`
  - Col, top to bottom. `/body/0/children/1`
    - Row, left to right. `/body/0/children/1/children/0`
      - Item Catalogue app: web and mobile; kind service. `/body/0/children/1/children/0/children/0`
    - Box Hold and pickup service; kind system. `/body/0/children/1/children/1`
      - Col, top to bottom. `/body/0/children/1/children/1/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/0`
          - Item Gateway: checks library cards; kind network. `/body/0/children/1/children/1/children/0/children/0/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/1`
          - Item Holds API: keeps the lines; kind service. `/body/0/children/1/children/1/children/0/children/1/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/2`
          - Item Pick queue: at-least-once delivery; kind queue. `/body/0/children/1/children/1/children/0/children/2/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/3`
          - Item Pickup worker: one job per hold; kind service. `/body/0/children/1/children/1/children/0/children/3/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/4`
          - Item Notifier: hosted messaging service; kind managed. `/body/0/children/1/children/1/children/0/children/4/children/0`
          - Item Holds store: relational database; kind store. `/body/0/children/1/children/1/children/0/children/4/children/1`
- Row, left to right. `/body/1`
  - Frame Place a hold: title, pickup branch, notice channel, confirm: 200 px placeholder. `/body/1/children/0`
  - Frame Hold shelf: title, library card, slot, collect-by date: 200 px placeholder. `/body/1/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| 1 place hold | link | Catalogue app (console) | Gateway (gateway) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/0` |
| 2 authorized call | link | Gateway (gateway) | Holds API (api) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/1` |
| 3 queue hold | link | Holds API (api) | Pick queue (queue) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/2` |
| 4 deliver job | link | Pick queue (queue) | Pickup worker (worker) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/3` |
| 5 send notice | link | Pickup worker (worker) | Notifier (evaluator) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/4` |
| 6 mark ready | link | Pickup worker (worker) | Holds store (database) | solid, tint 1; blue in center | arrow end | request path, numbered in order | `/links/5` |
| no direct write | link | Notifier (evaluator) | Holds store (database) | deny; red in center | stop plate end | blocked path | `/links/6` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | request path, numbered in order | 6 | `/legend/0` |
| deny | Dashed red | blocked path | 1 | `/legend/1` |

Foot: The numbered links follow one hold from the catalogue app to the holds store. `/foot`
