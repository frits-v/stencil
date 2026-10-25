// A hub-and-spoke network in one Azure region: an on-prem site reaches the hub
// VNet over ExpressRoute, the hub firewall inspects spoke traffic, and the
// spoke's AKS cluster reaches Storage and SQL Database over private endpoints.
// Negative cases in check.sh edit it for the azure rules.
package azure

import (
	"github.com/frits-v/stencil/cue/grammars:azure"
)

figure: azure.#Page & {
	title:   "Spoke traffic leaves through the hub firewall, and PaaS stays off the address space"
	kicker:  "Azure hub and spoke · one region"
	lede:    "On-prem reaches the hub over ExpressRoute. The spoke's cluster reaches Storage and SQL Database through private endpoints in its own VNet."
	badge:   "Customer"
	grammar: "azure"
	body: [{
		tag: "Row"
		gap: 8
		grow: [0, 0, 1]
		children: [
			{
				tag:   "Box"
				kind:  "onprem"
				tint:  1
				label: "On-prem site"
				children: [
					{tag: "Item", kind: "product", title: "Edge router", subtitle: "customer edge toward Microsoft"},
				]
			},
			{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "ExpressRoute", sub: "private peering · BGP"},
			{
				tag:   "Box"
				kind:  "azure"
				label: "Microsoft Azure"
				children: [{
					tag: "Col"
					gap: 8
					children: [
						{
							tag:   "Box"
							kind:  "region"
							label: "Primary region"
							children: [{
								tag: "Col"
								gap: 8
								children: [{
									tag: "Row"
									gap: 8
									children: [
										{
											tag:   "Box"
											kind:  "vnet"
											label: "Hub VNet"
											children: [
												{
													tag:   "Box"
													kind:  "subnet"
													label: "GatewaySubnet"
													children: [
														{tag: "Item", kind: "product", icon: "vpn-gateway", title: "Gateway", subtitle: "ExpressRoute gateway"},
													]
												},
												{
													tag:   "Box"
													kind:  "subnet"
													label: "AzureFirewallSubnet"
													children: [
														{tag: "Item", kind: "product", icon: "azure-firewall", title: "Firewall", subtitle: "Azure Firewall"},
													]
												},
											]
										},
										{tag: "Pipe", dir: "h", line: "solid", label: "peering", sub: "via the firewall"},
										{
											tag:   "Box"
											kind:  "vnet"
											label: "Spoke VNet"
											children: [
												{
													tag:   "Box"
													kind:  "subnet"
													label: "AKS nodes"
													children: [
														{tag: "Item", kind: "product", icon: "aks", title: "Cluster", subtitle: "Azure Kubernetes Service"},
													]
												},
												{
													tag:   "Box"
													kind:  "subnet"
													label: "Private endpoints"
													children: [
														{tag: "Item", kind: "product", icon: "private-endpoint", title: "Blob endpoint", subtitle: "private endpoint · Blob Storage"},
														{tag: "Item", kind: "product", icon: "private-endpoint", title: "SQL endpoint", subtitle: "private endpoint · SQL Database"},
													]
												},
											]
										},
									]
								}, {
									tag:   "Box"
									kind:  "services"
									label: "PaaS services"
									children: [
										{tag: "Item", kind: "product", icon: "storage-accounts", title: "Data lake", subtitle: "Azure Storage account"},
										{tag: "Item", kind: "product", icon: "sql-database", title: "Orders", subtitle: "Azure SQL Database"},
									]
								},
								]
							}]
						},
						{tag: "Item", kind: "product", icon: "entra-id", title: "Directory", subtitle: "Microsoft Entra ID"},
					]
				}]
			},
		]
	}]
	legend: [
		{line: "solid", text: "on-prem and spoke traffic through the hub"},
	]
}
