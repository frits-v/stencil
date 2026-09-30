# CUE authoring layer

Architecture figures are written as structure, not coordinates. `stencil.cue` defines the
vocabulary and the rules a figure must satisfy; a figure file (`g7.cue`) writes one topology
in that vocabulary; a view (`eraser.cue`) derives renderer input from it.

## Files

| File | Contents |
|---|---|
| `stencil.cue` | `#Page` and the node vocabulary: `Row`, `Col`, `Zone`, `Pcard`, `Fact`, `Note`, `Pipe`, `Tee`, plus `#LegendEntry` |
| `g7.cue` | g7, Dedicated Interconnect at 99.99%: `customer` and `internal: customer & {...}` |
| `eraser.cue` | `#EraserG7`, the view that turns a g7-shaped page into eraser-diagrams JSON; `eraserCustomer`, `eraserInternal` |
| `check.sh` | vet, export both views, compare the `customer` export with `examples/g7.json`, then the negative cases |
| `out/` | exported JSON and renders |

## Vet, export, render

The pinned CUE version is 0.17.1. From the repository root:

```bash
cue vet -c ./cue
cue export ./cue -e eraserCustomer --out json -o cue/out/g7-eraser.json --force
cue export ./cue -e eraserInternal --out json -o cue/out/g7-eraser-internal.json --force
cue export ./cue -e customer --out json      # the structural form; equals examples/g7.json
```

`-c` reports each non-concrete field by path (for example a figure with no `canvas`); without
it, vet only says that some instances are incomplete.

Rendering uses the eraser-diagrams stencil library `g7-stencil.mjs` and needs
`@eraserlabs/diagrams` resolvable from the script's own directory, so run the copy that sits
next to `node_modules`:

```bash
cd /tmp/eraser-test
node g7-stencil.mjs /path/to/stencil/cue/out/g7-eraser.json /path/to/stencil/cue/out/g7-eraser
```

It writes `g7-eraser.png`, `.html` and `.measured.json`.

`./cue/check.sh` runs vet and both exports, reports the entity and connection count of each
view, exports `customer` to `out/g7-customer.json` and compares it with `examples/g7.json`, and
then runs every negative case: a copy of the package with one edit to `g7.cue` that vet must
reject with a named error. The comparison normalizes key order only; any other difference
prints a unified diff and fails. The script fails if the `cue` binary is missing, if an edit
does not apply, if vet passes, or if vet fails for a different reason. Set `CUE` to the binary
path if the `cue` shim is not active, for example `CUE="$(mise which cue)" ./cue/check.sh`.

`examples/g7.json` is maintained by hand and is the Rust renderer's golden input. A change to
`g7.cue` that alters the customer canvas needs the same change in `examples/g7.json`.

## What the schema enforces

Every rule below is a CUE constraint; `cue vet -c` fails when one is broken.

- Remembered constants. Every text field rejects `64512`, `130.211.0.0/22`,
  `35.191.0.0/16` and `10.8.0.0/28` (word-bounded regular expressions, so `164512` passes).
- Product cards. Every `Pcard` carries `pn`, `fact` or `ask`, each a non-empty string. The
  rule covers every card, which includes the cards on hops and in regions. It is checked at
  the page (`_pcardsWithoutPnFactOrAsk`), keyed by the card's `fn`.
- Legend consistency. The set of pipe kinds used anywhere in the body, including tee spines and
  arms, equals the set of legend kinds. `_pipeKindsMissingFromLegend.<kind>` and
  `_legendKindsUnusedInBody.<kind>` name the offender. Legend kinds are unique.
- Region tint. `region-a` and `onprem-a` are tint a, `region-b` and `onprem-b` tint b, `blue`
  pipes tint a and `pink` pipes tint b. A tinted zone may not contain, at any depth, a pipe, tee
  arm or zone of the other tint (`_otherTintInsideZone`). A pipe that sits between two sibling
  zones in a `Row`, `Col` or `Zone` may not touch a zone of the other tint
  (`_pipeBesideZoneOfOtherTint`). The zone colours themselves come from the zone kind in the
  renderer, so a figure cannot tint a region by hand.
