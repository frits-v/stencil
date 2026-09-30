# Layout

Boxes come from flexbox. You set structure, grow, gap and justify; every size is computed.

| Field | Effect |
|---|---|
| grow | one weight 0 to 100 per child, sharing free space along the main axis like CSS fr. A weight above 0 grows from 0 and never shrinks below the child's min-content; weight 0 keeps the child's max-content size |
| grow absent, Row | 1 for every child except Pipe and Tee (0). A Col holding only pipes still gets 1, so a Row with a gutter needs explicit grow |
| grow absent, Col | 0 for every child: each keeps its content height and justify places the stack |
| justify | start (default), center, end or space-between along the main axis |
| gap | 0 to 64 px between children, default 8 |

The cross axis always stretches: Row children take the Row's height, Col children the Col's width. A Zone is a column: its label band, then its children at full width and content height. Nothing shrinks; content that does not fit overflows and child-inside-container reports it.

Pipes fill their gutter:

- Run axis across the parent (h in a Col or Zone, v in a Row): the pipe spans the gutter.
- Run axis along the parent (h in a Row, v in a Col or Zone): max-content length, centered; a v pipe is at least 36 px. A v Pipe between two zones in a Zone or Col draws the hop between them.
- A Tee is a spine with a hub tag and two h Pipe arms, at least 118 px wide.

Slots place pipes beside specific zones. Split the gutter into one slot Col per producer zone, and give the producer Col and the gutter Col the same grow list, so slot i is exactly as tall as zone i:

{"tag":"Row","grow":[4,3,13],"children":[
 {"tag":"Col","grow":[1,1],"children":[ZONE_1, ZONE_2]},
 {"tag":"Col","grow":[1,1],"children":[
  {"tag":"Col","gap":12,"justify":"center","children":[PIPES_FOR_ZONE_1]},
  {"tag":"Col","gap":12,"justify":"center","children":[PIPES_FOR_ZONE_2]}]},
 {"tag":"Zone","kind":"gcp","label":"Google Cloud","children":[...]}]}

Cards joined across: Row [Pcard, Pipe h, Pcard] gives equal card columns with the pipe at its natural width. For stacks, Row [Col, Pipe, Col, Pipe, Col] with grow [1,0,1,0,1].

Nesting inside Google Cloud: gcp, then vpc, then region-a or region-b, then subnet. project, perimeter and k8s wrap whatever they scope. On-prem sites and other clouds sit outside gcp, usually in the producer column.

| Zone kind | Means | Look in center |
|---|---|---|
| gcp | the Google Cloud frame | blue bar holding the label, light body |
| vpc | a VPC network | dashed gray border, no fill |
| region-a, region-b | a region; a is blue, b is pink | tinted fill |
| subnet | a subnet | dashed border, lavender |
| onprem-a, onprem-b | an on-prem site or another cloud; a pairs with region-a and blue pipes, b with region-b and pink | tinted fill, warm border |
| project | a project or folder | pale yellow |
| optional | an optional or proposed component | dashed blue |
| k8s | a cluster or namespace | pink, no border |
| perimeter | a VPC Service Controls perimeter | dashed orange, orange label |

Page width is 640 to 2560 (default 1280); the canvas adds 20 px on each side. Dense figures read well at 1440 to 1600.
