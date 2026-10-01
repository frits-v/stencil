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
			{icon: "bigquery", class: "product", names: ["BigQuery"]},
			{icon: "cloud-run-flat", class: "product", names: ["Cloud Run", "Cloud Run functions"]},
			{icon: "cloud-run", class: "product", names: ["Cloud Run", "Cloud Run functions"]},
			{icon: "cloud-sql", class: "product", names: ["Cloud SQL"]},
			{icon: "cloud-storage", class: "product", names: ["Cloud Storage"]},
			{icon: "compute-engine", class: "product", names: ["Compute Engine"]},
			{icon: "compute", class: "category", names: ["Compute Engine", "Cloud Run", "Cloud Run functions", "App Engine", "Cloud Functions", "Batch", "Bare Metal Solution", "Google Cloud VMware Engine"]},
			{icon: "containers", class: "category", names: ["Google Kubernetes Engine", "GKE", "Cloud Run", "Artifact Registry"]},
			{icon: "data-analytics", class: "category", names: ["BigQuery", "Dataflow", "Dataproc", "Pub/Sub", "Looker", "Dataplex", "Cloud Data Fusion", "Cloud Composer", "Datastream", "Dataform"]},
			{icon: "databases", class: "category", names: ["Cloud SQL", "AlloyDB", "Spanner", "Firestore", "Bigtable", "Memorystore", "Database Migration Service"]},
			{icon: "devops", class: "category", names: ["Cloud Build", "Artifact Registry", "Cloud Deploy", "Infrastructure Manager"]},
			{icon: "gke", class: "product", names: ["Google Kubernetes Engine", "GKE"]},
			{icon: "hybrid", class: "category", names: ["Google Distributed Cloud", "Cloud Interconnect", "Dedicated Interconnect", "Partner Interconnect", "Cross-Cloud Interconnect"]},
			{icon: "integration", class: "category", names: ["Pub/Sub", "Application Integration", "Workflows", "Eventarc", "Apigee", "API Gateway", "Cloud Tasks", "Cloud Scheduler"]},
			{icon: "networking", class: "category", names: ["Cloud Load Balancing", "Cloud CDN", "Cloud DNS", "Cloud NAT", "Cloud Router", "Cloud VPN", "Cloud Interconnect", "Network Connectivity Center", "Private Service Connect", "Virtual Private Cloud"]},
			{icon: "observability", class: "category", names: ["Cloud Logging", "Cloud Monitoring", "Cloud Trace", "Cloud Profiler", "Error Reporting"]},
			{icon: "scc", class: "product", names: ["Security Command Center"]},
			{icon: "security-identity", class: "category", names: ["Security Command Center", "Cloud KMS", "Secret Manager", "Identity and Access Management", "Identity-Aware Proxy", "Cloud Armor", "VPC Service Controls", "Certificate Authority Service", "Sensitive Data Protection"]},
			{icon: "serverless", class: "category", names: ["Cloud Run", "Cloud Run functions", "Cloud Functions", "App Engine", "Workflows", "Eventarc"]},
			{icon: "storage", class: "category", names: ["Cloud Storage", "Filestore", "Persistent Disk", "Hyperdisk", "Backup and DR Service", "Storage Transfer Service", "Google Cloud NetApp Volumes"]},
			{icon: "vertex-ai", class: "product", names: ["Vertex AI"]},
		]
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
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

