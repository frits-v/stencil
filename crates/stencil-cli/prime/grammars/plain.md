# Grammar plain

The default grammar: a page without "grammar" is a plain page, and a figure that names no vendor's products stays here. No item takes an icon, so icon-matches-product and remembered-constants are not applicable.

{{kinds}}

| Kind | Means |
|---|---|
| system | the outermost system: a bar holding the label over a body |
| boundary | a network, trust or security edge: dashed border, no fill |
| group | a locality, cluster or site; tint it to pair it with lines of the same slot |
| tile | an ownership scope: a team, account or tenant |
| assembly | the parts one actor runs, on a design sheet only: a dash-dot outline with a tab label |
| service | a running service the owner builds and operates; a block under iso |
| managed | a service the owner uses but does not run: a vendor's database engine, model endpoint or scheduler; a block |
| store | a database, bucket or file share; a cylinder |
| queue | a queue, topic or stream; a stack |
| network | a router, switch, firewall, load balancer or gateway; a tile |
| external | a system outside the figure's scope, drawn outside the system frame; a block |
| person | a human actor; a standing figure |
| device | an end-user device; a laptop, or a phone with "shape": "phone" |

CUE enforces two rules (`stencil prime cue`): an external item sits outside every system Box, and a solid pipe of one slot sits beside or inside no tinted group of another slot, so a group and the solid lines through it share a tint; dash, gray and deny stay outside the rule.

House style, what an unconfigured figure looks like: theme center; one system frame when the figure has one owner, none when it compares peers; gray for internal calls, solid tint 1 for the primary path and tint 2 for a second, dash for failover, control plane or identity, deny for a blocked path, each written in the legend as its meaning; a title that names the part (Jobs API), a subtitle that says what it is (owns job state), as-built names in facts with source built.

Lanes: put the participants of a time-ordered flow as Items directly in a Lanes node, each with an id; the heads take equal columns. A link with order 1-256 between two of its heads is a message: a straight arrow across one row of the band below the heads, rows in order top to bottom, each at least 36 px and tall enough for its tag; each head draws a dashed lifeline down to the band's bottom. Use solid for a call, dash for a reply, deny for a rejected call. Vet rejects an ordered link that is not between two heads of one Lanes node, and two messages of one node with the same order. No iso.

A design sheet under plain: take `stencil prime onepager`, set "grammar": "plain", make the gcp Box a system Box, each product Item a service, managed, store, queue or network, and drop every icon. vet names each one missed: kind-unknown, icon-outside-pack.

Figures that show the grammar, one per subject:

- examples/sequence.json: a request flow as a system Box holding one Lanes of five heads (person, services, a queue) and ten ordered messages, one dash reply and one deny; authored in cue/figures/sequence.cue.
- examples/org.json: an org chart as a system Box (the company), a Row of three tile divisions, tinted groups for teams holding person items, and solid links for reporting with a dash for a dotted line.
- examples/onprem-network.json: a site as a system Box, two boundary firewall zones, tinted groups for VLANs, router, switch and firewall as network items and servers as service and store items with built facts, one solid pipe per VLAN tinted like its group, and a deny pipe between the zones.
