# A university's admissions project reaches student records in two regions through one transit hub. `/title`

Customer · University enrolment · transit `/kicker`

The admissions VPC attaches to the transit hub as a spoke, and the hub routes it on to student records in europe-west4 and to certificates and migration in europe-north1. `/lede`

## Figure

Drawn elements: 17 (16 nodes, 1 link). Laid out at 1840 x 929.24 px, projection iso, grammar gcp; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Box Admissions project; kind project. `/body/0/children/0`
    - Col, top to bottom. `/body/0/children/0/children/0`
      - Item Admissions VPC: VPC network; kind product, icon networking, shape tile. `/body/0/children/0/children/0/children/0`
      - Item Admissions staff: offer review desk; kind person, shape figure. `/body/0/children/0/children/0/children/1`
  - Col, top to bottom. `/body/0/children/1`
    - Col, top to bottom. `/body/0/children/1/children/0`
      - Pipe VPC spoke · admissions VPC to the hub.
  - Box Google Cloud; kind gcp. `/body/0/children/2`
    - Row, left to right. `/body/0/children/2/children/0`
      - Col, top to bottom. `/body/0/children/2/children/0/children/0`
        - Item Transit hub: Network Connectivity Center; kind product, icon networking, product Network Connectivity Center, shape tile. `/body/0/children/2/children/0/children/0/children/0`
      - Box europe-west4; kind region, tint 1. `/body/0/children/2/children/0/children/1`
        - Col, top to bottom. `/body/0/children/2/children/0/children/1/children/0`
          - Item Student records: Google Cloud VMware Engine; kind product, icon compute, product Google Cloud VMware Engine, shape tower. `/body/0/children/2/children/0/children/1/children/0/children/0`
      - Box europe-north1; kind region, tint 2. `/body/0/children/2/children/0/children/2`
        - Col, top to bottom. `/body/0/children/2/children/0/children/2/children/0`
          - Item Degree certificates: Certificate Authority Service; kind product, icon security-identity, product Certificate Authority Service, shape block. `/body/0/children/2/children/0/children/2/children/0/children/0`
          - Item Records migration: Database Migration Service; kind product, icon databases, product Database Migration Service, shape cylinder. `/body/0/children/2/children/0/children/2/children/0/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| VPC spoke · admissions VPC to the hub | pipe h | Admissions VPC (edge) | Transit hub (ncc) | solid, tint 1; blue in center | arrow end | VPC spoke attachment | `/body/0/children/1/children/0/children/0` |
| unlabelled | link | Transit hub (ncc) | Records migration (dms) | gray; gray in center | arrow end | routed by the hub | `/links/0` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | VPC spoke attachment | 1 | `/legend/0` |
| gray | Solid gray | routed by the hub | 1 | `/legend/1` |
