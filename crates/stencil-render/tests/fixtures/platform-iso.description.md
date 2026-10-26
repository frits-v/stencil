# Match requests pass the edge proxy and the lobby tier, then the match API reads players and writes events. `/title`

Internal · Multiplayer game backend `/kicker`

Players connect through the edge proxy to the lobby web tier, which hands each match request to the match API. The API reads player records from the database and writes every match event to a stream. `/lede`

## Figure

Drawn elements: 17 (13 nodes, 4 links). Laid out at 1140 x 1001.9 px, projection iso, grammar plain; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Game backend; kind system. `/body/0`
  - Row, left to right. `/body/0/children/0`
    - Col, top to bottom. `/body/0/children/0/children/0`
      - Item Edge proxy: ends TLS; kind network, shape tile. `/body/0/children/0/children/0/children/0`
        - Fact, built: Envoy.
    - Col, top to bottom. `/body/0/children/0/children/1`
      - Box Lobby zone; kind boundary. `/body/0/children/0/children/1/children/0`
        - Row, left to right. `/body/0/children/0/children/1/children/0/children/0`
          - Item Lobby web tier: matchmaking; kind service, shape tower. `/body/0/children/0/children/1/children/0/children/0/children/0`
      - Col, top to bottom. `/body/0/children/0/children/1/children/1`
        - Item Player database: profiles and ratings; kind store, shape cylinder. `/body/0/children/0/children/1/children/1/children/0`
          - Fact, built: PostgreSQL · primary, replica.
    - Col, top to bottom. `/body/0/children/0/children/2`
      - Item Match API: match state; kind service, shape block. `/body/0/children/0/children/2/children/0`
    - Col, top to bottom. `/body/0/children/0/children/3`
      - Box Event pipeline; kind group. `/body/0/children/0/children/3/children/0`
        - Item Match events: one stream per match; kind queue, shape stack. `/body/0/children/0/children/3/children/0/children/0`
          - Fact, built: NATS JetStream.

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| HTTPS | link | Edge proxy (lb) | Lobby web tier (web) | solid, tint 1; blue in center | arrow end | player request path | `/links/0` |
| unlabelled | link | Lobby web tier (web) | Match API (api) | solid, tint 1; blue in center | arrow end | player request path | `/links/1` |
| unlabelled | link | Match API (api) | Player database (db) | gray; gray in center | arrow end | reads and writes | `/links/2` |
| unlabelled | link | Match API (api) | Match events (store) | gray; gray in center | arrow end | reads and writes | `/links/3` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | player request path | 2 | `/legend/0` |
| gray | Solid gray | reads and writes | 2 | `/legend/1` |
