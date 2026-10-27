# Blocks

Text, Callout and Frame are leaf tags for design documents, on any page and in a sheet's Views. Width comes from the container, as for a Fact; height from the wrapped text. Each may carry an id and be a link endpoint. A design brief is width 1440 with one Row (gap 16) of a Col of blocks beside a Col holding the figure, and a Row of Frames below. A one-pager is a design sheet: start from `stencil prime onepager`.

Text: an optional heading (13 px bold) over 1 to 64 body lines (12 px). Each line wraps to the block width, 4 px below the last. list is plain (default), numbered (1. 2. ...) or bulleted, with a 22 px hanging indent.

{ "tag": "Text", "id": "goals", "heading": "Goals", "list": "numbered", "body": ["Accept a hold placed in the catalogue app.", "Set each returned copy aside for the first hold in line."] }

Callout: a boxed note with a 4 px accent bar and a fill by kind (note, risk, decision, open), each with its own accent. title is optional, text required. Under wire the kind shows only through the title, so write it there.

{ "tag": "Callout", "kind": "risk", "title": "Risk: stale contact details", "text": "A notice sent to an old number leaves the copy unclaimed. A bounced notice flags the card." }

Frame: a placeholder for a screen or image: dashed border, two diagonals, the label centered on a chip. height is fixed at 40 to 1200 px (default 200), and neither grow nor a Row's stretch changes it.

{ "tag": "Frame", "id": "screen-editor", "label": "Place a hold: title, pickup branch, notice channel, confirm", "height": 200 }

Every heading, title, label and body line is a text field (1 to 400 chars, trimmed) and a text run that text-fits-box checks; body line i is reported at /…/body/i.
