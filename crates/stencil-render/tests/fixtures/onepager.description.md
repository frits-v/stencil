# Library holds, from the catalogue to the pickup shelf `A1-A2` `/title`

Internal · Library · hold and pickup `A1-A2` `/kicker`

A patron places a hold in the catalogue app. The service keeps a line of holds for each title, sets a returned copy aside at the patron's branch, and tells the patron it is ready to collect. `A1-A2` `/lede`

## Sheet

Drawn elements: 141 (63 nodes, 18 links, 24 entries, 36 marks). Canvas 1600 x 2577.5 px, projection flat, grammar plain. Line styles name a tint slot by number; the colour word is the one the center theme draws it in. References: a reference such as B3 or B2-C5 names the grid cells under an element's top-left and bottom-right corners, columns 1 to 8 left to right, rows A to F top to bottom. The reference before a pointer is where that element is drawn; a Zones cell lists the cells of every balloon, flag or diamond keyed to its entry.

## Title block (view 1)

Wordmark holds. `A1-A8` `/sheet`

### Problem · holds kept on paper `A2-A5` `/sheet/problem`

Branch staff track holds on paper slips and by phone.

| Point | Text | Marks | Pointer |
|---|---|---|---|
| P1 | A patron must call or visit a branch to place a hold. | balloon 1 Catalogue app `A4` `/sheet/problem/points/0/marks/0`; balloon 3 Holds API `A4` `/sheet/problem/points/0/marks/1` | `A2-A4` `/sheet/problem/points/0` |
| P2 | Staff phone each patron when a held copy comes in. | balloon 7 Notifier `A4` `/sheet/problem/points/1/marks/0` | `A2-A4` `/sheet/problem/points/1` |
| P3 | A lost slip takes the patron's place in line with it. | balloon 8 Holds store `A4` `/sheet/problem/points/2/marks/0`; flag 2 A hold stores a reference to the copy record, never a copy of it. `A4` `/sheet/problem/points/2/marks/1` | `A2-A4` `/sheet/problem/points/2` |
| P4 | A hold with no branch or an old phone number strands the copy. | diamond 3 Contact details. `A4` `/sheet/problem/points/3/marks/0`; flag 1 A hold reaches the pick queue only once its patron has confirmed the branch. `A4` `/sheet/problem/points/3/marks/1` | `A2-A4` `/sheet/problem/points/3` |

### Answer · holds placed online `A5-A7` `/sheet/answer`

The patron places the hold in the app; the service keeps the line, sets the copy aside and sends the notice.

One line per title. Each hold takes a place in its title's line. A returned copy goes to the first confirmed hold, and the app shows each patron their place.

Queued work. The API returns once a hold is queued, and a busy branch never holds a patron's request open.

### The patron's part, and only this `A7-A8` `/sheet/attention`

| Mark | Item | Actor | Zones | Pointer |
|---|---|---|---|---|
| diamond 1 | Pickup branch. The patron confirms the branch before a copy is set aside. | Patron | B4 | `A7-A8` `/sheet/attention/items/0` |
| diamond 2 | Collection. A ready copy waits seven days on the hold shelf at that branch. | Patron | B8 C2 | `A7-A8` `/sheet/attention/items/1` |
| diamond 3 | Contact details. Notices go to the email or phone on the card, so the patron keeps them current. | Patron | A4 B8 | `A7-A8` `/sheet/attention/items/2` |

## Who acts

| Actor | Id | Colour | Text | Pointer |
|---|---|---|---|---|
| Patron | operator | tint 4; amber in center | places and collects holds | `A1-A2` `/actors/0` |
| Holds API | api | tint 1; blue in center | keeps lines, queues jobs | `A2-A4` `/actors/1` |
| Pickup worker | worker | tint 6; green in center | one job per hold | `A4-A5` `/actors/2` |
| Notifier | model | tint 5; violet in center | sends ready notices | `A5-A6` `/actors/3` |

