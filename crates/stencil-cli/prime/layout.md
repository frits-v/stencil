# Layout

Boxes come from flexbox. You set structure, grow, gap and justify; every size is computed.

| Field | Effect |
|---|---|
| grow | one weight 0 to 100 per child, sharing free space along the main axis like CSS fr. A weight above 0 grows from 0 and never shrinks below the child's min-content; weight 0 keeps the child's max-content size |
| grow absent, Row | 1 for every child except Pipe and Tee (0). A Col holding only pipes still gets 1, so a Row with a gutter needs explicit grow |
| grow absent, Col | 0 for every child: each keeps its content height and justify places the stack |
| justify | start (default), center, end or space-between along the main axis |
| gap | 0 to 64 px between children, default 8 |

The cross axis always stretches: Row children take the Row's height, Col children the Col's width. A Box is a column: its label band, then its children at full width and content height; its container kind sets its border, padding and label, and a frame kind puts the label in a bar over a body. Nothing shrinks; content that does not fit overflows and child-inside-container reports it.

Pipes fill their gutter:

- Run axis across the parent (h in a Col or Box, v in a Row): the pipe spans the gutter.
- Run axis along the parent (h in a Row, v in a Col or Box): max-content length, centered; a v pipe is at least 36 px. A v Pipe between two Boxes in a Box or Col draws the hop between them.
- A Tee is a spine with a hub tag and two h Pipe arms, at least 118 px wide.

Slots place pipes beside specific Boxes. Split the gutter into one slot Col per producer Box, and give the producer Col and the gutter Col the same grow list, so slot i is exactly as tall as Box i:

{"tag":"Row","grow":[4,3,13],"children":[
 {"tag":"Col","grow":[1,1],"children":[BOX_1, BOX_2]},
 {"tag":"Col","grow":[1,1],"children":[
  {"tag":"Col","gap":12,"justify":"center","children":[PIPES_FOR_BOX_1]},
  {"tag":"Col","gap":12,"justify":"center","children":[PIPES_FOR_BOX_2]}]},
 {"tag":"Box","kind":"gcp","label":"Google Cloud","children":[...]}]}

Items joined across: Row [Item, Pipe h, Item] gives equal item columns with the pipe at its natural width. For stacks, Row [Col, Pipe, Col, Pipe, Col] with grow [1,0,1,0,1].

Which kinds a Box may take, what each means and where each may sit come from the grammar: `stencil prime grammar gcp` for the Google Cloud kinds and their nesting.

Page width is 640 to 2560 (default 1280); the canvas adds 20 px on each side. Dense figures read well at 1440 to 1600.
