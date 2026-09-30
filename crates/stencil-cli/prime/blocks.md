# Blocks

Text, Callout and Frame are leaf tags for one-page design documents. Width comes from the container, as for a Fact; height from the wrapped text. Each may carry an id and be a link endpoint. A typical one-pager is width 1440 with one Row (gap 16) of a Col of blocks beside a Col holding the figure, and a Row of Frames below.

Text: an optional heading (13 px bold) over 1 to 64 body lines (12 px). Each line wraps to the block width, 4 px below the last. list is plain (default), numbered (1. 2. ...) or bulleted, with a 22 px hanging indent.

{ "tag": "Text", "id": "goals", "heading": "Goals", "list": "numbered", "body": ["Accept a trigger written in plain language.", "Evaluate every new clip against each active trigger."] }

Callout: a boxed note with a 4 px accent bar and a tint by kind: note blue, risk red, decision green, open amber. title is optional, text required. Under wire the kind shows only through the title, so write it there.

{ "tag": "Callout", "kind": "risk", "title": "Risk: plan drift", "text": "A model update can change how a description is read. Each verdict records the plan version." }

Frame: a placeholder for a screen or image: dashed border, two diagonals, the label centered on a chip. height is fixed at 40 to 1200 px (default 200), and neither grow nor a Row's stretch changes it.

{ "tag": "Frame", "id": "screen-editor", "label": "Trigger editor: description, generated plan, approve", "height": 200 }

Every heading, title, label and body line is a text field (1 to 400 chars, trimmed) and a text run that text-fits-box checks; body line i is reported at /…/body/i.
