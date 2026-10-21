# One transit hub, one Shared VPC per environment, every path between them named. Prod and nonprod never peer. `/title`

Customer · Landing zone · hub and spoke · Dedicated Interconnect 99.99% `/kicker`

Two metros land four VLAN attachments on two Cloud Routers in the transit hub. Spokes attach through Network Connectivity Center. Tiers sit in their own subnets with no public IPs; ingress enters through one load balancer with Cloud Armor and IAP in front. `/lede`

## Figure

Drawn elements: 63 (63 nodes, 0 links). Canvas 1480 x 1355.8 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box On-prem · Metro 1; kind onprem, tint 1. `/body/0/children/0/children/0`
      - Item Edge router 1: port toward Google; kind product, icon hybrid. `/body/0/children/0/children/0/children/0`
      - Item Edge router 2: port toward Google; kind product, icon hybrid. `/body/0/children/0/children/0/children/1`
    - Box On-prem · Metro 2; kind onprem, tint 2. `/body/0/children/0/children/1`
      - Item Edge router 3: port toward Google; kind product, icon hybrid. `/body/0/children/0/children/1/children/0`
      - Item Edge router 4: port toward Google; kind product, icon hybrid. `/body/0/children/0/children/1/children/1`
    - Box Corporate network; kind project. `/body/0/children/0/children/2`
      - Item Perimeter firewall: on-prem · inspects both metros; kind product, icon security-identity. `/body/0/children/0/children/2/children/0`
      - Item Corporate DNS: authoritative for corp zones; kind product, icon networking. `/body/0/children/0/children/2/children/1`
      - Item Branch users: SD-WAN · no direct cloud path; kind product, icon compute. `/body/0/children/0/children/2/children/2`
  - Col, top to bottom. `/body/0/children/1`
    - Col, top to bottom. `/body/0/children/1/children/0`
      - Tee Dedicated Interconnect · Metro 1.
        - Pipe VLAN 1 · EAD 1 · BGP.
        - Pipe VLAN 2 · EAD 2 · BGP.
    - Col, top to bottom. `/body/0/children/1/children/1`
      - Tee Dedicated Interconnect · Metro 2.
        - Pipe VLAN 3 · EAD 1 · BGP.
        - Pipe VLAN 4 · EAD 2 · BGP.
    - Col, top to bottom. `/body/0/children/1/children/2`
      - Pipe HA VPN · backup path · IPsec · BGP.
      - Pipe DNS forwarding · corp zones · both directions.
  - Box Google Cloud; kind gcp. `/body/0/children/2`
    - Box Transit hub VPC · NCC hub · one network, two regions; kind vpc. `/body/0/children/2/children/0`
      - Row, left to right. `/body/0/children/2/children/0/children/0`
        - Box Region A; kind region, tint 1. `/body/0/children/2/children/0/children/0/children/0`
          - Item Cloud Router A: private ASN · RFC 6996; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/0/children/0`
            - Fact, doc: BGP timers from the Cloud Router doc.
          - Item Cloud NAT A: egress for private subnets; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/0/children/1`
          - Item HA VPN gateway A: backup to Metro 1; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/0/children/2`
        - Pipe failover · Region A ↔ Region B.
        - Box Region B; kind region, tint 2. `/body/0/children/2/children/0/children/0/children/2`
          - Item Cloud Router B: same private ASN as Region A; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/2/children/0`
          - Item Cloud NAT B: egress for private subnets; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/2/children/1`
          - Item HA VPN gateway B: backup to Metro 2; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/2/children/2`
      - Fact, doc: Hub advertises the Google VIP ranges named on the Private Google Access doc · Cloud DNS forwarding source range from the forwarding-zones doc. `/body/0/children/2/children/0/children/1`
    - Row, left to right. `/body/0/children/2/children/1`
      - Col, top to bottom. `/body/0/children/2/children/1/children/0`
        - Tee NCC spokes.
          - Pipe prod spoke · routes exchanged.
          - Pipe nonprod spoke · routes exchanged.
      - Col, top to bottom. `/body/0/children/2/children/1/children/1`
        - Box Shared VPC · prod · host project; kind project. `/body/0/children/2/children/1/children/1/children/0`
          - Row, left to right. `/body/0/children/2/children/1/children/1/children/0/children/0`
            - Box web subnet · Region A; kind subnet. `/body/0/children/2/children/1/children/1/children/0/children/0/children/0`
              - Item Ingress: External Application LB; kind product, icon networking. `/body/0/children/2/children/1/children/1/children/0/children/0/children/0/children/0`
                - Fact, doc: Cloud Armor · IAP.
              - Item Web tier: MIG · no public IP; kind product, icon compute-engine. `/body/0/children/2/children/1/children/1/children/0/children/0/children/0/children/1`
            - Pipe to app · internal LB.
            - Box app subnet · Region A; kind subnet. `/body/0/children/2/children/1/children/1/children/0/children/0/children/2`
              - Item App tier: Compute Engine MIG; kind product, icon compute-engine, product Compute Engine. `/body/0/children/2/children/1/children/1/children/0/children/0/children/2/children/0`
              - Item Async workers: Cloud Run · Direct VPC; kind product, icon cloud-run, product Cloud Run. `/body/0/children/2/children/1/children/1/children/0/children/0/children/2/children/1`
          - Pipe to data · private IP only · no route from web.
          - Box data subnet · Region A; kind subnet. `/body/0/children/2/children/1/children/1/children/0/children/2`
            - Row, left to right. `/body/0/children/2/children/1/children/1/children/0/children/2/children/0`
              - Item Primary database: Cloud SQL · private IP; kind product, icon cloud-sql, product Cloud SQL. `/body/0/children/2/children/1/children/1/children/0/children/2/children/0/children/0`
                - Fact, doc: CMEK · PSA path.
              - Item Cache: Memorystore · private; kind product, icon databases, product Memorystore. `/body/0/children/2/children/1/children/1/children/0/children/2/children/0/children/1`
        - Pipe prod ↔ nonprod · no peering · no shared routes.
        - Box Shared VPC · nonprod · host project; kind project. `/body/0/children/2/children/1/children/1/children/2`
          - Row, left to right. `/body/0/children/2/children/1/children/1/children/2/children/0`
            - Box GKE · private cluster · Region B; kind k8s. `/body/0/children/2/children/1/children/1/children/2/children/0/children/0`
              - Item Control plane: private endpoint only; kind product, icon gke. `/body/0/children/2/children/1/children/1/children/2/children/0/children/0/children/0`
              - Item Node pool: no external IPs · alias IPs; kind product, icon containers. `/body/0/children/2/children/1/children/1/children/2/children/0/children/0/children/1`
            - Pipe shared services · internal only.
            - Box Optional · shared services · Region B; kind optional. `/body/0/children/2/children/1/children/1/children/2/children/0/children/2`
              - Item Private DNS: Cloud DNS · private zones; kind product, icon networking, product Cloud DNS. `/body/0/children/2/children/1/children/1/children/2/children/0/children/2/children/0`
              - Item Flow logs and audit: Cloud Logging · org sink; kind product, icon observability, product Cloud Logging. `/body/0/children/2/children/1/children/1/children/2/children/0/children/2/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| Dedicated Interconnect · Metro 1 | tee spine | hub Dedicated Interconnect · Metro 1 | VLAN 1 and VLAN 2 | solid, tint 1; blue in center | no arrowheads | Metro 1 ↔ Region A | `/body/0/children/1/children/0/children/0` |
| VLAN 1 · EAD 1 · BGP | tee arm | hub Dedicated Interconnect · Metro 1 | Google Cloud (right neighbour) | solid, tint 1; blue in center | no arrowheads | Metro 1 ↔ Region A | `/body/0/children/1/children/0/children/0/arms/0` |
| VLAN 2 · EAD 2 · BGP | tee arm | hub Dedicated Interconnect · Metro 1 | Google Cloud (right neighbour) | solid, tint 1; blue in center | no arrowheads | Metro 1 ↔ Region A | `/body/0/children/1/children/0/children/0/arms/1` |
| Dedicated Interconnect · Metro 2 | tee spine | hub Dedicated Interconnect · Metro 2 | VLAN 3 and VLAN 4 | solid, tint 2; pink in center | no arrowheads | Metro 2 ↔ Region B | `/body/0/children/1/children/1/children/0` |
| VLAN 3 · EAD 1 · BGP | tee arm | hub Dedicated Interconnect · Metro 2 | Google Cloud (right neighbour) | solid, tint 2; pink in center | no arrowheads | Metro 2 ↔ Region B | `/body/0/children/1/children/1/children/0/arms/0` |
| VLAN 4 · EAD 2 · BGP | tee arm | hub Dedicated Interconnect · Metro 2 | Google Cloud (right neighbour) | solid, tint 2; pink in center | no arrowheads | Metro 2 ↔ Region B | `/body/0/children/1/children/1/children/0/arms/1` |
| HA VPN · backup path · IPsec · BGP | pipe h | Corporate network (left neighbour) | Google Cloud (right neighbour) | dash, tint 1; blue in center | no arrowheads | backup path, DNS forwarding and region failover | `/body/0/children/1/children/2/children/0` |
| DNS forwarding · corp zones · both directions | pipe h | Corporate network (left neighbour) | Google Cloud (right neighbour) | dash, tint 1; blue in center | no arrowheads | backup path, DNS forwarding and region failover | `/body/0/children/1/children/2/children/1` |
| failover · Region A ↔ Region B | pipe h | Region A (left neighbour) | Region B (right neighbour) | dash, tint 1; blue in center | no arrowheads | backup path, DNS forwarding and region failover | `/body/0/children/2/children/0/children/0/children/1` |
| NCC spokes | tee spine | hub NCC spokes | prod spoke and nonprod spoke | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/0/children/0` |
| prod spoke · routes exchanged | tee arm | hub NCC spokes | Shared VPC · prod · host project (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/0/children/0/arms/0` |
| nonprod spoke · routes exchanged | tee arm | hub NCC spokes | Shared VPC · prod · host project (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/0/children/0/arms/1` |
| to app · internal LB | pipe h | web subnet · Region A (left neighbour) | app subnet · Region A (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/1/children/0/children/0/children/1` |
| to data · private IP only · no route from web | pipe v | to app (sibling above) | data subnet · Region A (sibling below) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/1/children/0/children/1` |
| prod ↔ nonprod · no peering · no shared routes | pipe v | Shared VPC · prod · host project (neighbour above) | Shared VPC · nonprod · host project (neighbour below) | deny; red in center | no arrowheads | prohibited path | `/body/0/children/2/children/1/children/1/children/1` |
| shared services · internal only | pipe h | GKE · private cluster · Region B (left neighbour) | Optional · shared services · Region B (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/1/children/2/children/0/children/1` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | Metro 1 ↔ Region A | 3 | `/legend/0` |
| pink | Solid pink | Metro 2 ↔ Region B | 3 | `/legend/1` |
| dash | Dashed blue | backup path, DNS forwarding and region failover | 3 | `/legend/2` |
| gray | Solid gray | east-west inside Google Cloud | 6 | `/legend/3` |
| deny | Dashed red | prohibited path | 1 | `/legend/4` |

Foot: Illustrative topology · hop facts are placeholders, not looked-up values `/foot`
