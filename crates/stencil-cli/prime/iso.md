# Iso

"projection": "iso" on the Page (or render and check with --projection iso) draws the body as a 30 degree isometric floor plan. The flat layout is the floor plan: filled Boxes become slabs, an untinted Box of the strong tone (gcp vpc) becomes a ring on its parent, items stand on their slab, pipes and links run over the slabs, and every label lies on its surface: a Box name on its slab's back strip, a tag on its wire.

Shapes. An Item stands as a solid of a fixed footprint with its icon on top and its name and subtitle on the floor in front. Shape (9): card tile tower cylinder stack block figure laptop phone. The shape is shape on the Item, else the shape of its icon row (databases cylinders, storage stacks, compute towers, networking tiles), else its kind's (gcp product and plain service, store and external block; person figure; device laptop), else card. shape card keeps the flat card as a raised slab with its text on top; a card stretched across a Row or Col is a wide empty slab with its name in one corner. Footprint by height in px: tile 64 by 6, block 64 by 18, tower 44 by 44, cylinder 56 by 36, stack 56 by 30, figure 44 by 56, laptop 56 by 40, phone 36 by 44.

Pipes. A Pipe is a tube with its tag on the tube top. PipeForm (2): tube band. "form": "band" on the Pipe draws a flat wide arrow on the floor for one wide flow, with its own legend entry. Give every pipe of one line and tint the same form: a band beside tubes of the same line reads as another kind of connection. Name the device a pipe reaches in from and to; without a target the end lands on the nearest solid across its center in the Box beside it, and pipes-land reports an end that stops short. Land on a Box only when the Box is the endpoint, such as a VLAN. A Tee spine stands on its left neighbour. A deny pipe points from the side that is refused to the side it protects: start it in the zone whose traffic is blocked.

Links. A solid link is a tube per leg with joints and a cone at the arrowed end; a dashed or deny link is one stroked line with a cone. A link lies at the height of the tallest slab it crosses, end to end, so a link between two zones runs over both. It meets a solid through a 40 px stub on the side it attaches to; an arrow into the top or left side stops at the top face's edge; set to_side bottom or right for a face. Every end reaches its node: a link from a figure or a device meets the sprite, not the edge of its footprint. Links that share a side of one node spread along it, and two links on one route draw side by side with staggered tags; when the lane is too narrow, iso-links-apart or iso-links-clear says so: give them opposite sides, a via point, or merge them into one link whose label names both.

Text. Text reads along x (down-right); a vertical pipe's tag and a tag on a vertical link leg read along y (up-right), or set axis on the Pipe or Link. Body text is 1.3 times its flat size, a Box name 1.25 and the frame name 1.4; paddings and gaps 1.75. Names never wrap; other floor text keeps lines of at least 120 px, so a long subtitle in a narrow Box wraps to a staircase: keep subtitles to the product name, or widen the Box. Each Box keeps open floor behind and left of what stands on it, by the height of its tallest item, so a page may need more width.

Size. A body narrower than the canvas is zoomed, up to 1.6 times, until it spans 80 percent of it. The drawn canvas runs up to about 1.5 times the page width, and text shrinks with it on a slide. Keep an iso page at 1400 or less, and run check with --print-width 10 for a slide figure. Render at --scale 2.

Checks. check adds four checks under iso; under flat they are not applicable.

| Check | Unit | Defect at | Message |
|---|---|---|---|
| iso-labels-clear | pair: two labels, label and later solid, label and Box edge, label and link, text and its block (iso only) | later label's owner | label OWNER at X,Y along A overlaps label OWNER; or is covered by NODE; or is crossed by an edge of slab NODE; or by /links/i; or leaves its block |
| iso-links-clear | link leg (iso only) | /links/i | leg N runs D px beside an edge of zone NODE, closer than 24; or turns back; or last leg is L px, under 2 arrowheads; or link runs L px between ends S px apart; or leg N is L px between two turns; or leg N crosses block NODE |
| iso-link-ends | link end (iso only) | /links/i | start or end lies D px off the outline of NODE; or D px from a corner of NODE, under 8; or D px outside zone NODE; or node NODE has no solid |
| iso-links-apart | link pair (iso only) | the later link | shares L px with /links/j; or runs D px beside /links/j, closer than T; or ends D px from /links/j on NODE |

Leave open floor: Row grow 0, items 64 apart, a 48 px gap between Boxes for a link leg along it, and overlapping spans for solids joined straight across.

Outputs. The measured JSON canvas is the layout canvas; projection.canvas is the drawn one, the SVG viewBox. Read via coordinates from the layout canvas.
