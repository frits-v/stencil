# Orders reach PostgreSQL through Kafka, and sign-in stays with the identity provider `/title`

Internal · Self-hosted web stack · one Kubernetes cluster `/kicker`

NGINX routes branch traffic to the storefront, which publishes orders to Kafka for the worker to write to PostgreSQL. Sessions live in Redis, receipts in MinIO, and Argo CD deploys onto Kubernetes. `/lede`

## Figure

Drawn elements: 33 (27 nodes, 6 links). Canvas 1540 x 562.6 px, projection flat, grammar oss. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box Branch office; kind group, tint 1. `/body/0/children/0/children/0`
      - Item Branch staff: browser; kind person. `/body/0/children/0/children/0/children/0`
    - Box Corporate identity; kind group. `/body/0/children/0/children/1`
      - Item Identity provider: OpenID Connect; kind external. `/body/0/children/0/children/1/children/0`
  - Box Shop platform; kind system. `/body/0/children/1`
    - Box Production cluster; kind cluster. `/body/0/children/1/children/0`
      - Col, top to bottom. `/body/0/children/1/children/0/children/0`
        - Col, top to bottom. `/body/0/children/1/children/0/children/0/children/0`
          - Row, left to right. `/body/0/children/1/children/0/children/0/children/0/children/0`
            - Item Ingress: NGINX; kind network, icon nginx, product NGINX. `/body/0/children/1/children/0/children/0/children/0/children/0/children/0`
              - Fact, built: TLS terminated, port 443.
            - Pipe route.
            - Item Storefront: web front end; kind service. `/body/0/children/1/children/0/children/0/children/0/children/0/children/2`
            - Pipe publish.
            - Item Order events: Apache Kafka; kind queue, icon kafka, product Apache Kafka. `/body/0/children/1/children/0/children/0/children/0/children/0/children/4`
              - Fact, ask: partitions per topic.
            - Pipe consume.
            - Item Order worker: writes orders; kind service. `/body/0/children/1/children/0/children/0/children/0/children/0/children/6`
          - Row, left to right. `/body/0/children/1/children/0/children/0/children/0/children/1`
            - Item Sessions: Redis; kind store, icon redis, product Redis. `/body/0/children/1/children/0/children/0/children/0/children/1/children/0`
            - Item Catalog: MySQL; kind store, product MySQL. `/body/0/children/1/children/0/children/0/children/0/children/1/children/1`
            - Item Orders: PostgreSQL; kind store, icon postgresql, product PostgreSQL. `/body/0/children/1/children/0/children/0/children/0/children/1/children/2`
              - Fact, built: one primary, two replicas.
            - Item Receipts: MinIO; kind store, product MinIO. `/body/0/children/1/children/0/children/0/children/0/children/1/children/3`
        - Row, left to right. `/body/0/children/1/children/0/children/0/children/1`
          - Item Orchestrator: Kubernetes; kind service, icon kubernetes, product Kubernetes. `/body/0/children/1/children/0/children/0/children/1/children/0`
          - Item Delivery: Argo CD; kind service, icon argo, product Argo CD. `/body/0/children/1/children/0/children/0/children/1/children/1`
          - Item Metrics: Prometheus; kind service, icon prometheus, product Prometheus. `/body/0/children/1/children/0/children/0/children/1/children/2`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| route | pipe h | Ingress (left neighbour) | Storefront (right neighbour) | solid, tint 1; blue in center | no arrowheads | request path | `/body/0/children/1/children/0/children/0/children/0/children/0/children/1` |
| publish | pipe h | Storefront (left neighbour) | Order events (right neighbour) | solid, tint 2; pink in center | no arrowheads | order events | `/body/0/children/1/children/0/children/0/children/0/children/0/children/3` |
| consume | pipe h | Order events (left neighbour) | Order worker (right neighbour) | solid, tint 2; pink in center | no arrowheads | order events | `/body/0/children/1/children/0/children/0/children/0/children/0/children/5` |
| HTTPS | link | Branch staff (staff) | Ingress (ingress) | solid, tint 1; blue in center | arrow end | request path | `/links/0` |
| unlabelled | link | Ingress (ingress) | Identity provider (idp) | dash, tint 1; blue in center | arrow end | identity | `/links/1` |
| unlabelled | link | Storefront (storefront) | Sessions (sessions) | gray; gray in center | arrow end | uses | `/links/2` |
| unlabelled | link | Storefront (storefront) | Catalog (catalog) | gray; gray in center | arrow end | uses | `/links/3` |
| unlabelled | link | Order worker (worker) | Orders (orders) | solid, tint 2; pink in center | arrow end | order events | `/links/4` |
| unlabelled | link | Order worker (worker) | Receipts (receipts) | solid, tint 2; pink in center | arrow end | order events | `/links/5` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | request path | 2 | `/legend/0` |
| pink | Solid pink | order events | 4 | `/legend/1` |
| dash | Dashed blue | identity | 1 | `/legend/2` |
| gray | Solid gray | uses | 2 | `/legend/3` |
