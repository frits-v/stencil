# Orders reach PostgreSQL through Kafka, and sign-in stays with the identity provider `/title`

Internal · Self-hosted web stack · one Kubernetes cluster `/kicker`

NGINX routes branch traffic to the storefront, which publishes orders to Kafka for the worker to write to PostgreSQL. Sessions live in Redis, backups in MinIO, and Argo CD deploys onto Kubernetes. `/lede`

## Figure

Drawn elements: 31 (23 nodes, 8 links). Laid out at 1740 x 1040.24 px, projection iso, grammar oss; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box Corporate identity; kind group. `/body/0/children/0/children/0`
      - Item Identity provider: OpenID Connect; kind external, shape block. `/body/0/children/0/children/0/children/0`
    - Box Branch office; kind group, tint 1. `/body/0/children/0/children/1`
      - Item Branch staff: browser; kind person, shape figure. `/body/0/children/0/children/1/children/0`
  - Box Shop platform; kind system. `/body/0/children/1`
    - Box Production cluster; kind cluster. `/body/0/children/1/children/0`
      - Col, top to bottom. `/body/0/children/1/children/0/children/0`
        - Row, left to right. `/body/0/children/1/children/0/children/0/children/0`
          - Col, top to bottom. `/body/0/children/1/children/0/children/0/children/0/children/0`
            - Item Ingress: NGINX; kind network, icon nginx, product NGINX, shape tile. `/body/0/children/1/children/0/children/0/children/0/children/0/children/0`
          - Col, top to bottom. `/body/0/children/1/children/0/children/0/children/0/children/1`
            - Item Storefront: web front end; kind service, shape block. `/body/0/children/1/children/0/children/0/children/0/children/1/children/0`
          - Col, top to bottom. `/body/0/children/1/children/0/children/0/children/0/children/2`
            - Item Order events: Apache Kafka; kind queue, icon kafka, product Apache Kafka, shape stack. `/body/0/children/1/children/0/children/0/children/0/children/2/children/0`
          - Col, top to bottom. `/body/0/children/1/children/0/children/0/children/0/children/3`
            - Item Order worker: writes orders; kind service, shape block. `/body/0/children/1/children/0/children/0/children/0/children/3/children/0`
          - Col, top to bottom. `/body/0/children/1/children/0/children/0/children/0/children/4`
            - Item Orders: PostgreSQL; kind store, icon postgresql, product PostgreSQL, shape cylinder. `/body/0/children/1/children/0/children/0/children/0/children/4/children/0`
          - Col, top to bottom. `/body/0/children/1/children/0/children/0/children/0/children/5`
            - Item Backups: MinIO; kind store, icon minio, product MinIO, shape stack. `/body/0/children/1/children/0/children/0/children/0/children/5/children/0`
        - Row, left to right. `/body/0/children/1/children/0/children/0/children/1`
          - Item Orchestrator: Kubernetes; kind service, icon kubernetes, product Kubernetes, shape tower. `/body/0/children/1/children/0/children/0/children/1/children/0`
          - Item Sessions: Redis; kind store, icon redis, product Redis, shape cylinder. `/body/0/children/1/children/0/children/0/children/1/children/1`
          - Item Delivery: Argo CD; kind service, icon argo, product Argo CD, shape block. `/body/0/children/1/children/0/children/0/children/1/children/2`
          - Item Metrics: Prometheus; kind service, icon prometheus, product Prometheus, shape block. `/body/0/children/1/children/0/children/0/children/1/children/3`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| HTTPS | link | Branch staff (staff) | Ingress (ingress) | solid, tint 1; blue in center | arrow end | request path | `/links/0` |
| unlabelled | link | Ingress (ingress) | Identity provider (idp) | dash, tint 1; blue in center | arrow end | identity | `/links/1` |
| route | link | Ingress (ingress) | Storefront (storefront) | solid, tint 1; blue in center | arrow end | request path | `/links/2` |
| publish | link | Storefront (storefront) | Order events (events) | solid, tint 2; pink in center | arrow end | order events | `/links/3` |
| consume | link | Order events (events) | Order worker (worker) | solid, tint 2; pink in center | arrow end | order events | `/links/4` |
| unlabelled | link | Storefront (storefront) | Sessions (sessions) | gray; gray in center | arrow end | uses | `/links/5` |
| writes | link | Order worker (worker) | Orders (orders) | solid, tint 2; pink in center | arrow end | order events | `/links/6` |
| unlabelled | link | Orders (orders) | Backups (backups) | gray; gray in center | arrow end | uses | `/links/7` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | request path | 2 | `/legend/0` |
| pink | Solid pink | order events | 3 | `/legend/1` |
| dash | Dashed blue | identity | 1 | `/legend/2` |
| gray | Solid gray | uses | 2 | `/legend/3` |
