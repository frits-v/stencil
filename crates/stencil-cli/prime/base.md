# stencil prime

stencil turns structural JSON (boxes, items, facts, pipes, links; no coordinates) into SVG, PNG, measured JSON of every box, and checks over that geometry.

# Loop

Write fig.json, `stencil vet fig.json`, `stencil check fig.json`, `stencil render fig.json --out-dir out --scale 2`, Read out/fig.png, fix, repeat until check shows 0 failed and the PNG reads right. `render` also writes out/fig.description.md, the figure in words; `stencil describe fig.json` prints it. Exit 0 clean; 1 defect in the document (fix the JSON at the pointer); 2 could not run (arguments, paths, fonts).

# Grammars

A grammar gives a domain its Box and Item kinds, where each may sit, and its rules; Page.grammar names it. {{grammars}}

# Vocabulary

Every node object carries "tag"; the Page does not. * required, =default. A field with no type is text: 1-400 chars, trimmed, no control characters. Any node may carry id (a-z, 0-9, -; unique), which Link and Pipe from and to name. Unknown fields and values are rejected. kind names a kind of the grammar. tint picks a slot 1-8: the fill of a tintable Box, the wire of a solid or dash line (absent is 1); gray and deny ignore it. A fact's source is doc (read from the live doc), built (an as-built name: bucket, VLAN ID) or ask (an open question). chrome none drops badge, kicker, title and lede for a figure its document captions; one line and tint then needs no legend.

{{vocabulary}}

# Layout

- A Row splits its width by grow weight, like CSS fr. With no grow every child gets 1 except Pipe and Tee (0); a Col's children keep their content height. Weight 0 is max-content width; when one child must be narrow, weight every child: [4,3,13].
- A Box is a column: label, then children at full width; its kind sets border, padding and label style.
- A pipe tag sizes to its one-line label and widens its gutter: 1-3 words, detail in sub.
- Gutter: Row [producer Col, gutter Col, target Box]. The gutter Col holds one slot per producer Box: a Col, gap 12, of that Box's h Pipes. Name the Boxes in each Pipe's from and to so its slot centers on them.
- The legend lists each line and tint in use once, and nothing else; its label (Solid blue) is fixed; write the text as meaning.
- Lanes: equal head columns; a link with order between two heads is a message row below.

# Checks

`check` prints one line per check with its examined count, then `defect <check> <pointer>: <message>` lines. Examined 0 fails. Checks with no surface are not applicable; the iso- checks run under iso only.

| Check | Defect | Fix |
|---|---|---|
| child-inside-container | child box leaves the parent's content box | raise the column's weight, shorten the longest word, or widen the page |
| siblings-do-not-overlap | two sibling boxes intersect | fix the overflow reported with it |
| text-fits-box | text larger than its box | shorten the text or widen its container |
| remembered-constants | holds a literal the grammar lists (a doc example) | write the value from the live doc, or an ask |
| legend-consistency | a use without an entry, an unused entry, or one listed twice | add or drop the entry |
| links-routed | no clear route; drawn as a fallback L; an arrowed leg under 22 px; an arrowhead or stop plate under a tag or another end | set from_side and to_side, add via, or move an endpoint |
| links-avoid-boxes | the line crosses a box or its tag covers one | as links-routed; raise gap where links run |
| pipes-land | the pipe's center misses every box there | name its Boxes in from and to, or match the gutter's grow list |
| iso-labels-clear | overlap, crowding, a wire in text | open floor |
| iso-links-clear | leg by a Box edge, reversed, short, a detour, a jog, over a block | wider gap, via |
| iso-link-ends | end off its node or on a corner | move the end |
| iso-links-apart | two links share a stretch or an end | opposite sides |
| print-fit | with --print-width: prints below 8 pt at that width | widen the print or shorten the figure |
| icon-matches-product | the subtitle names a product whose own icon is another, or a product icon on a product without one | use the product's icon or a category icon |
| description-covers-drawing | fig.description.md misses or misnames a drawn unit | a stencil bug: report it |

# Rules no check enforces

- One figure, one story: what the reader should take away fits the title.
- Sequence and timeline figures: Lanes with ordered links, see `stencil prime grammar plain`.
- Design sheets, flat only: `stencil prime sheet`.

# Themes

center (default), paper, dusk, clear, clear-dark, wire; `stencil prime themes` says which to pick. Set theme on the page or pass --theme; layout is the same under every theme.

{{topics}}
