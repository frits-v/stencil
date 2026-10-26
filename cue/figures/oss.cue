// A web stack on Kubernetes: NGINX in front of the storefront, Kafka between
// the storefront and the order worker, PostgreSQL and Redis behind them, and
// an identity provider beside the platform.
// Negative cases in check.sh edit it for the oss rules.
package oss

import (
	"github.com/frits-v/stencil/cue/grammars:oss"
)

figure: oss.#Page & {
	title:   "Orders reach PostgreSQL through Kafka, and sign-in stays with the identity provider"
	kicker:  "Self-hosted web stack · one cluster"
	lede:    "NGINX routes requests to the storefront, which publishes orders to Kafka for the worker to write. Sessions live in Redis."
	badge:   "Internal"
	grammar: "oss"
	body: [{
		tag: "Row"
		gap: 8
		children: [
			{tag: "Item", kind: "external", title: "Identity provider", subtitle: "OpenID Connect"},
			{tag: "Pipe", dir: "h", line: "dash", label: "sign-in"},
			{
				tag:   "Box"
				kind:  "system"
				label: "Shop platform"
				children: [{
					tag:   "Box"
					kind:  "cluster"
					label: "Production cluster"
					children: [{
						tag: "Row"
						gap: 8
						children: [
							{tag: "Item", kind: "network", icon: "nginx", title: "Ingress", subtitle: "NGINX"},
							{tag: "Item", kind: "service", title: "Storefront", subtitle: "web front end"},
							{tag: "Item", kind: "queue", icon: "kafka", title: "Order events", subtitle: "Apache Kafka"},
							{tag: "Item", kind: "service", title: "Order worker", subtitle: "writes orders"},
							{tag: "Item", kind: "store", icon: "postgresql", title: "Orders", subtitle: "PostgreSQL"},
							{tag: "Item", kind: "store", icon: "redis", title: "Sessions", subtitle: "Redis"},
							{tag: "Item", kind: "store", title: "Catalog", subtitle: "MySQL"},
						]
					}]
				}]
			},
		]
	}]
	legend: [
		{line: "dash", text: "identity"},
	]
}
