// The Azure grammar: the container and item kinds of an Azure architecture
// figure, and the domain rules that need more than a kind list.
// `grammar` is exported to crates/stencil-model/grammars/azure.json.
package azure

import (
	"encoding/json"
	"list"
	"regexp"

	"github.com/frits-v/stencil/cue:stencil"
)

grammar: stencil.#Grammar & {
	name: "azure"
	containers: [
		{
			name:     "azure"
			role:     "frame"
			tintable: false
			border: {pattern: "solid", width: 3}
			padding: 0
			radius:  10
			label:   "bar"
			parents: ["page"]
		},
		{
			name:     "subscription"
			role:     "tile"
			tone:     "highlight"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "azure", "subscription"]
		},
		{
			name:     "resource-group"
			role:     "group"
			tone:     "neutral"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["azure", "subscription", "region"]
		},
		{
			name:         "region"
			role:         "group"
			tone:         "neutral"
			tintable:     true
			default_tint: 1
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["azure", "subscription"]
		},
		{
			name:     "vnet"
			role:     "boundary"
			tone:     "strong"
			tintable: false
			border: {pattern: "dashed", width: 2}
			padding: 10
			radius:  8
			label:   "plain"
			parents: ["region", "subscription", "resource-group"]
		},
		{
			name:     "subnet"
			role:     "group"
			tone:     "cool"
			tintable: false
			border: {pattern: "dashed", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["vnet"]
		},
		{
			name:     "onprem"
			role:     "group"
			tone:     "warm"
			tintable: true
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "optional"]
		},
		{
			name:     "optional"
			role:     "group"
			tone:     "emphasis"
			tintable: false
			border: {pattern: "dashed", width: 2}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "azure", "region", "vnet", "subnet", "subscription", "resource-group"]
		},
		{
			// An AKS cluster or a namespace.
			name:     "k8s"
			role:     "group"
			tone:     "soft"
			tintable: false
			border: {pattern: "none", width: 0}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["azure", "region", "vnet", "subnet", "subscription", "resource-group", "optional"]
		},
		{
			// The PaaS services reached from a VNet over private endpoints or VNet
			// integration, drawn beside the vnet.
			name:     "services"
			role:     "group"
			tone:     "neutral"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["azure", "region", "subscription", "resource-group"]
		},
		{
			// The parts one actor runs, on a design sheet only: a dash-dot outline
			// with its name on a tab.
			name:     "assembly"
			role:     "group"
			tone:     "neutral"
			tintable: true
			border: {pattern: "dash-dot", width: 1.1}
			padding: 14
			radius:  0
			label:   "tab"
			parents: ["page", "azure", "subscription", "resource-group", "region", "vnet", "subnet", "onprem", "optional", "k8s", "services"]
		},
	]
	items: [{
		name:  "product"
		icons: "azure"
		// One row per icon, in AzureIcon::ALL order. The archive's folders are
		// categories but every file is a service icon, so every row is class
		// product.
		products: [
			{icon: "aks", class: "product", names: ["Azure Kubernetes Service", "AKS"], shape: "tower"},
			{icon: "api-management", class: "product", names: ["Azure API Management", "API Management", "APIM"]},
			{icon: "app-service", class: "product", names: ["Azure App Service", "App Service", "Web Apps"]},
			{icon: "application-gateway", class: "product", names: ["Azure Application Gateway", "Application Gateway", "App Gateway"], shape: "tile"},
			{icon: "azure-files", class: "product", names: ["Azure Files", "Azure file share"], shape: "stack"},
			{icon: "azure-firewall", class: "product", names: ["Azure Firewall"], shape: "tile"},
			{icon: "azure-openai", class: "product", names: ["Azure OpenAI Service", "Azure OpenAI", "AOAI"]},
			{icon: "bastion", class: "product", names: ["Azure Bastion", "Bastion"], shape: "tile"},
			{icon: "cache-redis", class: "product", names: ["Azure Cache for Redis", "Redis Cache"], shape: "cylinder"},
			{icon: "container-apps-environments", class: "product", names: ["Azure Container Apps environment", "Azure Container Apps", "Container Apps", "ACA"]},
			{icon: "cosmos-db", class: "product", names: ["Azure Cosmos DB", "Cosmos DB"], shape: "cylinder"},
			{icon: "data-factory", class: "product", names: ["Azure Data Factory", "Data Factory", "ADF"]},
			{icon: "databricks", class: "product", names: ["Azure Databricks", "Databricks"], shape: "cylinder"},
			{icon: "dns", class: "product", names: ["Azure DNS", "DNS zone", "private DNS zone"], shape: "tile"},
			{icon: "entra-id", class: "product", names: ["Microsoft Entra ID", "Entra ID", "Azure AD", "Azure Active Directory"]},
			{icon: "event-grid-domains", class: "product", names: ["Azure Event Grid domain", "Event Grid domain"]},
			{icon: "event-grid-topics", class: "product", names: ["Azure Event Grid", "Event Grid", "Event Grid topic"]},
			{icon: "event-hubs", class: "product", names: ["Azure Event Hubs", "Event Hubs"]},
			{icon: "expressroute", class: "product", names: ["Azure ExpressRoute", "ExpressRoute", "ExpressRoute circuit"], shape: "tile"},
			{icon: "front-door", class: "product", names: ["Azure Front Door", "Front Door", "AFD", "Front Door and CDN profiles"], shape: "tile"},
			{icon: "functions", class: "product", names: ["Azure Functions", "Functions", "Function App", "Function Apps"]},
			{icon: "key-vault", class: "product", names: ["Azure Key Vault", "Key Vault", "AKV"]},
			{icon: "load-balancer", class: "product", names: ["Azure Load Balancer"], shape: "tile"},
			{icon: "log-analytics", class: "product", names: ["Log Analytics workspace", "Log Analytics", "Azure Monitor Logs"]},
			{icon: "logic-apps", class: "product", names: ["Azure Logic Apps", "Logic Apps"]},
			{icon: "machine-learning", class: "product", names: ["Azure Machine Learning", "Azure ML", "AML"]},
			{icon: "monitor", class: "product", names: ["Azure Monitor"]},
			{icon: "policy", class: "product", names: ["Azure Policy"]},
			{icon: "postgresql", class: "product", names: ["Azure Database for PostgreSQL flexible server", "Azure Database for PostgreSQL", "PostgreSQL flexible server"], shape: "cylinder"},
			{icon: "private-endpoint", class: "product", names: ["Azure Private Endpoint", "private endpoint"], shape: "tile"},
			{icon: "private-link", class: "product", names: ["Azure Private Link", "Private Link", "Private Link service"], shape: "tile"},
			{icon: "service-bus", class: "product", names: ["Azure Service Bus", "Service Bus"]},
			{icon: "sql-database", class: "product", names: ["Azure SQL Database", "SQL Database"], shape: "cylinder"},
			{icon: "sql-managed-instance", class: "product", names: ["Azure SQL Managed Instance", "SQL Managed Instance", "SQL MI"], shape: "cylinder"},
			{icon: "storage-accounts", class: "product", names: ["Azure Storage account", "Storage account", "Azure Storage", "Azure Blob Storage", "Blob Storage"], shape: "stack"},
			{icon: "synapse", class: "product", names: ["Azure Synapse Analytics", "Synapse"], shape: "cylinder"},
			{icon: "traffic-manager", class: "product", names: ["Azure Traffic Manager", "Traffic Manager"], shape: "tile"},
			{icon: "virtual-machines", class: "product", names: ["Azure Virtual Machines", "Virtual Machines", "Virtual Machine"], shape: "tower"},
			{icon: "virtual-network", class: "product", names: ["Azure Virtual Network", "Virtual Network"], shape: "tile"},
			{icon: "vm-scale-sets", class: "product", names: ["Azure Virtual Machine Scale Sets", "Virtual Machine Scale Sets", "VM Scale Sets", "VMSS"], shape: "tower"},
			{icon: "vpn-gateway", class: "product", names: ["Azure VPN Gateway", "VPN Gateway", "virtual network gateway", "ExpressRoute gateway"], shape: "tile"},
		]
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		// Under iso a product stands as a block unless its icon's row says otherwise;
		// an item asks for the raised card with shape: "card".
		shape: "block"
	}, {
		// A user, operator or team, drawn as a standing figure under iso.
		name:  "person"
		icons: "none"
		products: []
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		shape: "figure"
	}, {
		// A client endpoint, drawn as a laptop under iso; an item may ask for a phone.
		name:  "device"
		icons: "none"
		products: []
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		shape: "laptop"
	}]
	remembered: [
		{
			literal: "10.0.0.0/16"
			reason:  "doc example address space from the virtual network quickstarts, not a design value. A figure's address space comes from its IP plan, as a built fact or an ask."
		},
		{
			literal: "10.0.0.0/24"
			reason:  "doc example subnet range from the virtual network quickstarts, not a design value. A figure's subnet ranges come from its IP plan, as a built fact or an ask."
		},
		{
			literal: "168.63.129.16"
			reason:  "the platform address for DNS, DHCP and health probes that every NSG doc repeats: a fact of the platform, not a design value. A rule names the AzureLoadBalancer or AzurePlatformDNS service tag."
		},
		{
			literal: "65515"
			reason:  "the default ASN of a VPN gateway, settable per gateway, so a doc example and not a requirement. The gateway's configured ASN is a built fact."
		},
		{
			literal: "12076"
			reason:  "Microsoft's ASN on ExpressRoute peering, fixed by the platform: a fact, not a design value. Name the peering and leave the ASN to the live ExpressRoute doc."
		},
	]
}

