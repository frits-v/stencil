// The Google Cloud grammar: the container and item kinds of a GCP
// architecture figure, and the domain rules that need more than a kind list.
// `grammar` is exported to crates/stencil-model/grammars/gcp.json.
package gcp

import (
	"list"

	"github.com/frits-v/stencil/cue:stencil"
)

grammar: stencil.#Grammar & {
	name: "gcp"
	containers: [
		{
			name:     "gcp"
			role:     "frame"
			tintable: false
			border: {pattern: "solid", width: 3}
			padding: 0
			radius:  10
			label:   "bar"
			parents: ["page"]
		},
		{
			name:     "vpc"
			role:     "boundary"
			tone:     "strong"
			tintable: false
			border: {pattern: "dashed", width: 2}
			padding: 10
			radius:  8
			label:   "plain"
			parents: ["gcp", "project", "perimeter"]
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
			parents: ["gcp", "vpc", "perimeter", "project"]
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
			parents: ["region", "vpc", "project"]
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
			name:     "project"
			role:     "tile"
			tone:     "highlight"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "gcp", "vpc", "perimeter", "project"]
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
			parents: ["page", "gcp", "vpc", "region", "subnet", "project", "perimeter"]
		},
		{
			name:     "k8s"
			role:     "group"
			tone:     "soft"
			tintable: false
			border: {pattern: "none", width: 0}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["gcp", "vpc", "region", "subnet", "project", "perimeter", "optional"]
		},
		{
			name:     "perimeter"
			role:     "boundary"
			tone:     "accent"
			tintable: false
			border: {pattern: "dashed", width: 2.5}
			padding: 12
			radius:  10
			label:   "accent"
			parents: ["gcp", "vpc", "project"]
		},
		{
			name:     "apis"
			role:     "group"
			tone:     "neutral"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["gcp", "project", "perimeter"]
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
			parents: ["page", "gcp", "vpc", "region", "subnet", "onprem", "project", "optional", "k8s", "perimeter", "apis"]
		},
	]
	items: [{
		name:  "product"
		icons: "gcp"
		// One row per icon, in IconName::ALL order. product: a Unique Icons
		// archive path, standing for one product. category: a Category Icons
		// path, standing for a product family.
		products: [
			{icon: "agents", class: "category", names: ["Vertex AI Agent Builder", "Vertex AI Agent Engine", "Agent Engine", "Agentspace", "Gemini Enterprise", "Dialogflow"]},
			{icon: "ai-ml", class: "category", names: ["Vertex AI", "Gemini", "Document AI", "Vision AI", "Speech-to-Text", "Text-to-Speech", "Translation AI", "Natural Language AI", "Cloud TPU"]},
			{icon: "bigquery", class: "product", names: ["BigQuery"], shape: "cylinder"},
			{icon: "cloud-run-flat", class: "product", names: ["Cloud Run", "Cloud Run functions"]},
			{icon: "cloud-run", class: "product", names: ["Cloud Run", "Cloud Run functions"]},
			{icon: "cloud-sql", class: "product", names: ["Cloud SQL"], shape: "cylinder"},
			{icon: "cloud-storage", class: "product", names: ["Cloud Storage"], shape: "stack"},
			{icon: "compute-engine", class: "product", names: ["Compute Engine"], shape: "tower"},
			{icon: "compute", class: "category", names: ["Compute Engine", "Cloud Run", "Cloud Run functions", "App Engine", "Cloud Functions", "Batch", "Bare Metal Solution", "Google Cloud VMware Engine"], shape: "tower"},
			{icon: "containers", class: "category", names: ["Google Kubernetes Engine", "GKE", "Cloud Run", "Artifact Registry"], shape: "tower"},
			{icon: "data-analytics", class: "category", names: ["BigQuery", "Dataflow", "Dataproc", "Pub/Sub", "Looker", "Dataplex", "Cloud Data Fusion", "Cloud Composer", "Datastream", "Dataform"], shape: "cylinder"},
			{icon: "databases", class: "category", names: ["Cloud SQL", "AlloyDB", "Spanner", "Firestore", "Bigtable", "Memorystore", "Database Migration Service"], shape: "cylinder"},
			{icon: "devops", class: "category", names: ["Cloud Build", "Artifact Registry", "Cloud Deploy", "Infrastructure Manager"]},
			{icon: "gke", class: "product", names: ["Google Kubernetes Engine", "GKE"], shape: "tower"},
			{icon: "hybrid", class: "category", names: ["Google Distributed Cloud", "Cloud Interconnect", "Dedicated Interconnect", "Partner Interconnect", "Cross-Cloud Interconnect"], shape: "tile"},
			{icon: "integration", class: "category", names: ["Pub/Sub", "Application Integration", "Workflows", "Eventarc", "Apigee", "API Gateway", "Cloud Tasks", "Cloud Scheduler"]},
			{icon: "networking", class: "category", names: ["Cloud Load Balancing", "Cloud CDN", "Cloud DNS", "Cloud NAT", "Cloud Router", "Cloud VPN", "Cloud Interconnect", "Network Connectivity Center", "Private Service Connect", "Virtual Private Cloud"], shape: "tile"},
			{icon: "observability", class: "category", names: ["Cloud Logging", "Cloud Monitoring", "Cloud Trace", "Cloud Profiler", "Error Reporting"]},
			{icon: "scc", class: "product", names: ["Security Command Center"]},
			{icon: "security-identity", class: "category", names: ["Security Command Center", "Cloud KMS", "Secret Manager", "Identity and Access Management", "Identity-Aware Proxy", "Cloud Armor", "VPC Service Controls", "Certificate Authority Service", "Sensitive Data Protection"]},
			{icon: "serverless", class: "category", names: ["Cloud Run", "Cloud Run functions", "Cloud Functions", "App Engine", "Workflows", "Eventarc"]},
			{icon: "storage", class: "category", names: ["Cloud Storage", "Filestore", "Persistent Disk", "Hyperdisk", "Backup and DR Service", "Storage Transfer Service", "Google Cloud NetApp Volumes"], shape: "stack"},
			{icon: "vertex-ai", class: "product", names: ["Vertex AI"]},
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
			literal: "64512"
			reason:  "doc example ASN, not a requirement. Dedicated Interconnect takes any private ASN (RFC 6996); Partner Interconnect is fixed at 16550."
		},
		{
			literal: "130.211.0.0/22"
			reason:  "GFE health-check probe range. Applies only to backend types the health-check doc names. Not serverless NEG or Cloud Run."
		},
		{
			literal: "35.191.0.0/16"
			reason:  "GFE health-check probe range. Applies only to backend types the health-check doc names. Not serverless NEG or Cloud Run."
		},
		{
			literal: "10.8.0.0/28"
			reason:  "Serverless VPC Access connector range. Direct VPC egress does not use a connector; label it Private Google Access on the subnet instead."
		},
	]
}