## Legend `A1-A8` `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| solid-by-actor | Solid, actor colour | work moves, in the colour of the actor that sends it | 13 | `A1-A3` `/legend/0` |
| dash-by-actor | Dashed, actor colour | reads or writes the holds store | 2 | `A3-A5` `/legend/1` |
| solid-by-actor-rail | Solid, actor colour, rail | one topic that every job passes through | 1 | `A5-A7` `/legend/2` |
| solid-by-actor-planned | Hollow, actor colour | planned, not built yet | 1 | `A1-A2` `/legend/3` |
| gray | Solid gray | event from the library system | 1 | `A2-A4` `/legend/4` |
| deny | Dashed red | blocked path | 1 | `A4` `/legend/5` |

Mark kinds: a piece of the service (balloon, parts list); asked of a patron (flag, general notes); the patron's part (diamond, attention items); a numbered request step (step badge, numbered lines).

## View 2: Workflow `A1-C8` `/body/0`

Subtitle: one hold, from placing it to pickup. Note: a lane for each actor; time runs left to right.

- Phases: lanes Patron, Holds API, Pickup worker, Notifier; phases 1 Place, 2 Wait in line, 3 Set aside and notify, 4 Collect; step fill cell; detail A frames One job per hold, Send the ready notice, Mark the hold ready. `A1-C8` `/body/0/children/0`
  - Step Place a hold. Lane Patron, phase 1 Place, column 1. `B2` `/body/0/children/0/children/0`
    - Mark balloon 1 Catalogue app `B2` `/body/0/children/0/children/0/marks/0`
    - Line from the catalogue app.
    - Line pickup branch.
  - Step no copy is free yet. Lane Holds API, phase 1 Place, column 1, look text. `B2` `/body/0/children/0/children/1`
  - Step Join the title's line. Lane Holds API, phase 2 Wait in line, column 1. `B2-B3` `/body/0/children/0/children/2`
    - Mark balloon 3 Holds API `B3` `/body/0/children/0/children/2/marks/0`
    - Line one line per title.
    - Line place 3 of 5.
  - Step Confirm the branch. Lane Patron, phase 2 Wait in line, column 2. `B3-B4` `/body/0/children/0/children/3`
    - Mark diamond 1 Pickup branch. `B4` `/body/0/children/0/children/3/marks/0`
    - Mark flag 1 A hold reaches the pick queue only once its patron has confirmed the branch. `B4` `/body/0/children/0/children/3/marks/1`
    - Line or suspend while away.
  - Step Store the hold. Lane Holds API, phase 2 Wait in line, column 2, look chip, fill tint. `B3-B4` `/body/0/children/0/children/4`
    - Mark balloon 8 Holds store `B4` `/body/0/children/0/children/4/marks/0`
  - Step Copy checked in. Lane Pickup worker, phase 3 Set aside and notify, column 1. `B4-B5` `/body/0/children/0/children/5`
    - Mark balloon 5 Copy catalogue `B5` `/body/0/children/0/children/5/marks/0`
    - Line the check-in event names a returned copy.
      - Mark flag 4 Out of scope: interlibrary loans, fines, and holds on e-books. `B5` `/body/0/children/0/children/5/lines/0/marks/0`
  - Step One job per hold. Lane Pickup worker, phase 3 Set aside and notify, column 2, look stack. `B5-B6` `/body/0/children/0/children/6`
    - Mark balloon 6 Pickup worker `B6` `/body/0/children/0/children/6/marks/0`
    - Mark balloon 4 Pick queue `B6` `/body/0/children/0/children/6/marks/1`
    - Line set the copy aside.
    - Line call the notifier.
  - Step Send the ready notice. Lane Notifier, phase 3 Set aside and notify, column 2 to 3. `B5-C7` `/body/0/children/0/children/7`
    - Mark balloon 7 Notifier `B7` `/body/0/children/0/children/7/marks/0`
    - Mark flag 3 The notifier holds no credentials for the holds store. `B7` `/body/0/children/0/children/7/marks/1`
    - Line by email or text message.
  - Step Mark the hold ready. Lane Pickup worker, phase 3 Set aside and notify, column 3. `B6-B7` `/body/0/children/0/children/8`
    - Mark balloon 8 Holds store `B7` `/body/0/children/0/children/8/marks/0`
    - Mark flag 2 A hold stores a reference to the copy record, never a copy of it. `B7` `/body/0/children/0/children/8/marks/1`
    - Line shelf slot, collect-by date.
  - Step Collect the copy. Lane Patron, phase 4 Collect, column 1. `B7-B8` `/body/0/children/0/children/9`
    - Mark diamond 2 Collection. `B8` `/body/0/children/0/children/9/marks/0`
    - Line at the confirmed branch.
  - Step Expire uncollected holds. Lane Holds API, phase 4 Collect, column 1, look planned, fill cell. `B7-B8` `/body/0/children/0/children/10`
    - Mark balloon 9 Expiry scheduler `B8` `/body/0/children/0/children/10/marks/0`
  - Step A bounced notice flags the card for the desk. Lane Notifier, phase 4 Collect, column 1, look note, fill cell. `B7-C8` `/body/0/children/0/children/11`
    - Mark diamond 3 Contact details. `B8` `/body/0/children/0/children/11/marks/0`
