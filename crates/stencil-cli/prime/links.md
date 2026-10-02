# Links

A link is a routed orthogonal line between two nodes, drawn after layout on top of everything. It takes no space and moves nothing. Use it where a gutter Pipe cannot go: between items in different columns, down a stack, or a numbered request trace. A hop between two adjacent boxes reads better as a Pipe.

{ "from": "api", "to": "queue", "line": "solid", "tint": 1, "label": "3 queue plan", "from_side": "bottom", "to_side": "top" }

| Field | Meaning |
|---|---|
| from, to | ids of two different nodes; any node may carry id (a-z, 0-9, -, at most 64, unique in the page) |
| line, tint | Line and tint slot; the pair needs a legend entry like any pipe |
| label, sub | tag text, one line each; no label means no tag |
| arrow | none, end (default), start or both; the arrowhead sits on the endpoint's edge |
| from_side, to_side | top, right, bottom or left: the side whose midpoint the end attaches to |
| via | up to 8 {"x","y"} points in canvas px, passed in order |
| axis | iso only: x or y, the axis the tag reads along; absent follows the leg it sits on (a Pipe tag follows its dir); see stencil prime iso |
| order | 1-256; only between two heads of one Lanes node, where it makes the link a message row (stencil prime grammar plain) |

At most 256 links per page. The canvas is width + 40 px wide; read coordinates for via from out/<stem>.measured.json, where every node has x, y, width and height.

Attach points: with both sides set, those. With neither and no via, the facing pair (right to left, bottom to top, left to right, top to bottom) whose midpoints are closest. Otherwise each open end takes the side nearest what it faces: the first via point, the last via point, or the other end.

Router (every link but a Lanes message, which runs straight across its row):

- Obstacles are every leaf box (Item, Fact, Note, Text, Callout, Frame, each Pipe and Tee tag, each whole Tee), every Box label, and the page kicker, title, lede, legend entries and foot, except the two endpoints and whatever contains them. Box borders are not obstacles: a line entering a Box is how a path reaches it. Running along an edge is allowed.
- A* over a grid built from obstacle edges pushed out 8 px, costing 1 per px plus 40 per turn. The first move leaves through the from side and the last enters through the to side. With via, each leg is routed in turn. The same geometry always gives the same route.
- Fallback: no route, more than 512 grid lines, or more than 12 segments gives an L from attach point to attach point, horizontal leg first. It is still drawn, and links-routed reports it.
- The tag is a pipe tag centered on the longest segment, sized to its text, never wrapped. links-avoid-boxes checks it against every node except the endpoints' ancestors, so a tag on a Box label or frame bar goes unreported: look at the PNG.

What makes lines and tags collide:

- Two links whose longest segments share a lane: their tags stack. Set sides or via so each runs in its own lane.
- A long label on a short route: the tag covers the endpoint items. Shorten the label or move the items apart.
- Items at the default gap 8: no lane between them. Set gap 32 to 64 on the Col or Row the links run through.
- An endpoint boxed in by items on every side: no route exists and the link falls back.
- A route whose longest segment crosses a Box label: its tag lands on the label. Pick sides so the longest segment runs through open Box body.

Fixes, in order: set from_side and to_side; raise the gap where links run; add a via point in an empty lane; move an endpoint.