_nesting: stencil.#GrammarNesting & {#grammar: grammar}

// An azure figure: the core page with the azure kinds and nesting, plus the
// azure rules. Each check is a struct keyed by the offending item or literal,
// so a failure names it in its path.
#Page: P={
	_nesting.#KindedPage
	body: [...#Node]
	_nameless: {for n in body {n._nameless}}
	// The hop-fact rule. A built fact does not satisfy it: as-built names need
	// no live doc, and they say nothing about what the product is.
	_itemsWithoutSubtitleOrFact: {
		for k, _ in _nameless {(k): "product item" & "needs a subtitle, or a fact with source doc or ask"}
	}
	// The core rejects gcp's literals inside #Text, where a grammar cannot
	// reach; azure's are matched at word boundaries over the page's regular
	// fields, which hold every text field.
	_pageText: json.Marshal({for k, v in P {(k): v}})
	_rememberedInText: {
		for r in grammar.remembered if _pageText =~ "\\b\(regexp.QuoteMeta(r.literal))\\b" {
			(r.literal): "text field" & "contains \(r.literal): \(r.reason)"
		}
	}
}

// PaaS services are reached from a VNet over a private endpoint or VNet
// integration and are never addresses in it: named in a subtitle at word
// boundaries, or drawn with their product icon.
#PaasProductName: "\\b(Azure Storage|[Ss]torage accounts?|Blob Storage|Azure Files|SQL Database|Cosmos DB|Key Vault|Service Bus|Event Hubs|Event Grid|App Service|Functions|Function Apps?|Logic Apps|Azure OpenAI)\\b"
#PaasProductIcons: ["storage-accounts", "azure-files", "sql-database", "cosmos-db", "key-vault", "service-bus", "event-hubs", "event-grid-topics", "event-grid-domains", "app-service", "functions", "logic-apps", "azure-openai"]