- Ladder, detail A: When a pick fails or a hold goes unclaimed; a failed pick never blocks the queue; rung fill none. Frames One job per hold, Send the ready notice, Mark the hold ready. `C1-C8` `/body/0/children/1`
  - Rung 1 (bottom) Retry, the pickup worker; actor Pickup worker: A failed copy lookup or notifier call retries the hold job with backoff, three attempts in all. `C1-C8` `/body/0/children/1/rungs/0`
  - Rung 2 Dead letter, the Holds API; actor Holds API: After the third failure the job moves to a dead-letter subscription and the desk lists the hold as stuck. `C1-C8` `/body/0/children/1/rungs/1`
  - Rung 3 Ask the patron, the patron; actor Patron: An unclaimed copy sends a reminder on day five; the patron can extend the hold once or cancel it. `C1-C8` `/body/0/children/1/rungs/2`
    - Mark diamond 2 Collection. `C2` `/body/0/children/1/rungs/2/marks/0`
  - Rung 4 (top) Page the owner, the service owner; planned: A branch with more than ten stuck holds in an hour pages the service owner. `C1-C8` `/body/0/children/1/rungs/3`

| Lane | 1 Place `A1-A2` `/body/0/children/0/phases/0` | 2 Wait in line `A2` `/body/0/children/0/phases/1` | 3 Set aside and notify `A4` `/body/0/children/0/phases/2` | 4 Collect `A7` `/body/0/children/0/phases/3` |
|---|---|---|---|---|
| Patron `A1-B8` `/body/0/children/0/lanes/0` | Place a hold | Confirm the branch | none | Collect the copy |
| Holds API `B1-B8` `/body/0/children/0/lanes/1` | no copy is free yet | Join the title's line; Store the hold | none | Expire uncollected holds |
| Pickup worker `B1-B8` `/body/0/children/0/lanes/2` | none | none | Copy checked in; One job per hold; Mark the hold ready | none |
| Notifier `B1-C8` `/body/0/children/0/lanes/3` | none | none | Send the ready notice | A bounced notice flags the card for the desk |

### Lines in view 2

| Line | Kind | From | To | Style | Ends | Means | Actor | Marks | Pointer |
|---|---|---|---|---|---|---|---|---|---|
| unlabelled | link | Place a hold (s-describe) | Join the title's line (s-compile) | solid, tint 4, Patron; amber in center | arrow end | work moves, in the colour of the actor that sends it | Patron | none | `B2` `/links/0` |
| in line | link | Join the title's line (s-compile) | Confirm the branch (s-approve) | solid, tint 1, Holds API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Holds API | none | `B3` `/links/1` |
| unlabelled | link | Confirm the branch (s-approve) | Store the hold (s-store-plan) | solid, tint 4, Patron; amber in center | arrow end | work moves, in the colour of the actor that sends it | Patron | none | `B4` `/links/2` |
| reads confirmed holds | link | Store the hold (s-store-plan) | One job per hold (s-evaluate) | dash, tint 6, Pickup worker; green in center | arrow end | reads or writes the holds store | Pickup worker | none | `B5` `/links/3` |
| unlabelled | link | Copy checked in (s-arrive) | One job per hold (s-evaluate) | solid, tint 6, Pickup worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Pickup worker | none | `B5` `/links/4` |
| unlabelled | link | One job per hold (s-evaluate) | Send the ready notice (s-judge) | solid, tint 6, Pickup worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Pickup worker | none | `B6` `/links/5` |
| unlabelled | link | Send the ready notice (s-judge) | Mark the hold ready (s-write) | solid, tint 5, Notifier; violet in center | arrow end | work moves, in the colour of the actor that sends it | Notifier | none | `B7` `/links/6` |
| unlabelled | link | Mark the hold ready (s-write) | Collect the copy (s-review) | solid, tint 6, Pickup worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Pickup worker | none | `B7` `/links/7` |

