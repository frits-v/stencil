# Design sheets

A page with `sheet` is a design sheet: one system as an engineering drawing on one page, flat only, chrome kept.

- Frame: the canvas is the width plus 50 px on each side. A border 12 px in and a rule 32 px in hold a band of grid references, columns 1 to 8 along the top and bottom and rows A to F down both sides. The theme's sheet role paints it and may draw a faint drafting grid inside.
- Title block, view 1, across the top in four cells. The first is the page head: the badge, the kicker after `1 · `, the wordmark set large when given, the title (the motto under a wordmark), and the lede. The problem cell holds a lead and points P1 to P8; the answer cell a statement and up to four paragraphs; the attention cell items, each an id, a lead and a text. A paragraph is a string or {lead,text}; a lead is set bold with its text running on after it on the same line.
- Captions: each cell opens with its caption in capitals, authored (at most 60 characters) or the word Problem, Answer or Attention. A wordmark holds 24.
- Legend: a strip along the foot of the title block, inside its border: a row Who acts with a chip per actor, then a row Lines with the entries. A sheet draws at least one Pipe, Tee or Link for it to key.
- Actors: `actors` names who acts, each with its own tint slot. An Item, Box, Pipe, Tee or Link with `actor` takes its colours: wire for outlines and lines, ink for titles. A line without one takes the actor of the node it leaves (a Step's lane); gray and deny take none. With actors no tint is written, and a `by_actor` legend entry keys every solid or dashed use.
- Phases, Steps and lanes: `stencil prime phases`. Marks, the parts list and general notes: `stencil prime marks`.
- Views: the body holds Views and nothing else, each a body child, numbered from 2. A View is a heading (a numbered circle, the title, a subtitle, a rule to the right edge and the note) over its children in a column, gap 16. Inside a View every node works as on any page.
- Boxes: a frame-role Box such as gcp is an unfilled outline, a boundary Box a hairline; group and tile Boxes keep their tone.

{{vocabulary:View,Sheet,Problem,ProblemPoint,Answer,Attention,AttentionItem,Actor,Page,Item,Box,Pipe,Tee,Link,LegendEntry}}

# Vet

| Rule | Defect at | Message |
|---|---|---|
| needs-sheet | the View | view is drawn on a design sheet only; add sheet or remove it |
| sheet-in-iso | /sheet | a design sheet is drawn flat only |
| sheet-chrome-none | /chrome | a design sheet draws its head in the title block, so chrome cannot be none |
| sheet-body-not-view | /body/i | a design sheet holds Views; wrap T in a View |
| view-not-top-level | the View | a View is a body child of a sheet |
| sheet-without-lines | /sheet | a design sheet draws at least one Pipe, Tee or Link |
| list-empty | the list | problem-points or attention-items is empty |
| list-too-long | the list | N problem-points, above 8 (answer-paragraphs 4, attention-items 8) |
| sheet-text-too-long | the field | wordmark or caption of N characters, above 24 or 60 |
| key-duplicate | the later id | an actor or attention item id used twice |
| actor-tint-duplicate | the tint | tint N is already used by actor "ID" |
| actor-unknown | the field | actor "ID" is not declared |
| tint-on-actor-page | the tint | a page with actors colours by actor; remove tint |
| kind-tint-without-actor | the Box | K paints tint N by default, which reads as an actor |
| line-without-actor | the line | L line from F has no actor |
| actor-on-neutral-line | the actor | a gray or deny line takes no actor |
| actor-on-untintable-kind | the actor | K takes no colour, so it cannot name an actor |
| tee-arm-actor-differs | the arm's actor | a Tee arm acts for its Tee's actor |
| legend-by-actor-misused | the by_actor | by_actor names solid or dashed lines on a page with actors |

A via point may lie in the margin: x runs from 0 to the width plus 100.

# Description

On a sheet `render` writes `## Sheet`, the title block, Who acts, the legend and a section per View. Each pointer span follows a reference span, `B3` or `B2-C5`: the cells under the element's corners. description-covers-drawing then also reports `line L gives R, drawn at Z` and `line L gives no reference, drawn at Z`.
