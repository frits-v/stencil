# CUE authoring layer

Architecture figures are written as structure, not coordinates. The core package defines the
vocabulary every figure shares; a grammar package gives a domain its Box and Item kinds and adds
its rules; a figure file writes one topology against a grammar's `#Page`.

## Files

| File | Package | Contents |
|---|---|---|
| `cue.mod/module.cue` | | the module `github.com/frits-v/stencil/cue@v0`, language v0.17.1 |
| `core.cue` | `stencil` | `#Page` and the core nodes: `Row`, `Col`, `Lanes`, `Box`, `Item`, `Fact`, `Note`, `Pipe`, `Tee`, `Text`, `Callout`, `Frame`, plus `#LegendEntry`, `#Link`, `#Line`, `#TintSlot` and `#FactEntry` |
| `grammar.cue` | `stencil` | `#Grammar` and its parts, and `#GrammarNesting`, the kind and nesting rules every grammar shares |
| `theme.cue` | `stencil` | `#Theme`, the closed theme definition every theme file vets against: `cue vet -c -d '#Theme' . <file>.json` |
| `grammars/gcp.cue` | `gcp` | the Google Cloud grammar data and rules, and the gcp `#Page` |
| `grammars/plain.cue` | `plain` | the domain-neutral grammar data and the plain `#Page` |
| `figures/g7.cue` | `gcp` | g7, Dedicated Interconnect at 99.99%: `customer` and `internal: customer & {...}` |
| `figures/sequence.cue` | `plain` | a request flow in five lanes with ten ordered messages: `figure`, exported as `examples/sequence.json` |
| `check.sh` | | the gate, below |
| `out/` | | exported JSON and renders, not committed |

Layout notes forced by CUE's package rules:

- `cue.mod/` sits in `cue/`, so `cue/` is the module root and every `cue` command runs from
  inside it. From the repository root, `cue export ./cue/grammars:gcp` finds no module.
- The core package lives in the module root directory, whose name (`cue`) differs from its
  package name, so its import path carries the package: `github.com/frits-v/stencil/cue:stencil`.
- `grammars/` holds two packages, `gcp` and `plain`, so every command names one:
  `./grammars:gcp`. A bare `./grammars` is an error.
- `figures/` holds `g7.cue` (package `gcp`) and `sequence.cue` (package `plain`), so commands
  name the package: `./figures:gcp`, `./figures:plain`. Each is a separate instance from its
  grammar and imports it (`github.com/frits-v/stencil/cue/grammars:gcp` or `:plain`) to reach
  its `#Page`.

## Vet, export, render

The pinned CUE version is 0.17.1. From `cue/`:

```bash
cue vet -c ./figures:gcp
cue export ./figures:gcp -e customer --out json -o out/g7-customer.json --force
cue vet -c -d '#Page' ./grammars:gcp ../examples/onepager.json
cue export ./grammars:gcp -e grammar --out json -o ../crates/stencil-model/grammars/gcp.json --force
stencil render out/g7-customer.json --out-dir out
```

`-c` reports each non-concrete field by path (for example a figure with no `canvas`); without
it, vet only says that some instances are incomplete. `stencil render` writes `<stem>.svg`,
`<stem>.png` and `<stem>.measured.json` in `--out-dir`.

`./cue/check.sh` (from anywhere) runs, in order:

1. `cue vet -c` over the core, `./grammars:gcp`, `./grammars:plain`, `./figures:gcp` and
   `./figures:plain`.
2. Exports each grammar and compares it byte for byte with
   `crates/stencil-model/grammars/<name>.json`, which the Rust side embeds; a difference prints a
   unified diff and fails. Regenerate the committed file with the export command above.
3. Exports `customer` and compares it with `examples/g7.json`, and `figure` from
   `./figures:plain` with `examples/sequence.json`, normalizing key order only.
4. Vets every `examples/*.json` against the `#Page` of the grammar it names (`gcp` when absent). An example with a standing defect is
   listed in `known_failures` with the rule it breaks: it is printed as `KNOWN` on every run,
   any other error fails, and an entry that starts vetting clean fails as stale.
5. Vets a one-key `chrome: "none"` document with an empty legend, which must pass.
6. Runs every negative case: a copy of the cue tree with one edit to `figures/g7.cue` or
   `figures/sequence.cue` that vet must reject with a named error.

The script fails if the `cue` binary is missing, if an edit does not apply, if vet passes, or if
vet fails for a different reason. Set `CUE` to the binary path if the `cue` shim is not active,
for example `CUE="$(mise which cue)" ./cue/check.sh`.

