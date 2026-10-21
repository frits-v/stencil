# Design sheets

A page with `sheet` is a design sheet: one system drawn as an engineering drawing on one page. It is flat only, and it keeps its chrome.

- Frame: the canvas is the width plus 50 px on each side. A border 12 px in and a rule 32 px in hold a band of grid references, columns 1 to 8 along the top and bottom and rows A to F down both sides. The theme's sheet role paints it and may draw a faint drafting grid inside.
- Title block, view 1, across the top in four cells. The first is the page head: the badge, the kicker after `1 · `, the wordmark set large when given, the title (the motto under a wordmark), and the lede. The problem cell holds a lead and points P1 to P8; the answer cell a statement and up to four paragraphs; the attention cell items, each an id, a lead and a text. A paragraph is a string or {lead,text}; a lead is set bold with its text running on after it on the same line.
- Captions: each cell opens with its caption in capitals, the authored one or the word Problem, Answer or Attention. A caption holds at most 60 characters, a wordmark 24.
- Legend: a strip under the title block, its entries after the caption Lines. A sheet draws at least one Pipe, Tee or Link for it to key.
- Views: the body holds Views and nothing else, each a body child, numbered from 2. A View is a heading (a numbered circle, the title, a subtitle, a rule to the right edge and the note) over its children in a column, gap 16. Inside a View every node works as on any page.
- Boxes on a sheet: a frame-role Box such as gcp is an outline with its label and no fill, and a boundary Box a hairline; group and tile Boxes keep their tone.

{{vocabulary:View,Sheet,Problem,ProblemPoint,Answer,Attention,AttentionItem,Page}}

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
| key-duplicate | the later id | an attention item id used twice |

A via point may lie in the margin: x runs from 0 to the width plus 100.

# Description

On a sheet `render` writes `## Sheet`, the title block, the legend and one section per View with its lines. Each element's pointer span follows a reference span, `B3` or `B2-C5`: the cells under its top-left and bottom-right corners. description-covers-drawing then also reports `line L gives R, drawn at Z` and `line L gives no reference, drawn at Z`.
