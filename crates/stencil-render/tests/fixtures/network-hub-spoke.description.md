# Production and test reach the shared hub on their own spokes and never route to each other. `/title`

Internal · Airline check-in and boarding platform · hub and spoke `/kicker`

Kiosks, counters and the mobile app reach production through one ingress tier, which hands bookings to the check-in service and gate lists to the boarding workers. The shared-services hub runs in two regions and fails over between them; the test environment attaches as a second spoke with masked passenger data. `/lede`

## Figure

Drawn elements: 42 (42 nodes, 0 links). Canvas 1480 x 1357.6 px, projection flat, grammar plain. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Check-in and boarding platform; kind system. `/body/0`
  - Box Shared services hub · one network, two regions; kind boundary. `/body/0/children/0`
    - Row, left to right. `/body/0/children/0/children/0`
      - Box Region A; kind group, tint 1. `/body/0/children/0/children/0/children/0`
        - Item Hub router A: routes between the spokes; kind network. `/body/0/children/0/children/0/children/0/children/0`
          - Fact, built: FRRouting · private ASN.
        - Item Egress gateway A: to airports and partners; kind network. `/body/0/children/0/children/0/children/0/children/1`
        - Item Identity provider A: agent and crew sign-in; kind service. `/body/0/children/0/children/0/children/0/children/2`
          - Fact, built: Keycloak.
      - Pipe failover · Region A ↔ Region B.
      - Box Region B; kind group, tint 2. `/body/0/children/0/children/0/children/2`
        - Item Hub router B: same ASN as Region A; kind network. `/body/0/children/0/children/0/children/2/children/0`
        - Item Egress gateway B: to airports and partners; kind network. `/body/0/children/0/children/0/children/2/children/1`
        - Item Identity provider B: standby, synced from A; kind service. `/body/0/children/0/children/0/children/2/children/2`
    - Fact, doc: Both regions advertise one service range to each spoke · the reservation system is reached through the egress gateways only. `/body/0/children/0/children/1`
  - Row, left to right. `/body/0/children/1`
    - Col, top to bottom. `/body/0/children/1/children/0`
      - Tee Spokes.
        - Pipe production spoke · routes exchanged.
        - Pipe test spoke · routes exchanged.
    - Col, top to bottom. `/body/0/children/1/children/1`
      - Box Production · check-in and boarding; kind tile. `/body/0/children/1/children/1/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/0`
          - Box web subnet · Region A; kind group. `/body/0/children/1/children/1/children/0/children/0/children/0`
            - Item Ingress: kiosks, counters and app; kind network. `/body/0/children/1/children/1/children/0/children/0/children/0/children/0`
              - Fact, built: Envoy · rate limits.
            - Item Check-in web: counter and kiosk screens; kind service. `/body/0/children/1/children/1/children/0/children/0/children/0/children/1`
          - Pipe to app · internal load balancer.
          - Box app subnet · Region A; kind group. `/body/0/children/1/children/1/children/0/children/0/children/2`
            - Item Check-in service: seats, bags and passes; kind service. `/body/0/children/1/children/1/children/0/children/0/children/2/children/0`
            - Item Boarding workers: gate lists per departure; kind service. `/body/0/children/1/children/1/children/0/children/0/children/2/children/1`
        - Pipe to data · private addresses only · no route from web.
        - Box data subnet · Region A; kind group. `/body/0/children/1/children/1/children/0/children/2`
          - Row, left to right. `/body/0/children/1/children/1/children/0/children/2/children/0`
            - Item Passenger database: check-in and boarding state; kind store. `/body/0/children/1/children/1/children/0/children/2/children/0/children/0`
              - Fact, built: PostgreSQL · encrypted.
            - Item Seat map cache: per departing flight; kind store. `/body/0/children/1/children/1/children/0/children/2/children/0/children/1`
              - Fact, built: Redis.
      - Pipe production ↔ test · no peering · no shared routes.
      - Box Test · the next release; kind tile. `/body/0/children/1/children/1/children/2`
        - Row, left to right. `/body/0/children/1/children/1/children/2/children/0`
          - Box Cluster · Region B; kind group. `/body/0/children/1/children/1/children/2/children/0/children/0`
            - Item Check-in service: release candidate; kind service. `/body/0/children/1/children/1/children/2/children/0/children/0/children/0`
              - Fact, built: Kubernetes.
            - Item Boarding workers: release candidate; kind service. `/body/0/children/1/children/1/children/2/children/0/children/0/children/1`
          - Pipe shared services · internal only.
          - Box Test data · Region B; kind tile. `/body/0/children/1/children/1/children/2/children/0/children/2`
            - Item Masked passengers: nightly copy, names scrambled; kind store. `/body/0/children/1/children/1/children/2/children/0/children/2/children/0`
            - Item Departure replay: a recorded day of flights; kind service. `/body/0/children/1/children/1/children/2/children/0/children/2/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| failover · Region A ↔ Region B | pipe h | Region A (left neighbour) | Region B (right neighbour) | dash, tint 1; blue in center | no arrowheads | region failover | `/body/0/children/0/children/0/children/1` |
| Spokes | tee spine | hub Spokes | production spoke and test spoke | gray; gray in center | no arrowheads | calls and routes inside the platform | `/body/0/children/1/children/0/children/0` |
| production spoke · routes exchanged | tee arm | hub Spokes | Production · check-in and boarding (right neighbour) | gray; gray in center | no arrowheads | calls and routes inside the platform | `/body/0/children/1/children/0/children/0/arms/0` |
| test spoke · routes exchanged | tee arm | hub Spokes | Production · check-in and boarding (right neighbour) | gray; gray in center | no arrowheads | calls and routes inside the platform | `/body/0/children/1/children/0/children/0/arms/1` |
| to app · internal load balancer | pipe h | web subnet · Region A (left neighbour) | app subnet · Region A (right neighbour) | gray; gray in center | no arrowheads | calls and routes inside the platform | `/body/0/children/1/children/1/children/0/children/0/children/1` |
| to data · private addresses only · no route from web | pipe v | to app (sibling above) | data subnet · Region A (sibling below) | gray; gray in center | no arrowheads | calls and routes inside the platform | `/body/0/children/1/children/1/children/0/children/1` |
| production ↔ test · no peering · no shared routes | pipe v | Production · check-in and boarding (neighbour above) | Test · the next release (neighbour below) | deny; red in center | no arrowheads | prohibited path | `/body/0/children/1/children/1/children/1` |
| shared services · internal only | pipe h | Cluster · Region B (left neighbour) | Test data · Region B (right neighbour) | gray; gray in center | no arrowheads | calls and routes inside the platform | `/body/0/children/1/children/1/children/2/children/0/children/1` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| dash | Dashed blue | region failover | 1 | `/legend/0` |
| gray | Solid gray | calls and routes inside the platform | 6 | `/legend/1` |
| deny | Dashed red | prohibited path | 1 | `/legend/2` |
