# Staff and customers reach the API from their own devices. `/title`

Customer · Front door · cover figure `/kicker`

The operator runs the API from the shop; customers reach it from their phones. `/lede`

## Figure

Drawn elements: 17 (13 nodes, 4 links). Laid out at 1240 x 776.74 px, projection iso, grammar plain; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Box Shop; kind system. `/body/0/children/0`
    - Box Front door; kind group. `/body/0/children/0/children/0`
      - Row, left to right. `/body/0/children/0/children/0/children/0`
        - Item Operator: on call; kind person, shape figure. `/body/0/children/0/children/0/children/0/children/0`
        - Item Admin console: laptop; kind device, shape laptop. `/body/0/children/0/children/0/children/0/children/1`
        - Item API: gateway; kind service, shape block. `/body/0/children/0/children/0/children/0/children/2`
  - Col, top to bottom. `/body/0/children/1`
    - Box Customers; kind group. `/body/0/children/1/children/0`
      - Row, left to right. `/body/0/children/1/children/0/children/0`
        - Item Shop app: phone; kind device, shape phone. `/body/0/children/1/children/0/children/0/children/0`
        - Item Customer: shopping; kind person, shape figure. `/body/0/children/1/children/0/children/0/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| unlabelled | link | Operator (ops) | Admin console (laptop) | gray; gray in center | arrow end | uses | `/links/0` |
| unlabelled | link | Admin console (laptop) | API (api) | solid, tint 1; blue in center | arrow end | request path | `/links/1` |
| unlabelled | link | Customer (customer) | Shop app (phone) | gray; gray in center | arrow end | uses | `/links/2` |
| unlabelled | link | Shop app (phone) | API (api) | solid, tint 1; blue in center | arrow end | request path | `/links/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | request path | 2 | `/legend/0` |
| gray | Solid gray | uses | 2 | `/legend/1` |