`examples/g7.json` is maintained by hand and is the Rust renderer's golden input. A change to
`figures/g7.cue` that alters the customer canvas needs the same change in `examples/g7.json`.

## Core vocabulary

- `Box` is any container: `kind` names a container kind of the grammar, `tint` (1 to 8)
  picks a slot on a tintable kind. `Item` is any named leaf: `kind` names an item kind, with
  `title`, optional `subtitle` and `icon`, and up to 8 `facts`.
- A fact entry, and the standalone `Fact` node, has `text` and an optional `source`: `doc` (the
  default, read from the live documentation), `built` (an as-built name off the running
  system) or `ask` (an open question for the reader).
- `Pipe`, `Tee` (and each Tee arm), `Link` and legend entries carry `line`: `gray`, `solid`,
  `dash` or `deny`, and an optional `tint`. A solid or dash line without tint is slot 1.
- `Lanes` is a container of 1 to 32 lane heads with an optional `gap` of 0 to 64. A `Link` may
  carry `order`, 1 to 256, which makes it a message of the Lanes node whose heads it joins.
- `#Page` adds `grammar` (`gcp`, `plain` or a path ending in `.json`) and `chrome` (`full` or
  `none`). `theme` is a built-in theme name (`center`, `paper`, `dusk`, `clear`, `clear-dark`, `wire`, or one of the imported tier: `tokyo-night`, `solarized-light`, `solarized-dark`, `material-dark`, `gruvbox-dark`, `dracula`, `nord`) or a theme file path ending in `.json`; `theme_overrides` is an open struct the Rust loader checks against `#Theme`.

## What the core enforces

Every rule below is a CUE constraint; `cue vet -c` fails when one is broken. Each named check is
a hidden struct keyed by the offending item, so the error path names it.

- Remembered constants. Every text field rejects `64512`, `130.211.0.0/22`, `35.191.0.0/16` and
  `10.8.0.0/28` (word-bounded regular expressions, so `164512` passes). They stay in the core
  under every grammar, because the core `#Text` applies them and a grammar cannot reach into it.
- Legend consistency on keys. The key of a solid or dash line is `<line>-<tint>`, slot 1 when
  tint is absent (`solid-1`, `dash-1`, `solid-3`); a gray or deny line keys as the line alone.
  The keys of every Pipe, Tee spine and Tee arm in the body, and of every link, equal the
  legend's keys: `_pipeKeysMissingFromLegend.<key>`, `_linkKeysMissingFromLegend.<key>` and
  `_legendKeysUnusedInBody.<key>` name the offender, and `_legendKeysAreUnique` rejects a key
  listed twice. The legend holds at most 16 entries. A `chrome: "none"` page whose lines share
  exactly one key may have an empty legend.
- Tint without effect. A tint on a gray or deny line is reported as
  `_lineKey._tintWithoutEffect` on the carrier.
- Ids and links. Every node, tee arms included, may carry an `id` matching
  `^[a-z0-9][a-z0-9-]{0,63}$`, and ids are unique across the page (`_idsUsedTwice.<id>`). A
  link's `from` and `to` name node ids (`_linkEndpointsUnknown.<id>`) and differ
  (`_linkToItself.<id>`). A page holds at most 256 links and a link at most 8 `via` points.
  Each point has x from 0 to the canvas width (`_viaOutsidePage`) and y of 0 or more; the
  canvas height is known only after layout.
- Pipe targets. A Pipe's `from` and `to` name node ids (`_pipeTargetsUnknown.<id>`) and
  differ (`_pipeTargetsEqual` on the Pipe); a Tee arm names neither
  (`_targetOnTeeArm.<arm index>` on the Tee).
- Ordered links. A link with `order` joins two different heads of one Lanes node, a head being
  a direct child of it (`_orderedLinkNotBetweenLaneHeads.<link index>`), and no two messages of
  one Lanes node share an order (`_linkOrderUsedTwice.<order>`).
- Canvas. `canvas` is `"customer"` or `"internal"` and must be concrete, so each exported
  figure has exactly one.
- Page and container fields. `width` is optional, 640 to 2560; the renderer defaults it to
  1280, so the export omits it unless a figure sets it. On a `Row` or `Col`, `gap` is 0 to 64,
  `justify` is `start`, `center`, `end` or `space-between`, and `grow` carries one weight from
  0 to 100 per child (`_growLengthMatchesChildren`).
