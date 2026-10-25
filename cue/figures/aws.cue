// hybrid: an on-prem site over AWS Direct Connect into one region, a VPC
// across two availability zones, and the regional services the VPC reaches
// over endpoints beside it.
// Negative cases in check.sh edit it for the aws rules.
package aws

import (
	"github.com/frits-v/stencil/cue/grammars:aws"
)

hybrid: aws.#Page & {
	title:   "On-prem reaches a two-AZ VPC over Direct Connect; S3 and the secrets stay outside it"
	kicker:  "AWS Direct Connect · one region, two availability zones"
	lede:    "The database runs in both zones. Amazon S3, Amazon DynamoDB and AWS Secrets Manager are regional services the VPC reaches over endpoints."
	badge:   "Customer"
	grammar: "aws"

	body: [{
		tag: "Row"
		gap: 8
		grow: [0, 0, 1]
		children: [
			{
				tag:   "Box"
				kind:  "onprem"
				tint:  1
				label: "On-prem data center"
				children: [
					{tag: "Item", kind: "product", icon: "networking-content-delivery", title: "Customer router", subtitle: "port toward AWS · BGP"},
					{tag: "Item", kind: "person", title: "Operators", subtitle: "run the release"},
				]
			},
			{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "Direct Connect", sub: "dedicated connection · BGP"},
			{
				tag:   "Box"
				kind:  "aws"
				label: "AWS Cloud"
				children: [{
					tag:   "Box"
					kind:  "region"
					label: "Region"
					children: [
						{tag: "Fact", text: "Region and zones from the landing-zone design"},
						{
							tag:   "Box"
							kind:  "vpc"
							label: "Application VPC"
							children: [
								{tag: "Item", kind: "product", icon: "vpc", title: "Virtual private gateway", subtitle: "Amazon-side ASN from the gateway configuration"},
								{tag: "Item", kind: "product", icon: "vpc", title: "S3 gateway endpoint", subtitle: "gateway endpoint · Amazon S3", facts: [{text: "on the private route tables"}]},
								{
									tag: "Row"
									children: [
										{
											tag:   "Box"
											kind:  "az"
											label: "Availability Zone A"
											children: [{
												tag:   "Box"
												kind:  "subnet"
												label: "Private subnet A"
												children: [
													{tag: "Item", kind: "product", icon: "rds", title: "Orders database", subtitle: "Amazon RDS for PostgreSQL · primary"},
													{tag: "Item", kind: "product", icon: "lambda", title: "Order worker", subtitle: "AWS Lambda · VPC-attached"},
													{tag: "Item", kind: "product", icon: "privatelink", title: "Secrets endpoint", subtitle: "interface endpoint", facts: [{text: "for AWS Secrets Manager"}]},
													{tag: "Fact", text: "No route to an internet gateway"},
												]
											}]
										},
										{
											tag:   "Box"
											kind:  "az"
											label: "Availability Zone B"
											children: [{
												tag:   "Box"
												kind:  "subnet"
												label: "Private subnet B"
												children: [
													{tag: "Item", kind: "product", icon: "rds", title: "Orders standby", subtitle: "Amazon RDS for PostgreSQL · Multi-AZ standby"},
													{tag: "Item", kind: "product", icon: "route-53", title: "Inbound resolver", subtitle: "Route 53 Resolver inbound endpoint"},
												]
											}]
										},
									]
								},
							]
						},
						{
							tag:   "Box"
							kind:  "services"
							label: "Regional services"
							children: [
								{tag: "Item", kind: "product", icon: "s3", title: "Order exports", subtitle: "Amazon S3"},
								{tag: "Item", kind: "product", icon: "dynamodb", title: "Session table", subtitle: "Amazon DynamoDB"},
								{tag: "Item", kind: "product", icon: "secrets-manager", title: "Database credentials", subtitle: "AWS Secrets Manager"},
							]
						},
					]
				}]
			},
		]
	}]

	legend: [
		{line: "solid", tint: 1, text: "On-prem ↔ Region"},
	]
}
