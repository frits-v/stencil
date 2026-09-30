// g7: Dedicated Interconnect at 99.99%. Two metros, two EADs, four VLAN
// attachments, one transit VPC spanning two regions, a Cloud Router per region.
package stencil

// The customer canvas. It is written open on exactly the slots the internal
// canvas fills: the canvas value (a default) and the workshop labels (empty
// defaults, concatenated onto zone labels). Everything else is concrete, so a
// canvas derived by unification can add labels but cannot change a pipe.
customer: #Page & {
	canvas: *"customer" | "internal"

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
				// Each metro zone takes half the row height, so gutter half i
				// sits level with metro i.
				tag: "Col"
				grow: [1, 1]
				children: [
					{
						tag:   "Zone"
						kind:  "onprem-a"
						label: "On-prem · Metro 1" + _workshop.metro1
						children: [
							{tag: "Pcard", icon: "hybrid", fn: "On-prem router 1", pn: "port toward Google"},
							{tag: "Pcard", icon: "hybrid", fn: "On-prem router 2", pn: "port toward Google"},
						]
					},
					{
						tag:   "Zone"
						kind:  "onprem-b"
						label: "On-prem · Metro 2" + _workshop.metro2
						children: [
							{tag: "Pcard", icon: "hybrid", fn: "On-prem router 3", pn: "port toward Google"},
							{tag: "Pcard", icon: "hybrid", fn: "On-prem router 4", pn: "port toward Google"},
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
							{tag: "Pipe", dir: "h", kind: "blue", label: "VLAN 1", sub: "EAD 1 · BGP"},
							{tag: "Pipe", dir: "h", kind: "blue", label: "VLAN 2", sub: "EAD 2 · BGP"},
						]
					},
					{
						tag:     "Col"
						gap:     12
						justify: "center"
						children: [
							{tag: "Pipe", dir: "h", kind: "pink", label: "VLAN 3", sub: "EAD 1 · BGP"},
							{tag: "Pipe", dir: "h", kind: "pink", label: "VLAN 4", sub: "EAD 2 · BGP"},
						]
					},
				]
			},
			{
				tag:   "Zone"
				kind:  "gcp"
				label: "Google Cloud"
				children: [{
					tag:   "Zone"
					kind:  "vpc"
					label: "Transit VPC · one network, two regions" + _workshop.vpc
					children: [
						{
							tag:   "Zone"
							kind:  "region-a"
							label: "Region A" + _workshop.regionA
							children: [
								{tag: "Pcard", icon: "networking", fn: "Cloud Router A", pn: "private ASN · RFC 6996"},
								{tag: "Fact", text: "BGP peering · link-local /29 · keepalive and hold from the Cloud Router BGP-timer doc"},
							]
						},
						{
							tag:   "Pipe"
							dir:   "v"
							kind:  "dash"
							label: "failover · Region A ↔ Region B"
							if _workshop.failover != "" {
								sub: _workshop.failover
							}
						},
						{
							tag:   "Zone"
							kind:  "region-b"
							label: "Region B" + _workshop.regionB
							children: [
								{tag: "Pcard", icon: "networking", fn: "Cloud Router B", pn: "same private ASN as Region A"},
								{tag: "Fact", text: "Advertise the Google VIP ranges named on the Private Google Access doc"},
							]
						},
					]
				}]
			},
		]
	}]

	legend: [
		{kind: "blue", text: "Metro 1 ↔ Region A"},
		{kind: "pink", text: "Metro 2 ↔ Region B"},
		{kind: "dash", text: "region failover, not a fifth line"},
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
