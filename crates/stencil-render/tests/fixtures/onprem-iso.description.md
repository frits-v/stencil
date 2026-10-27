# Building management network of an office tower `/title`

Internal · Building management · office tower `/kicker`

The core switch trunks the heating and security VLANs to the edge firewall. Controllers push trends and alarms out to a vendor portal in the DMZ, and no session that starts in the DMZ reaches them. `/lede`

## Figure

Drawn elements: 29 (28 nodes, 1 link). Laid out at 1740 x 1449.48 px, projection iso, grammar plain; the projected drawing is larger, by an amount that depends on the theme. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Office tower; kind system. `/body/0`
  - Row, left to right. `/body/0/children/0`
    - Box Inside zone; kind boundary. `/body/0/children/0/children/0`
      - Col, top to bottom. `/body/0/children/0/children/0/children/0`
        - Box VLAN 10 · heating; kind group, tint 1. `/body/0/children/0/children/0/children/0/children/0`
          - Row, left to right. `/body/0/children/0/children/0/children/0/children/0/children/0`
            - Item Boiler controller; kind service, shape block. `/body/0/children/0/children/0/children/0/children/0/children/0/children/0`
              - Fact, built: blr-ctl-01.
            - Item Air handler control; kind service, shape block. `/body/0/children/0/children/0/children/0/children/0/children/0/children/1`
              - Fact, built: ahu-ctl-01.
        - Item Core switch: Layer 3, routes between VLANs; kind network, shape tile. `/body/0/children/0/children/0/children/0/children/1`
          - Fact, built: sw-core-01.
        - Box VLAN 20 · security; kind group, tint 2. `/body/0/children/0/children/0/children/0/children/2`
          - Row, left to right. `/body/0/children/0/children/0/children/0/children/2/children/0`
            - Item Access control; kind service, shape block. `/body/0/children/0/children/0/children/0/children/2/children/0/children/0`
              - Fact, built: acs-01.
            - Item Camera recorder; kind store, shape cylinder. `/body/0/children/0/children/0/children/0/children/2/children/0/children/1`
              - Fact, built: nvr-01.
    - Col, top to bottom. `/body/0/children/0/children/1`
      - Col, top to bottom. `/body/0/children/0/children/1/children/0`
        - Pipe VLAN 10.
        - Pipe VLAN 20.
    - Col, top to bottom. `/body/0/children/0/children/2`
      - Item Edge firewall: stateful, zone policy; kind network, shape card. `/body/0/children/0/children/2/children/0`
        - Fact, built: fw-edge-01.
        - Fact, built: permits inside to DMZ.
        - Fact, built: permits inside to transit.
        - Fact, built: denies DMZ to inside.
    - Col, top to bottom. `/body/0/children/0/children/3`
      - Col, top to bottom. `/body/0/children/0/children/3/children/0`
        - Pipe VLAN 30.
        - Pipe DMZ to inside.
    - Box DMZ; kind boundary. `/body/0/children/0/children/4`
      - Box VLAN 99 · transit; kind group, tint 4. `/body/0/children/0/children/4/children/0`
        - Item Edge router: ISP uplink; kind network, shape tile. `/body/0/children/0/children/4/children/0/children/0`
          - Fact, built: rt-edge-01.
      - Box VLAN 30 · vendor access; kind group, tint 3. `/body/0/children/0/children/4/children/1`
        - Row, left to right. `/body/0/children/0/children/4/children/1/children/0`
          - Item Vendor portal; kind service, shape block. `/body/0/children/0/children/4/children/1/children/0/children/0`
            - Fact, built: portal-01.
          - Item Alarm relay; kind service, shape block. `/body/0/children/0/children/4/children/1/children/0/children/1`
            - Fact, built: alarm-01.

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| VLAN 10 | pipe h | Core switch (core) | Edge firewall (fw) | solid, tint 1; blue in center | no arrowheads | VLAN 10, heating | `/body/0/children/0/children/1/children/0/children/0` |
| VLAN 20 | pipe h | Core switch (core) | Edge firewall (fw) | solid, tint 2; pink in center | no arrowheads | VLAN 20, security | `/body/0/children/0/children/1/children/0/children/1` |
| VLAN 30 | pipe h | Edge firewall (fw) | VLAN 30 · vendor access (vlan30) | solid, tint 3; teal in center | no arrowheads | VLAN 30, vendor access | `/body/0/children/0/children/3/children/0/children/0` |
| DMZ to inside | pipe h | Edge firewall (fw) | VLAN 30 · vendor access (vlan30) | deny; red in center | stop plate start | blocked by the firewall | `/body/0/children/0/children/3/children/0/children/1` |
| VLAN 99 · transit | link | Edge firewall (fw) | Edge router (rt) | solid, tint 4; amber in center | arrow end | VLAN 99, transit | `/links/0` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | VLAN 10, heating | 1 | `/legend/0` |
| pink | Solid pink | VLAN 20, security | 1 | `/legend/1` |
| solid-3 | Solid teal | VLAN 30, vendor access | 1 | `/legend/2` |
| solid-4 | Solid amber | VLAN 99, transit | 1 | `/legend/3` |
| deny | Dashed red | blocked by the firewall | 1 | `/legend/4` |
