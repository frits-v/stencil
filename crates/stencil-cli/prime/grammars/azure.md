# Grammar azure

The Azure vocabulary: set "grammar": "azure" on the page. A design sheet under azure: take `stencil prime onepager`, a plain sheet, set "grammar": "azure", make the system Box an azure Box and each item a product with its icon; vet names each one missed as kind-unknown.

{{kinds}}

{{icons}}

| Kind | Means |
|---|---|
| azure | the Azure frame |
| subscription | a subscription: pale yellow |
| resource-group | a resource group: neutral, never tinted |
| region | a region, tinted: tint 1 (default) blue, tint 2 pink |
| vnet | a virtual network: dashed gray border, no fill; under iso a ring |
| subnet | a subnet: dashed border, lavender |
| onprem | an on-prem site or another cloud; tint 1 pairs with region tint 1 |
| optional | an optional or proposed component: dashed blue |
| k8s | an AKS cluster or namespace: pink, no border |
| services | PaaS services reached from a VNet over private endpoints, beside every vnet |
| assembly | the parts one actor runs, on a design sheet only |

Nesting: azure, then subscription, then region, then vnet, then subnet; a resource-group wraps what it holds. onprem sits outside azure. Tint pairing is gcp's: a solid pipe of one slot sits beside or inside no tinted region or onprem of another slot.

Placement: Storage, SQL Database, Cosmos DB, Key Vault, Service Bus, Event Hubs, App Service, Functions, Logic Apps, Azure OpenAI and the other PaaS services are not addresses in a VNet: draw them in a services box beside the vnet, with the private endpoint or VNet integration inside. Virtual machines, AKS node pools, Container Apps environments, App Service Environments, Application Gateway, Azure Firewall, Bastion and SQL Managed Instance sit in a subnet. Entra ID, Front Door, Traffic Manager, DNS zones and Azure Policy are global or tenant-wide: outside every region. vet rejects both placements and a product with no subtitle, doc fact or ask.

remembered-constants rejects 10.0.0.0/16, 10.0.0.0/24, 168.63.129.16, 65515 and 12076: doc examples or platform facts, never design values.

Lines: gray for internal calls, solid tint 1 and tint 2 for two paired paths, dash for failover or control plane, deny for a blocked path.

Rules no check enforces: a product on a hop carries a fact read from the live doc (source doc) or an ask, and its subtitle the official name (Azure Firewall, Azure SQL Database).
