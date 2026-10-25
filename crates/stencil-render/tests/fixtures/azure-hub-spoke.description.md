# Spoke traffic leaves through the hub firewall, and PaaS stays off the address space `/title`

Customer · Azure hub and spoke · ExpressRoute with VPN backup · one region `/kicker`

On-prem reaches the hub over ExpressRoute private peering, with a site-to-site VPN as backup. The hub firewall inspects what crosses between spokes. The spoke cluster reaches Storage and SQL Database through private endpoints in its own VNet. `/lede`

## Figure

Drawn elements: 35 (35 nodes, 0 links). Canvas 1480 x 910.6 px, projection flat, grammar azure. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box On-prem site; kind onprem, tint 1. `/body/0/children/0/children/0`
      - Item Edge router: customer edge toward Microsoft; kind product. `/body/0/children/0/children/0/children/0`
      - Item VPN device: IPsec peer for the backup tunnel; kind product. `/body/0/children/0/children/0/children/1`
      - Item Directory servers: synchronized to Microsoft Entra ID; kind product, product Microsoft Entra ID. `/body/0/children/0/children/0/children/2`
  - Col, top to bottom. `/body/0/children/1`
    - Pipe ExpressRoute · private peering · BGP.
    - Pipe Site-to-site VPN · backup path · IPsec · BGP.
  - Box Microsoft Azure; kind azure. `/body/0/children/2`
    - Col, top to bottom. `/body/0/children/2/children/0`
      - Box Primary region; kind region, tint 1. `/body/0/children/2/children/0/children/0`
        - Col, top to bottom. `/body/0/children/2/children/0/children/0/children/0`
          - Row, left to right. `/body/0/children/2/children/0/children/0/children/0/children/0`
            - Box Hub VNet; kind vnet. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0`
              - Box GatewaySubnet; kind subnet. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0/children/0`
                - Item Gateway: ExpressRoute gateway; kind product, icon vpn-gateway, product ExpressRoute gateway. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0/children/0/children/0`
                  - Fact, doc: SKU and zone redundancy from the ExpressRoute gateway doc.
              - Box AzureFirewallSubnet; kind subnet. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0/children/1`
                - Item Firewall: Azure Firewall; kind product, icon azure-firewall, product Azure Firewall. `/body/0/children/2/children/0/children/0/children/0/children/0/children/0/children/1/children/0`
                  - Fact, ask: Premium or Standard SKU: open question.
            - Pipe VNet peering · spoke traffic via the firewall.
            - Box Spoke VNet; kind vnet. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2`
              - Box AKS nodes; kind subnet. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/0`
                - Item Cluster: Azure Kubernetes Service; kind product, icon aks, product Azure Kubernetes Service. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/0/children/0`
                - Item Build agents: Azure Virtual Machines; kind product, icon virtual-machines, product Azure Virtual Machines. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/0/children/1`
              - Box Private endpoints; kind subnet. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/1`
                - Item Blob endpoint: private endpoint · Blob Storage; kind product, icon private-endpoint, product private endpoint. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/1/children/0`
                - Item SQL endpoint: private endpoint · SQL Database; kind product, icon private-endpoint, product private endpoint. `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/1/children/1`
          - Pipe Private Link · from the endpoints to the services.
          - Box PaaS services; kind services. `/body/0/children/2/children/0/children/0/children/0/children/2`
            - Row, left to right. `/body/0/children/2/children/0/children/0/children/0/children/2/children/0`
              - Item Data lake: Azure Storage account; kind product, icon storage-accounts, product Azure Storage account. `/body/0/children/2/children/0/children/0/children/0/children/2/children/0/children/0`
              - Item Orders: Azure SQL Database; kind product, icon sql-database, product Azure SQL Database. `/body/0/children/2/children/0/children/0/children/0/children/2/children/0/children/1`
              - Item Secrets: Azure Key Vault; kind product, icon key-vault, product Azure Key Vault. `/body/0/children/2/children/0/children/0/children/0/children/2/children/0/children/2`
      - Row, left to right. `/body/0/children/2/children/0/children/1`
        - Item Directory: Microsoft Entra ID; kind product, icon entra-id, product Microsoft Entra ID. `/body/0/children/2/children/0/children/1/children/0`
        - Item Public entry: Azure Front Door; kind product, icon front-door, product Azure Front Door. `/body/0/children/2/children/0/children/1/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| ExpressRoute · private peering · BGP | pipe h | On-prem site (left neighbour) | Microsoft Azure (right neighbour) | solid, tint 1; blue in center | no arrowheads | on-prem to the region over ExpressRoute | `/body/0/children/1/children/0` |
| Site-to-site VPN · backup path · IPsec · BGP | pipe h | On-prem site (left neighbour) | Microsoft Azure (right neighbour) | dash, tint 1; blue in center | no arrowheads | backup path over the internet | `/body/0/children/1/children/1` |
| VNet peering · spoke traffic via the firewall | pipe h | Hub VNet (left neighbour) | Spoke VNet (right neighbour) | gray; gray in center | no arrowheads | inside Azure: peering and Private Link | `/body/0/children/2/children/0/children/0/children/0/children/0/children/1` |
| Private Link · from the endpoints to the services | pipe v | VNet peering (neighbour above) | PaaS services (neighbour below) | gray; gray in center | no arrowheads | inside Azure: peering and Private Link | `/body/0/children/2/children/0/children/0/children/0/children/1` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | on-prem to the region over ExpressRoute | 1 | `/legend/0` |
| dash | Dashed blue | backup path over the internet | 1 | `/legend/1` |
| gray | Solid gray | inside Azure: peering and Private Link | 2 | `/legend/2` |

Foot: Illustrative topology · hop facts are placeholders, not looked-up values `/foot`
