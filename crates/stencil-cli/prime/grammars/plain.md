# Grammar plain

A domain-neutral grammar for system diagrams: set "grammar": "plain" on the page. No item takes an icon, and no rule applies beyond the core, so remembered-constants is not applicable.

{{kinds}}

| Kind | Means |
|---|---|
| system | the outermost system: a bar holding the label over a body |
| boundary | a network, trust or security edge: dashed border, no fill |
| group | a locality, cluster or site; tint it to pair it with lines of the same slot |
| tile | an ownership scope: a team, account or tenant |
| service, store, external, person | the items: a running service, a data store, a system outside the figure's scope, a human actor |

Lanes: put the participants of a time-ordered flow as Items directly in a Lanes node, each with an id. A link with order 1-256 between two of its heads is a message: a straight arrow across one row of the band below the heads, rows in order top to bottom, each at least 36 px and tall enough for its tag; each head draws a dashed lifeline down to the band's bottom. Use solid for a call, dash for a reply, deny for a rejected call. Vet rejects an ordered link that is not between two heads of one Lanes node, and two messages of one node with the same order. No iso.

Figures that show the grammar, one per subject:

- examples/sequence.json: a request flow as a system Box holding one Lanes of five heads (person, services, a store) and ten ordered messages, one dash reply and one deny; authored in cue/figures/sequence.cue.
- examples/org.json: an org chart as a system Box (the company), a Row of three tile divisions, tinted groups for teams holding person items, and solid links for reporting with a dash for a dotted line.
- examples/onprem-network.json: a site as a system Box, two boundary firewall zones, tinted groups for VLANs, router, switch, firewall and servers as service and store items with built facts, one solid pipe per VLAN tinted like its group, and a deny pipe between the zones.
