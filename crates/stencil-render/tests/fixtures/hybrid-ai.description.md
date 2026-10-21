# Training stays on-prem. Tuning, serving and evidence run on Google Cloud. AWS and Azure feed data over private paths only. `/title`

Customer · Hybrid multi-cloud AI platform `/kicker`

Three producers outside the frame, one Shared VPC across two regions, one service perimeter around every AI service. Every hop is a labeled line; the public internet is drawn once, as the prohibited path. `/lede`

## Figure

Drawn elements: 52 (52 nodes, 0 links). Canvas 1480 x 1177.6 px, projection flat, grammar gcp. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box On-prem · GPU data center; kind onprem, tint 1. `/body/0/children/0/children/0`
      - Item GPU training cluster: Kubernetes on bare metal; kind product, icon compute. `/body/0/children/0/children/0/children/0`
        - Fact, doc: Checkpoints leave over the Interconnect only.
      - Item Data lake: object storage · clips, telemetry; kind product, icon storage. `/body/0/children/0/children/0/children/1`
      - Item Corporate identity: SAML IdP · group sync; kind product, icon security-identity. `/body/0/children/0/children/0/children/2`
    - Box AWS · us-east-1; kind onprem, tint 2. `/body/0/children/0/children/1`
      - Item Telemetry landing: S3 bucket · event stream; kind product, icon storage. `/body/0/children/0/children/1/children/0`
      - Item Feature service: EKS · federated role, no keys; kind product, icon containers. `/body/0/children/0/children/1/children/1`
    - Box Azure · East US; kind project. `/body/0/children/0/children/2`
      - Item Document corpus: Blob Storage · contracts; kind product, icon storage. `/body/0/children/0/children/2/children/0`
      - Item Copilot integration: Azure OpenAI · retrieval calls; kind product, icon integration. `/body/0/children/0/children/2/children/1`
  - Col, top to bottom. `/body/0/children/1`
    - Col, top to bottom. `/body/0/children/1/children/0`
      - Pipe Interconnect · VLAN · BGP · checkpoints, clips.
      - Pipe Workforce identity · SAML · groups to IAM.
    - Col, top to bottom. `/body/0/children/1/children/1`
      - Pipe HA VPN · IPsec · BGP · S3 to Region B.
      - Pipe Workload identity · federation · no keys.
    - Col, top to bottom. `/body/0/children/1/children/2`
      - Pipe Cross-Cloud Interconnect · private path · corpus.
      - Pipe Public internet · no public IP · no keys.
  - Box Google Cloud; kind gcp. `/body/0/children/2`
    - Box VPC Service Controls perimeter · every AI and data service; kind perimeter. `/body/0/children/2/children/0`
      - Box Shared VPC · prod · one network, two regions; kind vpc. `/body/0/children/2/children/0/children/0`
        - Box Region A · tuning and serving; kind region, tint 1. `/body/0/children/2/children/0/children/0/children/0`
          - Row, left to right. `/body/0/children/2/children/0/children/0/children/0/children/0`
            - Col, top to bottom. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0`
              - Item Online inference: GKE · GPU node pool; kind product, icon gke, product GKE. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0/children/0`
                - Fact, doc: Workload Identity Federation.
            - Pipe serve · internal load balancer.
            - Col, top to bottom. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2`
              - Item Gateway: External Application LB; kind product, icon networking. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/0`
                - Fact, doc: Cloud Armor · IAP.
          - Fact, doc: Direct VPC egress for every serverless caller · Private Google Access on the subnet · restricted VIP from the Private Google Access doc. `/body/0/children/2/children/0/children/0/children/0/children/1`
        - Pipe failover · Region A ↔ Region B · checkpoints and adapters replicate.
        - Box Region B · replica and batch; kind region, tint 2. `/body/0/children/2/children/0/children/0/children/2`
          - Row, left to right. `/body/0/children/2/children/0/children/0/children/2/children/0`
            - Item Serving replica: GKE · GPU pool · warm; kind product, icon gke, product GKE. `/body/0/children/2/children/0/children/0/children/2/children/0/children/0`
      - Box Vertex AI · managed, reached over Private Service Connect; kind project. `/body/0/children/2/children/0/children/1`
        - Row, left to right. `/body/0/children/2/children/0/children/1/children/0`
          - Col, top to bottom. `/body/0/children/2/children/0/children/1/children/0/children/0`
            - Item Tuning: Vertex AI · tuning; kind product, icon vertex-ai, product Vertex AI. `/body/0/children/2/children/0/children/1/children/0/children/0/children/0`
              - Fact, doc: CMEK · private endpoint.
            - Item Batch inference: Cloud Run jobs; kind product, icon cloud-run, product Cloud Run. `/body/0/children/2/children/0/children/1/children/0/children/0/children/1`
          - Pipe register · tuned adapters.
          - Col, top to bottom. `/body/0/children/2/children/0/children/1/children/0/children/2`
            - Item Model registry: Vertex AI Registry; kind product, icon vertex-ai, product Vertex AI. `/body/0/children/2/children/0/children/1/children/0/children/2/children/0`
            - Item Agent runtime: Vertex AI Agent Engine; kind product, icon agents, product Vertex AI Agent Engine. `/body/0/children/2/children/0/children/1/children/0/children/2/children/1`
      - Box Google APIs · Private Google Access · data the regions read and write; kind apis. `/body/0/children/2/children/0/children/2`
        - Row, left to right. `/body/0/children/2/children/0/children/2/children/0`
          - Item Feature store: BigQuery · vectors; kind product, icon bigquery, product BigQuery. `/body/0/children/2/children/0/children/2/children/0/children/0`
          - Item Checkpoint bucket: Cloud Storage · dual-region; kind product, icon cloud-storage, product Cloud Storage. `/body/0/children/2/children/0/children/2/children/0/children/1`
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
| Interconnect · VLAN · BGP · checkpoints, clips | pipe h | On-prem · GPU data center (left neighbour) | Google Cloud (right neighbour) | solid, tint 1; blue in center | no arrowheads | private data path into Region A | `/body/0/children/1/children/0/children/0` |
| Workforce identity · SAML · groups to IAM | pipe h | On-prem · GPU data center (left neighbour) | Google Cloud (right neighbour) | dash, tint 1; blue in center | no arrowheads | identity, control plane and failover | `/body/0/children/1/children/0/children/1` |
| HA VPN · IPsec · BGP · S3 to Region B | pipe h | AWS · us-east-1 (left neighbour) | Google Cloud (right neighbour) | solid, tint 2; pink in center | no arrowheads | private data path into Region B | `/body/0/children/1/children/1/children/0` |
| Workload identity · federation · no keys | pipe h | AWS · us-east-1 (left neighbour) | Google Cloud (right neighbour) | dash, tint 1; blue in center | no arrowheads | identity, control plane and failover | `/body/0/children/1/children/1/children/1` |
| Cross-Cloud Interconnect · private path · corpus | pipe h | Azure · East US (left neighbour) | Google Cloud (right neighbour) | solid, tint 1; blue in center | no arrowheads | private data path into Region A | `/body/0/children/1/children/2/children/0` |
| Public internet · no public IP · no keys | pipe h | Azure · East US (left neighbour) | Google Cloud (right neighbour) | deny; red in center | no arrowheads | prohibited path | `/body/0/children/1/children/2/children/1` |
| serve · internal load balancer | pipe h | Online inference (left neighbour) | Gateway (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/0/children/0/children/0/children/0/children/1` |
| failover · Region A ↔ Region B · checkpoints and adapters replicate | pipe v | Region A · tuning and serving (sibling above) | Region B · replica and batch (sibling below) | dash, tint 1; blue in center | no arrowheads | identity, control plane and failover | `/body/0/children/2/children/0/children/0/children/1` |
| register · tuned adapters | pipe h | Tuning (left neighbour) | Agent runtime (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/0/children/1/children/0/children/1` |
| findings | pipe h | Audit logs (left neighbour) | Posture (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/0/children/0/children/1` |
| export | pipe h | Posture (left neighbour) | Cost and usage (right neighbour) | gray; gray in center | no arrowheads | east-west inside Google Cloud | `/body/0/children/2/children/1/children/0/children/0/children/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | private data path into Region A | 2 | `/legend/0` |
| pink | Solid pink | private data path into Region B | 1 | `/legend/1` |
| dash | Dashed blue | identity, control plane and failover | 3 | `/legend/2` |
| gray | Solid gray | east-west inside Google Cloud | 4 | `/legend/3` |
| deny | Dashed red | prohibited path | 1 | `/legend/4` |

Foot: Illustrative topology · hop facts are placeholders, not looked-up values `/foot`
