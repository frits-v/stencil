# A branch office reaches two regions over one Interconnect through the transit hub. `/title`

Customer · Landing zone · transit `/kicker`

One Dedicated Interconnect with two VLAN attachments lands on the transit hub, which routes the branch on to europe-north1. `/lede`

## Figure

Drawn elements: 17 (16 nodes, 1 link). Laid out at 1840 x 929.24 px, projection iso, grammar gcp; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Box Branch office; kind onprem. `/body/0/children/0`
    - Col, top to bottom. `/body/0/children/0/children/0`
      - Item Edge router: Cloud Router peer; kind product, icon networking, product Cloud Router, shape tile. `/body/0/children/0/children/0/children/0`
      - Item Branch users: SD-WAN users; kind person, shape figure. `/body/0/children/0/children/0/children/1`
  - Col, top to bottom. `/body/0/children/1`
    - Col, top to bottom. `/body/0/children/1/children/0`
      - Pipe Dedicated Interconnect · VLAN 1 and VLAN 2 attachments.
  - Box Google Cloud; kind gcp. `/body/0/children/2`
    - Row, left to right. `/body/0/children/2/children/0`
      - Col, top to bottom. `/body/0/children/2/children/0/children/0`
        - Item Transit hub: Network Connectivity Center; kind product, icon networking, product Network Connectivity Center, shape tile. `/body/0/children/2/children/0/children/0/children/0`
      - Box europe-west4; kind region, tint 1. `/body/0/children/2/children/0/children/1`
        - Col, top to bottom. `/body/0/children/2/children/0/children/1/children/0`
          - Item VMware cluster: Google Cloud VMware Engine; kind product, icon compute, product Google Cloud VMware Engine, shape tower. `/body/0/children/2/children/0/children/1/children/0/children/0`
      - Box europe-north1; kind region, tint 2. `/body/0/children/2/children/0/children/2`
        - Col, top to bottom. `/body/0/children/2/children/0/children/2/children/0`
          - Item Certificates: Certificate Authority Service; kind product, icon security-identity, product Certificate Authority Service, shape block. `/body/0/children/2/children/0/children/2/children/0/children/0`
          - Item Migration: Database Migration Service; kind product, icon databases, product Database Migration Service, shape cylinder. `/body/0/children/2/children/0/children/2/children/0/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| Dedicated Interconnect · VLAN 1 and VLAN 2 attachments | pipe h | Edge router (edge) | Transit hub (ncc) | solid, tint 1; blue in center | arrow end | Interconnect attachment | `/body/0/children/1/children/0/children/0` |
| unlabelled | link | Transit hub (ncc) | Migration (dms) | gray; gray in center | arrow end | east-west | `/links/0` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | Interconnect attachment | 1 | `/legend/0` |
| gray | Solid gray | east-west | 1 | `/legend/1` |
