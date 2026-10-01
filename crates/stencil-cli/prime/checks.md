# Checks

`stencil vet` parses, applies the vet rules and runs remembered-constants, legend-consistency and icon-matches-product. `stencil check` lays out and renders in memory, runs every check and writes nothing. `stencil render` applies the vet rules only, so a legend defect does not stop it: run check before render. `--print-width <inches>` (0.5 to 200) runs print-fit; render prints its lines after the paths and exits 1 on failure, files kept.

Line formats, stable for scripts:

    check <name>: examined <n> <units>, <d> defects
    check <name>: examined 0 <units>, FAILED: nothing examined
    check <name>: examined 0 links, not applicable: page has no links
    check print-fit: examined 0 text runs, not applicable: no print width
    defect <name> <pointer>: <message>
    violation <rule> <pointer>: <message>
    error document is not valid stencil JSON at line <l>, column <c>: <cause>
    stencil check: <n> checks, <p> passed, <f> failed[, <a> not applicable]
    stencil vet: <v> violations, checks not run

A pointer is an RFC 6901 JSON pointer into your document, such as /body/0/children/2 or /links/3; the page root prints as "". Violations stop vet, check and render before any check runs; fix them first.

Examined counts: each check reports how many units it looked at, and a check that examined 0 fails, because a pass over nothing proves nothing. A page with no Pipe, Tee or Link fails legend-consistency this way: every figure has at least one hop. The link checks on a page without links are not applicable and neither pass nor fail. Exit 0 needs at least one pass and no failure.

| Check | Unit | Defect at | Message |
|---|---|---|---|
| child-inside-container | relation: parent and child | the child | box WxH at (X, Y) extends outside the content box WxH at (X, Y) of PARENT |
| siblings-do-not-overlap | pair of siblings | the later sibling | box WxH at (X, Y) overlaps OTHER box WxH at (X, Y) by WxH |
| text-fits-box | text run | the node owning the run | PART "TEXT" measured WxH in box WxH; or PART box WxH at (X, Y) extends outside the node box WxH at (X, Y) |
| remembered-constants | text field | the field | contains LITERAL: REASON |
| legend-consistency | relation: each line use (Pipe, Tee arm, Tee spine, Link) and each legend entry, keyed on line and tint | the use, or /legend/i | USER line L tint T has no legend entry; legend line L tint T is never used; legend line L tint T is already listed at /legend/j |
| links-routed | link | /links/i | no route from A to B avoids every obstacle; drawn as a fallback L |
| links-avoid-boxes | pair: segment and obstacle, tag and node | /links/i | segment N from (X, Y) to (X, Y) enters NODE box WxH at (X, Y); tag WxH at (X, Y) overlaps NODE box WxH at (X, Y) |
| pipes-land | each pipe end with a neighbor or a from or to target | /body/... (the pipe) | SIDE neighbor NODE has no box across the pipe's center Y; or from (to) target NODE has none, or lies on the other side |
| iso-labels-clear | pair: label and label, label and block, label and Box, text and its block (iso only) | the later label's owner | ROLE billboard X,Y WxH overlaps ROLE billboard OWNER; or covers block NODE; or is crossed by an edge of slab NODE; or leaves its block |
| iso-links-clear | link leg (iso only) | /links/i | leg N runs D px beside an edge of zone NODE, closer than 24; or turns back; or last leg is L px, under 2 arrowheads |
| print-fit | text run (with --print-width) | the run's node, or /links/i | PART "TEXT" prints at P pt, below 8 pt (S px on a C px canvas at W in) |
| icon-matches-product | item with an icon or subtitle, kind with an icon table | the item | subtitle names P, whose icon is I; the item carries O; or subtitle names P, which has no product icon; the item carries the product icon I |

Numbers carry 2 decimals; the tolerance is 0.01 px, and touching edges pass. text-fits-box also checks the tag of every labeled link. PART is a snake_case part name: function_name (an Item's title), product_name (its subtitle), fact, built, ask, tag_label, tag_sub, hub_text, label, text, badge_text, legend_label, legend_text, heading, marker, body_line.

Reading defects:

- child-inside-container on an Item: a word wider than its column. Raise the column's weight or shorten the word.
- pipes-land: a gutter slot taller or shorter than the Box beside it. Name the Box in from or to, or give the producer Col the gutter Col's grow list.
- child-inside-container on a gutter or pipe: the gutter weight is too small for the tag. Shorten the label or raise the weight.
- text-fits-box at /title or another page field: one unbreakable string wider than the page.
- siblings-do-not-overlap follows an overflow; fix the child-inside-container defect first.
- print-fit: the canvas is the drawn one, wider under iso. Print wider or narrow the page.
- icon-matches-product: a product with its own icon (BigQuery, Cloud Run) takes it or none; others take a category icon.
