# Two firewall zones and four VLANs at the head office `/title`

Internal · On-prem network · head office site `/kicker`

The core switch trunks the staff and server VLANs to the edge firewall, which routes them to the DMZ and the transit VLAN. Traffic that starts in the DMZ never reaches the inside zone. `/lede`

## Figure

Drawn elements: 29 (29 nodes, 0 links). Canvas 1320 x 570.6 px, projection flat, grammar plain. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Head office site; kind system. `/body/0`
  - Row, left to right. `/body/0/children/0`
    - Box Inside zone; kind boundary. `/body/0/children/0/children/0`
      - Item Core switch: Layer 3, routes between VLANs; kind network. `/body/0/children/0/children/0/children/0`
        - Fact, built: sw-core-01.
      - Box VLAN 10 · staff; kind group, tint 1. `/body/0/children/0/children/0/children/1`
        - Row, left to right. `/body/0/children/0/children/0/children/1/children/0`
          - Item Print server; kind service. `/body/0/children/0/children/0/children/1/children/0/children/0`
            - Fact, built: prn-01.
          - Item Directory server; kind service. `/body/0/children/0/children/0/children/1/children/0/children/1`
            - Fact, built: dc-01.
      - Box VLAN 20 · servers; kind group, tint 2. `/body/0/children/0/children/0/children/2`
        - Row, left to right. `/body/0/children/0/children/0/children/2/children/0`
          - Item Application server; kind service. `/body/0/children/0/children/0/children/2/children/0/children/0`
            - Fact, built: app-01.
          - Item File server; kind store. `/body/0/children/0/children/0/children/2/children/0/children/1`
            - Fact, built: fs-01.
    - Col, top to bottom. `/body/0/children/0/children/1`
      - Pipe VLAN 10 · staff trunk.
      - Pipe VLAN 20 · server trunk.
      - Pipe DMZ to inside.
    - Col, top to bottom. `/body/0/children/0/children/2`
      - Item Edge firewall: stateful, zone policy; kind network. `/body/0/children/0/children/2/children/0`
        - Fact, built: fw-edge-01.
    - Col, top to bottom. `/body/0/children/0/children/3`
      - Pipe VLAN 30 · DMZ.
      - Pipe VLAN 99 · transit.
    - Box DMZ; kind boundary. `/body/0/children/0/children/4`
      - Box VLAN 30 · public services; kind group, tint 3. `/body/0/children/0/children/4/children/0`
        - Row, left to right. `/body/0/children/0/children/4/children/0/children/0`
          - Item Web server; kind service. `/body/0/children/0/children/4/children/0/children/0/children/0`
            - Fact, built: web-01.
          - Item Mail relay; kind service. `/body/0/children/0/children/4/children/0/children/0/children/1`
            - Fact, built: mx-01.
      - Box VLAN 99 · transit; kind group, tint 4. `/body/0/children/0/children/4/children/1`
        - Item Edge router: ISP uplink; kind network. `/body/0/children/0/children/4/children/1/children/0`
          - Fact, built: rt-edge-01.

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| VLAN 10 · staff trunk | pipe h | Inside zone (left neighbour) | Edge firewall (right neighbour) | solid, tint 1; blue in center | no arrowheads | VLAN 10, staff | `/body/0/children/0/children/1/children/0` |
| VLAN 20 · server trunk | pipe h | Inside zone (left neighbour) | Edge firewall (right neighbour) | solid, tint 2; pink in center | no arrowheads | VLAN 20, servers | `/body/0/children/0/children/1/children/1` |
| DMZ to inside | pipe h | Inside zone (left neighbour) | Edge firewall (right neighbour) | deny; red in center | stop plate start | blocked by the firewall | `/body/0/children/0/children/1/children/2` |
| VLAN 30 · DMZ | pipe h | Edge firewall (left neighbour) | DMZ (right neighbour) | solid, tint 3; teal in center | no arrowheads | VLAN 30, DMZ | `/body/0/children/0/children/3/children/0` |
| VLAN 99 · transit | pipe h | Edge firewall (left neighbour) | DMZ (right neighbour) | solid, tint 4; amber in center | no arrowheads | VLAN 99, transit | `/body/0/children/0/children/3/children/1` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | VLAN 10, staff | 1 | `/legend/0` |
| pink | Solid pink | VLAN 20, servers | 1 | `/legend/1` |
| solid-3 | Solid teal | VLAN 30, DMZ | 1 | `/legend/2` |
| solid-4 | Solid amber | VLAN 99, transit | 1 | `/legend/3` |
| deny | Dashed red | blocked by the firewall | 1 | `/legend/4` |
