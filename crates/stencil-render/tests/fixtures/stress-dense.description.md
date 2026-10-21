# Fifteen Cloud Run services in five rows behind one service perimeter `/title`

Internal · Service perimeter · Cloud Run `/kicker`

Five rows of cards and pipes inside a service perimeter. `/lede`

## Figure

Drawn elements: 39 (39 nodes, 0 links). Canvas 1320 x 932.6 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Google Cloud; kind gcp. `/body/0`
  - Box Service perimeter; kind perimeter. `/body/0/children/0`
    - Col, top to bottom. `/body/0/children/0/children/0`
      - Row, left to right. `/body/0/children/0/children/0/children/0`
        - Item Service 1.1: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/0/children/0`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 1.1.
        - Item Service 1.2: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/0/children/2`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 1.2.
        - Item Service 1.3: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/0/children/4`
          - Fact, doc: Reads its config from a bucket in the same project.
      - Pipe failover.
      - Row, left to right. `/body/0/children/0/children/0/children/2`
        - Item Service 2.1: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/2/children/0`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 2.1.
        - Item Service 2.2: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/2/children/2`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 2.2.
        - Item Service 2.3: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/2/children/4`
          - Fact, doc: Reads its config from a bucket in the same project.
      - Row, left to right. `/body/0/children/0/children/0/children/3`
        - Item Service 3.1: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/3/children/0`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 3.1.
        - Item Service 3.2: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/3/children/2`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 3.2.
        - Item Service 3.3: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/3/children/4`
          - Fact, doc: Reads its config from a bucket in the same project.
      - Tee egress.
        - Pipe allowed.
        - Pipe reply.
      - Row, left to right. `/body/0/children/0/children/0/children/5`
        - Item Service 4.1: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/5/children/0`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 4.1.
        - Item Service 4.2: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/5/children/2`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 4.2.
        - Item Service 4.3: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/5/children/4`
          - Fact, doc: Reads its config from a bucket in the same project.
      - Row, left to right. `/body/0/children/0/children/0/children/6`
        - Item Service 5.1: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/6/children/0`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 5.1.
        - Item Service 5.2: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/6/children/2`
          - Fact, doc: Reads its config from a bucket in the same project.
        - Pipe step 5.2.
        - Item Service 5.3: Cloud Run; kind product, icon cloud-run, product Cloud Run. `/body/0/children/0/children/0/children/6/children/4`
          - Fact, doc: Reads its config from a bucket in the same project.

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| step 1.1 | pipe h | Service 1.1 (left neighbour) | Service 1.2 (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/0/children/1` |
| step 1.2 | pipe h | Service 1.2 (left neighbour) | Service 1.3 (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/0/children/3` |
| failover | pipe v | Service 1.2 (neighbour above) | Service 2.2 (neighbour below) | dash, tint 1; blue in center | no arrowheads | failover | `/body/0/children/0/children/0/children/1` |
| step 2.1 | pipe h | Service 2.1 (left neighbour) | Service 2.2 (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/2/children/1` |
| step 2.2 | pipe h | Service 2.2 (left neighbour) | Service 2.3 (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/2/children/3` |
| step 3.1 | pipe h | Service 3.1 (left neighbour) | Service 3.2 (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/3/children/1` |
| step 3.2 | pipe h | Service 3.2 (left neighbour) | Service 3.3 (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/3/children/3` |
| egress | tee spine | hub egress | allowed and reply | deny; red in center | no arrowheads | blocked egress | `/body/0/children/0/children/0/children/4` |
| allowed | tee arm | hub egress | open | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/4/arms/0` |
| reply | tee arm | hub egress | open | solid, tint 2; pink in center | no arrowheads | reply path | `/body/0/children/0/children/0/children/4/arms/1` |
| step 4.1 | pipe h | Service 4.1 (left neighbour) | Service 4.2 (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/5/children/1` |
| step 4.2 | pipe h | Service 4.2 (left neighbour) | Service 4.3 (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/5/children/3` |
| step 5.1 | pipe h | Service 5.1 (left neighbour) | Service 5.2 (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/0/children/0/children/6/children/1` |
| step 5.2 | pipe h | Service 5.2 (left neighbour) | Service 5.3 (right neighbour) | gray; gray in center | no arrowheads | internal call | `/body/0/children/0/children/0/children/6/children/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| gray | Solid gray | internal call | 5 | `/legend/0` |
| blue | Solid blue | request path | 6 | `/legend/1` |
| pink | Solid pink | reply path | 1 | `/legend/2` |
| dash | Dashed blue | failover | 1 | `/legend/3` |
| deny | Dashed red | blocked egress | 1 | `/legend/4` |
