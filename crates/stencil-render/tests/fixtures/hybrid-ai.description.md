# Radar and satellite observations upload to Google Cloud, where nowcast models are tuned and served inside one perimeter. `/title`

Customer · Weather forecasting platform `/kicker`

A radar network and a satellite ground station upload observations, and a partner forecasting agency shares its model output from its own project. Nowcast models are tuned in Vertex AI and served from two regions; the public internet appears once, as the prohibited path. `/lede`

## Figure

Drawn elements: 52 (52 nodes, 0 links). Canvas 1480 x 1193.2 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box Radar network · on-prem; kind onprem, tint 1. `/body/0/children/0/children/0`
      - Item Radar processors: volume scans every 5 minutes; kind product, icon compute. `/body/0/children/0/children/0/children/0`
        - Fact, doc: Scans leave over HTTPS only.
      - Item Scan archive: object storage · raw volumes; kind product, icon storage. `/body/0/children/0/children/0/children/1`
      - Item Staff identity: SAML IdP · group sync; kind product, icon security-identity. `/body/0/children/0/children/0/children/2`
    - Box Satellite ground station; kind onprem, tint 2. `/body/0/children/0/children/1`
      - Item Downlink store: imagery passes · events; kind product, icon storage. `/body/0/children/0/children/1/children/0`
      - Item Pass processor: calibrates · federated role; kind product, icon containers. `/body/0/children/0/children/1/children/1`
    - Box Partner agency · own project; kind project. `/body/0/children/0/children/2`
      - Item Ensemble output: gridded model runs; kind product, icon storage. `/body/0/children/0/children/2/children/0`
      - Item Warning exchange: severe weather alerts; kind product, icon integration. `/body/0/children/0/children/2/children/1`
  - Col, top to bottom. `/body/0/children/1`
    - Col, top to bottom. `/body/0/children/1/children/0`
      - Pipe HTTPS upload · signed URLs · scans.
      - Pipe Workforce identity · SAML · groups to IAM.
    - Col, top to bottom. `/body/0/children/1/children/1`
      - Pipe Pub/Sub push · pass events · to Region B.
      - Pipe Workload identity · federation · no keys.
    - Col, top to bottom. `/body/0/children/1/children/2`
      - Pipe Analytics Hub · shared dataset.
      - Pipe Public internet · no public IP · no keys.
  - Box Google Cloud; kind gcp. `/body/0/children/2`
    - Box VPC Service Controls perimeter · every model and observation store; kind perimeter. `/body/0/children/2/children/0`
      - Box Shared VPC · forecasting · one network, two regions; kind vpc. `/body/0/children/2/children/0/children/0`
        - Box Region A · nowcasts; kind region, tint 1. `/body/0/children/2/children/0/children/0/children/0`
          - Row, left to right. `/body/0/children/2/children/0/children/0/children/0/children/0`
            - Col, top to bottom. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0`
              - Item Nowcast inference: GKE · GPU node pool; kind product, icon gke, product GKE. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0/children/0`
                - Fact, doc: Workload Identity Federation.
            - Pipe serve · internal load balancer.
            - Col, top to bottom. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2`
              - Item Forecast API: External Application LB; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/0`
                - Fact, doc: Cloud Armor · IAP.
          - Fact, doc: Direct VPC egress for every serverless caller · Private Google Access on the subnet · restricted VIP from the Private Google Access doc. `/body/0/children/2/children/0/children/0/children/0/children/1`
        - Pipe failover · Region A ↔ Region B · models and checkpoints replicate.
        - Box Region B · replica and batch; kind region, tint 2. `/body/0/children/2/children/0/children/0/children/2`
          - Row, left to right. `/body/0/children/2/children/0/children/0/children/2/children/0`
            - Item Serving replica: GKE · GPU pool · warm; kind product, icon gke, product GKE. `/body/0/children/2/children/0/children/0/children/2/children/0/children/0`
      - Box Vertex AI · managed, reached over Private Service Connect; kind project. `/body/0/children/2/children/0/children/1`
        - Row, left to right. `/body/0/children/2/children/0/children/1/children/0`
          - Col, top to bottom. `/body/0/children/2/children/0/children/1/children/0/children/0`
            - Item Nowcast tuning: Vertex AI · tuning; kind product, icon vertex-ai, product Vertex AI. `/body/0/children/2/children/0/children/1/children/0/children/0/children/0`
              - Fact, doc: CMEK · private endpoint.
            - Item Hourly forecast runs: Cloud Run jobs; kind product, icon cloud-run, product Cloud Run. `/body/0/children/2/children/0/children/1/children/0/children/0/children/1`
          - Pipe register · tuned models.
          - Col, top to bottom. `/body/0/children/2/children/0/children/1/children/0/children/2`
            - Item Model registry: Vertex AI Registry; kind product, icon vertex-ai, product Vertex AI. `/body/0/children/2/children/0/children/1/children/0/children/2/children/0`
            - Item Forecast assistant: Vertex AI Agent Engine; kind product, icon agents, product Vertex AI Agent Engine. `/body/0/children/2/children/0/children/1/children/0/children/2/children/1`
      - Box Google APIs · Private Google Access · data the regions read and write; kind apis. `/body/0/children/2/children/0/children/2`
        - Row, left to right. `/body/0/children/2/children/0/children/2/children/0`
          - Item Observation store: BigQuery · scans and passes; kind product, icon bigquery, product BigQuery. `/body/0/children/2/children/0/children/2/children/0/children/0`
          - Item Model checkpoints: Cloud Storage · dual-region; kind product, icon cloud-storage, product Cloud Storage. `/body/0/children/2/children/0/children/2/children/0/children/1`
            - Fact, doc: CMEK · retention lock.
    - Box Evidence · common folder, outside the perimeter; kind project. `/body/0/children/2/children/1`
      - Box Google APIs · org-level; kind apis. `/body/0/children/2/children/1/children/0`
        - Row, left to right. `/body/0/children/2/children/1/children/0/children/0`
          - Item Audit logs: Cloud Logging · org sink; kind product, icon observability, product Cloud Logging. `/body/0/children/2/children/1/children/0/children/0/children/0`
          - Pipe findings.
          - Item Posture: Security Command Center; kind product, icon scc, product Security Command Center. `/body/0/children/2/children/1/children/0/children/0/children/2`
          - Pipe export.
          - Item Cost and usage: Billing export · BigQuery; kind product, icon bigquery, product BigQuery. `/body/0/children/2/children/1/children/0/children/0/children/4`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| HTTPS upload · signed URLs · scans | pipe h | Radar network · on-prem (left neighbour) | Google Cloud (right neighbour) | solid, tint 1; blue in center | no arrowheads | upload and sharing into Region A | `/body/0/children/1/children/0/children/0` |
