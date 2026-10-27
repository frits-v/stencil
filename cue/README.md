# CUE authoring layer

Architecture figures are written as structure, not coordinates. The core package defines the
vocabulary every figure shares; a grammar package gives a domain its Box and Item kinds and adds
its rules; a figure file writes one topology against a grammar's `#Page`.

## Files

| File | Package | Contents |
|---|---|---|
| `cue.mod/module.cue` | | the module `github.com/frits-v/stencil/cue@v0`, language v0.17.1 |
| `core.cue` | `stencil` | `#Page` and the core nodes: `Row`, `Col`, `Lanes`, `Box`, `Item`, `Fact`, `Note`, `Pipe`, `Tee`, `Text`, `Callout`, `Frame`, `View`, `Phases`, `Step`, `PartsList`, `GeneralNotes`, `Ladder`, `HowTo`, plus `#Sheet` and its cells, `#Mark`, `#MarkedText`, `#MarkLegend`, `#PartEntry`, `#GeneralNote`, `#Rung`, `#HowToItem`, `#CommandRun`, `#TerminalLine`, `#Actor`, `#Phase`, `#SheetSurface`, `#Outline`, `#LineStep`, `#LegendEntry`, `#Link`, `#Line`, `#Bend`, `#Corner`, `#TintSlot` and `#FactEntry` |
| `grammar.cue` | `stencil` | `#Grammar` and its parts (`#LineKind` among them), and `#GrammarNesting`, the kind and nesting rules every grammar shares |
| `icons_aws.cue` | `stencil` | `#IconAwsStems` and `#IconAws`, the file stems of the aws icon pack |
| `icons_azure.cue` | `stencil` | `#IconAzureStems` and `#IconAzure`, the file stems of the azure icon pack |
| `icons_gcp.cue` | `stencil` | `#IconGcpStems` and `#IconGcp`, the file stems of the gcp icon pack; each pack has its own `icons_<pack>.cue`, and `#Icon` in `core.cue` is their union |
| `icons_oss.cue` | `stencil` | `#IconOssStems` and `#IconOss`, the file stems of the oss icon pack |
| `theme.cue` | `stencil` | `#Theme`, the closed theme definition every theme file vets against: `cue vet -c -d '#Theme' . <file>.json` |
| `grammars/aws.cue` | `aws` | the AWS grammar data and service rules, and the aws `#Page` |
| `grammars/azure.cue` | `azure` | the Azure grammar data and product rules, and the azure `#Page` |
| `grammars/gcp.cue` | `gcp` | the Google Cloud grammar data and product rules, and the gcp `#Page` |
| `grammars/oss.cue` | `oss` | plain's kinds with the oss pack on service, store, queue and network, a cluster container, and the oss `#Page`, which walks the body with plain's `#Node` for the external-outside-system rule |
| `grammars/plain.cue` | `plain` | the default, vendor-neutral grammar data and the plain `#Page` with its external-outside-system rule |
| `figures/aws.cue` | `aws` | an on-prem site over Direct Connect into a two-AZ VPC with its regional services beside it: `hybrid`, the base of the aws negative cases |
| `figures/azure.cue` | `azure` | a hub-and-spoke network in one Azure region: `figure`, the base of the azure negative cases |
| `figures/oss.cue` | `oss` | a web stack on Kubernetes with an identity provider beside the platform: `figure`, the base of the oss negative cases |
| `figures/g7.cue` | `gcp` | g7, Dedicated Interconnect at 99.99%: `customer` and `internal: customer & {...}` |
| `figures/sequence.cue` | `plain` | a request flow in five lanes with ten ordered messages: `figure`, exported as `examples/sequence.json` |
| `figures/sheet.cue` | `plain` | the library hold-and-pickup design sheet, a title block, four actors, a Workflow view with a detail ladder, a System view (an assembly, a rail, a store with compartments, step badges, a valve and planned work) with its parts list, general notes and a desk screen, and a How-tos view of four terminals: `onepager`, exported as `examples/onepager.json` |
| `figures/sheet-small.cue` | `plain` | the smallest design sheet, with a parts list and general notes whose every key is marked once, and a link whose via lies in the sheet's wider margin: `small`, the base of the sheet negative cases |
| `figures/sheet-gcp.cue` | `gcp` | the smallest design sheet under gcp, a person and a Google Cloud frame holding a region and a product with an icon: `cloud`, the base of the sheet negative cases that need a gcp kind |
| `check.sh` | | the gate, below |
| `out/` | | exported JSON and renders, not committed |

Layout notes forced by CUE's package rules:

- `cue.mod/` sits in `cue/`, so `cue/` is the module root and every `cue` command runs from
  inside it. From the repository root, `cue export ./cue/grammars:gcp` finds no module.
