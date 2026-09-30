# CUE authoring layer

Architecture figures are written as structure, not coordinates. `stencil.cue` defines the
vocabulary and the rules a figure must satisfy; a figure file (`g7.cue`) writes one topology
in that vocabulary.

## Files

| File | Contents |
|---|---|
| `stencil.cue` | `#Page` and the node vocabulary: `Row`, `Col`, `Zone`, `Pcard`, `Fact`, `Note`, `Pipe`, `Tee`, `Text`, `Callout`, `Frame`, plus `#LegendEntry` and `#Link` |
| `g7.cue` | g7, Dedicated Interconnect at 99.99%: `customer` and `internal: customer & {...}` |
| `check.sh` | vet, export `customer`, compare it with `examples/g7.json`, vet `examples/onepager.json` against `#Page`, then the negative cases |
| `out/` | exported JSON and renders |

## Vet, export, render

The pinned CUE version is 0.17.1. From the repository root:

```bash
cue vet -c ./cue
cue export ./cue -e customer --out json -o cue/out/g7-customer.json --force
stencil render cue/out/g7-customer.json --out-dir cue/out
```

`-c` reports each non-concrete field by path (for example a figure with no `canvas`); without
it, vet only says that some instances are incomplete. `stencil render` writes `<stem>.svg`,
`<stem>.png` and `<stem>.measured.json` in `--out-dir`.

`./cue/check.sh` runs vet, exports `customer` to `out/g7-customer.json` and compares it with
`examples/g7.json`, vets `examples/onepager.json` against `#Page`
(`cue vet -c -d '#Page' . ../examples/onepager.json`), and then runs every negative case: a
copy of the package with one edit to `g7.cue` that vet must reject with a named error. The
comparison normalizes key order only; any other difference prints a unified diff and fails.
The script fails if the `cue` binary is missing, if an edit does not apply, if vet passes, or
if vet fails for a different reason. Set `CUE` to the binary path if the `cue` shim is not
active, for example `CUE="$(mise which cue)" ./cue/check.sh`.

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
  arms, together with the kinds of the page's links, equals the set of legend kinds.
  `_pipeKindsMissingFromLegend.<kind>`, `_linkKindsMissingFromLegend.<kind>` and
  `_legendKindsUnusedInBody.<kind>` name the offender. Legend kinds are unique.
- Ids and links. Every node, tee arms included, may carry an `id` matching
  `^[a-z0-9][a-z0-9-]{0,63}$`, and ids are unique across the page (`_idsUsedTwice.<id>`). A
  link's `from` and `to` name node ids (`_linkEndpointsUnknown.<id>`) and differ
  (`_linkToItself.<id>`). A page holds at most 256 links and a link at most 8 `via` points.
  Each point has x from 0 to the canvas width (`_viaOutsidePage`) and y of 0 or more; the
  canvas height is known only after layout.
- Region tint. `region-a` and `onprem-a` are tint a, `region-b` and `onprem-b` tint b, `blue`
  pipes tint a and `pink` pipes tint b. A tinted zone may not contain, at any depth, a pipe, tee
  arm or zone of the other tint (`_otherTintInsideZone`). A pipe that sits between two sibling
  zones in a `Row`, `Col` or `Zone` may not touch a zone of the other tint
  (`_pipeBesideZoneOfOtherTint`). The zone colours themselves come from the zone kind in the
  renderer, so a figure cannot tint a region by hand.
- Canvas. `canvas` is `"customer"` or `"internal"` and must be concrete, so each exported
  figure has exactly one.
- Page and container fields. `width` is optional, 640 to 2560; the renderer defaults it to
  1280, so the export omits it unless a figure sets it. On a `Row` or `Col`, `gap` is 0 to 64,
  `justify` is `start`, `center`, `end` or `space-between`, and `grow` carries one weight from
  0 to 100 per child (`_growLengthMatchesChildren`).
- Sizes, matching the Rust model in `crates/stencil-model/src/document.rs`. Every text field is
  1 to 400 characters. A page body and the children of a `Row`, `Col` or `Zone` hold 1 to 256
  nodes, so an empty container is rejected. A `Text` block holds 1 to 64 body lines, and a
  `Frame` height is 40 to 1200. Unique legend kinds bound the legend at five entries, inside
  the Rust model's limit of 16.
- Closed vocabulary. Tags, zone kinds, pipe kinds, note kinds, icon stems, themes, arrows,
  link sides, list kinds and callout kinds are enumerations; an unknown field on a node is
  rejected.

## Two canvases

`customer` is concrete except for two kinds of slot: `canvas` defaults to `"customer"`, and
the hidden `_workshop` labels default to empty strings and are concatenated onto zone labels
and the failover pipe. `internal: customer & {canvas: "internal", _workshop: {...}}` fills the
slots. Because everything else is concrete, the internal canvas can add labels but any change
to a pipe, card or legend entry is a unification conflict. While `canvas` is `"customer"`,
every `_workshop` label must be empty, so workshop text cannot reach the customer canvas.

## Limits

- Region placement is checked where CUE can see it: inside a zone's subtree and between
  sibling zones. A pipe that reaches its zone only through a renderer-side layout, rather than
  through the structural tree, is outside the schema's reach and unchecked.
- When a node fails, the per-page sets built from it come up empty, so one error can also
  report every legend kind as unused. The first error on a `body` path is the cause.
- `cue vet -c` checks concreteness of regular fields only. A hidden check that evaluates to an
  incomplete value, rather than an error, passes silently. The usual cause is a lookup of a
  missing key in an open struct, so every lookup table the checks index (`#TintOf`) is a closed
  definition covering all keys, membership tests use `list.Contains`, and every rule has a
  negative case in `check.sh` that must fire.
