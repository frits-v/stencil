# Marks, parts list and general notes

A design sheet keys its drawings to three lists. A balloon names a part of the parts list, a flag a general note, a diamond an attention item of the title block. Each draws its key's number: its position in its list, from 1.

- Marks: `marks` holds up to 4 of {part/note/attention}, each the id of a key. On an Item, Box or Step they stand right of the title or label, on a problem point at the end of its row, on a Pipe or Link inside its tag after the label and sub (a Link with marks has a label). A Step line written {text,marks} carries its own. `corner_mark` on an Item, Box or Step centres one glyph on its top-right corner. A node marks one key once.
- Parts list: a PartsList node holds parts. A part with `node` is that Item, Box or Pipe: name, what and actor come from the node, which draws the balloon without a mark. A part without `node` writes `name`, and may write what, actor and planned. A row draws the balloon, name, what, why and its zones: the grid references of every balloon of the part. A name wraps at spaces and hyphens; one longer word (a snake_case identifier) runs into the why column, and no check reports it.
- General notes: a GeneralNotes node holds notes, each a text and a detail, drawn with its flag and the cells it is flagged at, then the foot.
- Attention items draw their diamond and the cells they are marked at.
- A sheet holds one PartsList and one GeneralNotes. `sheet.marks` gives the legend strip's text per kind; the strip samples each kind the sheet draws.

{{vocabulary:PartsList,GeneralNotes,PartEntry,GeneralNote,MarkLegend,Item,Box,Pipe,Link}}

# Vet

| Rule | Defect at | Message |
|---|---|---|
| needs-sheet | the field or node | marks, parts-list or general-notes is drawn on a design sheet only |
| mark-unknown | the mark | K "ID" is not in the parts list (general notes, attention items) |
| mark-repeated | the later mark | K "ID" is already marked here |
| mark-legend-unused | /sheet/marks/K | no K key on this sheet; remove the text |
| link-marks-without-tag | /links/i/marks | a link's marks sit in its tag; give it a label |
| keyed-list-twice | the second list | the sheet has its parts list at P |
| part-node-unknown | the node | "ID" names no Item, Box or Pipe |
| part-node-twice | the later node | node "ID" is already part "P" |
| part-field-from-node | the field | part "ID" takes its F from node "N" |
| part-name-missing | the part | part "ID" names a node or writes its name |
| list-empty | the list | parts or general-notes is empty |
| list-too-long | the list | N marks, above 4 (parts 48, general-notes 16) |
| key-duplicate | the later id | a part or note id used twice |

# Checks

| Check | Unit | Defect at | Message |
|---|---|---|---|
| keys-used | key | the actor or key | actor "ID" is never named by a lane, a node, a line or an attention item; part "ID" is never ballooned; general note "ID" is never flagged; attention item "ID" is never marked |
| marks-clear | mark | the carrier or list | G box B extends outside its carrier; overlaps T; is not centred on the top-right corner; numeral box leaves its K; reads N, its key is number M |

description-covers-drawing also reports `draws G mark glyphs for M marks` and `where-used reads "W", marks at Z` on a sheet.