_nesting: stencil.#GrammarNesting & {#grammar: grammar}

// A gcp figure: the core page with the gcp kinds and nesting, plus the gcp
// rules. Each check is a struct keyed by the offending item, so a failure
// names it in its path.
#Page: _nesting.#KindedPage & {
	body: [...#Node]
	_nameless: {for n in body {n._nameless}}
	// The hop-fact rule. A built fact does not satisfy it: as-built names need
	// no live doc, and they say nothing about what the product is.
	_itemsWithoutSubtitleOrFact: {
		for k, _ in _nameless {(k): "product item" & "needs a subtitle, or a fact with source doc or ask"}
	}
	// An apis Box holds Google APIs reached from inside the gcp frame; one
	// with no gcp Box above it is drawn in the wrong place. A gcp Box absorbs
	// the labels below it, so only the stray ones reach the page.
	_apisAbove: {for n in body {n._apisLabels}}
	_apisOutsideGcp: {
		for k, _ in _apisAbove {(k): "apis box" & "sits outside the gcp frame"}
	}
}

// Products that are Google APIs reached over Private Google Access, never
// addresses in a VPC: named in a subtitle at word boundaries, or drawn with
// their product icon.
#ApiProductName: "\\b(Cloud Storage|BigQuery|Pub/Sub|Artifact Registry|Cloud Logging)\\b"
#ApiProductIcons: ["cloud-storage", "bigquery"]

// Serverless products run outside the customer's VPC and are reached over
// Private Service Connect, an internal load balancer or Direct VPC egress;
// they are never addresses in a VPC.
#ServerlessProductName: "\\b(Cloud Run|Cloud Run functions|Cloud Functions|App Engine|Vertex AI)\\b"
#ServerlessProductIcons: ["cloud-run", "vertex-ai", "serverless"]

// Managed data products with a private IP live in a Google-managed producer
// network, reached from the customer's VPC over private services access
// peering or a Private Service Connect endpoint; they are not addresses in
// the VPC. Spanner, Firestore and Bigtable are Google APIs and belong with
// those.
#ManagedDataProductName:  "\\b(Cloud SQL|AlloyDB|Memorystore)\\b"
#ManagedDataProductIcons: ["cloud-sql"]

