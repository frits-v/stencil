# Iso

"projection": "iso" on the Page (or render and check with --projection iso) draws the body as a 30 degree isometric floor plan. The flat layout is the floor plan: filled Boxes become slabs, an untinted Box of the strong tone (gcp vpc) becomes a ring on its parent, items stand on their slab, pipes and links run over the slabs, and every label lies on its surface: a Box name on its slab's back strip, a tag on its wire.

Shapes. An Item stands as a solid of a fixed footprint with its icon on top and its name and subtitle on the floor in front. Shape (9): card tile tower cylinder stack block figure laptop phone. The shape is shape on the Item, else the shape of its icon row (databases cylinders, storage stacks, compute towers, networking tiles), else its kind's (gcp product and plain service, store and external block; person figure; device laptop), else card. shape card keeps the flat card as a raised slab with its text on top; stretched across a Row or Col it is a wide empty slab. Footprint by height in px: tile 64 by 6, block 64 by 18, tower 44 by 44, cylinder 56 by 36, stack 56 by 30, figure 44 by 56, laptop 56 by 40, phone 36 by 44.

Pipes. A Pipe is a tube with its tag on the tube top. PipeForm (2): tube band. "form": "band" on the Pipe draws a flat wide arrow on the floor for one wide flow; its legend entry takes "form": "band" too. A band beside tubes of one line reads as another connection. Name the device a pipe reaches in from and to; without a target the end lands on the nearest solid across its center in the Box beside it; pipes-land reports a short end. Land on a Box only when the Box is the endpoint, such as a VLAN. A Tee spine stands on its left neighbour. A deny pipe points from the side that is refused to the side it protects: start it in the zone whose traffic is blocked.

Links. A solid link is a tube per leg with joints and a cone at the arrowed end; a dashed or deny link is one stroked line. A link lies at the height of the tallest slab it crosses. It meets a solid through a 40 px stub on the side it attaches to; an arrow into the top or left side stops at the top face's edge; set to_side bottom or right for a face. A step between solids whose spans do not overlap sits mid-run, clear of edges and other links. Links that share a side of one node spread along it, and two links on one route draw side by side with staggered tags; when the lane is too narrow, iso-links-apart or iso-links-clear says so: give them opposite sides, a via point, or one link whose label names both.

Text. Text reads along x (down-right); a vertical pipe's tag and a tag on a vertical link leg read along y (up-right), or set axis. Body text is 1.3 times its flat size, a Box name 1.25, the frame name 1.4; paddings and gaps 1.75. Names never wrap; other floor text keeps lines of at least 120 px: keep subtitles short, or widen the Box. Each Box keeps floor behind and left of what stands on it, by its tallest item's height, so a page may need more width.

Size. A body narrower than the canvas is zoomed, up to 1.6 times, until it spans 80 percent of it. The drawn canvas runs up to about 1.5 times the page width, and text shrinks with it on a slide. Keep an iso page at 1400 or less and check with --print-width 10 for a slide. Render at --scale 2.

Checks. check adds four checks under iso, not applicable under flat.

| Check | Unit | Defect at | Message |
|---|---|---|---|
| iso-labels-clear | pair: labels, label and solid, label and edge, label and wire, tag and foreign wire, tag and its own, text and block, block and slab (iso only) | later label's owner | label OWNER at X,Y along A overlaps label OWNER, or lies D px from it, under 4; is covered by NODE, or lies D px from it, under 8; is crossed by an edge of slab NODE, by /links/i or by pipe NODE; is passed under by /links/i; tag of OWNER lies nearer /links/j than its own path; leaves its block; has no block; block NODE overlaps the lip of slab ZONE |
| iso-links-clear | link leg (iso only) | /links/i | leg N runs D px beside an edge of zone NODE, closer than 24; turns back; steps D px from the start or end, within a stub; runs back against leg M across a leg under a stub; last leg under 2 arrowheads; link runs L px between ends S px apart; leg N is L px between two turns; crosses block NODE |
| iso-link-ends | link end (iso only) | /links/i | start or end lies D px off the outline of NODE; or D px from a corner of NODE, under 8; or D px outside zone NODE; or node NODE has no solid |
| iso-links-apart | link pair (iso only) | the later link | shares L px with /links/j; or runs D px beside /links/j, closer than T; or ends D px from /links/j on NODE |

Leave open floor: Row grow 0, items 64 apart, a 48 px gap between Boxes for a link leg, and overlapping spans for solids joined straight across.

Outputs. Measured JSON canvas and links are the layout ones; projection.canvas (the SVG viewBox) and projection.links are the drawn canvas and paths. via points use layout px.