// What a subnet does hold although its subtitle names a PaaS service: an App
// Service Environment, deployed into a subnet, and a private endpoint, a NIC
// in the subnet that names its target.
#InSubnetProductName: "\\b(App Service Environment|[Pp]rivate [Ee]ndpoints?)\\b"
#InSubnetProductIcons: ["private-endpoint"]

// Global and tenant-wide resources have no region; one drawn inside a region
// reads as regional. Azure Monitor is not listed: a Log Analytics workspace is
// regional.
#GlobalProductName: "\\b(Entra ID|Azure AD|Azure Active Directory|Front Door|Traffic Manager|DNS zones?|Azure Policy)\\b"
#GlobalProductIcons: ["entra-id", "front-door", "traffic-manager", "dns", "policy"]

// The azure walk over the body. Each node carries:
//   _nameless       the titles of product items with no subtitle and no doc or
//                   ask fact
//   _paasProducts   the titles of product items that are PaaS services
//                   (#PaasProductName, #PaasProductIcons) and not deployed into
//                   a subnet (#InSubnetProductName, #InSubnetProductIcons)
//   _globalProducts the same for global and tenant-wide resources
//                   (#GlobalProductName, #GlobalProductIcons)
// Tint pairing is the core's, in #GrammarNesting (cue/grammar.cue).
#Node: {
	tag: string
	if tag == "Row" || tag == "Col" || tag == "Lanes" || tag == "View" {
		children: [...#Node]
		_nameless: {for c in children {c._nameless}}
		_paasProducts: {for c in children {c._paasProducts}}
		_globalProducts: {for c in children {c._globalProducts}}
	}
	if tag == "Box" {
		kind: string
		children: [...#Node]
		_nameless: {for c in children {c._nameless}}
		_paasProducts: {for c in children {c._paasProducts}}
		_globalProducts: {for c in children {c._globalProducts}}

		// A PaaS service is not an address in a VNet; it belongs in a services
		// Box beside it, with the private endpoint or VNet integration inside.
		if kind == "vnet" {
			_paasInsideVnet: {
				for k, _ in _paasProducts {(k): "product" & "sits inside a vnet; draw it in a services box beside the vnet and the private endpoint or VNet integration inside"}
			}
		}

		// A global or tenant-wide resource has no region; draw it outside every
		// region Box.
		if kind == "region" {
			_globalInsideRegion: {
				for k, _ in _globalProducts {(k): "product" & "sits inside a region; a global or tenant-wide resource sits outside every region"}
			}
		}
	}
	if tag == "Item" {
		kind:      string
		title:     string
		icon?:     string
		subtitle?: string
		facts?: [...]
		_hopFacts: [
			if facts != _|_ for f in facts
			let source = [if f.source != _|_ {f.source}, "doc"][0]
			if source != "built" {f},
		]
		_nameless: {if kind == "product" && subtitle == _|_ && len(_hopFacts) == 0 {(title): true}}
		_isPaas: [
			if subtitle != _|_ if subtitle =~ #PaasProductName {true},
			if icon != _|_ if list.Contains(#PaasProductIcons, icon) {true},
		]
		_inSubnet: [
			if subtitle != _|_ if subtitle =~ #InSubnetProductName {true},
			if icon != _|_ if list.Contains(#InSubnetProductIcons, icon) {true},
		]
		_paasProducts: {if kind == "product" && len(_isPaas) > 0 && len(_inSubnet) == 0 {(title): true}}
		_isGlobal: [
			if subtitle != _|_ if subtitle =~ #GlobalProductName {true},
			if icon != _|_ if list.Contains(#GlobalProductIcons, icon) {true},
		]
		_globalProducts: {if kind == "product" && len(_isGlobal) > 0 {(title): true}}
	}
	if !list.Contains(["Row", "Col", "Lanes", "View", "Box", "Item"], tag) {
		_nameless: {}
		_paasProducts: {}
		_globalProducts: {}
	}
	...
}