// Global resources have no region; a hub drawn inside a region reads as
// regional. Only products whose resource model has no location belong here:
// Cloud Armor and Cloud DNS also have regional policies and zones.
#GlobalProductName: "\\b(Network Connectivity Center)\\b"

// The gcp walk over the body. Each node carries:
//   _nameless     the titles of product items with no subtitle and no doc or
//                 ask fact
//   _apiProducts  the titles of product items that are Google APIs
//                 (#ApiProductName, #ApiProductIcons)
//   _serverlessProducts, _managedDataProducts, _globalProducts  the same for
//                 serverless products, managed data products and global
//                 resources (#ServerlessProductName, #ServerlessProductIcons,
//                 #ManagedDataProductName, #ManagedDataProductIcons,
//                 #GlobalProductName)
//   _apisLabels   the labels of apis Boxes below it with no gcp Box between
// Tint pairing is the core's, in #GrammarNesting (cue/grammar.cue).
#Node: {
	tag: string
	if tag == "Row" || tag == "Col" || tag == "Lanes" || tag == "View" {
		children: [...#Node]
		_nameless: {for c in children {c._nameless}}
		_apiProducts: {for c in children {c._apiProducts}}
		_serverlessProducts: {for c in children {c._serverlessProducts}}
		_managedDataProducts: {for c in children {c._managedDataProducts}}
		_globalProducts: {for c in children {c._globalProducts}}
		_apisLabels: {for c in children {c._apisLabels}}
	}
	if tag == "Box" {
		kind:  string
		label: string
		children: [...#Node]
		_nameless: {for c in children {c._nameless}}
		_apiProducts: {for c in children {c._apiProducts}}
		_serverlessProducts: {for c in children {c._serverlessProducts}}
		_managedDataProducts: {for c in children {c._managedDataProducts}}
		_globalProducts: {for c in children {c._globalProducts}}
		_apisLabels: {
			if kind != "gcp" for c in children {c._apisLabels}
			if kind == "apis" {(label): true}
		}

		// Google APIs are reached over Private Google Access, never placed in
		// a VPC; they belong in an apis Box beside it.
		if kind == "vpc" {
			_productInsideVpc: {
				for k, _ in _apiProducts {(k): "product" & "sits inside a vpc; draw it in an apis box"}
			}
			// A serverless product is not an address in a VPC: it is reached over
			// Private Service Connect, an internal load balancer or Direct VPC
			// egress. Draw it beside the vpc.
			_serverlessInsideVpc: {
				for k, _ in _serverlessProducts {(k): "product" & "sits inside a vpc; a serverless product is reached over Private Service Connect; draw it beside the vpc"}
			}
			// A managed data product with a private IP lives in a producer
			// network peered to the VPC or reached through a Private Service
			// Connect endpoint. Draw it beside the vpc and name the attachment.
			_managedDataInsideVpc: {
				for k, _ in _managedDataProducts {(k): "product" & "sits inside a vpc; a managed data product lives in a producer network reached over private services access or Private Service Connect; draw it beside the vpc and name the attachment"}
			}
		}
		// A global resource has no region; draw it outside every region Box.
		if kind == "region" {
			_globalInsideRegion: {
				for k, _ in _globalProducts {(k): "product" & "sits inside a region; a global resource sits outside every region"}
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
		_isApi: [
			if subtitle != _|_ if subtitle =~ #ApiProductName {true},
			if icon != _|_ if list.Contains(#ApiProductIcons, icon) {true},
		]
		_apiProducts: {if kind == "product" && len(_isApi) > 0 {(title): true}}
		_isServerless: [
			if subtitle != _|_ if subtitle =~ #ServerlessProductName {true},
			if icon != _|_ if list.Contains(#ServerlessProductIcons, icon) {true},
		]
		_serverlessProducts: {if kind == "product" && len(_isServerless) > 0 {(title): true}}
		_isManagedData: [
			if subtitle != _|_ if subtitle =~ #ManagedDataProductName {true},
			if icon != _|_ if list.Contains(#ManagedDataProductIcons, icon) {true},
		]
		_managedDataProducts: {if kind == "product" && len(_isManagedData) > 0 {(title): true}}
		_isGlobal: [
			if subtitle != _|_ if subtitle =~ #GlobalProductName {true},
		]
		_globalProducts: {if kind == "product" && len(_isGlobal) > 0 {(title): true}}
		_apisLabels: {}
	}
	if !list.Contains(["Row", "Col", "Lanes", "View", "Box", "Item"], tag) {
		_nameless: {}
		_apiProducts: {}
		_serverlessProducts: {}
		_managedDataProducts: {}
		_globalProducts: {}
		_apisLabels: {}
	}
	...
}
