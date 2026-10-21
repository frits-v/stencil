# One web path from the load balancer to the data layer. `/title`

Customer · Data platform · cover figure `/kicker`

Requests enter at the load balancer, run through the web tier to the API, and land in the database, reached over private services access, and the object store. `/lede`

## Figure

Drawn elements: 17 (13 nodes, 4 links). Laid out at 1140 x 986.3 px, projection iso, grammar gcp; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Google Cloud; kind gcp. `/body/0`
  - Row, left to right. `/body/0/children/0`
    - Col, top to bottom. `/body/0/children/0/children/0`
      - Item Edge: Cloud Load Balancing; kind product, icon networking, product Cloud Load Balancing, shape tile. `/body/0/children/0/children/0/children/0`
    - Col, top to bottom. `/body/0/children/0/children/1`
      - Box Shared VPC · europe-west4; kind vpc. `/body/0/children/0/children/1/children/0`
        - Row, left to right. `/body/0/children/0/children/1/children/0/children/0`
          - Item Web tier: Compute Engine; kind product, icon compute-engine, product Compute Engine, shape tower. `/body/0/children/0/children/1/children/0/children/0/children/0`
      - Col, top to bottom. `/body/0/children/0/children/1/children/1`
        - Item Database: Cloud SQL; kind product, icon cloud-sql, product Cloud SQL, shape cylinder. `/body/0/children/0/children/1/children/1/children/0`
          - Fact, built: private services access.
    - Col, top to bottom. `/body/0/children/0/children/2`
      - Item API: Cloud Run; kind product, icon cloud-run, product Cloud Run, shape block. `/body/0/children/0/children/2/children/0`
    - Col, top to bottom. `/body/0/children/0/children/3`
      - Box Google APIs; kind apis. `/body/0/children/0/children/3/children/0`
        - Item Object store: Cloud Storage; kind product, icon cloud-storage, product Cloud Storage, shape stack. `/body/0/children/0/children/3/children/0/children/0`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| HTTPS | link | Edge (lb) | Web tier (web) | solid, tint 1; blue in center | arrow end | request path | `/links/0` |
| unlabelled | link | Web tier (web) | API (api) | solid, tint 1; blue in center | arrow end | request path | `/links/1` |
| unlabelled | link | API (api) | Database (db) | gray; gray in center | arrow end | service call | `/links/2` |
| unlabelled | link | API (api) | Object store (store) | gray; gray in center | arrow end | service call | `/links/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | request path | 2 | `/legend/0` |
| gray | Solid gray | service call | 2 | `/legend/1` |
