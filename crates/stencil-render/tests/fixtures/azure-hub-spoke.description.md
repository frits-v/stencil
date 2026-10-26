# Spoke traffic leaves through the hub firewall, and the booking data stays off the address space `/title`

Customer · Hotel reservation platform · Azure hub and spoke · one region `/kicker`

The booking cluster runs in a spoke VNet peered to the hub, and the hub firewall inspects what leaves the spoke. The cluster reaches room photos in Azure Storage, reservations in Azure SQL Database and its secrets in Azure Key Vault through private endpoints in its own VNet. Azure Front Door takes guest traffic and Microsoft Entra ID signs in staff, both outside the region. `/lede`

## Figure

Drawn elements: 28 (28 nodes, 0 links). Canvas 1480 x 964.6 px, projection flat, grammar azure. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box Microsoft Azure; kind azure. `/body/0`
  - Col, top to bottom. `/body/0/children/0`
    - Box Primary region; kind region, tint 1. `/body/0/children/0/children/0`
      - Col, top to bottom. `/body/0/children/0/children/0/children/0`
        - Row, left to right. `/body/0/children/0/children/0/children/0/children/0`
          - Box Hub VNet; kind vnet. `/body/0/children/0/children/0/children/0/children/0/children/0`
            - Box AzureBastionSubnet; kind subnet. `/body/0/children/0/children/0/children/0/children/0/children/0/children/0`
              - Item Operator access: Azure Bastion; kind product, icon bastion, product Azure Bastion. `/body/0/children/0/children/0/children/0/children/0/children/0/children/0/children/0`
                - Fact, doc: SKU and scale units from the Azure Bastion doc.
            - Box AzureFirewallSubnet; kind subnet. `/body/0/children/0/children/0/children/0/children/0/children/0/children/1`
              - Item Firewall: Azure Firewall; kind product, icon azure-firewall, product Azure Firewall. `/body/0/children/0/children/0/children/0/children/0/children/0/children/1/children/0`
                - Fact, ask: Premium or Standard SKU: open question.
          - Pipe VNet peering · spoke traffic via the firewall.
          - Box Spoke VNet; kind vnet. `/body/0/children/0/children/0/children/0/children/0/children/2`
            - Box AKS nodes; kind subnet. `/body/0/children/0/children/0/children/0/children/0/children/2/children/0`
              - Item Booking cluster: Azure Kubernetes Service; kind product, icon aks, product Azure Kubernetes Service. `/body/0/children/0/children/0/children/0/children/0/children/2/children/0/children/0`
              - Item Channel sync: Azure Virtual Machines; kind product, icon virtual-machines, product Azure Virtual Machines. `/body/0/children/0/children/0/children/0/children/0/children/2/children/0/children/1`
            - Box Private endpoints; kind subnet. `/body/0/children/0/children/0/children/0/children/0/children/2/children/1`
              - Item Blob endpoint: private endpoint · Blob Storage; kind product, icon private-endpoint, product private endpoint. `/body/0/children/0/children/0/children/0/children/0/children/2/children/1/children/0`
              - Item SQL endpoint: private endpoint · SQL Database; kind product, icon private-endpoint, product private endpoint. `/body/0/children/0/children/0/children/0/children/0/children/2/children/1/children/1`
              - Item Key Vault endpoint: private endpoint · Key Vault; kind product, icon private-endpoint, product private endpoint. `/body/0/children/0/children/0/children/0/children/0/children/2/children/1/children/2`
        - Pipe Private Link · from the endpoints to the services.
        - Box PaaS services; kind services. `/body/0/children/0/children/0/children/0/children/2`
          - Row, left to right. `/body/0/children/0/children/0/children/0/children/2/children/0`
            - Item Room photos: Azure Storage account; kind product, icon storage-accounts, product Azure Storage account. `/body/0/children/0/children/0/children/0/children/2/children/0/children/0`
            - Item Reservations: Azure SQL Database; kind product, icon sql-database, product Azure SQL Database. `/body/0/children/0/children/0/children/0/children/2/children/0/children/1`
            - Item Booking secrets: Azure Key Vault; kind product, icon key-vault, product Azure Key Vault. `/body/0/children/0/children/0/children/0/children/2/children/0/children/2`
    - Row, left to right. `/body/0/children/0/children/1`
      - Item Staff sign-in: Microsoft Entra ID; kind product, icon entra-id, product Microsoft Entra ID. `/body/0/children/0/children/1/children/0`
      - Item Guest entry: Azure Front Door; kind product, icon front-door, product Azure Front Door. `/body/0/children/0/children/1/children/1`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| VNet peering · spoke traffic via the firewall | pipe h | Hub VNet (left neighbour) | Spoke VNet (right neighbour) | gray; gray in center | no arrowheads | inside Azure: peering and Private Link | `/body/0/children/0/children/0/children/0/children/0/children/1` |
| Private Link · from the endpoints to the services | pipe v | VNet peering (neighbour above) | PaaS services (neighbour below) | gray; gray in center | no arrowheads | inside Azure: peering and Private Link | `/body/0/children/0/children/0/children/0/children/1` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| gray | Solid gray | inside Azure: peering and Private Link | 2 | `/legend/0` |

Foot: Illustrative topology · hop facts are placeholders, not looked-up values `/foot`
