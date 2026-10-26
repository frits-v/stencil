# A parcel carrier tracks every shipment through five stages behind one service perimeter `/title`

Internal · Parcel tracking · Cloud Run `/kicker`

Each parcel moves from intake through sorting, linehaul and last-mile delivery to returns, and the perimeter blocks egress from every stage. `/lede`

## Figure

Drawn elements: 39 (39 nodes, 0 links). Canvas 1320 x 932.6 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Google Cloud; kind gcp. `/body/0`
  - Box Service perimeter; kind perimeter. `/body/0/children/0`
    - Col, top to bottom. `/body/0/children/0/children/0`
      - Row, left to right. `/body/0/children/0/children/0/children/0`
        - Item Shipment booking: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/0/children/0`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe label.
        - Item Label printing: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/0/children/2`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe pickup.
        - Item Pickup scan: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/0/children/4`
          - Fact, doc: Publishes each parcel event to the tracking topic.
      - Pipe failover.
      - Row, left to right. `/body/0/children/0/children/0/children/2`
        - Item Hub arrival: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/2/children/0`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe arrival.
        - Item Sort planner: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/2/children/2`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe sort plan.
        - Item Chute assignment: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/2/children/4`
          - Fact, doc: Publishes each parcel event to the tracking topic.
      - Row, left to right. `/body/0/children/0/children/0/children/3`
        - Item Trailer loading: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/3/children/0`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe load list.
        - Item Route tracker: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/3/children/2`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe position.
        - Item Depot arrival: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/3/children/4`
          - Fact, doc: Publishes each parcel event to the tracking topic.
      - Tee egress.
        - Pipe allowed.
        - Pipe reply.
      - Row, left to right. `/body/0/children/0/children/0/children/5`
        - Item Courier manifest: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/5/children/0`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe manifest.
        - Item Delivery tracker: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/5/children/2`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe status.
        - Item Proof of delivery: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/5/children/4`
          - Fact, doc: Publishes each parcel event to the tracking topic.
      - Row, left to right. `/body/0/children/0/children/0/children/6`
        - Item Return request: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/6/children/0`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe return.
        - Item Return label: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/6/children/2`
          - Fact, doc: Publishes each parcel event to the tracking topic.
        - Pipe refund.
        - Item Refund trigger: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/6/children/4`
          - Fact, doc: Publishes each parcel event to the tracking topic.

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| label | pipe h | Shipment booking (left neighbour) | Label printing (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/0/children/1` |
| pickup | pipe h | Label printing (left neighbour) | Pickup scan (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/0/children/3` |
| failover | pipe v | Label printing (neighbour above) | Sort planner (neighbour below) | dash, tint 1; blue in center | no arrowheads | failover | `/body/0/children/0/children/0/children/1` |
| arrival | pipe h | Hub arrival (left neighbour) | Sort planner (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/2/children/1` |
| sort plan | pipe h | Sort planner (left neighbour) | Chute assignment (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/2/children/3` |
| load list | pipe h | Trailer loading (left neighbour) | Route tracker (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/3/children/1` |
| position | pipe h | Route tracker (left neighbour) | Depot arrival (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/3/children/3` |
| egress | tee spine | hub egress | allowed and reply | deny; red in center | no arrowheads | blocked egress | `/body/0/children/0/children/0/children/4` |
| allowed | tee arm | hub egress | open | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/4/arms/0` |
| reply | tee arm | hub egress | open | solid, tint 2; pink in center | no arrowheads | reply path | `/body/0/children/0/children/0/children/4/arms/1` |
| manifest | pipe h | Courier manifest (left neighbour) | Delivery tracker (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/5/children/1` |
| status | pipe h | Delivery tracker (left neighbour) | Proof of delivery (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/5/children/3` |
| return | pipe h | Return request (left neighbour) | Return label (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/6/children/1` |
| refund | pipe h | Return label (left neighbour) | Refund trigger (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/6/children/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| gray | Solid gray | internal call | 5 | `/legend/0` |
| blue | Solid blue | request path | 6 | `/legend/1` |
| pink | Solid pink | reply path | 1 | `/legend/2` |
| dash | Dashed blue | failover | 1 | `/legend/3` |
| deny | Dashed red | blocked egress | 1 | `/legend/4` |