- Gutter pairing, in the g7 view. Gutter pipe p leaves card p of the left column. A blue pipe
  must leave a card in an `onprem-a` zone and lands in the `region-a` zone; pink pairs with
  `onprem-b` and `region-b` (`_leavesCardInItsMetro`). The view also asserts one pipe per card.
- Canvas. `canvas` is `"customer"` or `"internal"` and must be concrete, so each exported
  figure has exactly one.
- Page and container fields. `width` is optional, 640 to 2560; the renderer defaults it to
  1280, so the export omits it unless a figure sets it. On a `Row` or `Col`, `gap` is 0 to 64,
  `justify` is `start`, `center`, `end` or `space-between`, and `grow` carries one weight from
  0 to 100 per child (`_growLengthMatchesChildren`).
- Closed vocabulary. Tags, zone kinds, pipe kinds, note kinds and icon stems are enumerations;
  an unknown field on a node is rejected.

## Two canvases

`customer` is concrete except for two kinds of slot: `canvas` defaults to `"customer"`, and
the hidden `_workshop` labels default to empty strings and are concatenated onto zone labels
and the failover pipe. `internal: customer & {canvas: "internal", _workshop: {...}}` fills the
slots. Because everything else is concrete, the internal canvas can add labels but any change
to a pipe, card or legend entry is a unification conflict. While `canvas` is `"customer"`,
every `_workshop` label must be empty, so workshop text cannot reach the customer canvas.

## The eraser view

`#EraserG7` accepts one shape and asserts it by unification:

```
Row [ Col [ onprem Zones of Pcards ],
      Col [ Col [ h Pipes ], Col [ h Pipes ] ],
      Zone gcp [ Zone vpc [ Zone, v Pipe, Zone ] ] ]
```

The gutter halves are read in order as one list of pipes, one per card, in card order.

Coordinates come from the parameters in `P` (column widths, zone headers, item heights, gaps)
and from index arithmetic: a zone's height is its label, its items and its gaps; items stack
below the label; zones stack below each other; the row is as tall as its taller column. A zone
label longer than the zone wraps, and the estimate of its line count pushes the items down.
The renderer anchors a pipe tag's bottom edge at the midpoint of the pipe's path, so a failover
pipe with a second tag line gets a taller gap.

For `customer`, the export matches the hand-authored reference JSON field for field except
entity ids and the foot text.

## Limits

- The view handles the g7 shape only. A general layout (nested rows and columns at any depth)
  needs recursion over the tree, which CUE comprehensions do not provide; each new shape is a
  new view, or the job of a renderer that walks the structural export.
- The `g7-stencil.mjs` Zone component knows six kinds (`gcp`, `vpc`, `region-a`, `region-b`,
  `onprem-a`, `onprem-b`). A page using `subnet`, `project`, `optional`, `k8s` or `perimeter`
  vets but does not render on this path.
- Region placement is checked where CUE can see it: inside a zone's subtree and between
  sibling zones. A pipe in a separate gutter column has no structural link to the zone it
  lands in; the g7 view supplies that link by pairing, and checks it there.
- When a node fails, the per-page sets built from it come up empty, so one error can also
  report every legend kind as unused. The first error on a `body` path is the cause.
- `cue vet -c` checks concreteness of regular fields only. A hidden check that evaluates to an
  incomplete value, rather than an error, passes silently. The usual cause is a lookup of a
  missing key in an open struct, so every lookup table the checks index (`#TintOf`,
  `#MetroFor`, `#RegionFor`) is a closed definition covering all keys, membership tests use
  `list.Contains`, and every rule has a negative case in `check.sh` that must fire.
- Text widths are estimates (7 px per character at 12 px bold). A label that wraps in the
  browser but not in the estimate still overlaps; read the render.
