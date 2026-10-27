# Reception and patients book through one booking API `/title`

Customer · Appointment booking · clinic `/kicker`

Reception staff book and move appointments from the admin console on a laptop. Patients book from the clinic's phone app, and both reach the same booking API. `/lede`

## Figure

Drawn elements: 17 (13 nodes, 4 links). Laid out at 1240 x 776.74 px, projection iso, grammar plain; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Box Clinic; kind system. `/body/0/children/0`
    - Box Front desk; kind group. `/body/0/children/0/children/0`
      - Row, left to right. `/body/0/children/0/children/0/children/0`
        - Item Receptionist: checks patients in; kind person, shape figure. `/body/0/children/0/children/0/children/0/children/0`
        - Item Admin console: laptop; kind device, shape laptop. `/body/0/children/0/children/0/children/0/children/1`
        - Item Booking API: appointments; kind service, shape block. `/body/0/children/0/children/0/children/0/children/2`
  - Col, top to bottom. `/body/0/children/1`
    - Box Patients; kind group. `/body/0/children/1/children/0`
      - Row, left to right. `/body/0/children/1/children/0/children/0`
        - Item Clinic app: phone; kind device, shape phone. `/body/0/children/1/children/0/children/0/children/0`
        - Item Patient: books a visit; kind person, shape figure. `/body/0/children/1/children/0/children/0/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| unlabelled | link | Receptionist (ops) | Admin console (laptop) | gray; gray in center | arrow end | uses | `/links/0` |
| unlabelled | link | Admin console (laptop) | Booking API (api) | solid, tint 1; blue in center | arrow end | booking request | `/links/1` |
| unlabelled | link | Patient (customer) | Clinic app (phone) | gray; gray in center | arrow end | uses | `/links/2` |
| unlabelled | link | Clinic app (phone) | Booking API (api) | solid, tint 1; blue in center | arrow end | booking request | `/links/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | booking request | 2 | `/legend/0` |
| gray | Solid gray | uses | 2 | `/legend/1` |