- The core package lives in the module root directory, whose name (`cue`) differs from its
  package name, so its import path carries the package: `github.com/frits-v/stencil/cue:stencil`.
- `grammars/` holds one package per grammar, `aws`, `azure`, `gcp`, `oss` and `plain`, so every command
  names one: `./grammars:gcp`. A bare `./grammars` is an error.
- `figures/` holds `aws.cue` (package `aws`), `azure.cue` (package `azure`), `g7.cue` and
  `sheet-gcp.cue` (package `gcp`), `oss.cue` (package `oss`) and `sequence.cue`, `sheet.cue` and `sheet-small.cue`
  (package `plain`), so commands name the package: `./figures:aws`, `./figures:azure`,
  `./figures:gcp`, `./figures:oss`, `./figures:plain`. Each is a separate instance from its grammar and imports it
  (`github.com/frits-v/stencil/cue/grammars:gcp`, `:aws`, `:azure`, `:oss` or `:plain`) to reach its
  `#Page`.

## Vet, export, render

The pinned CUE version is 0.17.1. From `cue/`:

```bash
cue vet -c ./figures:gcp
cue export ./figures:gcp -e customer --out json -o out/g7-customer.json --force
cue vet -c -d '#Page' ./grammars:plain ../examples/design-brief.json
cue export ./grammars:gcp -e grammar --out json -o ../crates/stencil-model/grammars/gcp.json --force
stencil render out/g7-customer.json --out-dir out
```

`-e` names the field that holds the page. `customer` is g7's own field name; a figure of
your own exports the field its file binds the page to, for example `-e topology` for
`topology: gcp.#Page & {...}`.

`-c` reports each non-concrete field by path (for example a figure with no `title`); without
it, vet only says that some instances are incomplete. `stencil render` writes `<stem>.svg`,
`<stem>.png` and `<stem>.measured.json` in `--out-dir`.

`./cue/check.sh` (from anywhere) runs, in order:

1. `cue vet -c` over the core, `./grammars:aws`, `./grammars:azure`, `./grammars:gcp`,
   `./grammars:oss`, `./grammars:plain`, `./figures:aws`, `./figures:azure`, `./figures:gcp`,
   `./figures:oss` and `./figures:plain`.
2. Exports each grammar and compares it byte for byte with
   `crates/stencil-model/grammars/<name>.json`, which the Rust side embeds; a difference prints a
   unified diff and fails. Regenerate the committed file with the export command above.
3. Exports `customer` and compares it with `examples/g7.json`, `figure` from
   `./figures:plain` with `examples/sequence.json`, and `onepager` from `./figures:plain` with
   `examples/onepager.json`, normalizing key order only.
4. Vets every `examples/*.json` against the `#Page` of the grammar it names (`plain` when absent). An example with a standing defect is
   listed in `known_failures` with the rule it breaks: it is printed as `KNOWN` on every run,
   any other error fails, and an entry that starts vetting clean fails as stale.
5. Vets a one-key `chrome: "none"` document with an empty legend, and a document of the
   products an azure subnet or region does hold, which must both pass.
6. Runs every negative case: a copy of the cue tree with one edit to `figures/aws.cue`,
   `figures/azure.cue`, `figures/g7.cue`, `figures/oss.cue`, `figures/sequence.cue`, `figures/sheet.cue`,
   `figures/sheet-small.cue` or
   `figures/sheet-gcp.cue` that vet must reject with a named error.

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
- A page with `sheet` is a design sheet: a title block (`problem` with 1 to 8 points, `answer`
  with at most 4 paragraphs, `attention` with 1 to 8 items, captions of at most 60
  characters, a wordmark of at most 24) and a body of `View`s. On a sheet,
  `_sheetInIso`, `_sheetChromeNone`, `_sheetBodyNotView.<i>`, `_viewNotTopLevel.<title>`,
  `_sheetWithoutLines` and `_keysUsedTwice."attention.<id>"` name the offender, and a via may
  lie up to the width plus 100. Off a sheet, a `View` is `_needsSheet.view`.
- `actors` names who acts on a sheet, each with its own tint slot; `actor` on an Item, Box,
  Pipe, Tee or Link colours it, and a line without one takes the actor of the node it
  leaves, a Step's lane included. A use with an actor keys `<line>-by-actor`, which a
  `by_actor` legend entry names. On a sheet `_actorTintsTwice`, `_actorsUnknown`,
  `_actorsUnused`, `_tintOnActorPage`, `_lineWithoutActor`, `_actorOnNeutralLine`,
  `_legendByActorMisused`, `_teeArmActorDiffers` and, from the grammar,
  `_actorOnUntintableKind` and `_kindTintWithoutActor` name the offender.