| Workforce identity · SAML · groups to IAM | pipe h | Radar network · on-prem (left neighbour) | Google Cloud (right neighbour) | dash, tint 1; blue in center | no arrowheads | identity, control plane and failover | `/body/0/children/1/children/0/children/1` |
| Pub/Sub push · pass events · to Region B | pipe h | Satellite ground station (left neighbour) | Google Cloud (right neighbour) | solid, tint 2; pink in center | no arrowheads | event push into Region B | `/body/0/children/1/children/1/children/0` |
| Workload identity · federation · no keys | pipe h | Satellite ground station (left neighbour) | Google Cloud (right neighbour) | dash, tint 1; blue in center | no arrowheads | identity, control plane and failover | `/body/0/children/1/children/1/children/1` |
| Analytics Hub · shared dataset | pipe h | Partner agency · own project (left neighbour) | Google Cloud (right neighbour) | solid, tint 1; blue in center | no arrowheads | upload and sharing into Region A | `/body/0/children/1/children/2/children/0` |
| Public internet · no public IP · no keys | pipe h | Partner agency · own project (left neighbour) | Google Cloud (right neighbour) | deny; red in center | no arrowheads | prohibited path | `/body/0/children/1/children/2/children/1` |
| serve · internal load balancer | pipe h | Nowcast inference (left neighbour) | Forecast API (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/0/children/0/children/0/children/0/children/1` |
| failover · Region A ↔ Region B · models and checkpoints replicate | pipe v | Region A · nowcasts (sibling above) | Region B · replica and batch (sibling below) | dash, tint 1; blue in center | no arrowheads | identity, control plane and failover | `/body/0/children/2/children/0/children/0/children/1` |
| register · tuned models | pipe h | Nowcast tuning (left neighbour) | Forecast assistant (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/0/children/1/children/0/children/1` |
| findings | pipe h | Audit logs (left neighbour) | Posture (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/0/children/0/children/1` |
| export | pipe h | Posture (left neighbour) | Cost and usage (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/0/children/0/children/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | upload and sharing into Region A | 2 | `/legend/0` |
| pink | Solid pink | event push into Region B | 1 | `/legend/1` |
| dash | Dashed blue | identity, control plane and failover | 3 | `/legend/2` |
| gray | Solid gray | east-west inside Google Cloud | 4 | `/legend/3` |
| deny | Dashed red | prohibited path | 1 | `/legend/4` |

Foot: Illustrative topology · hop facts are placeholders, not looked-up values `/foot`
