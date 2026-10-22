# Phases

A Phases node is a swimlane inside a View: a lane per actor top to bottom, phases left to right, and Steps placed by lane, phase and column. It takes the full width of the sheet and holds Steps only. Lanes (stencil prime grammar plain) is a sequence diagram and unrelated.

- Lanes: `lanes` lists actor ids, each once. A lane draws a band in the actor's fill mixed with the paper, a bar in its wire at the left, its name in capitals in its ink and its text under the name.
- Phases: `phases` lists ids and titles, numbered from 1 in a circle along the top. A phase holds as many columns as its Steps reach, up to 6.
- Steps: `lane` and `phase` name a lane and a phase of the parent; `col` (default 1) and `span` (default 1) place it in the phase's columns. A column is as wide as its widest Step needs, up to 240 px, and the node shares what is left among the filled columns. Steps of one lane whose columns overlap stack in document order; otherwise a boxed Step fills its lane's height and a chip or text look sits at its middle.
- A Step is its title and up to 6 lines. look: box (an outline in the actor's wire), chip (a pill in the actor's fill, one stored thing), note (a hairline, an aside), stack (three offset boxes, many of one thing), planned (dashed, not built) or text (no box, secondary ink).
- Band: `band` on the Phases node is wash (default: the band, with the sheet grid drawn over it) or none (no band; the bar, name and lane rules stay).
- Fill: `fill` on a Step, or `step_fill` on the Phases node for every Step that sets none, is tint (the actor's fill; lines take the actor's ink), cell (the sheet cell) or none (transparent; the lane and grid show through). Absent, box and stack take cell, chip tint, note and planned none. Every look keeps its outline; an unfilled stack draws its back sheets as the edges clear of the front. A text look draws no box and takes no fill. Under wire every fill is white or none.
- Links: a Step with an id is a link endpoint. A link leaving a Step takes its lane's actor. Lane names, lane texts and phase heads are obstacles, so a route runs around them; leave a column between Steps a link joins, or set from_side and to_side.

{{vocabulary:Phases,Step,Phase,StepLook,BandLook,StepFill}}

# Vet

| Rule | Defect at | Message |
|---|---|---|
| needs-sheet | the node | phases or step is drawn on a design sheet only; add sheet or remove it |
| phases-child-not-step | the child | a Phases node holds Steps only |
| step-outside-phases | the Step | a Step sits directly in a Phases node |
| phase-lane-twice | the lane | actor "ID" already has a lane |
| step-lane-unknown | the lane | lane "ID" is not a lane of this Phases node |
| step-phase-unknown | the phase | phase "ID" is not a phase of this Phases node |
| step-span-out-of-range | col or span | columns C to D run past column 6 |
| step-text-fill | the fill | a text Step draws no box, so it takes no fill |
| list-empty | the list | lanes or phases is empty |
| list-too-long | the list | N lanes, above 8 (phases 8, steps 64, step-lines 6) |
| key-duplicate | the later id | a phase id used twice in one Phases node |

child-inside-container also holds each Step inside its lane's band and its phase: `step box B leaves lane ID`, `step box B leaves phase T`. The description writes the Phases node with a band of none and its step_fill, each Step with its lane, phase, columns, look and a fill it sets or that differs from its look's, and a table of lanes by phases listing each cell's Steps.