- A `Phases` node holds `Step`s only, each in one of its `lanes` and `phases` and inside
  column 6: `_lanesUsedTwice`, `_phaseIdsUsedTwice`, `_stepLanesUnknown`,
  `_stepPhasesUnknown`, `_stepSpanOutOfRange` and `_stepOutsidePhases`. A Phases node's
  `band` is `wash` or `none` and its `step_fill`, like a Step's `fill`, is `tint`, `cell` or
  `none`; a text-look Step with its own `fill` is `_stepTextFill`. The Step node is
  `#StepNode`, since `#Step` is the theme's face lightness step.
- A mark is `{part: <id>}`, `{note: <id>}` or `{attention: <id>}`: a balloon keyed to the
  `PartsList`, a flag to the `GeneralNotes`, a diamond to an attention item. An Item, Box,
  Pipe, Link, Step or problem point carries up to 4 in `marks`; an Item, Box or Step one
  `corner_mark`; a Step line is a string or `{text, marks}`; a part with `node` puts its
  balloon on that Item, Box or Pipe. On a sheet `_partMarksUnknown`, `_noteMarksUnknown`,
  `_attentionMarksUnknown`, `_marksRepeated`, `_partBalloonRepeated`, `_markLegendUnused`,
  `_linkMarksWithoutTag`, `_keyedListTwice`, `_partNodesUnknown`, `_partNodesTwice`,
  `_partFieldFromNode`, `_partNameMissing` and `_keysUsedTwice."part.<id>"` or
  `."note.<id>"` name the offender, and the keys-used twins `_partsUnmarked`,
  `_notesUnflagged` and `_attentionUnmarked` name a key no mark uses. Off a sheet marks,
  a parts list and general notes are `_needsSheet.marks`, `."parts-list"` and
  `."general-notes"`.
- A `Ladder` frames 1 to 16 Steps of one Phases node, its `around`, and climbs 1 to 6
  `rungs`, each with a title, an optional `who` and `actor`, its text, `planned` and up to 4
  `marks`. On a sheet `_laddersTooMany` names a ninth ladder, `_ladderAroundUnknown` an
  `around` id that names no Step and `_ladderAroundTwoPhases` a ladder whose Steps lie in
  two Phases nodes; a rung's actor counts as named. Off a sheet a ladder is
  `_needsSheet.ladder`.
- A `HowTo` holds 1 to 6 `items`, each a title, up to 5 `commands` (a string, or
  `{words}` between chips), optional `text` and `beside`, and up to 12 `terminal` lines
  of a `kind` (`command`, `output`, `comment`, `ok`), an `indent` from 0 to 8 and a text.
  `_howToItemEmpty` names an item with neither text nor terminal lines and `_besideAlone`
  `beside` without both. Off a sheet a HowTo is `_needsSheet."how-to"`.
- An Item's `outline` is `card`, `store`, `person` or `stack`; a store holds up to 6
  `compartments`, each a string or `{text, marks}`. An Item or Box may be `planned`, and a
  solid Link or Pipe `planned` too, keying `<key>-planned`. A Pipe with `form: "rail"` keys
  `<key>-rail`. A Link or Pipe carries a step badge `step` (1 to 64) and a Link a `valve`;
  `sheet.marks.step` is the step badge's legend text. A Box kind with a `dash-dot` border or a
  `tab` label (the `assembly` kind of every grammar) draws on a sheet only. On a sheet
  `_plannedNotSolid`, `_lineIntoPlanned`, `_compartmentsWithoutStore`,
  `_outlinePersonWithIcon`, `_railNotVertical`, `_railOutsideRow`, `_railWithTargets`,
  `_linkStepUsedTwice`, `_valveWithoutArrow` and `_markLegendUnused.step` name the offender.
  Off a sheet these are `_needsSheet."item-outline"`, `.compartments`, `.planned`,
  `."sheet-look"`, `."rail-form"`, `."link-step"`, `."pipe-step"` and `.valve`.
- `Lanes` is a container of 1 to 32 lane heads with an optional `gap` of 0 to 64. A `Link` may
  carry `order`, 1 to 256, which makes it a message of the Lanes node whose heads it joins.
