# Outlines, rails, valves, step badges and planned lines

On a design sheet a drawing says how its parts are built and in which order work moves through them.

- Outlines: an Item's `outline` is card (the default), store (a cylinder; up to 6 `compartments`, each a string or {text,marks}, in rows of at most three), person (a head and shoulders over the title; no icon) or stack (three offset cards: many of one thing). Links meet a stack's front card and a store's drawn lid.
- Assembly: a Box of kind assembly is the parts one actor runs: a dash-dot outline in the actor's wire with its label on a tab across the top edge. A link that ends neither inside nor at it routes around it.
- Rail: a Pipe with `form: rail` is a heavy vertical bus, a direct child of a Row with `dir: v` and no from or to; its tag stands at the foot of the bar. Links attach to it: on a long side level with what they face, 12 px inside its ends, or at either end of the bar, where they draw a dot in place of an arrowhead.
- Step badge: `step` (1 to 64, unique across links and pipes) on a Link or Pipe sets a numbered square at the left of its tag; a step alone makes a tag. The legend strip samples it with `sheet.marks.step`.
- Valve: `valve` on a Link draws a bow-tie on its leg before the arrowhead, 26 px back from it; the route keeps a straight run for it. A link with marks but no label or step sets them under its valve.
- Planned: `planned` on an Item or Box dashes its outline with no fill; on a solid Link or Pipe it draws the line hollow, a closed outline in the actor's wire around a paper core, so it reads apart from a dashed line. A line into planned work is planned too, and needs a planned legend entry.
- Lines: every bend and hop of a dashed, dotted or hollow line is drawn solid, so a turn never falls in a gap. A tag, badge or valve keeps 15 px of straight past a rounded bend; a tag that no leg holds moves its leg a few px. Ends that meet one side of a node spread along it, a leg on another link's line shifts off it where it can, and a leg keeps 12 px off a Box outline it runs beside. Where links of two actors cross, the later one hops in a small arc.

{{vocabulary:Outline,Item,Box,Pipe,Link,LegendEntry}}

# Vet

| Rule | Defect at | Message |
|---|---|---|
| needs-sheet | the field | item-outline, compartments, planned, sheet-look, rail-form, link-step, pipe-step or valve is drawn on a design sheet only |
| planned-not-solid | planned | a planned line is a solid line drawn hollow; set line solid |
| line-into-planned | to or from | "ID" is planned, so a line to it is planned too; set planned |
| compartments-without-store | compartments | compartments are drawn in a store |
| outline-person-with-icon | icon | a person outline draws no icon |
| rail-not-vertical | dir | a rail stands vertical |
| rail-outside-row | the Pipe | a rail is a column of a Row |
| rail-with-targets | from or to | a rail is attached by links, not targets |
| step-out-of-range | step | step N is outside 1 to 64 |
| link-step-duplicate | the later step | step N is already used by P |
| valve-without-arrow | valve | a valve sits before an arrowhead |
| list-too-long | compartments | N compartments, above 6 |

# Checks

On a sheet links-routed also reports a tag or valve within reach of a bend, a bend in a dash gap, a leg along another link's or a Box outline, a tag over another tag or line, two ends of one node closer than 12 px, a leg that turns back, and a rail end off its bar or without its dot. marks-clear also reads each step badge (inside its tag, its numeral inside the square), valve, hop (clear of bends, ends and tags) and arrival, which no glyph may cover. pipes-land skips rails, and a page whose only pipes are rails reports page has no gutter pipes.
