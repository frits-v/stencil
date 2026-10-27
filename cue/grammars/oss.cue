// The oss grammar: plain's kinds for a figure of self-hosted open-source
// components, with each project's own logo as its icon. A plain page becomes
// an oss page by setting "grammar": "oss" and adding icons; every kind keeps
// its meaning, and the cluster container is the one kind plain does not have.
// `grammar` is exported to crates/stencil-model/grammars/oss.json.
package oss

import (
	"list"

	"github.com/frits-v/stencil/cue:stencil"
	"github.com/frits-v/stencil/cue/grammars:plain"
)

grammar: stencil.#Grammar & {
	name: "oss"
	containers: [
		{
			name:     "system"
			role:     "frame"
			tintable: false
			border: {pattern: "solid", width: 3}
			padding: 0
			radius:  10
			label:   "bar"
			parents: ["page"]
		},
		{
			name:     "boundary"
			role:     "boundary"
			tone:     "strong"
			tintable: false
			border: {pattern: "dashed", width: 2}
			padding: 10
			radius:  8
			label:   "plain"
			parents: ["page", "system", "group", "tile"]
		},
		{
			name:     "group"
			role:     "group"
			tone:     "neutral"
			tintable: true
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "system", "boundary", "group", "tile"]
		},
		{
			name:     "tile"
			role:     "tile"
			tone:     "highlight"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "system", "boundary", "tile"]
		},
		{
			// A Kubernetes cluster or namespace: pink, no border, never tinted.
			name:     "cluster"
			role:     "group"
			tone:     "soft"
			tintable: false
			border: {pattern: "none", width: 0}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["system", "boundary", "group", "tile"]
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
			parents: ["page", "system", "boundary", "group", "tile", "cluster"]
		},
	]
	// plain's item kinds in plain's order. service, store, queue and network
	// draw from the oss pack; each row lists the names a subtitle uses for the
	// project, and a row without an icon is a project with no bundled logo.
	items: [for entry in _itemKinds {
		name:  entry.name
		icons: entry.icons
		products: entry.products
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		shape: entry.shape
	}]
	remembered: []
}

_itemKinds: [
	{
		name:  "service"
		icons: "oss"
		shape: "block"
		products: [
			{icon: "airflow", class: "product", names: ["Apache Airflow", "Airflow"]},
			{icon: "argo", class: "product", names: ["Argo", "Argo CD", "Argo Workflows"]},
			{icon: "docker", class: "product", names: ["Docker", "Docker Engine", "Docker Compose"], shape: "tower"},
			{icon: "grafana", class: "product", names: ["Grafana", "Grafana OSS"]},
			{class: "product", names: ["Helm"]},
			{icon: "jaeger", class: "product", names: ["Jaeger"]},
			{icon: "jenkins", class: "product", names: ["Jenkins"]},
			{icon: "keycloak", class: "product", names: ["Keycloak"]},
			{icon: "kubernetes", class: "product", names: ["Kubernetes", "K8s"], shape: "tower"},
			{icon: "loki", class: "product", names: ["Grafana Loki", "Loki"]},
			{icon: "opentelemetry", class: "product", names: ["OpenTelemetry", "OTel", "OpenTelemetry Collector"]},
			{icon: "prometheus", class: "product", names: ["Prometheus"]},
			{class: "product", names: ["Apache Spark", "Spark"]},
			{icon: "vault", class: "product", names: ["HashiCorp Vault", "Vault"]},
		]
	},
	{name: "managed", icons: "none", shape: "block", products: []},
	{
		name:  "store"
		icons: "oss"
		shape: "cylinder"
		products: [
			{icon: "cassandra", class: "product", names: ["Apache Cassandra", "Cassandra"]},
			{icon: "clickhouse", class: "product", names: ["ClickHouse"]},
			{class: "product", names: ["Elasticsearch"]},
			{icon: "etcd", class: "product", names: ["etcd"]},
			{class: "product", names: ["MariaDB Server", "MariaDB"]},
			{class: "product", names: ["MinIO", "MinIO AIStor"]},
			{icon: "mongodb", class: "product", names: ["MongoDB", "MongoDB Community Edition", "MongoDB Atlas"]},
			{class: "product", names: ["MySQL"]},
			{icon: "opensearch", class: "product", names: ["OpenSearch", "OpenSearch Dashboards"]},
			{icon: "postgresql", class: "product", names: ["PostgreSQL", "Postgres"]},
			{icon: "redis", class: "product", names: ["Redis", "Redis Open Source"]},
			{icon: "valkey", class: "product", names: ["Valkey"]},
		]
	},
	{
		name:  "queue"
		icons: "oss"
		shape: "stack"
		products: [
			{icon: "kafka", class: "product", names: ["Apache Kafka", "Kafka"]},
			{icon: "nats", class: "product", names: ["NATS", "NATS JetStream"]},
			{class: "product", names: ["Apache Pulsar", "Pulsar"]},
			{icon: "rabbitmq", class: "product", names: ["RabbitMQ"]},
		]
	},
	{
		name:  "network"
		icons: "oss"
		shape: "tile"
		products: [
			{icon: "envoy", class: "product", names: ["Envoy Proxy", "Envoy"]},
			{icon: "haproxy", class: "product", names: ["HAProxy"]},
			{icon: "istio", class: "product", names: ["Istio"]},
			{icon: "linkerd", class: "product", names: ["Linkerd"]},
			{icon: "nginx", class: "product", names: ["NGINX", "NGINX Open Source"]},
			{icon: "traefik", class: "product", names: ["Traefik Proxy", "Traefik"]},
		]
	},
	{name: "external", icons: "none", shape: "block", products: []},
	{name: "person", icons: "none", shape: "figure", products: []},
	{name: "device", icons: "none", shape: "laptop", products: []},
]

_nesting: stencil.#GrammarNesting & {#grammar: grammar}

// An oss figure: the core page with the oss kinds, nesting and tint pairing,
// and plain's own rule through plain's walk: an external item sits outside
// every system Box.
#Page: _nesting.#KindedPage & {
	body: [...plain.#Node]
}