- `#Page` adds `grammar` (`aws`, `azure`, `gcp`, `oss`, `plain` or a path ending in `.json`) and `chrome` (`full` or
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
- Link bends. `corner` on the page is 0 to 16 and `bend` is `arc`, `curve` or `spline`. A grammar's
  `lines` holds at most one entry per line (`_linesAreUnique`), each with an optional `corner`
  and `bend` that win over the page's for links of that line.
- Badge. `badge` is an optional text field, the pill before the kicker; a page without one
  draws none.
- Page and container fields. `width` is optional, 640 to 2560; the renderer defaults it to
  1280, so the export omits it unless a figure sets it. On a `Row` or `Col`, `gap` is 0 to 64,
  `justify` is `start`, `center`, `end` or `space-between`, and `grow` carries one weight from
  0 to 100 per child (`_growLengthMatchesChildren`).
- Sizes, matching the Rust model in `crates/stencil-model/src/document.rs`. Every text field is
  1 to 400 characters. A page body and the children of a `Row`, `Col` or `Box` hold 1 to 256
  nodes, so an empty container is rejected. A `Text` block holds 1 to 64 body lines, a `Frame`
  height is 40 to 1200, and an Item holds at most 8 facts.
- Closed vocabulary. Tags, lines, fact sources, note kinds, the icon stems of the bundled
  packs, themes, arrows, link sides, list kinds and callout kinds are enumerations; an unknown
  field on a node is rejected.

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
  Box ancestor, or `page` at the top level. Row, Col, Lanes and View are transparent.
  `_kindParentNotAllowed.<kind>` names the offender: `subnet: cannot sit in page`.
- Tint without effect. A tint on a Box whose kind is not tintable is reported as
  `_tintWithoutEffect` on the Box.
- Icons. An Item's `icon` and each products row's `icon` are stems of the pack the kind draws
  from, `#PackStems` selecting the pack's list by comprehension; `_iconOutsidePack` on the
  Item and `_iconsOutsidePack.<stem>` on the item kind name the offender. A products row may
  have no icon, for a product with no bundled icon; the check skips it.
- Grammar name. A page that sets `grammar` must name the package's grammar.

## azure rules

- Remembered constants. No regular field of the page holds `10.0.0.0/16`, `10.0.0.0/24`,
  `168.63.129.16`, `65515` or `12076` at word boundaries (`_rememberedInText.<literal>`). The
  rule reads the grammar's `remembered` list and matches it over the page's fields marshalled
  to JSON, since a grammar package cannot reach into the core `#Text`.
- Hop facts, as gcp has them (`_itemsWithoutSubtitleOrFact.<title>`).
- PaaS services outside a VNet. A `product` item whose `subtitle` names Storage, SQL Database,
  Cosmos DB, Key Vault, Service Bus, Event Hubs, Event Grid, App Service, Functions, Logic Apps
  or Azure OpenAI, or that carries one of their icons, may not have a `vnet` Box among its
  ancestors (`_paasInsideVnet.<title>`). A subtitle naming an App Service Environment or a
  private endpoint, and the `private-endpoint` icon, are exempt.
- Global resources outside a region. A `product` item whose `subtitle` names Entra ID, Front
  Door, Traffic Manager, a DNS zone or Azure Policy, or that carries one of their icons, may not
  have a `region` Box among its ancestors (`_globalInsideRegion.<title>`).

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

## aws rules

- Hop facts, as under gcp (`_itemsWithoutSubtitleOrFact.<title>`).
- Regional services outside a VPC. A `product` item whose `subtitle` names S3, DynamoDB, SQS,
  SNS, EventBridge, ECR, Secrets Manager, KMS, Step Functions, Kinesis, Athena, Bedrock,
  CloudWatch or API Gateway at word boundaries, or carries one of their icons, may not have a
  `vpc` Box among its ancestors (`_regionalServiceInsideVpc.<title>`). A subtitle naming an
  endpoint draws the endpoint and is not read.
- Global services outside a region. A `product` item whose `subtitle` names IAM, Route 53,
  CloudFront, WAF, Organizations or Global Accelerator, or whose `icon` is `iam` or
  `cloudfront`, may not have a `region` Box among its ancestors (`_globalInsideRegion.<title>`).
  A subtitle that names Resolver, Identity Center or regional is a regional form and is not read.
- Remembered constants. 10.0.0.0/16, 10.0.1.0/24 and 169.254.169.254 are rejected in the page
  head, the legend, links and node text (`#AwsText`); 64512 is the core's.

## Two canvases

g7 draws two canvases from one source. `customer` is concrete except for two kinds of slot: the
hidden `_audience` defaults to `"customer"` and sets the badge, and the hidden `_workshop`
labels default to empty strings and are concatenated onto box labels and the failover pipe.
`internal: customer & {_audience: "internal", _workshop: {...}}` fills the slots. Because
everything else is concrete, the internal canvas can add labels but any change to a pipe, item
or legend entry is a unification conflict. While `_audience` is `"customer"`, every `_workshop`
label must be empty, so workshop text cannot reach the customer canvas. The pattern is g7's own:
a figure with one audience sets `badge`, or leaves it out.

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
