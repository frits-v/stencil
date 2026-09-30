// Derived view: a g7-shaped #Page to the coordinate JSON that the
// eraser-diagrams stencil library (g7-stencil.mjs) renders. Every x and y is
// computed here from column widths, row indices and fixed item heights.
//
// The view accepts one shape, the g7 shape, and asserts it:
//   body: [Row [ Col [onprem Zones of cards],
//                Col [ Col [h Pipes], ... ], one pipe per card, in card order,
//                Zone gcp [ Zone vpc [Zone, v Pipe, Zone, ...] ] ]]
package stencil

import (
	"list"
	"strings"
)

#EraserG7: {
	T: #Page

	// Geometry parameters for the g7 shape.
	P: {
		margin:  20  // page padding, left and top
		headerH: 90  // kicker, h1 and lede
		leftW:   200 // on-prem column
		gutterW: 160 // VLAN pipe gutter
		zoneGap: 20  // between stacked on-prem zones
		itemH: {Pcard: 46, Fact: 26}
		onprem: {head: 32, gap: 12, pad: 14, inset: 12}
		region: {head: 28, gap: 6, pad: 12, inset: 12}
		gcpHead:  52 // blue bar plus inner padding
		gcpPad:   18
		gcpInset: 16
		vpcHead:  34
		vpcPad:   12
		vpcInset: 16
		// Vertical pipe between region bands. The renderer anchors the tag's
		// bottom edge at the midpoint of the path, so the tag occupies the
		// upper half of the gap; a two-line tag needs a gap of about twice
		// its height. The one-line values reproduce the reference render.
		failover: {
			oneLine: {gap: 48, labelY: 6, tagH: 36}
			twoLine: {gap: 100, labelY: 3, tagH: 47}
		}
		labelCharW: 7 // zone label width budget per character at 12px bold
		labelLineH: 15
		tagCharW:   7 // pipe tag width budget per character at 12px bold
		tagPadW:    26
		legendGap:  16
		footGap:    22
	}

	let pageW = [if T.width != _|_ {T.width}, 1280][0] - 2*P.margin
	let y0 = P.margin + P.headerH

	// Shape assertion: unifying T with this pattern fixes the list lengths
	// and tags the view relies on.
	T: body: [{
		tag: "Row"
		children: [
			{tag: "Col"},
			{tag: "Col", children: [...{tag: "Col"}]},
			{tag: "Zone", kind: "gcp", children: [{tag: "Zone", kind: "vpc"}]},
		]
	}]
	_row:  T.body[0]
	_left: _row.children[0]
	_gutter: [for half in _row.children[1].children for pipe in half.children {pipe}]
	_gcp:   _row.children[2]
	_vpc:   _gcp.children[0]
	_stack: _vpc

	// Left column: stacked on-prem zones.
	_leftH: [for z in _left.children {(#Stack & {S: P.onprem, H: P.itemH, L: _labelFit, children: z.children, label: z.label, innerW: P.leftW - 2*P.onprem.inset}).height}]
	_leftY: [for i, _ in _leftH {y0 + list.Sum([for j, h in _leftH if j < i {h}]) + P.zoneGap*i}]
	_leftStacks: [for i, z in _left.children {
		#Stack & {
			S:       P.onprem, H: P.itemH, L: _labelFit, children: z.children, label: z.label
			zoneId:  "onprem-\(i)"
			originX: P.margin + P.onprem.inset
			originY: _leftY[i]
			innerW:  P.leftW - 2*P.onprem.inset
		}
	}]
	_leftColumnH: list.Sum(_leftH) + P.zoneGap*(len(_leftH)-1)

	// Cards in the left column, flattened in reading order. Gutter pipe p
	// leaves card p.
	_cards: [for i, z in _left.children for k, c in z.children if c.tag == "Pcard" {
		id:       _leftStacks[i].entities[k].id
		y:        _leftStacks[i].entities[k].y
		zoneKind: z.kind
	}]
	_onePipePerCard: len(_gutter) & len(_cards)

	// Google Cloud frame, VPC and the region stack.
	let gcpX = P.margin + P.leftW + P.gutterW
	let gcpW = P.margin + pageW - gcpX
	let vpcX = gcpX + P.gcpInset
	let vpcY = y0 + P.gcpHead
	let vpcW = gcpW - 2*P.gcpInset
	let stackX = vpcX + P.vpcInset
	let stackW = vpcW - 2*P.vpcInset
	_stackH: [for i, c in _stack.children {
		if c.tag == "Zone" {(#Stack & {S: P.region, H: P.itemH, L: _labelFit, children: c.children, label: c.label, innerW: stackW - 2*P.region.inset}).height}
		if c.tag == "Pipe" {_failover["\(i)"].gap}
	}]
	_stackY: [for i, _ in _stackH {vpcY + P.vpcHead + list.Sum([for j, h in _stackH if j < i {h}])}]
	let vpcH = P.vpcHead + list.Sum(_stackH) + P.vpcPad
	let gcpH = P.gcpHead + vpcH + P.gcpPad
	_regionStacks: [for i, c in _stack.children if c.tag == "Zone" {
		#Stack & {
			S:       P.region, H: P.itemH, L: _labelFit, children: c.children
			zoneId:  "region-\(i)"
			originX: stackX + P.region.inset
			originY: _stackY[i]
			innerW:  stackW - 2*P.region.inset
			kind:    c.kind
			label:   c.label
			index:   i
		}
	}]
	_labelFit: {charW: P.labelCharW, lineH: P.labelLineH}
	_failover: {for i, c in _stack.children if c.tag == "Pipe" {
		"\(i)": [if c.sub != _|_ {P.failover.twoLine}, P.failover.oneLine][0]
	}}
	_regionId: {for i, c in _stack.children if c.tag == "Zone" {(c.kind): "region-\(i)"}}

	// Pipe colour decides both ends: a blue pipe leaves an onprem-a card and
	// lands in region-a, pink the b pair. Any other kind in the gutter fails.
	// Definitions, so a lookup with any other kind is an error rather than
	// an incomplete value that vet skips.
	#MetroFor: {blue: "onprem-a", pink: "onprem-b"}
	#RegionFor: {blue: "region-a", pink: "region-b"}

	let rowH = list.Max([_leftColumnH, gcpH])
	let legendY = y0 + rowH + P.legendGap

	_swatch: {gray: "Solid gray", blue: "Solid blue", pink: "Solid pink", dash: "Dashed", deny: "Dashed red"}

	out: {
		entities: list.Concat([
			[
				{tag: "Note", id: "kicker", kind: "kicker", x: P.margin, y: P.margin, width: pageW, text: "\(strings.ToUpper(T.canvas)) · \(T.kicker)"},
				{tag: "Note", id: "h1", kind: "h1", x: P.margin, y: P.margin + 20, width: pageW, text: T.title},
				{tag: "Note", id: "lede", kind: "lede", x: P.margin, y: P.margin + 50, width: pageW, text: T.lede},
			],
			list.Concat([for i, z in _left.children {
				list.Concat([
					[{tag: "Zone", id: "onprem-\(i)", kind: z.kind, title: z.label, x: P.margin, y: _leftY[i], width: P.leftW, height: _leftH[i]}],
					_leftStacks[i].entities,
				])
			}]),
			[
				{tag: "Zone", id: "gcp", kind: "gcp", title: _gcp.label, x: gcpX, y: y0, width: gcpW, height: gcpH},
				{tag: "Zone", id: "vpc", kind: "vpc", title: _vpc.label, containerId: "gcp", x: vpcX, y: vpcY, width: vpcW, height: vpcH},
			],
			list.Concat([for r in _regionStacks {
				list.Concat([
					[{tag: "Zone", id: r.zoneId, kind: r.kind, title: r.label, containerId: "vpc", x: stackX, y: _stackY[r.index], width: stackW, height: r.height}],
					r.entities,
				])
			}]),
			[
				{tag: "Note", id: "legend", kind: "legend", x: P.margin, y: legendY, width: pageW, text: strings.Join([for e in T.legend {"**\(_swatch[e.kind])** = \(e.text)"}], " &nbsp;&nbsp;&nbsp; ")},
				if T.foot != _|_ {
					{tag: "Note", id: "foot", kind: "foot", x: P.margin, y: legendY + P.footGap, width: pageW, text: T.foot}
				},
			],
		])

		connections: [
			for p, pipe in _gutter {
				let w = list.Max([104, P.tagCharW*list.Max([len(strings.Runes(pipe.label)), [if pipe.sub != _|_ {len(strings.Runes(pipe.sub))}, 0][0]]) + P.tagPadW])
				tag:   "Pipe" & pipe.tag
				id:    "vlan-\(p)"
				from:  _cards[p].id
				to:    _regionId[#RegionFor[pipe.kind]]
				kind:  pipe.kind
				label: pipe.label
				if pipe.sub != _|_ {sub: pipe.sub}
				fromPort: "right"
				toPort:   "left"
				x:        P.margin + P.leftW
				y:        _cards[p].y + div(P.itemH.Pcard, 2)
				points: [{x: 0, y: 0}, {x: P.gutterW, y: 0}]
				labelPlacement: {x: div(P.gutterW-w, 2), y: -23, width: w, height: 47}
				_gutterPipeIsHorizontal: pipe.dir & "h"
				_leavesCardInItsMetro: {
					if _cards[p].zoneKind != #MetroFor[pipe.kind] {
						(pipe.label): "\(pipe.kind) pipe" & "leaves a card in \(_cards[p].zoneKind)"
					}
				}
			},
			for i, c in _stack.children if c.tag == "Pipe" {
				let w = list.Max([240, P.tagCharW*list.Max([len(strings.Runes(c.label)), [if c.sub != _|_ {len(strings.Runes(c.sub))}, 0][0]]) + P.tagPadW])
				tag:   "Pipe"
				id:    "stack-\(i)"
				from:  "region-\(i-1)" & _regionStacks[[for k, r in _regionStacks if r.index == i-1 {k}][0]].zoneId
				to:    "region-\(i+1)" & _regionStacks[[for k, r in _regionStacks if r.index == i+1 {k}][0]].zoneId
				kind:  c.kind
				label: c.label
				if c.sub != _|_ {sub: c.sub}
				fromPort: "bottom"
				toPort:   "top"
				x:        stackX + div(stackW, 2)
				y:        _stackY[i]
				points: [{x: 0, y: 0}, {x: 0, y: _failover["\(i)"].gap}]
				labelPlacement: {x: -div(w, 2), y: _failover["\(i)"].labelY, width: w, height: _failover["\(i)"].tagH}
				_stackPipeIsVertical: c.dir & "v"
			},
		]
	}
}

// A zone's children stacked top to bottom: label, items separated by gap,
// bottom padding. Emits one entity per Pcard or Fact.
#Stack: {
	S: {head: int, gap: int, pad: int, inset: int}
	H: {Pcard: int, Fact: int}
	L: {charW: int, lineH: int}
	children: [...#Node]
	label:   string
	zoneId:  string
	originX: int
	originY: int
	innerW:  int
	...

	// A zone label wider than the zone wraps; each extra line pushes the
	// items down.
	_labelLines: div(len(strings.Runes(label))*L.charW+innerW-1, innerW)
	_head:       S.head + (_labelLines-1)*L.lineH
	_h: [for c in children {H[c.tag]}]
	height: _head + list.Sum(_h) + S.gap*(len(_h)-1) + S.pad
	entities: [for k, c in children {
		id:          "\(zoneId)-\(k)"
		containerId: zoneId
		x:           originX
		y: originY + _head + list.Sum([for j, h in _h if j < k {h}]) + S.gap*k
		width: innerW
		if c.tag == "Pcard" {
			tag: "Pcard"
			if c.icon != _|_ {icon: c.icon}
			fn: c.fn
			pn: strings.Join([
				if c.pn != _|_ {c.pn},
				if c.fact != _|_ {c.fact},
				if c.ask != _|_ {"ask: \(c.ask)"},
			], " · ")
		}
		if c.tag == "Fact" {
			tag:  "Fact"
			text: c.text
		}
	}]
}

eraserCustomer: (#EraserG7 & {T: customer}).out
eraserInternal: (#EraserG7 & {T: internal}).out