- Sizes, matching the Rust model in `crates/stencil-model/src/document.rs`. Every text field is
  1 to 400 characters. A page body and the children of a `Row`, `Col` or `Box` hold 1 to 256
  nodes, so an empty container is rejected. A `Text` block holds 1 to 64 body lines, a `Frame`
  height is 40 to 1200, and an Item holds at most 8 facts.
- Closed vocabulary. Tags, lines, fact sources, note kinds, icon stems, themes, arrows, link
  sides, list kinds and callout kinds are enumerations; an unknown field on a node is rejected.

## What every grammar enforces

`#GrammarNesting` in `grammar.cue` reads a grammar's data. A grammar package instantiates it
and builds its `#Page` on `#KindedPage`:

```cue
_nesting: stencil.#GrammarNesting & {#grammar: grammar}
#Page:    _nesting.#KindedPage & {...}
```

- Kinds. A Box `kind` is one of the grammar's container kinds and an Item `kind` one of its
  item kinds; anything else is an empty disjunction on `kind`.
- Nesting. The kind of each Box and Item must list, in its `parents`, the kind of its nearest
  Box ancestor, or `page` at the top level. Row, Col and Lanes are transparent.
  `_kindParentNotAllowed.<kind>` names the offender: `subnet: cannot sit in page`.
- Tint without effect. A tint on a Box whose kind is not tintable is reported as
  `_tintWithoutEffect` on the Box.
- Grammar name. A page that sets `grammar` must name the package's grammar.

## gcp rules

- Hop facts. Every `product` item carries a `subtitle`, or a fact whose source is `doc` or
  `ask` (`_itemsWithoutSubtitleOrFact.<title>`). A `built` fact does not count: as-built names
  need no live doc and say nothing about what the product is.
- Tint pairing over slots. A Box's effective tint is its own `tint`, or its kind's
  `default_tint`, on a tintable kind: `region` without tint is slot 1, `onprem` without tint is
  untinted, and every other gcp kind is untinted. A tinted Box may not contain, at any depth, a
  solid Pipe, Tee spine or Tee arm, or a tinted Box, of another slot
  (`_otherTintInsideZone.<key>`, keyed `solid-2` or `onprem-2`). A solid pipe that sits between
  sibling Boxes in a `Row`, `Col`, `Lanes` or `Box` may not touch a tinted Box of another slot
  (`_pipeBesideZoneOfOtherTint.<label>`). Dash, gray and deny lines are outside the rule: the g7
  failover pipe sits between Region A and Region B.
- Products outside a VPC. A `product` item whose `subtitle` names Cloud Storage, BigQuery,
  Pub/Sub, Artifact Registry or Cloud Logging at word boundaries, or whose `icon` is
  `cloud-storage` or `bigquery`, may not have a `vpc` Box among its ancestors
  (`_productInsideVpc.<title>`): these are Google APIs reached over Private Google Access.
- apis inside gcp. Every `apis` Box has a `gcp` Box among its ancestors
  (`_apisOutsideGcp.<label>`), which its parents list cannot say when a project or perimeter
  sits between.

## Two canvases

`customer` is concrete except for two kinds of slot: `canvas` defaults to `"customer"`, and the
hidden `_workshop` labels default to empty strings and are concatenated onto box labels and the
failover pipe. `internal: customer & {canvas: "internal", _workshop: {...}}` fills the slots.
Because everything else is concrete, the internal canvas can add labels but any change to a
pipe, item or legend entry is a unification conflict. While `canvas` is `"customer"`, every
`_workshop` label must be empty, so workshop text cannot reach the customer canvas.

## Limits

- Tint pairing is checked where CUE can see it: inside a Box's subtree and between siblings. A
  pipe that reaches its Box only through a renderer-side layout, rather than through the
  structural tree, is outside the schema's reach and unchecked.
- When a node fails, the per-page sets built from it come up empty, so one error can also
  report every legend key as unused, and CUE prints only some of the errors of an invalid
  document. The first error on a `body` path is the cause.
- `cue vet -c` checks concreteness of regular fields only. A hidden check that evaluates to an
  incomplete value, rather than an error, passes silently. Two causes are known: a lookup of a
  missing key in an open struct, and a field that refers to itself (`{line: line}` inside a
  struct that declares `line`). So grammar data is looked up by comprehension over the lists,
  never by struct key, membership tests use `list.Contains`, and every rule has a negative case
  in `check.sh` that must fire.