## View 3: System `C1-E8` `/body/1`

Subtitle: the pieces, and why each exists. Note: balloons key the drawing to the parts list.

- Row, left to right. `/body/1/children/0`
  - Col, top to bottom. `/body/1/children/0/children/0`
    - Row, left to right. `/body/1/children/0/children/0/children/0`
      - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/0`
        - Row, left to right. `/body/1/children/0/children/0/children/0/children/0/children/0`
          - Item Patron: one library card; kind person, outline person, actor Patron. `D1` `/body/1/children/0/children/0/children/0/children/0/children/0/children/0`
        - Row, left to right. `/body/1/children/0/children/0/children/0/children/0/children/1`
          - Item Catalogue app: web and mobile; kind service, actor Patron. `D1` `/body/1/children/0/children/0/children/0/children/0/children/1/children/0`
            - Mark balloon 1 Catalogue app `D1` `/body/1/children/0/children/1/children/0/parts/0/node`
      - Box Hold and pickup service; kind system. `C2-E6` `/body/1/children/0/children/0/children/0/children/1`
        - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0`
          - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/0`
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/0`
              - Item Gateway: checks library cards; kind network, actor Holds API. `C3-D3` `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/0/children/0`
                - Mark balloon 2 Gateway `C3` `/body/1/children/0/children/1/children/0/parts/1/node`
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/1`
              - Item Holds API: keeps the lines; kind service, actor Holds API. `C5-D5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/0/children/1/children/0`
                - Mark balloon 3 Holds API `C5` `/body/1/children/0/children/1/children/0/parts/2/node`
          - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1`
            - Pipe Pick queue · at-least-once delivery.
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1`
              - Box Pickup path · one job per hold; kind assembly, tint 6, actor Pickup worker. `D4-D5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0`
                - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0`
                  - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/0`
                    - Item Pickup worker: one job per hold; kind service, outline stack, actor Pickup worker. `D4-D5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/0/children/0`
                      - Mark balloon 6 Pickup worker `D5` `/body/1/children/0/children/1/children/0/parts/5/node`
                  - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/1`
                    - Item Notifier: hosted messaging service; kind managed, actor Notifier. `D4-D5` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/1/children/0/children/0/children/1/children/0`
                      - Mark balloon 7 Notifier `D5` `/body/1/children/0/children/1/children/0/parts/6/node`
          - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2`
            - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/0`
              - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/0/children/0`
                - Item Copy catalogue: circulation records; kind store. `D2-E3` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/0/children/0/children/0`
                  - Mark balloon 5 Copy catalogue `D3` `/body/1/children/0/children/1/children/0/parts/4/node`
            - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/1`
              - Item Holds store: relational database; kind store, outline store, actor Pickup worker. `D3-E4` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/1/children/0`
                - Mark balloon 8 Holds store `D4` `/body/1/children/0/children/1/children/0/parts/7/node`
                - Compartment Holds.
                - Compartment Title lines.
                - Compartment Notices.
                - Compartment Copy refs.
                  - Mark flag 2 A hold stores a reference to the copy record, never a copy of it. `E4` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/1/children/0/compartments/3/marks/0`
            - Col, top to bottom. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/2`
              - Row, left to right. `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/2/children/0`
                - Item Expiry scheduler: hosted scheduler; kind managed, actor Holds API, planned. `D5-E6` `/body/1/children/0/children/0/children/0/children/1/children/0/children/2/children/2/children/0/children/0`
                  - Mark balloon 9 Expiry scheduler `D6` `/body/1/children/0/children/1/children/0/parts/8/node`
    - General notes: what the service asks of its patrons. `E1-E6` `/body/1/children/0/children/0/children/1`
      - Note 1 (approved-plans-only): A hold reaches the pick queue only once its patron has confirmed the branch. The worker never sets a copy aside for an unconfirmed hold. Zones A4 B4 D4. `E1-E6` `/body/1/children/0/children/0/children/1/notes/0`
      - Note 2 (evidence-by-reference): A hold stores a reference to the copy record, never a copy of it. Copy details follow the catalogue. Zones A4 B7 E4. `E1-E6` `/body/1/children/0/children/0/children/1/notes/1`
      - Note 3 (no-store-credentials): The notifier holds no credentials for the holds store. The blocked line in view 3 is a database grant, not a convention. Zones B7 D5. `E1-E6` `/body/1/children/0/children/0/children/1/notes/2`
      - Note 4 (out-of-scope): Out of scope: interlibrary loans, fines, and holds on e-books. The service fills holds with copies this library owns. Zones B5. `E1-E6` `/body/1/children/0/children/0/children/1/notes/3`
      - Foot: Each flag on the drawings marks where its note applies.
  - Col, top to bottom. `/body/1/children/0/children/1`
    - Parts list: what the service is made of, and why each part exists. `C6-E8` `/body/1/children/0/children/1/children/0`
    - Frame Hold shelf at the desk: title, library card, slot and collect-by date: 240 px placeholder. `E6-E8` `/body/1/children/0/children/1/children/1`

### Parts list

| Part | Name | What | Why | Actor | Zones | Pointer |
|---|---|---|---|---|---|---|
| 1 | Catalogue app | web and mobile | Patrons place and track holds without a call to the branch. | Patron | A4 B2 D1 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/0` |
| 2 | Gateway | checks library cards | One signed-in entry point; the API never sees a call without a card. | Holds API | C3 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/1` |
| 3 | Holds API | keeps the lines | Keeps one line per title and refuses to queue a hold nobody has confirmed. | Holds API | A4 B3 C5 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/2` |
| 4 | Pick queue | at-least-once delivery | The API returns once a job is queued, so a busy branch never holds a request open. | Holds API | B6 D3 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/3` |
| 5 | Copy catalogue | circulation records | A hold is filled only from a copy on the shelf, and the check-in event names it. | none | B5 D3 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/4` |
| 6 | Pickup worker | one job per hold | Holds are independent, so a failed pick retries alone. | Pickup worker | B6 D5 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/5` |
| 7 | Notifier | hosted messaging service | Sends the ready notice, and has no write path to the store. | Notifier | A4 B7 D5 | `D6-D8` `/body/1/children/0/children/1/children/0/parts/6` |
| 8 | Holds store | relational database | Every hold keeps its place in line, its branch and the notices sent for it. | Pickup worker | A4 B4 B7 D4 | `E6-E8` `/body/1/children/0/children/1/children/0/parts/7` |
| 9 | Expiry scheduler, planned | hosted scheduler | Releases a copy left on the shelf past its date, so the next patron in line gets it. | Holds API | B8 D6 | `E6-E8` `/body/1/children/0/children/1/children/0/parts/8` |

### Lines in view 3

| Line | Kind | From | To | Style | Ends | Means | Actor | Marks | Pointer |
|---|---|---|---|---|---|---|---|---|---|
| Pick queue · at-least-once delivery | pipe v, rail | rail; in from Holds API (api); in from Copy catalogue (bucket) | rail; out to Pickup worker (worker) | solid, tint 1, Holds API; blue in center | no arrowheads | one topic that every job passes through | Holds API | balloon 4 Pick queue `D3` `/body/1/children/0/children/1/children/0/parts/3/node` | `D2-D3` `/body/1/children/0/children/0/children/0/children/1/children/0/children/1/children/0` |
| places, tracks | link | Patron (operator-person) | Catalogue app (console) | solid, tint 4, Patron; amber in center | arrow end | work moves, in the colour of the actor that sends it | Patron | none | `D1` `/links/8` |
| step 1: place hold | link | Catalogue app (console) | Gateway (gateway) | solid, tint 4, Patron; amber in center | arrow end | work moves, in the colour of the actor that sends it | Patron | none | `C2` `/links/9` |
| step 2: authorized call | link | Gateway (gateway) | Holds API (api) | solid, tint 1, Holds API; blue in center | arrow end | work moves, in the colour of the actor that sends it | Holds API | none | `C4` `/links/10` |
| step 3: queue job · confirmed holds only | link | Holds API (api) | Pick queue (queue) | solid, tint 1, Holds API; blue in center | dot end, valve before the end | work moves, in the colour of the actor that sends it | Holds API | flag 1 A hold reaches the pick queue only once its patron has confirmed the branch. `D4` `/links/11/marks/0` | `D4` `/links/11` |
| step 4: deliver job | link | Pick queue (queue) | Pickup worker (worker) | solid, tint 1, Holds API; blue in center | dot start, arrow end | work moves, in the colour of the actor that sends it | Holds API | none | `D3` `/links/12` |
| step 5: send notice | link | Pickup worker (worker) | Notifier (evaluator) | solid, tint 6, Pickup worker; green in center | arrow end | work moves, in the colour of the actor that sends it | Pickup worker | none | `D5` `/links/13` |
| step 6: mark ready | link | Pickup worker (worker) | Holds store (verdict-store) | dash, tint 6, Pickup worker; green in center | arrow end | reads or writes the holds store | Pickup worker | none | `D4` `/links/14` |
| schedule expiry | link | Holds API (api) | Expiry scheduler (replay) | solid, tint 1, Holds API, planned; blue in center | arrow end | planned, not built yet | Holds API | none | `D5` `/links/15` |
| copy returned | link | Copy catalogue (bucket) | Pick queue (queue) | gray; gray in center | dot end | event from the library system | none | none | `D2` `/links/16` |
| no direct write | link | Notifier (evaluator) | Holds store (verdict-store) | deny; red in center | stop plate end | blocked path | none | flag 3 The notifier holds no credentials for the holds store. `D5` `/links/17/marks/0` | `D4` `/links/17` |

## View 4: How-tos `F1-F8` `/body/2`

Subtitle: what desk staff type. Note: real output, trimmed.

- How-to column. `/body/2/children/0`

### How-to 4.1: Place a hold `F1-F4` `/body/2/children/0/items/0`

- Command hold place.
- Words then.
- Command hold confirm.

Place the hold for the patron, check the branch it names, then confirm it. No copy is set aside before that.

```text
$ hold place --card 2104-8831 --branch east "Rivers of the North"
  hold h-5120 placed, 3 of 5 in line
  title: Rivers of the North; branch: East; notice: text message
