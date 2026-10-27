# An assembly plant sends line-camera frames to defect inspection over two private paths. `/title`

Customer · Vehicle plant · line inspection `/kicker`

Line-camera frames cross a primary and a failover attachment into the plant VPC, which hands them to the inspection API, the defect model and the quality warehouse. `/lede`

## Figure

Drawn elements: 22 (16 nodes, 6 links). Laid out at 1640 x 728.74 px, projection iso, grammar gcp; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box Plant floor; kind onprem, tint 1. `/body/0/children/0/children/0`
      - Item Plant router: plant OT · BGP; kind product, icon hybrid, shape tile. `/body/0/children/0/children/0/children/0`
  - Box Google Cloud; kind gcp. `/body/0/children/1`
    - Row, left to right. `/body/0/children/1/children/0`
      - Box Plant VPC · europe-west4; kind vpc. `/body/0/children/1/children/0/children/0`
        - Row, left to right. `/body/0/children/1/children/0/children/0/children/0`
          - Item Attachments: Cloud Router; kind product, icon networking, product Cloud Router, shape tile. `/body/0/children/1/children/0/children/0/children/0/children/0`
          - Item Service endpoint: Private Service Connect; kind product, icon networking, product Private Service Connect, shape tile. `/body/0/children/1/children/0/children/0/children/0/children/1`
      - Col, top to bottom. `/body/0/children/1/children/0/children/1`
        - Item Inspection API: Cloud Run; kind product, icon cloud-run, product Cloud Run, shape block. `/body/0/children/1/children/0/children/1/children/0`
      - Col, top to bottom. `/body/0/children/1/children/0/children/2`
        - Item Defect model: Vertex AI; kind product, icon vertex-ai, product Vertex AI, shape block. `/body/0/children/1/children/0/children/2/children/0`
      - Box Google APIs; kind apis. `/body/0/children/1/children/0/children/3`
        - Item Quality warehouse: BigQuery; kind product, icon bigquery, product BigQuery, shape cylinder. `/body/0/children/1/children/0/children/3/children/0`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| VLAN 1 | link | Plant router (router) | Attachments (attachments) | solid, tint 1; blue in center | arrow end | primary attachment | `/links/0` |
| VLAN 2 | link | Plant router (router) | Attachments (attachments) | dash, tint 1; blue in center | arrow end | failover attachment | `/links/1` |
| unlabelled | link | Attachments (attachments) | Service endpoint (psc) | gray; gray in center | arrow end | service call | `/links/2` |
| unlabelled | link | Service endpoint (psc) | Inspection API (gateway) | gray; gray in center | arrow end | service call | `/links/3` |
| unlabelled | link | Inspection API (gateway) | Defect model (model) | gray; gray in center | arrow end | service call | `/links/4` |
| unlabelled | link | Defect model (model) | Quality warehouse (warehouse) | gray; gray in center | arrow end | service call | `/links/5` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | primary attachment | 1 | `/legend/0` |
| dash | Dashed blue | failover attachment | 1 | `/legend/1` |
| gray | Solid gray | service call | 4 | `/legend/2` |
