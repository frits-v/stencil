# stencil prime

stencil turns structural JSON (boxes, items, facts, pipes, links; no coordinates) into SVG, PNG, measured JSON of every box, and checks over that geometry.

# Loop

Write fig.json, `stencil vet fig.json`, `stencil check fig.json`, `stencil render fig.json --out-dir out --scale 2`, Read out/fig.png, fix, repeat until check shows 0 failed and the PNG reads right. Exit 0 clean; 1 defect in the document (the line names a JSON pointer such as /body/0/children/2; fix the JSON); 2 could not run (arguments, paths, fonts). `stencil schema` prints the JSON Schema.

`stencil gallery out [--examples dir]` renders every example in every theme, with index.html and gallery.json; a failed check exits 1.

# Grammars

Every figure shares the core below. A grammar gives a domain its kinds: what a Box or Item kind may be, where each may sit, and the domain rules. Page.grammar names it. {{grammars}}

# Vocabulary

Every node object carries "tag"; the Page does not. * required, =default. A field with no type is text: 1-400 chars, no leading or trailing space, no control characters. Any node may carry id (a-z, 0-9, -; unique), which Link from and to name. Unknown fields and values are rejected. kind names a kind of the grammar. tint picks a slot 1-8: the fill of a tintable Box, the wire of a solid or dash line (absent is 1); gray and deny ignore it. A fact's source is doc (read from the live doc), built (an as-built name: bucket, VLAN ID, project id) or ask (an open question). chrome none drops badge, kicker, title and lede when the document captions the figure.

{{vocabulary}}

# Layout

- A Row splits its width by grow weight, like CSS fr. With no grow, every child gets 1 except Pipe and Tee (0); in a Col every child gets 0 and keeps its content height. Weight 0 means max-content width, which for a column of items is wide, so when one child must be narrow give every Row child a weight: [4,3,13].
- A Box is a column: its label, then its children at full width. Its kind sets its border, padding and label; a frame kind draws a bar holding the label.
- A tag sizes to its label on one line and widens its gutter (a pipe tag wraps only when a weight squeezes the gutter). Keep labels to 1-3 words; put detail in sub.
- Gutter: Row [producer Col, gutter Col, target Box]. The gutter Col holds one slot per producer Box: a Col with justify center and gap 12 holding that Box's h Pipes. Give the producer Col the same grow list as the gutter Col ([1,1,1]) so each slot matches its Box's height and the pipes land beside it.
- Item text wraps at the item width with the widest word as the floor, so one long word widens the item past its container.
- The legend lists each line and tint a Pipe, Tee or Link uses, once, and nothing else. Its label (Solid blue) is fixed by line and slot; write the text as meaning.
- Lanes lays its heads out as equal columns.

# Checks

`check` prints one line per check with its examined count, then `defect <check> <pointer>: <message>` lines. Examined 0 fails. Link checks on a page without links, and pipes-land on a page whose pipes have no Row or Col neighbor, are not applicable.

| Check | Examines | Defect | Fix |
|---|---|---|---|
| child-inside-container | each parent-child pair | child box leaves the parent's content box | raise that column's weight, shorten the longest word, or widen the page |
| siblings-do-not-overlap | each sibling pair | two sibling boxes intersect | fix the overflow reported with it |
| text-fits-box | each text run | text larger than its box | shorten the text or widen its container |
| remembered-constants | each text field | contains a literal the grammar lists, doc examples rather than requirements | write the value from the live doc, or an ask |
| legend-consistency | each line use and legend entry | a line and tint without an entry, an unused entry, or one listed twice | add or drop the entry |
| links-routed | each link | no clear route; drawn as a fallback L | set from_side and to_side, add via, or move an endpoint |
| links-avoid-boxes | each segment-obstacle and tag-node pair | the line crosses a box or its tag covers one | same as links-routed; raise gap where links run |
| pipes-land | each h or v pipe end that faces a Row or Col sibling | the pipe's center is outside every box in the neighbor's subtree, so it points at empty space | give the producer column the gutter's grow weights, or move the pipe to the slot beside its Box |
| iso-labels-clear | iso only: labels, blocks, boxes | overlap or Box edge in text | open floor |
| iso-links-clear | iso only: link legs | leg by Box edge, reversed or short | wider gap |

# Rules no check enforces

- One figure, one story: what the reader should take away fits the title.
- Sequence and timeline figures are refused until a timeline grammar exists.

# Themes

- center: Architecture Center light, the default.
- dusk: dark, for dark slides.
- wire: monochrome wireframe for design docs.

Set theme on the page, or pass --theme to render and check; layout is identical under all three.

{{topics}}
