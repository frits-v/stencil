// g7: Dedicated Interconnect at 99.99%. Two metros, two EADs, four VLAN
// attachments, one transit VPC spanning two regions, a Cloud Router per region.
package gcp

import (
	"github.com/frits-v/stencil/cue/grammars:gcp"
)

// The customer canvas. It is written open on exactly the slots the internal
// canvas fills: the canvas value (a default) and the workshop labels (empty
// defaults, concatenated onto box labels). Everything else is concrete, so a
// canvas derived by unification can add labels but cannot change a pipe.
customer: gcp.#Page & {
	canvas:  *"customer" | "internal"
	grammar: "gcp"

	_workshop: {
		metro1:   *"" | string
		metro2:   *"" | string
		vpc:      *"" | string
		regionA:  *"" | string
		regionB:  *"" | string
		failover: *"" | string
	}
	// Workshop labels stay off the customer canvas.
	if canvas == "customer" {
		_workshop: [string]: ""
	}

	title:  "Four lines. Two metros. Two regions. Failover sits between the regions."
	kicker: "Dedicated Interconnect 99.99% · two metros, two regions"
	lede:   "VLAN IDs, EAD, and BGP sit on the hops. A reader following the lines sees four attachments, not one bundled cable."
	foot:   "Illustrative topology · each metro has one VLAN in each EAD"

	body: [{
		tag: "Row"
		gap: 8
		grow: [0, 0, 1]
		children: [
			{
				// Each metro box takes half the row height, so gutter half i
				// sits level with metro i.
				tag: "Col"
				grow: [1, 1]
				children: [
					{
						tag:   "Box"
						kind:  "onprem"
						tint:  1
						label: "On-prem · Metro 1" + _workshop.metro1
						children: [
							{tag: "Item", kind: "product", icon: "hybrid", title: "On-prem router 1", subtitle: "port toward Google"},
							{tag: "Item", kind: "product", icon: "hybrid", title: "On-prem router 2", subtitle: "port toward Google"},
						]
					},
					{
						tag:   "Box"
						kind:  "onprem"
						tint:  2
						label: "On-prem · Metro 2" + _workshop.metro2
						children: [
							{tag: "Item", kind: "product", icon: "hybrid", title: "On-prem router 3", subtitle: "port toward Google"},
							{tag: "Item", kind: "product", icon: "hybrid", title: "On-prem router 4", subtitle: "port toward Google"},
						]
					},
				]
			},
			{
				// One pipe per VLAN attachment, in router order. Each half sits
				// beside its metro, so the halves share the column height.
				tag: "Col"
				grow: [1, 1]
				children: [
					{
						tag:     "Col"
						gap:     12
						justify: "center"
						children: [
							{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "VLAN 1", sub: "EAD 1 · BGP"},
							{tag: "Pipe", dir: "h", line: "solid", tint: 1, label: "VLAN 2", sub: "EAD 2 · BGP"},
						]
					},
					{
						tag:     "Col"
						gap:     12
						justify: "center"
						children: [
							{tag: "Pipe", dir: "h", line: "solid", tint: 2, label: "VLAN 3", sub: "EAD 1 · BGP"},
							{tag: "Pipe", dir: "h", line: "solid", tint: 2, label: "VLAN 4", sub: "EAD 2 · BGP"},
						]
					},
				]
			},
			{
				tag:   "Box"
				kind:  "gcp"
				label: "Google Cloud"
				children: [{
					tag:   "Box"
					kind:  "vpc"
					label: "Transit VPC · one network, two regions" + _workshop.vpc
					children: [
						{
							tag:   "Box"
							kind:  "region"
							tint:  1
							label: "Region A" + _workshop.regionA
							children: [
								{tag: "Item", kind: "product", icon: "networking", title: "Cloud Router A", subtitle: "private ASN · RFC 6996"},
								{tag: "Fact", text: "BGP peering · link-local /29 · keepalive and hold from the Cloud Router BGP-timer doc"},
							]
						},
						{
							tag:   "Pipe"
							dir:   "v"
							line:  "dash"
							label: "failover · Region A ↔ Region B"
							if _workshop.failover != "" {
								sub: _workshop.failover
							}
						},
						{
							tag:   "Box"
							kind:  "region"
							tint:  2
							label: "Region B" + _workshop.regionB
							children: [
								{tag: "Item", kind: "product", icon: "networking", title: "Cloud Router B", subtitle: "same private ASN as Region A"},
								{tag: "Fact", text: "Advertise the Google VIP ranges named on the Private Google Access doc"},
							]
						},
					]
				}]
			},
		]
	}]

	legend: [
		{line: "solid", tint: 1, text: "Metro 1 ↔ Region A"},
		{line: "solid", tint: 2, text: "Metro 2 ↔ Region B"},
		{line: "dash", text: "region failover, not a fifth line"},
	]
}

// The internal canvas: the same pipes, plus workshop labels. The values are
// instructions naming where each label comes from, not the labels themselves.
internal: customer & {
	canvas: "internal"
	_workshop: {
		metro1:   " · colo from the workshop"
		metro2:   " · colo from the workshop"
		vpc:      " · dynamic routing mode from the 99.99% topology doc"
		regionA:  " · region name from the workshop"
		regionB:  " · region name from the workshop"
		failover: "failover name from the workshop"
	}
}
