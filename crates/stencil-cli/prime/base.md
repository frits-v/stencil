# stencil prime

stencil turns structural JSON (boxes, items, facts, pipes, links; no coordinates) into SVG, PNG, measured JSON of every box, and checks over that geometry.

# Loop

Write fig.json, `stencil vet fig.json`, `stencil check fig.json`, `stencil render fig.json --out-dir out --scale 2`, Read out/fig.png, fix, repeat until check shows 0 failed and the PNG reads right. Exit 0 clean; 1 defect in the document (fix the JSON at the named pointer); 2 could not run (arguments, paths, fonts). `stencil schema` prints the JSON Schema.

`stencil gallery out` renders the examples in the designed themes; a failed check exits 1.

# Grammars

A grammar gives a domain its Box and Item kinds, where each may sit, and its rules; Page.grammar names it. {{grammars}}

# Vocabulary

Every node object carries "tag"; the Page does not. * required, =default. A field with no type is text: 1-400 chars, trimmed, no control characters. Any node may carry id (a-z, 0-9, -; unique), which Link and Pipe from and to name; a Pipe's slot centers on the nodes it names. Unknown fields and values are rejected. kind names a kind of the grammar. tint picks a slot 1-8: the fill of a tintable Box, the wire of a solid or dash line (absent is 1); gray and deny ignore it. A fact's source is doc (read from the live doc), built (an as-built name: bucket, VLAN ID) or ask (an open question). chrome none drops badge, kicker, title and lede for a figure its document captions; one line and tint then needs no legend.

{{vocabulary}}

# Layout

- A Row splits its width by grow weight, like CSS fr. With no grow every child gets 1 except Pipe and Tee (0); a Col's children keep their content height. Weight 0 is max-content width; when one child must be narrow, weight every child: [4,3,13].
- A Box is a column: label, then children at full width; its kind sets border, padding and label style.
- A pipe tag sizes to its one-line label and widens its gutter: 1-3 words, detail in sub.
- Gutter: Row [producer Col, gutter Col, target Box]. The gutter Col holds one slot per producer Box: a Col, gap 12, of that Box's h Pipes. Name the Boxes in each Pipe's from and to so its slot centers on them.
- An Item is never narrower than its widest word.
- The legend lists each line and tint in use once, and nothing else; its label (Solid blue) is fixed; write the text as meaning.
- Lanes: equal head columns; a link with order between two heads is a message row below.

# Checks

`check` prints one line per check with its examined count, then `defect <check> <pointer>: <message>` lines. Examined 0 fails. Checks with no surface (links, pipe neighbors, iso, --print-width, an icon table) are not applicable.

| Check | Examines | Defect | Fix |
|---|---|---|---|
| child-inside-container | each parent-child pair | child box leaves the parent's content box | raise the column's weight, shorten the longest word, or widen the page |
| siblings-do-not-overlap | each sibling pair | two sibling boxes intersect | fix the overflow reported with it |
| text-fits-box | each text run | text larger than its box | shorten the text or widen its container |
| remembered-constants | each text field | holds a literal the grammar lists (a doc example) | write the value from the live doc, or an ask |
| legend-consistency | each line use and legend entry | a line and tint without an entry, an unused entry, or one listed twice | add or drop the entry |
| links-routed | each link | no clear route; drawn as a fallback L | set from_side and to_side, add via, or move an endpoint |
| links-avoid-boxes | each segment-obstacle and tag-node pair | the line crosses a box or its tag covers one | as links-routed; raise gap where links run |
| pipes-land | each pipe end facing a Row or Col sibling or a from or to target | the pipe's center misses every box there | name its Boxes in from and to, or match the gutter's grow list |
| iso-labels-clear | iso only: labels, blocks, boxes | overlap or Box edge in text | open floor |
| iso-links-clear | iso only: link legs | leg by Box edge, reversed or short | wider gap |
| print-fit | each text run, only with --print-width | prints below 8 pt at that width | widen the print or shorten the figure |
| icon-matches-product | each item with an icon or subtitle whose kind has an icon table | the subtitle names a product whose own icon is another, or a product icon on a product without one | use the product's icon or a category icon |

# Rules no check enforces

- One figure, one story: what the reader should take away fits the title.
- Sequence and timeline figures: Lanes with ordered links, see `stencil prime grammar plain`.

# Themes

center (default), paper, dusk, clear, clear-dark, wire; `stencil prime themes` says which to pick. Set theme on the page or pass --theme, a name or a theme .json path; layout is identical under every theme.

{{topics}}