// The gcp walk over the body. Each node carries:
//   _tinted       the tinted lines and Boxes in its subtree, keyed "solid-2" or
//                 "region-1", each mapped to its tint slot
//   _nameless     the titles of product items with no subtitle and no doc or
//                 ask fact
//   _apiProducts  the titles of product items that are Google APIs
//                 (#ApiProductName, #ApiProductIcons)
//   _apisLabels   the labels of apis Boxes below it with no gcp Box between
// A Box also carries _tint, its effective tint as a list of zero or one slot,
// and a solid Pipe carries _solidTint the same way. Tint pairing compares
// slots; dash, gray and deny lines and untinted Boxes are outside it.
#Node: {
	tag: string
	if tag == "Row" || tag == "Col" || tag == "Lanes" {
		children: [...#Node]
		_tinted: {for c in children {c._tinted}}
		_nameless: {for c in children {c._nameless}}
		_apiProducts: {for c in children {c._apiProducts}}
		_apisLabels: {for c in children {c._apisLabels}}
		#SiblingTint
	}
	if tag == "Box" {
		kind:  string
		tint?: int
		label: string
		children: [...#Node]
		_kindData: [for c in grammar.containers if c.name == kind {c}]
		_tint: list.Take([
			for c in _kindData if c.tintable if tint != _|_ {tint},
			for c in _kindData if c.tintable if c.default_tint != _|_ {c.default_tint},
		], 1)
		_below: {for c in children {c._tinted}}
		_tinted: {_below, for t in _tint {"\(kind)-\(t)": t}}
		_nameless: {for c in children {c._nameless}}
		_apiProducts: {for c in children {c._apiProducts}}
		_apisLabels: {
			if kind != "gcp" for c in children {c._apisLabels}
			if kind == "apis" {(label): true}
		}
		#SiblingTint

		// Google APIs are reached over Private Google Access, never placed in
		// a VPC; they belong in an apis Box beside it.
		if kind == "vpc" {
			_productInsideVpc: {
				for k, _ in _apiProducts {(k): "product" & "sits inside a vpc; draw it in an apis box"}
			}
		}

		// A tinted Box holds no solid line, tee arm or Box of another tint at
		// any depth.
		for boxTint in _tint {
			_otherTintInsideZone: {
				for k, t in _below if t != boxTint {(k): "\(k)" & "inside \(kind) tint \(boxTint)"}
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
		_tinted: {}
		_nameless: {if kind == "product" && subtitle == _|_ && len(_hopFacts) == 0 {(title): true}}
		_isApi: [
			if subtitle != _|_ if subtitle =~ #ApiProductName {true},
			if icon != _|_ if list.Contains(#ApiProductIcons, icon) {true},
		]
		_apiProducts: {if kind == "product" && len(_isApi) > 0 {(title): true}}
		_apisLabels: {}
	}
	if tag == "Pipe" {
		line:  string
		tint?: int
		_solidTint: [if line == "solid" {[if tint != _|_ {tint}, 1][0]}]
		_tinted: {for t in _solidTint {"solid-\(t)": t}}
		_nameless: {}
		_apiProducts: {}
		_apisLabels: {}
	}
	if tag == "Tee" {
		line:  string
		tint?: int
		arms: [...]
		let spineTint = [if tint != _|_ {tint}, 1][0]
		_tinted: {
			if line == "solid" {"solid-\(spineTint)": spineTint}
			for arm in arms if arm.line == "solid"
			let armTint = [if arm.tint != _|_ {arm.tint}, 1][0] {"solid-\(armTint)": armTint}
		}
		_nameless: {}
		_apiProducts: {}
		_apisLabels: {}
	}
	if !list.Contains(["Row", "Col", "Lanes", "Box", "Item", "Pipe", "Tee"], tag) {
		_tinted: {}
		_nameless: {}
		_apiProducts: {}
		_apisLabels: {}
	}
	...
}

// A solid pipe that sits between two sibling Boxes must not touch a tinted
// Box of another slot. Row, Col, Lanes and Box embed this, so the check holds
// wherever siblings sit.
#SiblingTint: {
	children: [...]
	_pipeBesideZoneOfOtherTint: {
		for i, c in children if c.tag == "Pipe" for pipeTint in c._solidTint {
			for j in [i - 1, i + 1] if j >= 0 && j < len(children) {
				let neighbor = children[j]
				if neighbor.tag == "Box" for boxTint in neighbor._tint if boxTint != pipeTint {
					(c.label): "solid tint \(pipeTint) pipe" & "beside \(neighbor.kind) tint \(boxTint)"
				}
			}
		}
	}
}