# the hold waits for confirmation
$ hold confirm h-5120
✓ h-5120 confirmed for East branch
```

### How-to 4.2: Extend a hold `F5-F8` `/body/2/children/0/items/1`

- Command hold extend.

Adds seven days to a ready hold; the patron gets the new collect-by date by text.

```text
$ hold extend h-5120 --days 7
  h-5120 collect-by date moved to 2026-11-09
✓ notice sent by text message
```

### How-to 4.3: Cancel a hold `F1-F4` `/body/2/children/0/items/2`

- Command hold cancel.

Releases the hold and its place in line. A copy already set aside goes to the next hold.

```text
$ hold cancel h-5141 --reason "bought a copy"
  hold h-5141 cancelled, 4 holds left in line
# the next hold in line moves up one place
```

### How-to 4.4: List holds `F5-F8` `/body/2/children/0/items/3`

- Command hold list.

Text beside the terminal.

Filters by card, title and branch. A ready hold stays listed until it is collected or expires.

```text
$ hold list --card 2104-8831
  3 holds on card 2104-8831
    h-5120 ready at East, collect by 2026-11-02
    h-5133 in line, 2 of 4
    h-5141 waiting for a free copy
```

Foot: Balloons key the drawings to the parts list, flags to the general notes, diamonds to the attention items in the title block. `F1-F8` `/foot`
