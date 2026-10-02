# Stencil renderer: contract specification

Stencil turns a JSON description of an architecture figure into an SVG, a PNG and a measured JSON. The input describes structure: zones, product cards, facts, notes and the labeled pipes between them. It carries no coordinates. taffy computes every box. cosmic-text measures every string with font files bundled in this repository, and resvg renders with the same files, so a measured width is the rendered width.

The visual grammar is the Google Cloud Architecture Center stencil used by the drafting-diagrams skill (`drafting-diagrams/stencil/_gcp.css`, the gold figures under `drafting-diagrams/stencil/golds/`). Each tag below corresponds to one component of that stylesheet.

Authoring happens in CUE (`cue/stencil.cue`). `cue export` produces the JSON this spec defines, and the Rust side never evaluates CUE. The Rust types in section 1 are the contract. `cue/stencil.cue` mirrors them, and where the two disagree the Rust types win and the CUE file is updated.

Repository layout:

```
Cargo.toml                      workspace
crates/stencil-model/           types, serde, JSON Schema, geometry-free checks, TextMeasurer trait
crates/stencil-text/            TextMeasurer on cosmic-text, bundled font loading
crates/stencil-layout/          taffy tree from the model, geometry, geometry checks
crates/stencil-render/          SVG writer, PNG via resvg, measured JSON
crates/stencil-cli/             binary `stencil`
assets/fonts/                   Inter static TTFs, OFL.txt, FONTS.md (asset pre-step, section 8.3)
assets/icons/                   23 Google Cloud icons, PROVENANCE.md (asset pre-step, section 8.3)
schema/stencil.schema.json      generated JSON Schema, committed
examples/g7.json                golden document
examples/golden/                reference render of the g7 gold
cue/                            CUE authoring schema and figures
docs/api-notes/                 verified third-party API notes (taffy, cosmic-text, resvg)
```

## 1. Document model

A document is one `Page`. `Page.body` is a list of nodes. Every node object carries a `tag` field naming its type. Unknown fields are rejected at every level, and so are unknown tags and unknown enum values.

The vocabulary is split into a core and grammars in section 13. The tables below follow it: `Box` and `Item` replace `Zone` and `Pcard`, a grammar supplies the kinds a Box or Item may take (section 13.2), and `line` with `tint` replaces the color-named `kind` on lines (section 13.1). Where this section and section 13 differ, section 13 wins.

### 1.1 Field summary

| Tag | Field | Type | Required | Default |
|---|---|---|---|---|
| Page | title | text | yes | |
| Page | kicker | text | yes | |
| Page | lede | text | yes | |
| Page | foot | text | no | absent |
| Page | width | integer 640 to 2560 | no | 1280 |
| Page | canvas | `customer` or `internal` | yes | |
| Page | grammar | `gcp`, `plain` or a path ending in `.json` (section 13.2) | no | `gcp` |
| Page | chrome | `full` or `none` (section 13.7) | no | `full` |
| Page | body | list of Node, 1 to 256 | yes | |
| Page | legend | list of LegendEntry, 0 to 16 | yes | |
| LegendEntry | line | Line | yes | |
| LegendEntry | tint | integer 1 to 8 (section 13.1) | no | section 13.1 rule 2 |
| LegendEntry | text | text | yes | |
| Row, Col | gap | integer 0 to 64, px | no | 8 |
| Row, Col | grow | list of integers 0 to 100, one per child | no | see section 2.3 |
| Row, Col | justify | `start`, `center`, `end`, `space-between` | no | `start` |
| Row, Col | children | list of Node, 1 to 256 | yes | |
| Lanes | gap, children | as Row, children 1 to 32 (section 13.6) | | 32 |
| Box | kind | a container kind of the grammar (section 13.3 for gcp) | yes | |
| Box | tint | integer 1 to 8 | no | the kind's default |
| Box | label | text | yes | |
| Box | children | list of Node, 1 to 256 | yes | |
| Item | kind | an item kind of the grammar (`product` in gcp) | yes | |
| Item | icon | IconName | no | absent |
| Item | title | text | yes | |
| Item | subtitle | text | no | absent |
| Item | facts | list of fact entries (`text`, `source`), 0 to 8 (section 13.7) | no | empty |
| Fact | text | text | yes | |
| Fact | source | `doc`, `built` or `ask` | no | `doc` |
| Note | kind | `kicker`, `h1`, `lede`, `legend`, `foot` | yes | |
| Note | text | text | yes | |
| Pipe | dir | `h` or `v` | yes | |
| Pipe | line | Line | yes | |
| Pipe | tint | integer 1 to 8 | no | section 13.1 rule 2 |
| Pipe | label | text | yes | |
| Pipe | sub | text | no | absent |
| Tee | line | Line | yes | |
| Tee | tint | integer 1 to 8 | no | section 13.1 rule 2 |
| Tee | hub | text | yes | |
| Tee | arms | exactly two Pipe objects, each `dir: "h"` | yes | |

A text value is a string of 1 to 400 Unicode scalar values with no control characters (U+0000 to U+001F, U+007F). Leading or trailing whitespace is a violation.

An optional field (default `absent`, or `gap`, `grow` and `justify`) written as `null` means absent. serde reads `null` as `None`, and the generated schema admits `null` for every optional field, so the two agree. The measured JSON `document` carries the `null` through unchanged (section 5.4). `width` is not optional in this sense: `"width": null` is a parse error, and only an absent `width` takes the default. `cue export` never writes `null`.

Line: `gray`, `solid`, `dash`, `deny`. The retired PipeKind maps to it as section 13.1's table shows: `blue` and `pink` are `solid` with tint 1 and 2.

Container and item kinds come from the page's grammar (section 13.2). The retired ZoneKind maps onto the gcp grammar of section 13.3: `region-a` and `region-b` are `region` with tint 1 and 2, `onprem-a` and `onprem-b` are `onprem` with tint 1 and 2, and every other zone kind keeps its name.

IconName: the file stem of one of the 23 icons in `assets/icons/` (section 8).

### 1.2 Rust types

Section 13.1 replaces `Zone`, `Pcard`, `ZoneKind`, `PipeKind` and the `kind` fields of Pipe, Tee, Link and LegendEntry in the block below; `crates/stencil-model/src/document.rs` follows section 13.1, and the grammar types live in `crates/stencil-model/src/grammar.rs` (section 13.2).

These definitions are copied verbatim into `crates/stencil-model/src/document.rs`. `schema/stencil.schema.json` is generated from them by schemars.

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PAGE_WIDTH_DEFAULT: u32 = 1280;
pub const PAGE_WIDTH_MIN: u32 = 640;
pub const PAGE_WIDTH_MAX: u32 = 2560;
pub const GAP_DEFAULT_PX: u16 = 8;
pub const GAP_MAX_PX: u16 = 64;
pub const GROW_WEIGHT_MAX: u16 = 100;
pub const TEXT_SCALARS_MAX: usize = 400;
pub const CHILDREN_MAX: usize = 256;
pub const LEGEND_ENTRIES_MAX: usize = 16;
pub const DEPTH_MAX: usize = 24;
pub const NODES_MAX: usize = 4096;

fn page_width_default() -> u32 {
    PAGE_WIDTH_DEFAULT
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[schemars(length(min = 1, max = 400))]
    pub title: String,
    #[schemars(length(min = 1, max = 400))]
    pub kicker: String,
    #[schemars(length(min = 1, max = 400))]
    pub lede: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub foot: Option<String>,
    #[serde(default = "page_width_default")]
    #[schemars(range(min = 640, max = 2560))]
    pub width: u32,
    pub canvas: Canvas,
    #[schemars(length(min = 1, max = 256))]
    pub body: Vec<Node>,
    #[schemars(length(max = 16))]
    pub legend: Vec<LegendEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Canvas {
    Customer,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegendEntry {
    pub kind: PipeKind,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tag")]
pub enum Node {
    Row(Row),
    Col(Col),
    Zone(Zone),
    Pcard(Pcard),
    Fact(Fact),
    Note(Note),
    Pipe(Pipe),
    Tee(Tee),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Row {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 64))]
    pub gap: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grow: Option<Vec<u16>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Col {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 64))]
    pub gap: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grow: Option<Vec<u16>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ZoneKind {
    Gcp,
    Vpc,
    RegionA,
    RegionB,
    Subnet,
    OnpremA,
    OnpremB,
    Project,
    Optional,
    K8s,
    Perimeter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    pub kind: ZoneKind,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pcard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconName>,
    #[serde(rename = "fn")]
    #[schemars(length(min = 1, max = 400))]
    pub function_name: String,
    #[serde(rename = "pn", default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub product_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub fact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub ask: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NoteKind {
    Kicker,
    H1,
    Lede,
    Legend,
    Foot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub kind: NoteKind,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum PipeDir {
    #[serde(rename = "h")]
    Horizontal,
    #[serde(rename = "v")]
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipeKind {
    Gray,
    Blue,
    Pink,
    Dash,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pipe {
    pub dir: PipeDir,
    pub kind: PipeKind,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub sub: Option<String>,
}

/// A Tee arm is written with `"tag": "Pipe"`, the same object shape as a Pipe node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tag")]
pub enum TeeArm {
    Pipe(Pipe),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tee {
    pub kind: PipeKind,
    #[schemars(length(min = 1, max = 400))]
    pub hub: String,
    pub arms: [TeeArm; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum IconName {
    Agents,
    AiMl,
    Bigquery,
    CloudRunFlat,
    CloudRun,
    CloudSql,
    CloudStorage,
    ComputeEngine,
    Compute,
    Containers,
    DataAnalytics,
    Databases,
    Devops,
    Gke,
    Hybrid,
    Integration,
    Networking,
    Observability,
    Scc,
    SecurityIdentity,
    Serverless,
    Storage,
    VertexAi,
}
```

`IconName::file_name(self) -> &'static str` returns the stem plus `.svg` (`IconName::AiMl` is `ai-ml.svg`). `Node::tag_name(&self) -> &'static str` returns the `tag` string.

Two serde behaviors are load-bearing and each has a test (section 10). First, `deny_unknown_fields` on the payload structs must reject an unknown field inside a tagged node, even though the tag is on the enum. Second, the generated schema must accept the `tag` property on every node object and reject unknown properties.

### 1.3 Validation (vet rules)

Section 13.1 adds the grammar-aware rules (`tint-out-of-range`, `grammar-unknown`, `kind-unknown`, `kind-parent-not-allowed`, `icon-outside-pack`, `facts-too-many`, `lanes-too-many`, `lanes-in-iso`) and makes `validate_page` take the resolved grammar. Pointers below that name `fn` or `pn` read `title` and `subtitle` on an Item.

serde handles types and required fields. `validate_page` then enforces the rules below and returns every violation it finds, not only the first. Each violation carries the JSON Pointer and the message listed for its rule. Pointers use the serialized field names (`fn`, `pn`, `dir`), and `…` stands for the pointer of the node that holds the field. In a message, `<n>`, `<g>`, `<c>` and `<w>` are decimal integers and `<XXXX>` is four uppercase hex digits. `VetRule::as_str(self) -> &'static str` returns the rule name in kebab-case as in the first column, so the CLI does not carry a second copy of the names.

| Rule | Condition | Pointer | Message |
|---|---|---|---|
| `text-empty` | a text value is empty | the text field, for example `/title`, `/legend/2/text` or `/body/0/children/1/fn` | `text is empty` |
| `text-too-long` | a text value exceeds 400 Unicode scalar values | the text field | `text has <n> scalar values, above 400` |
| `text-control-character` | a text value contains U+0000 to U+001F or U+007F | the text field | `text contains control character U+<XXXX>`, naming the first one |
| `text-untrimmed` | a text value starts or ends with whitespace | the text field | `text starts or ends with whitespace` |
| `page-width-out-of-range` | `width` outside 640 to 2560 | `/width` | `width <n> is outside 640 to 2560` |
| `body-empty` | `body` has no nodes | `/body` | `body has no nodes` |
| `children-empty` | a Row, Col or Zone has no children | `/…/children` | `container has no children` |
| `children-too-many` | a Row, Col or Zone has more than 256 children, or `body` has more than 256 nodes | `/…/children`, or `/body` | `<n> children, above 256` |
| `legend-too-long` | `legend` has more than 16 entries | `/legend` | `legend has <n> entries, above 16` |
| `gap-out-of-range` | `gap` above 64 | `/…/gap` | `gap <n> is above 64` |
| `grow-length-mismatch` | `grow` present and its length differs from `children` length | `/…/grow` | `grow has <g> weights for <c> children` |
| `grow-out-of-range` | a `grow` weight above 100 | `/…/grow/<i>`, one violation per weight among the first 257 (CHILDREN_MAX + 1); `grow-length-mismatch` covers a longer list | `grow weight <w> is above 100` |
| `tee-arm-not-horizontal` | a Tee arm has `dir: "v"` | `/…/arms/<i>/dir` | `Tee arm dir is "v", expected "h"` |
| `depth-exceeded` | a node's depth is above 24 (depth as defined below) | the node | `depth <n> is above 24` |
| `nodes-exceeded` | more than 4096 nodes, Tee arms included (count as defined below) | `/body` | `more than 4096 nodes` |

An empty `body` gives `body-empty`, never `children-empty`.

Depth and node count have one definition, shared by `validate_page`, `NodeEntry.depth` and `cue/stencil.cue` wherever it expresses these limits. Each element of `body` has depth 1. A child of a Row, Col or Zone at depth d has depth d + 1, and each arm of a Tee at depth d has depth d + 1. The node count is the length of `body_nodes(page)`: every body node at any depth plus every Tee arm. Page-level nodes (`/kicker`, `/title` and the others in section 1.4) are not counted. `body_nodes` stops after NODES_MAX + 1 entries, and it takes at most NODES_MAX + 1 elements of `body` and of each `children` list onto its pending stack, because the k-th element of a list cannot be emitted before k earlier entries, so an element past that index is never reached. `validate_page` reports `nodes-exceeded` at `/body` when the result has NODES_MAX + 1 entries, and checks the per-node rules on the entries `body_nodes` returned, so a page past the limit also gets the per-node violations of its first 4097 nodes. Layout only calls `body_nodes` on a vetted page. The text rules read the text values of those same entries, the page-level fields and the first LEGEND_ENTRIES_MAX + 1 legend entries, so a legend past the limit gets one `legend-too-long` and the text violations of its first 17 entries. `text_fields` is called only on a vetted page. `legend_consistency` and the SVG writer's geometry order also read only the first LEGEND_ENTRIES_MAX + 1 legend entries, because both are public over any `&Page`. Every walk over document content therefore shares a vet bound. `depth-exceeded` reports each node at depth 25 and does not report its descendants again. `nodes-exceeded` reports once, at `/body`.

A Page with no Pipe and no Tee passes vet but fails the legend check with zero examined (section 6). This is intended. Every stencil figure has at least one hop, and a figure without one is outside the stencil's scope. Implementers must not special-case it.

### 1.4 Node identity

The input has no id fields. A node's id is the RFC 6901 JSON Pointer of its object in the input document, for example `/body/0/children/2/children/0`. The page root is the empty pointer `""`. The layout also creates page-level nodes for the page's own fields, and their pointers resolve to those fields: `/kicker`, `/title`, `/lede`, `/body` (the array), `/legend` (the array), `/legend/0` and so on, and `/foot`. Tee arms are nodes at `/…/arms/0` and `/…/arms/1`.

The same pointers appear as `data-id` in the SVG, as keys in the measured JSON and in every check message. Each pointer resolves with `serde_json::Value::pointer` against the input document.

## 2. Layout

### 2.1 General rules

- Layout builds one taffy 0.14 tree (`taffy = "=0.14.0"`). Every node and every visual part is a taffy node, including icons, text leaves (one leaf per string, not one per line), wires, dots, tags and label bands. Text leaves carry a context and are sized by the measure function (section 3). API details that differ from older taffy releases are recorded in `docs/api-notes/taffy.md`. In particular, the 0.14 closure receives `LayoutInput` and returns `LayoutOutput`, taffy calls it only for childless nodes, and leaves are measured through `taffy::compute_leaf_layout`.
- Rounding is off (`TaffyTree::disable_rounding()`), and geometry is read through `unrounded_layout`. Absolute positions are computed once, in a pre-order walk that sums parent-relative locations. The walk is bounded by the node count.
- Boxes use border-box sizing. Border widths are set on the taffy `border` rect so content offsets match the stroke the renderer draws.
- Body nodes carry no margin, so spacing between them comes from `gap` only. Margins appear in two places: on the page-level nodes (the root's children), as listed in section 2.2, and between parts inside a node, as listed in sections 2.4 to 2.8.
- `flex_shrink` is 0 for every taffy node, body nodes and parts alike. Content that does not fit overflows its container, and the child-inside-container check reports it. The one exception is the Pipe tag part, which shrinks and wraps before a wire drops below its minimum length. Shrink 0 with `flex_basis` auto holds an item at its max-content size, so a part whose text must wrap inside its parent (the Pcard `text` column in section 2.5 and the `/kicker` text part in section 2.2) takes `flex_grow` 1 and `flex_basis` 0 instead, and keeps min-width auto so its widest word still sets its floor.
- A CSS `normal` line height is 1.2 times the font size. Every text style has an explicit line height (section 2.9).
- Every definite width the measure closure passes to the measurer carries 0.01 px extra (`WRAP_EPSILON_PX`, section 3). Once `compute_layout_with_measure` returns, every text leaf is measured again at its final width plus the same epsilon. The result is stored as that leaf's `TextRun`, which holds the line breaks the renderer draws and the metrics the text-fit check compares. The epsilon keeps a string laid out at exactly its max-content width on one line, during layout and in the re-measure alike. taffy computes in f32, so a parent sized to a text's max-content width can hand the leaf back a width one ulp short of it (a Pipe tag is label + 16 + 3 wide, and label + 19 - 19 can come out below label). If the closure passed that width unchanged, the leaf would wrap during layout and taffy would size its box for two lines, while the re-measure at +0.01 finds one line. text-fits-box would not catch the resulting one-line run in a two-line box, because the run is shorter than its box.
- Alignment keywords in sections 2.2 to 2.8 are written as the taffy 0.14 associated consts: `AlignItems::STRETCH`, `AlignItems::CENTER`, `AlignSelf::STRETCH`, `AlignSelf::CENTER`, `JustifyContent::START` and so on. In 0.14 these types are structs with a `keyword` and a `safety` field, not enums, so `AlignItems::Stretch` does not compile. The consts carry the default `AlignmentSafety::Unsafe`, and that is intended: a centered child that overflows stays centered and overflows both edges, where it is visible and child-inside-container reports it. `docs/api-notes/taffy.md` lists the types and the `Justify` mapping.

### 2.2 Page

The root is a flex column with `align_items` set to `AlignItems::STRETCH`, so `/title`, `/lede`, `/legend` and `/foot` take the full content width (1280 px at the default width). taffy's `None` also behaves as stretch, but the root sets it explicitly. Its `size.width` is definite at `width + 40`, so the default canvas is 1320 px wide. Padding is 20 on every side. The root style carries the definite width because percentage and stretch sizing resolve against the root's own style, not against the `available_space` argument (see `docs/api-notes/taffy.md`). The layout is computed with `Definite(width + 40)` by `MaxContent`. The canvas height is the root's computed height.

Root children, top to bottom:

| Pointer | Box | Inside |
|---|---|---|
| `/kicker` | flex row, `AlignItems::CENTER`, gap 6, margin-bottom 6 | badge part (padding 2/7, radius 4, text style `badge`, text `Customer` or `Internal` uppercased), kicker text part (style `kicker`, uppercased, `flex_grow` 1, `flex_basis` 0, min-width auto, so a long kicker wraps inside the page width) |
| `/title` | text leaf | style `title` |
| `/lede` | text leaf, margin 4 top, 16 bottom | style `lede` |
| `/body` | flex column, gap 8, `AlignItems::STRETCH` | the body nodes, each with `flex_grow` 0 and `flex_basis` auto |
| `/legend` | omitted when `legend` is empty; flex row, wrap, column gap 16, row gap 6, margin-top 10 | one LegendEntry node per entry |
| `/foot` | omitted when `foot` is absent; text leaf, margin-top 8 | style `foot` |

A LegendEntry node is a flex row, `AlignItems::CENTER`, gap 6, holding three parts. The swatch is 26 by 2 and is drawn as a wire: a `<line>` along its box's center line with `stroke-width="2"` and the section 5.2 color and dasharray for its kind; an entry with `form: band` draws a `<rect>` 26 by `BAND_SWATCH_HEIGHT_PX` = 8 centered on that line instead, filled in the wire color, or filled with the page background and outlined 1.5 px in the line's pattern when the line is patterned, as the band of section 12.7 is. The label uses style `legend_label` with the fixed text for its kind. The text uses style `legend_text`.

| PipeKind | Legend label |
|---|---|
| gray | Solid gray |
| blue | Solid blue |
| pink | Solid pink |
| dash | Dashed blue |
| deny | Dashed red |

### 2.3 Row and Col

| | Row | Col |
|---|---|---|
| taffy display | Flex | Flex |
| flex_direction | Row | Column |
| gap (main axis) | `gap` or 8 | `gap` or 8 |
| align_items | `AlignItems::STRETCH` | `AlignItems::STRETCH` |
| justify_content | from `justify`, default `JustifyContent::START` | from `justify`, default `JustifyContent::START` |
| padding, border | 0 | 0 |

`justify` maps `start` to `JustifyContent::START`, `center` to `CENTER`, `end` to `END` and `space-between` to `SPACE_BETWEEN`. `START` is used rather than `FLEX_START`; the two place items identically in the non-reversed directions stencil uses.

Grow weights distribute free space along the main axis the way CSS grid `fr` tracks do.

- `grow` absent on a Row: every child that is not a Pipe or Tee gets weight 1, and Pipe and Tee children get weight 0. The rule looks at the child's own tag only. A Col or Row that holds only Pipes, such as the g7 VLAN gutter, gets weight 1 and takes an equal share of the width with the card columns, so a figure with that shape sets `grow` explicitly, as g7 does with `"grow": [0, 0, 1]`.
- `grow` absent on a Col: every child gets weight 0, so each child keeps its content height and `justify` places the stack. This follows `_gcp.css`, where `.pair` (the Row analog) is `1fr 1fr` and `.stack` (the Col analog) does not grow its children. A Col that should split its height, like the VLAN column and the on-prem column in g7, is written with explicit weights such as `"grow": [1, 1]`.
- Weight w > 0: `flex_grow = w`, `flex_basis = 0`, `min_size` main axis auto. The child's content-based minimum still applies, so it never goes below min-content.
- Weight 0: `flex_grow = 0`, `flex_basis = auto`, so the child takes its max-content size.

A Row of two Pcards therefore gives two equal columns, like `.pair`. A Row of Pcard, Pipe, Pcard gives two equal card columns with the pipe at its natural width between them, like the gutter columns of the dense figures. A frame that should take all remaining width, like the Google Cloud frame in g7, is written with explicit weights such as `"grow": [0, 0, 1]`.

### 2.4 Zone

A Zone is a flex column with `AlignItems::STRETCH` and gap 8. Every child node of a Zone (for gcp, of its `body` part) has `flex_grow` 0 and `flex_basis` auto. Its first child is the label band. The label band is the label text leaf (part `Label`), and the 8 px gap under it separates it from the first child node. The band height is therefore the label style's line height plus 8.

| kind | border (px, style, color) | fill | radius | padding | label style |
|---|---|---|---|---|---|
| gcp | 3 solid #1A73E8 | frame #FFFFFF, body #FAFBFC | 4 4 10 10 | see below | `gcp_bar` |
| vpc | 2 dashed #5F6368 | none | 8 | 10 | `zone_label` |
| region-a | 1.5 solid #BDC1C6 | #D2E3FC | 8 | 12 | `zone_label` |
| region-b | 1.5 solid #BDC1C6 | #FCE4EC | 8 | 12 | `zone_label` |
| subnet | 1.5 dashed #9AA0A6 | #EDE7F6 | 8 | 12 | `zone_label` |
| onprem-a | 1.5 solid #D7CCC8 | #D2E3FC | 8 | 12 | `zone_label` |
| onprem-b | 1.5 solid #D7CCC8 | #FCE4EC | 8 | 12 | `zone_label` |
| project | 1.5 solid #FFE082 | #FFF8E1 | 8 | 12 | `zone_label` |
| optional | 2 dashed #4284F3 | #F8FBFF | 8 | 12 | `zone_label` |
| k8s | none (0) | #FCE4EC | 8 | 12 | `zone_label` |
| perimeter | 2.5 dashed #E37400 | #FFFBF5 | 10 | 12 | `perimeter_label` |

The a and b tints follow the component reference for this stencil. Metro 1 and Region A are blue, and Metro 2 and Region B are pink, from end to end. The g7 gold HTML sets the same tints, but `.zone.region` and `.zone.onprem` override them in the gold's cascade. Its Chrome render therefore shows neutral gray regions and neutral warm-gray on-prem zones, and the golden comparison tolerates both fill differences (section 9.4). Where the gold and this table disagree, this table is the contract.

The gcp kind is the Google Cloud frame. It is a flex column with padding 0, border 3 and gap 0, and it holds two parts:

- `bar`: padding 7 top and bottom, 16 left and right, fill #1A73E8, containing the label text leaf (part `Label`, style `gcp_bar`). Its height is 7 + 21.6 + 7 = 35.6 px.
- `body`: flex column, `flex_grow` 1, padding 16 top, 14 left, right and bottom, gap 8, `AlignItems::STRETCH`, fill #FAFBFC. It contains the zone's child nodes.

The child-inside-container check measures gcp children against the `body` part's content box.

### 2.5 Pcard

- Flex row, `AlignItems::CENTER`, gap 10, padding 6 top and bottom, 10 left and right, border 1.5 #DADCE0, radius 8, fill #FFFFFF, `min_size.height` 44.
- The `icon` part is a 28 by 28 leaf, present only when `icon` is set.
- The `text` part is a flex column with `flex_grow` 1, `flex_basis` 0, min-width auto and `AlignItems::STRETCH`. Basis 0 gives the column the card's remaining width instead of its max-content width, so `fn`, `pn`, `fact` and `ask` wrap inside the card. Min-width auto keeps the widest word as the column's floor, so a single word wider than the card widens the card instead of overflowing its own text box: the card then overflows its container, which child-inside-container reports, and the run still fits its part box, so text-fits-box does not report it. In order it holds:
  - `fn`: style `card_function`.
  - `pn`: style `card_product`, margin-top 1.
  - `fact_box`: margin-top 4, padding 4/8, radius 4, fill #F1F3F4, containing `fact` (style `fact`).
  - `ask_box`: margin-top 4, padding 4/8, radius 4, fill #FEF7E0, containing `ask` (style `ask`). The rendered and measured text is `Ask: ` followed by the value.
- With icon, `fn` and `pn`, the card is 46 px tall when both lines fit on one line: 6 + 15.6 + 1 + 14.4 + 6 = 43 of content and padding plus 3 px of border. The 44 px minimum applies only to shorter cards: with icon and `fn` and no `pn`, 6 + 28 + 6 + 3 = 43 is raised to 44. A card with a fact or ask grows to fit it.

### 2.6 Fact and Note

- Fact: a flex column with `AlignItems::STRETCH`, padding 4 top and bottom, 8 left and right, radius 4, fill #F1F3F4, containing one text leaf (part `Text`) in style `fact`. The stretch gives the leaf the Fact's content width, so a long fact wraps.
- Note: a single text leaf. Its style follows `kind`: `kicker` is `kicker` (uppercased), `h1` is `title`, `lede` is `lede`, `legend` is `note_legend`, and `foot` is `foot`. A Note never draws the canvas badge, which appears only on the page kicker. A Note with kind `legend` is free legend-styled text. The legend check does not examine it.

### 2.7 Pipe

A Pipe is a laid-out element in the gutter between two boxes, the same construction as `.pipe` in `_gcp.css`. It is never an overlay and has no endpoints in the model.

Internal structure, in order along the run axis: `dot_start`, `wire_start`, `tag`, `wire_end`, `dot_end`.

| Part | h | v |
|---|---|---|
| pipe box | flex row, `AlignItems::CENTER`, `JustifyContent::CENTER` | flex column, `AlignItems::CENTER`, `JustifyContent::CENTER`, `min_size.height` 36 |
| dot_start, dot_end | 8 by 8, shrink 0 | 8 by 8, shrink 0 |
| wire_start, wire_end | `flex_grow` 1, `flex_basis` 0, `min_size.width` 14, height 2 | `flex_grow` 1, `flex_basis` 0, `min_size.height` 12 (16 for `deny`, as `.pipe.deny.v .wire` in `_gcp.css`), width 2 |
| tag | flex column, `AlignItems::CENTER`, padding 6/8, border 1.5, radius 6, fill #FFFFFF, `flex_shrink` 1, `max_size.width` 100% | same, no shrink needed |
| label | text leaf, style `tag_label`, centered | same |
| sub | text leaf, style `tag_sub`, margin-top 2, centered, only when `sub` is set | same |

Both wires take an equal share of the free run length, so the tag sits in the middle of the gutter. Tag border color is #DADCE0. For `deny` the tag border is #F4C7C3 and the label color is #C5221F.

A Pipe with `arrow` (section 11.2) draws an arrowhead in place of the dot at each end it names. The tip lies on the dot box's outer edge along the run axis and the base `ARROWHEAD_LENGTH_PX` (10) inward, so the wire on that end stops at the base: after layout, `wire_start` or `wire_end` gives up 2 px on its dot side. The dot part keeps its 8 by 8 box, which is where the arrowhead is drawn, and no other box moves.

How a pipe fills its gutter depends on how its run axis relates to the parent's axes.

- The run axis is the parent's cross axis (h inside a column container, v inside a Row). `align_self` is `AlignSelf::STRETCH`, so the pipe spans the full gutter and the wires take up the length.
- The run axis is the parent's main axis (h inside a Row, v inside a column container). `align_self` is `AlignSelf::CENTER`. The pipe's run length is its max-content length (two dots, two minimum wires, the tag) unless the parent gives it a grow weight. For a v pipe that length is at least 36 px.

Column containers are Col, every Zone (for gcp, its `body` part) and the Page body. A Tee arm sits in a grid cell, not a flex container, and section 2.8 governs its alignment.

### 2.8 Tee

A Tee maps to a taffy grid, following `.tee` in `_gcp.css`.

- Grid: `grid_template_columns = [length(14), fr(1)]`, `grid_template_rows = [fr(1), auto, fr(1)]`, `min_size.width` 118. Inside a Row, `align_self` is `AlignSelf::STRETCH`. Inside a column container the parent's `AlignItems::STRETCH` stretches it across.
- `spine` part: `grid_column` line 1, `grid_row` lines 1 to 4 (`Line { start: line(1), end: line(4) }`, or `end: line(-1)`; not `span(4)`, which from line 1 creates an implicit fourth row in this 3-row template), width 2, `justify_self` `AlignSelf::CENTER`, margin 18 top and bottom. It is drawn like a wire: a `<line>` along the spine box's center line, `stroke-width="2"`, in the section 5.2 wire color and dasharray for the Tee's `kind`, so `dash` and `deny` spines are dashed.
- `hub` part: `grid_row` line 2, `grid_column` lines 1 to 3, `justify_self` `AlignSelf::CENTER`. It is a tag box built like the Pipe tag, holding one text leaf `hub_text` in style `tag_label`. Its border and label colors follow the Pipe tag rule in section 2.7: a `deny` Tee's hub has border #F4C7C3 and label color #C5221F, and every other kind has border #DADCE0 and label color #202124.
- Arms: the arm at `/…/arms/0` goes in `grid_row` line 1 and the arm at `/…/arms/1` in `grid_row` line 3, both in `grid_column` line 2, with `align_self` `AlignSelf::CENTER` and `justify_self` `AlignSelf::STRETCH` (`justify_self` has type `Option<AlignSelf>` in 0.14). Each arm is a Pipe h laid out as in section 2.7, with its own kind and label.

The hub is drawn after the spine. The two may overlap, and both are parts, so the overlap check does not examine them.

### 2.9 Text styles

Every string on the canvas uses one of these styles. Family, weight, size, line height, letter spacing and the uppercase flag are defined once in `stencil_model::text::TEXT_STYLES` (section 3), and the idempotence and parity tests in section 10 iterate over that list. Code names a style by `TextStyleName`, never by string, so a misspelled style does not compile. The colors live in `stencil_layout::styles`, keyed by `TextStyleName`, because `badge` depends on the canvas and `tag_label` on the pipe kind, which a style value does not carry.

| Style | Weight | Size px | Line height px | Letter spacing em | Color | Transform |
|---|---|---|---|---|---|---|
| badge | 800 | 10 | 12.0 | 0.07 | customer #174EA6 on #E8F0FE, internal #7B1FA2 on #F3E5F5 | uppercase |
| kicker | 700 | 11 | 13.2 | 0.08 | #1A73E8 | uppercase |
| title | 700 | 20 | 24.0 | -0.02 | #202124 | |
| lede | 400 | 13 | 15.6 | 0 | #5F6368 | |
| zone_label | 700 | 12 | 14.4 | 0 | #5F6368 | |
| gcp_bar | 700 | 18 | 21.6 | 0.01 | #FFFFFF | |
| perimeter_label | 700 | 13 | 15.6 | 0 | #B06000 | |
| card_function | 700 | 13 | 15.6 | 0 | #202124 | |
| card_product | 400 | 12 | 14.4 | 0 | #5F6368 | |
| fact | 600 | 12 | 16.2 | 0 | #5F6368 | |
| ask | 600 | 12 | 16.2 | 0 | #B06000 | |
| tag_label | 700 | 12 | 15.6 | 0 | #202124 (deny #C5221F) | |
| tag_sub | 600 | 12 | 15.6 | 0 | #5F6368 | |
| note_legend | 400 | 12 | 14.4 | 0 | #5F6368 | |
| legend_label | 700 | 12 | 14.4 | 0 | #202124 | |
| legend_text | 400 | 12 | 14.4 | 0 | #5F6368 | |
| foot | 400 | 11 | 13.2 | 0 | #5F6368 | |
| block_body | 400 | 12 | 17.4 | 0 | #202124 | |

The section 11.3 blocks reuse three of these: the Text heading and the Callout title are `card_function`, the Frame label is `zone_label`, and the Text body lines, numbered list markers and Callout text are `block_body`.

All styles use family Inter. Uppercasing uses `str::to_uppercase` and runs before measurement, so the measured string is the rendered string. The remembered-constants check reads the untransformed input.

### 2.10 Dense figures

The densest existing figure in this diagram set is `lytx/docs/diagrams/hercules-dataflow.html` in the dp monorepo. It has a 13-track grid, a service perimeter behind several rows, expanded cards with chip lists, numbered step badges inside pipe tags and zones that span rows as backgrounds. The MVP does not reproduce it. The model must not rule it out either, and each feature maps as follows.

| Feature | MVP | Later |
|---|---|---|
| Perimeter zone | ZoneKind `perimeter` | |
| Cards with a looked-up fact | Pcard `fact`, Fact node | |
| Gutter columns of fixed width between card columns | Row with `grow` weights: cards weight 1, pipes weight 0 | |
| Columns aligned across many rows | not available: each Row sizes its own columns | a `Grid` tag with named column tracks shared by its rows |
| Zone behind a band of rows | a Zone wrapping a Col of Rows | spanning backgrounds on `Grid` |
| Expanded card with chips | not available | an `Xcard` tag |
| Numbered step badge in a tag | not available | a `step` field on Pipe |

The golden stress test (section 10, stencil-layout) builds a document shaped like this figure from MVP tags only. It checks that layout completes and that every check passes, so an MVP change that made the dense shape unworkable fails a test.

### 2.11 Parts per node

Layout emits each node's parts in the order below, and render, the measured JSON and the text-fits-box check read them from `NodeGeometry.parts`. A part in brackets is present only when its field is set. Parts in the last column carry a `TextRun`, and every other part has `text: None`. The TagLabel, TagSub and HubText runs and the Frame Label run have `TextAlign::Center`, and every other run has `TextAlign::Start`.

| NodeTag | Parts, in order | Parts carrying a TextRun |
|---|---|---|
| Page, Body, Legend, Row, Col | none | |
| Kicker | Badge, BadgeText, Text | BadgeText (style `badge`), Text (style `kicker`) |
| Title, Lede, Foot, Note | Text | Text |
| LegendEntry | Swatch, LegendLabel, LegendText | LegendLabel, LegendText |
| Zone, kind other than gcp | Label | Label |
| Zone, kind gcp | Bar, Label, Body | Label |
| Pcard | [Icon], Text, FunctionName, [ProductName], [FactBox, Fact], [AskBox, Ask] | FunctionName, [ProductName], [Fact], [Ask] |
| Fact | Text | Text (style `fact`) |
| Pipe, as a body node or a Tee arm | DotStart, WireStart, Tag, TagLabel, [TagSub], WireEnd, DotEnd | TagLabel, [TagSub] |
| Tee | Spine, Hub, HubText | HubText |
| Text | [Heading], then for each body line [Marker], BodyLine | [Heading], [Marker] for `numbered`, BodyLine |
| Callout | Accent, [Heading], Text | [Heading], Text |
| Frame | LabelChip, Label | Label |

- A text-leaf node (Title, Lede, Foot, Note) is itself the taffy text leaf. It carries one `Text` part whose bounds equal its border box.
- The Fact node's `Text` part is its text leaf, with bounds equal to the Fact's content box.
- The Pcard `Text` part is the flex column that holds the other text parts. It carries no run.
- The Zone `Label` part is the label band's text leaf. In a gcp zone it lies inside `Bar`.
- The two arms of a Tee are child nodes with tag Pipe, not parts of the Tee.
- A Text block repeats Marker and BodyLine once per body line, in line order, so the i-th BodyLine part, and the i-th Marker when the list is `numbered` or `bulleted`, belong to `/…/body/i`. A `plain` Text has no Marker parts. A numbered Marker carries the run `1.`, `2.` and so on; a bulleted Marker carries no run and is the 22 by 17.4 cell the 4 px dot is centered in.
- The Callout Accent is the 4 px bar along the inside of the left border, as tall as the border box minus the top and bottom borders.
- The Frame LabelChip is the page-background box behind the label, so the diagonals stop at its edges.

The text-fits-box check examines exactly the present parts in the last column, so the g7 count of 40 in section 9.4 follows from this table.

## 3. Text measurement

The trait and its value types live in `stencil_model::text`. Layout depends on the trait only, never on cosmic-text, so layout tests run against a fixed-metrics fake. stencil-text implements the trait on cosmic-text.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFamily {
    Inter,
}

impl FontFamily {
    pub fn css_name(self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Regular,
    SemiBold,
    Bold,
    ExtraBold,
}

impl FontWeight {
    /// 400, 600, 700, 800.
    pub fn css_value(self) -> u16;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub family: FontFamily,
    pub weight: FontWeight,
    pub size_px: f32,
    pub line_height_px: f32,
    pub letter_spacing_em: f32,
}

/// Added to every definite width layout passes to a measurer (section 2.1).
pub const WRAP_EPSILON_PX: f32 = 0.01;

/// The 18 styles of section 2.9, in table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextStyleName {
    Badge,
    Kicker,
    Title,
    Lede,
    ZoneLabel,
    GcpBar,
    PerimeterLabel,
    CardFunction,
    CardProduct,
    Fact,
    Ask,
    TagLabel,
    TagSub,
    NoteLegend,
    LegendLabel,
    LegendText,
    Foot,
    BlockBody,
}

impl TextStyleName {
    /// The section 2.9 name, for example "tag_label".
    pub fn as_str(self) -> &'static str;
    /// The TEXT_STYLES entry whose `name` is self.
    pub fn text_style(self) -> &'static NamedTextStyle;
}

/// One row of the section 2.9 table, colors excluded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamedTextStyle {
    pub name: TextStyleName,
    pub style: TextStyle,
    /// True for `badge` and `kicker`: the string is uppercased before measurement.
    pub uppercase: bool,
}

/// The 18 styles of section 2.9, in table order, which is TextStyleName order:
/// TEXT_STYLES[i].name as usize == i.
pub const TEXT_STYLES: [NamedTextStyle; 18];

#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    /// Byte range of this line in the measured string, trailing spaces excluded.
    pub byte_start: usize,
    pub byte_end: usize,
    pub width_px: f32,
    /// Baseline offset from the top of the text block.
    pub baseline_px: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextMetrics {
    pub width_px: f32,
    pub height_px: f32,
    pub line_count: u32,
    pub lines: Vec<TextLine>,
}

pub trait TextMeasurer {
    fn measure(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width_px: Option<f32>,
    ) -> Result<TextMetrics, MeasureError>;
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MeasureError {
    #[error("text is empty")]
    EmptyText,
    #[error("text style is out of range: {reason}")]
    InvalidStyle { reason: &'static str },
    #[error("max width {max_width_px} is negative or not finite")]
    InvalidMaxWidth { max_width_px: f32 },
    #[error("font {family} has no glyph for {character:?} (U+{codepoint:04X})")]
    MissingGlyph { family: &'static str, character: char, codepoint: u32 },
    #[error("text backend failed: {message}")]
    Backend { message: String },
}
```

`TEXT_STYLES` and `WRAP_EPSILON_PX` live in stencil-model, not stencil-layout, because stencil-text's tests use both and stencil-text depends only on stencil-model. stencil-layout re-exports `WRAP_EPSILON_PX`.

Contract, which every implementation and the fake satisfy:

- `max_width_px: None` means no wrapping, so the result is the max-content size. `Some(w)` wraps at word boundaries. A word wider than `w` stays whole on its own line, and the reported `width_px` then exceeds `w`. `Some(0.0)` gives the min-content width, which is the widest word.
- A word is an unbreakable segment between two line-break opportunities, and the two implementations find those differently. For `CosmicTextMeasurer` they are the UAX #14 opportunities that cosmic-text's `Wrap::Word` uses, so a hyphen or slash can end a word: `"On-prem router 1"` at `Some(0.0)` gives the lines `On-`, `prem`, `router` and `1`, and `"link-local /29"` gives `link-` and `local /29`. For the fake a word is a run of characters between U+0020 spaces (section 3.1). In both, a line's `width_px` excludes trailing spaces.
- `line_count == lines.len() as u32`, and `line_count >= 1`.
- `width_px` is the maximum of `lines[i].width_px`, and `height_px == line_count as f32 * line_height_px`.
- Letter spacing is added after every shaped glyph, the last one included. cosmic-text adds it per glyph (`x_advance + letter_spacing` in `shape.rs`), while CSS and SVG `letter-spacing` add it per character. The two agree for text with no ligatures and no multi-glyph clusters, and the one-directional parity test in section 10 tolerates the difference. The fake does no shaping, so for it a glyph is a Unicode scalar value.
- The function has no side effects that change later results. The same input gives bit-identical output.
- Boundary assertions return errors. Empty text gives `EmptyText`. A size outside 6 to 96, a line height below the size or not finite, or a letter spacing outside -0.2 to 0.5 gives `InvalidStyle`. A negative or non-finite `max_width_px` gives `InvalidMaxWidth`. A character with no glyph in the bundled family gives `MissingGlyph`.

How layout calls the measurer: taffy's closure cannot return an error. The closure records the first `MeasureError` together with the node's pointer and returns a zero size. After `compute_layout_with_measure` returns, `layout_page` returns `LayoutError::Measure` if an error was recorded. The closure maps taffy's inputs as follows: a known width becomes `Some(width.max(0.0) + WRAP_EPSILON_PX)`, `AvailableSpace::Definite(w)` becomes `Some(w.max(0.0) + WRAP_EPSILON_PX)`, `MinContent` becomes `Some(0.0)` and `MaxContent` becomes `None`. The epsilon is the one the post-layout re-measure adds, so layout and re-measure agree on the line breaks (section 2.1). The clamp comes first because `taffy::compute_leaf_layout` subtracts margins from the available space, and a negative width would turn an overflow, which is a defect (exit 1), into `InvalidMaxWidth` (exit 2). `f32::max` also maps a NaN width to 0.

The closure also runs for leaves with no context (icon, dot, wire, swatch, spine), because taffy calls it for every childless node. Context `None` goes through `taffy::compute_leaf_layout` with a measure function that returns `Size::ZERO`, so the leaf's style `size` and `min_size` decide its box.

### 3.1 Fixed-metrics fake

`stencil_model::text::FixedMetricsMeasurer` is public so tests in other crates can use it.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct FixedMetricsMeasurer {
    /// Advance of every character, including spaces, in em.
    pub advance_em: f32,
    /// Characters reported as MissingGlyph.
    pub missing_glyphs: Vec<char>,
}

impl Default for FixedMetricsMeasurer {
    fn default() -> Self {
        FixedMetricsMeasurer { advance_em: 0.5, missing_glyphs: Vec::new() }
    }
}

impl TextMeasurer for FixedMetricsMeasurer { /* rules below */ }
```

- Every Unicode scalar value advances `advance_px = (advance_em + letter_spacing_em) * size_px`, computed once in f32. A line of n scalars, interior spaces included and the trailing space excluded, is `n as f32 * advance_px` wide, one multiplication, never a running sum.
- Line breaks happen only at U+0020. Wrapping is greedy, and a line's width excludes its trailing space.
- Line `i` has `baseline_px = i as f32 * line_height_px + 0.8 * line_height_px`.
- Worked example: `"On-prem router 1"` in `card_function` (13 px, no spacing) is 16 scalars times 6.5, which is 104.0 px wide and 15.6 px tall, on one line.

### 3.2 cosmic-text implementation

`stencil_text::CosmicTextMeasurer` holds a `cosmic_text::FontSystem`. It is built with `FontSystem::new_with_locale_and_db("en-US".into(), database)`, where `database` holds only the four bundled Inter faces (section 8). System fonts are never loaded, so font fallback cannot pick another face.

Each measurement follows these steps:

1. Create a `Buffer` with `Metrics::new(size_px, line_height_px)`.
2. Set `Attrs` to family `Inter`, the weight and `letter_spacing(letter_spacing_em)`.
3. Call `set_size(max_width_px, None)` and use `Wrap::Word`.
4. Call `set_text` with `Shaping::Advanced`.
5. Read `layout_runs()`. Each run gives one `TextLine`: `width_px` from `line_w`, `baseline_px` from `line_y`, and the byte range from the first and last glyph clusters, trimmed of trailing spaces.

Any glyph with `glyph_id == 0` produces `MissingGlyph` for the character at that cluster. Results may be cached by `(text, style bits, max width bits)`.

## 4. Crate APIs

### 4.1 Workspace

```toml
[workspace]
resolver = "3"
members = [
    "crates/stencil-model",
    "crates/stencil-text",
    "crates/stencil-layout",
    "crates/stencil-render",
    "crates/stencil-cli",
]

[workspace.package]
edition = "2024"
rust-version = "1.94"
publish = false

[workspace.dependencies]
taffy = "=0.14.0"
cosmic-text = { version = "=0.19.0", default-features = false, features = ["std", "swash"] }
resvg = { version = "=0.48.1", default-features = false, features = ["text"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.2"
thiserror = "2"
clap = { version = "4.6", features = ["derive"] }
base64 = "0.23"
sha2 = "0.11"
jsonschema = { version = "0.58", default-features = false }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
indexing_slicing = "deny"
unreachable = "deny"
```

Each member's `Cargo.toml` opts in to the workspace lints. `[workspace.lints]` applies only to members that do, so without this block the `unsafe_code` forbid and the clippy denies never take effect:

```toml
[lints]
workspace = true
```

cosmic-text and resvg are built without their default features. cosmic-text keeps `std` and `swash` and drops `fontconfig`. resvg keeps `text` and drops `system-fonts`, `memmap-fonts`, `raster-images` and `svgz`, so usvg's fontdb 0.24 has no file-system or system-font code at all and the icons, which are plain SVG in data URIs, still decode. cosmic-text's `std` feature still enables `fs` and `memmap` on its own fontdb 0.23, so on the cosmic-text side the no-system-font rule (section 3.2) rests on never calling `load_system_fonts`.

`jsonschema` is a dev-dependency of stencil-model only. Its default features pull in HTTP remote-reference resolution and a TLS stack, so they are off and the schema tests never reach the network. `sha2` 0.11 returns a hybrid-array digest, so hex encoding is written out explicitly: two lowercase hex digits per byte (`format!("{byte:02x}")` over the digest bytes), never a `{:x}` implementation on the digest type.

`clippy.toml` sets `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true` and `allow-indexing-slicing-in-tests = true`. Library crates contain no `unwrap`, `expect`, `panic!`, indexing that can go out of bounds, or `unreachable!`, and the clippy denies make that mechanical. These settings reach `#[test]` functions and `#[cfg(test)]` modules only, so integration-test files that index or call `unreachable!` in helpers allow the lints at the top of the file. The binary's `main` is the only place that calls `std::process::exit`.

resvg re-exports `usvg` and `tiny_skia`, and usvg re-exports `fontdb`. cosmic-text re-exports its own `fontdb`. The two crates can pin different fontdb versions, so no `fontdb::Database` value crosses between them. Each consumer loads the same font bytes into its own database from `stencil_text::BUNDLED_FONTS`.

Dependency graph: stencil-model has no workspace dependencies. stencil-text and stencil-layout depend on stencil-model. stencil-render depends on stencil-model, stencil-layout and stencil-text. stencil-cli depends on all four, and has `resvg` (the workspace version) as a dev-dependency so the golden test can decode and compose PNGs through `resvg::tiny_skia`.

No crate enables serde_json's `preserve_order` feature, directly or through a dependency's features. Cargo unifies features across the packages in one build, dev-dependencies included when tests are built, so one crate enabling it would change the key order, and so the bytes, of every measured JSON (section 5.4). None of the pinned dependencies enables it (jsonschema, referencing, jsonschema-value, schemars, taffy, cosmic-text, usvg). The `jsonschema` dev-dependency does enable serde_json's `float_roundtrip` through jsonschema and jsonschema-value whenever tests are built. That feature changes float parsing only, not serialization, and the measured JSON bytes are the same in test and release builds, so the difference is expected.

Every loop over document content is bounded by the vet limits in section 1, and every public entry point asserts its preconditions and returns an error when they fail.

### 4.2 stencil-model

Document order, as used by `validate_page` and `text_fields`, is independent of the input's key order. Nodes are visited in pre-order. Within a node, fields are visited in their section 1.2 struct declaration order, and a field holding nodes is walked completely before the next field. For a Page that is `title`, `kicker`, `lede`, `foot`, `width`, `canvas`, `body`, `legend`, so `/title` comes before `/kicker` even though a kicker is drawn above the title. A Pcard gives `fn`, `pn`, `fact`, `ask`, and a Tee gives `kind`, `hub`, then its arms. Layout, render and the measured JSON use a different order, the geometry order of section 4.4.

```rust
pub mod document;   // section 1.2 types
pub mod text;       // section 3 trait, value types, FixedMetricsMeasurer
pub mod pointer;
pub mod checks;

pub use document::*;

/// serde_json parse followed by validate_page.
pub fn parse_page(json_text: &str) -> Result<Page, ModelError>;

/// All vet violations, in document order. Empty means valid.
pub fn validate_page(page: &Page) -> Vec<Violation>;

pub fn page_schema() -> schemars::Schema;

/// Pre-order walk of body nodes and Tee arms, each Tee's arms directly after the Tee.
/// This is the body portion of the section 4.4 geometry order. Stops after
/// NODES_MAX + 1 entries (section 1.3).
pub fn body_nodes(page: &Page) -> Vec<NodeEntry<'_>>;

/// Every authored text value with its pointer, in document order.
pub fn text_fields(page: &Page) -> Vec<TextField<'_>>;

#[derive(Debug, Clone, PartialEq)]
pub struct NodeEntry<'a> {
    pub pointer: NodePointer,
    pub parent: Option<NodePointer>,
    /// Section 1.3 depth: body elements have depth 1, and the children of a Row, Col or
    /// Zone and the arms of a Tee have their parent's depth plus 1.
    pub depth: usize,
    pub node: NodeRef<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeRef<'a> {
    Node(&'a Node),
    TeeArm(&'a Pipe),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField<'a> {
    pub pointer: NodePointer,
    pub text: &'a str,
}

// pointer.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodePointer(String);
impl NodePointer {
    pub fn root() -> Self;
    pub fn child(&self, token: &str) -> Self;
    pub fn index(&self, index: usize) -> Self;
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Display for NodePointer { /* the raw pointer; the root prints as "" */ }

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("document is not valid stencil JSON at line {line}, column {column}: {message}")]
    Json { line: usize, column: usize, message: String },
    #[error("document violates {} vet rule(s)", .0.len())]
    Invalid(Vec<Violation>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub pointer: NodePointer,
    pub rule: VetRule,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VetRule {
    TextEmpty,
    TextTooLong,
    TextControlCharacter,
    TextUntrimmed,
    PageWidthOutOfRange,
    BodyEmpty,
    ChildrenEmpty,
    ChildrenTooMany,
    LegendTooLong,
    GapOutOfRange,
    GrowLengthMismatch,
    GrowOutOfRange,
    TeeArmNotHorizontal,
    DepthExceeded,
    NodesExceeded,
}

impl VetRule {
    /// Kebab-case name used on the CLI, as in section 1.3.
    pub fn as_str(self) -> &'static str;
}

// checks.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckName {
    ChildInsideContainer,
    SiblingsDoNotOverlap,
    TextFitsBox,
    RememberedConstants,
    LegendConsistency,
    LinksRouted,
    LinksAvoidBoxes,
    PipesLand,
}

impl CheckName {
    /// Kebab-case name used on the CLI.
    pub fn as_str(self) -> &'static str;
    /// Noun for `count` examined units, singular when count is 1 (section 6).
    pub fn unit(self, count: u64) -> &'static str;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Defect {
    pub pointer: NodePointer,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckReport {
    pub check: CheckName,
    pub examined: u64,
    pub defects: Vec<Defect>,
    /// Why the surface this check examines does not exist on the page, for example
    /// "page has no links". None for every report that looked at the page.
    pub not_applicable: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed,
    Failed,
    NotApplicable,
}

impl CheckReport {
    /// examined 0, no defects, `not_applicable: Some(reason)`.
    pub fn not_applicable(check: CheckName, reason: &'static str) -> Self;
    /// Passed: no reason, examined > 0, no defects. NotApplicable: a reason, examined 0,
    /// no defects. Every other combination is Failed.
    pub fn outcome(&self) -> CheckOutcome;
    /// outcome() == Passed.
    pub fn passed(&self) -> bool;
}

pub const REMEMBERED_CONSTANTS: [RememberedConstant; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RememberedConstant {
    pub literal: &'static str,
    pub reason: &'static str,
}

pub fn remembered_constants(page: &Page) -> CheckReport;
pub fn legend_consistency(page: &Page) -> CheckReport;
```

### 4.3 stencil-text

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontFile {
    pub file_name: &'static str,
    pub weight: FontWeight,
    pub bytes: &'static [u8],
    /// Lowercase hex SHA-256 of `bytes`, copied from assets/fonts/FONTS.md.
    pub sha256: &'static str,
}

/// Inter-Regular, Inter-SemiBold, Inter-Bold, Inter-ExtraBold, via include_bytes!.
pub const BUNDLED_FONTS: [FontFile; 4];

/// The public check over the four compiled-in faces; `verify_fonts` stays crate-private so
/// only the font-swap test can pass it other files.
pub fn verify_bundled_fonts() -> Result<(), FontError>;

/// Parses each file and asserts family "Inter" and the listed weight. Crate-private so the
/// font-swap test can pass a modified copy of BUNDLED_FONTS.
pub(crate) fn verify_fonts(files: &[FontFile]) -> Result<(), FontError>;

pub struct CosmicTextMeasurer { /* FontSystem, cache */ }

impl CosmicTextMeasurer {
    /// Calls verify_bundled_fonts, then builds a FontSystem over the four faces only.
    pub fn new() -> Result<Self, FontError>;
}

impl stencil_model::text::TextMeasurer for CosmicTextMeasurer { /* section 3.2 */ }

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("bundled font {file_name} does not parse")]
    Unparseable { file_name: &'static str },
    #[error("bundled font {file_name} has family {found:?}, expected \"Inter\"")]
    FamilyMismatch { file_name: &'static str, found: String },
    #[error("bundled font {file_name} has weight {found}, expected {expected}")]
    WeightMismatch { file_name: &'static str, expected: u16, found: u16 },
}
```

The SHA-256 of each font is checked in a test, not at runtime, because the bytes are compiled in. The font files and `FONTS.md` exist before this crate is started (asset pre-step, section 8.3).

### 4.4 stencil-layout

```rust
pub mod styles;   // section 2.9 colors by TextStyleName, canvas and pipe kind; the styles are stencil_model::text::TEXT_STYLES
pub mod checks;

pub use stencil_model::text::WRAP_EPSILON_PX;
pub const GEOMETRY_EPSILON_PX: f32 = 0.01;
/// Arrowhead of a link or pipe end (section 11.2): length from base to tip, width at the base.
pub const ARROWHEAD_LENGTH_PX: f32 = 10.0;
pub const ARROWHEAD_WIDTH_PX: f32 = 8.0;

/// Asserts validate_page(page) is empty, builds the taffy tree, computes layout,
/// re-measures text at final widths, routes the links, and returns absolute geometry in
/// canvas px.
pub fn layout_page(
    page: &Page,
    measurer: &mut dyn TextMeasurer,
) -> Result<PageGeometry, LayoutError>;

#[derive(Debug, Clone, PartialEq)]
pub struct PageGeometry {
    pub canvas: Size,
    /// In geometry order (below). nodes[0] is the page root (pointer "").
    pub nodes: Vec<NodeGeometry>,
    /// One route per `Page.links` entry, in link order (section 11.2).
    pub links: Vec<LinkRoute>,
}

impl PageGeometry {
    pub fn node(&self, pointer: &NodePointer) -> Option<&NodeGeometry>;
    pub fn children(&self, index: usize) -> Vec<usize>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeGeometry {
    pub pointer: NodePointer,
    pub tag: NodeTag,
    /// Serialized kind for Zone, Pipe (Tee arms included), Tee and LegendEntry,
    /// for example "region-a" or "blue"; None for every other tag.
    pub kind: Option<&'static str>,
    pub parent: Option<usize>,
    /// Border box.
    pub bounds: BoxRect,
    /// Region children must stay inside: border box minus border and padding,
    /// and for gcp zones the body part's content box.
    pub content: BoxRect,
    /// In the order of section 2.11.
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTag {
    Page, Kicker, Title, Lede, Body, Legend, LegendEntry, Foot,
    Row, Col, Zone, Pcard, Fact, Note, Pipe, Tee,
    Text, Callout, Frame,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxRect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size { pub width: f32, pub height: f32 }

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: PartName,
    pub bounds: BoxRect,
    pub text: Option<TextRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartName {
    Badge, BadgeText, Text,
    Label, Bar, Body,
    Icon, FunctionName, ProductName, FactBox, Fact, AskBox, Ask,
    DotStart, WireStart, Tag, TagLabel, TagSub, WireEnd, DotEnd,
    Spine, Hub, HubText,
    Swatch, LegendLabel, LegendText,
    Heading, Marker, BodyLine, Accent, LabelChip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// The string as drawn, after any uppercase transform and the "Ask: " prefix.
    pub text: String,
    pub style: TextStyle,
    pub color: &'static str,
    pub align: TextAlign,
    pub metrics: TextMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign { Start, Center }

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("page fails vet: {} violation(s)", .0.len())]
    Invalid(Vec<Violation>),
    #[error("text at {pointer} could not be measured: {source}")]
    Measure { pointer: NodePointer, #[source] source: MeasureError },
    #[error("taffy failed at {pointer}: {message}")]
    Taffy { pointer: NodePointer, message: String },
    #[error("layout produced a non-finite box at {pointer}")]
    NonFinite { pointer: NodePointer },
}

// checks.rs
pub fn child_inside_container(geometry: &PageGeometry) -> CheckReport;
pub fn siblings_do_not_overlap(geometry: &PageGeometry) -> CheckReport;
pub fn text_fits_box(geometry: &PageGeometry) -> CheckReport;
pub fn links_routed(geometry: &PageGeometry) -> CheckReport;
pub fn links_avoid_boxes(geometry: &PageGeometry) -> CheckReport;
/// Reads each pipe's `dir` from the page and every box from the geometry.
pub fn pipes_land(page: &Page, geometry: &PageGeometry) -> CheckReport;
```

`LayoutError::Taffy` and `LayoutError::NonFinite` carry the pointer of the node whose box failed when there is one: a taffy node not attached to the tree, or a non-finite border or content box, reports its record's pointer. A failure of `compute_layout_with_measure`, which computes the whole tree at once, a failure while walking the tree for absolute origins, and a non-finite canvas size are page-level and carry the root pointer `""`.

Geometry order. `PageGeometry.nodes` order is `""`, `/kicker`, `/title`, `/lede`, `/body`, then each body node in pre-order with a Tee's arms `/…/arms/0`, `/…/arms/1` directly after the Tee and before its next sibling, then `/legend`, `/legend/0` … `/legend/n`, then `/foot`. `/legend` and its entries are absent when `legend` is empty, and `/foot` is absent when `foot` is. This is the section 2.2 root-child order, not section 4.2 document order. `render_svg`, `measured_json` and the siblings-do-not-overlap defect rule use this order. Document order stays the order of `validate_page`, `text_fields` and the remembered-constants check.

### 4.5 stencil-render

```rust
pub mod palette;  // every color in sections 2.4 to 2.9 and 5.2 as &'static str constants

/// Walks page and geometry in the section 4.4 geometry order and asserts the pointers match.
pub fn render_svg(page: &Page, geometry: &PageGeometry) -> Result<SvgDocument, RenderError>;

#[derive(Debug, Clone, PartialEq)]
pub struct SvgDocument {
    pub svg: String,
    /// Number of <text> elements written: one per line of every TextRun.
    pub text_elements: usize,
}

/// Largest pixmap render_png allocates, in pixels (section 5.3, step 5).
pub const PNG_PIXELS_MAX: u64 = 1 << 27;

/// Calls verify_bundled_fonts, parses the SVG with a usvg fontdb that holds only
/// BUNDLED_FONTS, asserts the tree holds exactly `expected_text_elements` text nodes
/// and that every font lookup resolved (section 5.3), renders at `scale` within
/// PNG_PIXELS_MAX, and encodes PNG.
pub fn render_png(
    svg: &str,
    expected_text_elements: usize,
    scale: DeviceScale,
) -> Result<Vec<u8>, RenderError>;

/// Section 5.4 shape. `document` is the input parsed as serde_json::Value.
pub fn measured_json(document: &serde_json::Value, geometry: &PageGeometry) -> serde_json::Value;

pub fn icon_svg_bytes(icon: IconName) -> &'static [u8];
pub fn icon_data_uri(icon: IconName) -> String;

/// Section 5.1 rounding. The SVG writer and measured_json both call it.
/// Callers pass finite values only; layout returns NonFinite otherwise.
pub fn format_number(value: f32) -> NumberRepr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberRepr {
    /// The rounded value has no fractional part.
    Integer(i64),
    /// At most 2 decimals.
    Decimal(f64),
}

impl NumberRepr {
    /// Integer through serde_json::Number from i64, Decimal through
    /// serde_json::Value::from(f64), so no Option is unwrapped.
    pub fn to_json(self) -> serde_json::Value;
}

/// The SVG attribute form: an integer with no decimal point, or f64 Display.
impl std::fmt::Display for NumberRepr { /* section 5.1 */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceScale(u8);

impl DeviceScale {
    pub const DEFAULT: DeviceScale = DeviceScale(2);
    /// Accepts 1 to 4.
    pub fn new(value: u8) -> Result<Self, RenderError>;
    pub fn get(self) -> u8;
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("device scale {value} is outside 1 to 4")]
    ScaleOutOfRange { value: u8 },
    #[error("geometry node {found} does not match document node {expected}")]
    GeometryMismatch { expected: NodePointer, found: NodePointer },
    #[error("generated SVG does not parse: {message}")]
    Svg { message: String },
    #[error("{count} text element(s) rendered no glyphs")]
    TextNotRendered { count: usize },
    #[error("parsed SVG holds {found} text node(s), more than the {expected} expected")]
    TextCountExceeded { expected: usize, found: usize },
    #[error("{lookups} font or glyph lookup(s) found no bundled face")]
    FontNotResolved { lookups: usize },
    #[error("cannot allocate a {width}x{height} pixmap")]
    PixmapAllocation { width: u32, height: u32 },
    #[error("PNG encoding failed: {message}")]
    PngEncode { message: String },
    #[error(transparent)]
    Fonts(#[from] FontError),
}
```

### 4.6 stencil-cli

`src/lib.rs` exposes `pub fn run(arguments: Vec<std::ffi::OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> ExitCode`, and integration tests call it directly. `src/main.rs` collects `std::env::args_os()`, calls `run` and exits with the returned code. Argument parsing uses clap derive. Section 7 has the commands.

`src/lib.rs` also exposes `pub mod pipeline`, the steps `run` composes, so the section 9.4 golden test drives the same code path without the argument parser: `read_input` (at most `INPUT_BYTES_MAX` bytes), `load_document` (parse, vet, then the input as a `serde_json::Value`), `output_names`, `render_page` (layout with `CosmicTextMeasurer`, SVG, PNG and measured JSON in memory), `model_checks`, `all_checks` (the ten reports in `CheckName` order, section 12.9), `write_outputs`, and the `Failure` enum that section 7 maps to exit codes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Clean = 0,
    Defects = 1,
    CouldNotRun = 2,
}
```

## 5. Outputs

`stencil render` writes three files named after the input's file stem: `<stem>.svg`, `<stem>.png` and `<stem>.measured.json`.

### 5.1 Numbers

Rounding is off during layout, so coordinates can be fractional. Every number written to SVG or JSON goes through one function, `stencil_render::format_number` (section 4.5). It converts the value to f64 with `f64::from`, rounds it to 2 decimals with `(value * 100.0).round() / 100.0`, and maps `-0.0` to `0.0`. A result with no fractional part is written as an integer: in JSON as an integer `serde_json::Number` built from `i64`, and in SVG with no decimal point. Any other result is written as the shortest decimal that round-trips, through `serde_json::Number::from_f64` in JSON and Rust's f64 `Display` in SVG, so it has at most 2 decimals and no trailing zero or dot. Integral values never go through `from_f64`, because serde_json writes f64 `1320.0` as `1320.0` and `-0.0` as `-0.0`. Running the same input twice gives byte-identical SVG, PNG and JSON.

### 5.2 SVG

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="1320" height="652.4" viewBox="0 0 1320 652.4">
  <rect x="0" y="0" width="1320" height="652.4" fill="#FFFFFF"/>
  <g data-id="" data-tag="Page">
    <g data-id="/kicker" data-tag="Kicker">
      <rect x="20" y="20" width="74.6" height="16" rx="4" fill="#E8F0FE"/>
      <text x="27" y="32.6" xml:space="preserve" font-family="Inter" font-size="10" font-weight="800" letter-spacing="0.7" fill="#174EA6">CUSTOMER</text>
      <text x="100.6" y="33.1" xml:space="preserve" font-family="Inter" font-size="11" font-weight="700" letter-spacing="0.88" fill="#1A73E8">DEDICATED INTERCONNECT 99.99% · TWO METROS, TWO REGIONS</text>
    </g>
    <g data-id="/body" data-tag="Body">
      <g data-id="/body/0" data-tag="Row">
        <g data-id="/body/0/children/2" data-tag="Zone" data-kind="gcp"> … </g>
      </g>
    </g>
  </g>
</svg>
```

The coordinates in this example are illustrative.

Structure:

- One `<g>` per node, nested the same way as the node tree and in section 4.4 geometry order. Each carries `data-id` (the node pointer) and `data-tag` (the `NodeTag` name). Zones, Pipes (Tee arms included), Tees and legend entries also carry `data-kind`, taken from `NodeGeometry.kind`. A `<g>` has no transform. Every coordinate is absolute canvas px.
- Inside a node's group, shapes come first, then that node's text, then the child groups. Inside a Tee the order is spine, hub, then arm groups.
- Boxes are `<rect>` with `rx` for the radius and fill and stroke from the tables in section 2. A stroke of width `w` is drawn on a rect inset by `w / 2`, so it stays inside the border box as a CSS border does. The gcp frame uses a `<path>` for its 4 4 10 10 corners and draws the bar before the frame stroke.
- Dashed borders and dashed wires use `stroke-dasharray="6 5"`.
- Text is one `<text>` element per line, with `font-family="Inter"` exactly and no fallback list. `font-size`, `font-weight` and `fill` come from the style, and `letter-spacing` is written in px (em times size) and only when it is nonzero. `x` is the text box left edge, or for centered text `left + (box width - line width) / 2`. `text-anchor` is never used. `y` is the text box top plus the line's `baseline_px`. Every `<text>` carries `xml:space="preserve"`: vet allows interior runs of spaces (`a  b`), cosmic-text measures each space, and SVG's default whitespace handling would collapse them, drawing the line narrower than measured and shifting centered lines by half a space. Text content is XML-escaped.
- Icons are `<image x y width="28" height="28" href="data:image/svg+xml;base64,…"/>`. The payload is the base64 of the icon file's exact bytes, and the icon file is never parsed, rewritten or minified.
- Wires are `<line>` along the wire box's center line with `stroke-width="2"` and the kind color. Dots are `<circle r="4">` at the dot box center, filled with the kind color.

| PipeKind | Wire and dot color | Dashed |
|---|---|---|
| gray | #5F6368 | no |
| blue | #1A73E8 | no |
| pink | #C2185B | no |
| dash | #1A73E8 | yes |
| deny | #C5221F | yes |

The only `http` string in the SVG is the SVG namespace URI. There are no external references of any kind.

When the SVG is opened outside resvg, text renders correctly only if Inter is installed on that machine. The PNG is the deliverable, and the SVG is its exact source for resvg.

### 5.3 PNG

`render_png` works in this order:

1. Call `stencil_text::verify_bundled_fonts`. A failure returns `RenderError::Fonts`.
2. Build `usvg::Options` with `fontdb` set to an `Arc` of a database that contains only the four bundled faces (loaded with `load_font_data`), `font_family` set to `"Inter"`, no resources directory, an `image_href_resolver` that decodes `data:` URIs and resolves every other href to nothing, and a strict `font_resolver`:
   - `select_font` queries the database with the element's own families, weight, stretch and style and appends no generic family. usvg's default selector appends Serif to every query, which resolves a missing family as soon as a generic family points at a loaded face.
   - `select_fallback` returns `None`. usvg's default fallback substitutes any loaded face that covers a character, so with four Inter faces loaded it would draw a character from the wrong weight.
   - Both closures count every lookup that finds no face.

   No generic family is pointed at Inter on this database (`set_serif_family` and its siblings are never called). `docs/api-notes/resvg.md` records the resolver and the failure it closes.
3. Parse with `usvg::Tree::from_str`.
4. Count `usvg::Node::Text` nodes, descending recursively through `Node::Group` children only: not into a text node's flattened group and not into the tree of an `<image>` (icons hold no text). The walk visits at most 16 × NODES_MAX = 65,536 groups, well above the one `<g>` per geometry node a vetted page writes, and returns `Svg` when the tree holds more. When a `<text>` element's font does not resolve, usvg 0.48 drops the element and raises no error. `parser/text.rs` returns before it pushes the `Node::Text`, and text layout returns `None` when it placed no glyphs, so a dropped string leaves no node behind. The count is compared with `expected_text_elements`, which the caller takes from `SvgDocument::text_elements`. Fewer returns `TextNotRendered { count: expected - found }`. More returns `TextCountExceeded`, which means the caller passed the wrong number. Equal goes on to the lookup tally from step 2: a span whose family does not resolve inside a `<text>` that still places other glyphs, or a character that no longer falls back, leaves the node count intact, so any counted miss returns `FontNotResolved { lookups }`. The tally counts resolver calls, not `<text>` elements, so one span with two unresolvable characters counts 2.
5. Compute `ceil(width * scale)` and `ceil(height * scale)` in f64. `width` and `height` are the parsed tree's size, which is the SVG's `width` and `height` attributes, so they are the canvas size already rounded to 2 decimals by section 5.1. A layout height of 652.004 is written as 652 and renders 1304 px tall at scale 2, not 1305. When either extent is not finite or below 1, or their product is above `PNG_PIXELS_MAX`, return `PixmapAllocation` before anything is allocated. Otherwise allocate a `tiny_skia::Pixmap` of that size, fill it with white (`Color::WHITE`), and render with `resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut())`.
6. Encode with `pixmap.encode_png()`.

`stencil_render::PNG_PIXELS_MAX` is 2^27 = 134,217,728 pixels, a 512 MiB RGBA buffer. No vet limit bounds the canvas height: 4096 nodes stacked in Cols give a canvas close to 200,000 px tall. On a 64-bit target `tiny_skia::Pixmap::new` only checks its size arithmetic, so it never returns `None` for such a canvas, and the allocation either aborts the process or maps memory lazily and thrashes while the pixmap is filled. The budget is therefore the only allocation guard. At scale 2 it admits a 1320 px wide canvas up to 25,420 px tall and a 2600 px wide canvas up to 12,905 px tall. `stencil check` renders at the default scale, so a document over the budget at scale 2 fails `check` with exit 2 as it fails `render`.

The default scale is 2, so the default canvas renders 2640 px wide, the same size as the Chrome reference render.

### 5.4 Measured JSON

```json
{
  "canvas": { "height": 652.4, "width": 1320 },
  "document": { "body": [ … ], "kicker": "…", "legend": [ … ], "title": "…" },
  "nodes": [
    { "height": 652.4, "id": "", "parts": {}, "tag": "Page", "width": 1320, "x": 0, "y": 0 },
    { "height": 16, "id": "/kicker",
      "parts": {
        "badge": { "height": 16, "width": 74.6, "x": 20, "y": 20 },
        "badge_text": { "height": 12, "line_count": 1, "width": 60.6, "x": 27, "y": 22 },
        "text": { "height": 13.2, "line_count": 1, "width": 1199.4, "x": 100.6, "y": 21.4 }
      },
      "tag": "Kicker", "width": 1280, "x": 20, "y": 20 },
    { "height": 46.3, "id": "/body/0/children/1/children/0/children/0", "kind": "blue",
      "parts": { "dot_end": { … }, "dot_start": { … }, "tag": { … }, "tag_label": { … }, "tag_sub": { … }, "wire_end": { … }, "wire_start": { … } },
      "tag": "Pipe", "width": 138.4, "x": 228, "y": 131.6 }
  ]
}
```

- `document` is the input parsed into a `serde_json::Value`, with no defaults applied and no fields added. It is value-equal to the input.
- `nodes` lists every node in section 4.4 geometry order. `id` is the node pointer, and each id resolves in `document` with `Value::pointer`. `kind` appears on Zone, Pipe (Tee arms included), Tee and LegendEntry, taken from `NodeGeometry.kind`. Coordinates are the absolute border box.
- `parts` maps each snake_case `PartName` (section 2.11; `FunctionName` is `function_name`) to its box. Parts that carry a `TextRun` add `line_count`. A Text block repeats `body_line` and `marker` once per body line, so those two names are keyed `body_line/<i>` and `marker/<i>` with the zero-based line index; byte order puts `body_line/10` before `body_line/2`.
- `links` is present only when `Page.links` is not empty, so a page without links writes the same bytes as before section 11.2. Each entry has `id` (`/links/<i>`), `from` and `to` (the endpoint node pointers), `kind`, `points` (the routed polyline as `{x, y}` objects), `status` (`routed` or `fallback`), `tag` (the tag box, absent without a label) and `parts` (`tag`, `tag_label` and `tag_sub`, keyed and boxed as node parts).
- Every number follows section 5.1.
- Object keys are in ascending byte order at every level, `document` included, because serde_json's default `Map` is a `BTreeMap`. The output is deterministic but follows neither the input's key order nor the section 2.11 part order. This depends on `preserve_order` staying off (section 4.1).

## 6. Checks

Every check returns a `CheckReport` with the number of units it examined. A report passes only when `examined > 0` and there are no defects. A check that examined nothing is a failure, and the CLI prints it as one. The exceptions are checks whose surface does not exist on the page. The two link checks on a page without links return `CheckReport::not_applicable` with the reason `page has no links`. `pipes-land` returns it with `page has no pipes` on a page without a Pipe or Tee arm, and with `no pipe has a neighbor` when pipes exist but no pipe end faces a neighbor (the rule below). A not-applicable report neither passes nor fails. A page with links whose link checks examine nothing still fails, and so does a page whose pipes `pipes-land` could not look at: a pipe with no geometry node is examined and reported as a defect, never skipped. The legend check has no such exception (section 1.3).

| Check | Crate | Unit examined | Defect when | Epsilon |
|---|---|---|---|---|
| `child-inside-container` | layout | each (parent, child) node pair, root excluded as child | child border box extends outside the parent's `content` box | 0.01 px |
| `siblings-do-not-overlap` | layout | each unordered pair of nodes with the same parent | the two border boxes intersect with both overlap width and overlap height above epsilon (touching edges pass) | 0.01 px |
| `text-fits-box` | layout | each text part (every `TextRun`) | measured width exceeds the part's width, or measured height exceeds the part's height, or the part box extends outside its node's border box | 0.01 px |
| `remembered-constants` | model | each authored text field (`text_fields`) | the field contains a listed literal at a word boundary | |
| `legend-consistency` | model | each pipe-kind use (every Pipe, every Tee arm, every Tee spine) plus each of the first LEGEND_ENTRIES_MAX + 1 legend entries | a used kind has no legend entry, a legend entry's kind is never used, or a kind appears twice in the legend | |
| `links-routed` | layout | each link | the route has `status: Fallback` | |
| `links-avoid-boxes` | layout | each (segment, obstacle) pair of every link, with the link's section 11.2 obstacles, plus each (tag, node) pair for every geometry node that draws a box of its own (not a Row, Col, Lanes, Body, Legend or Page) and is not a strict ancestor of an endpoint | a segment enters an obstacle's interior by more than the epsilon, or the tag overlaps the node box with both overlap width and height above the epsilon | 0.01 px |
| `pipes-land` | layout | each pipe end that faces a neighbor: for every Pipe and Tee arm, one per side (left and right for h, above and below for v) that has a neighbor, as defined below; a side whose Pipe names a `from` or `to` target examines the target instead (section 13.8) | the pipe's center on the cross axis lies outside every box in the neighbor's subtree that is not a Row or Col | 0.01 px |
| `print-fit` | layout | each text run, as `text-fits-box` counts them, when `--print-width` is set (section 13.10) | the run prints below 8 pt at the print width | 0.01 pt |
| `icon-matches-product` | model | each Item whose kind has a products table and that has an `icon` or a `subtitle` (section 13.10) | the subtitle names a product whose own icon is another, or a product without one and the item carries a product icon | |

`CheckName::unit(count)` returns the noun printed after the examined count:

| Check | Count 1 | Any other count, 0 included |
|---|---|---|
| `child-inside-container` | relation | relations |
| `siblings-do-not-overlap` | pair | pairs |
| `text-fits-box` | text run | text runs |
| `remembered-constants` | text field | text fields |
| `legend-consistency` | relation | relations |
| `links-routed` | link | links |
| `links-avoid-boxes` | pair | pairs |
| `pipes-land` | pipe end | pipe ends |
| `iso-labels-clear` | pair | pairs |
| `iso-links-clear` | link leg | link legs |
| `print-fit` | text run | text runs |
| `icon-matches-product` | item | items |

`text-fits-box` also examines the TagLabel and TagSub runs of every link tag, with `/links/<i>` as the owner and the tag box as the box the run must stay inside.

`pipes-land` asks whether each pipe points at something on both sides of its gutter. A Pipe h, Tee arms included, is anchored at its nearest ancestor-or-self whose parent is a Row, and its neighbors are the anchor's siblings directly before it (left) and directly after it (right) in that Row. A Pipe v is anchored the same way in a Col, with neighbors above and below. Only a Row or Col parent anchors a pipe: a Pipe v whose column container is a Zone, like the g7 failover pipe, is not examined, and neither is a pipe with no Row (for h) or Col (for v) ancestor. Each side that has a neighbor is one examined pipe end, so a pipe between two neighbors counts 2 and a pipe with no neighbor on either side counts 0. The end lands when the pipe's center on the cross axis (y for h, x for v) lies within the extent on that axis, the epsilon included at both edges, of some node in the neighbor's subtree, the neighbor itself included, whose tag is not Row or Col: a Zone, Pcard, Fact, Note, Text, Callout, Frame, Pipe or Tee counts. A Row or Col only arranges its children, so a gutter beside a Col of zones at content height does not land on the Col's stretched box. The check reads each pipe's `dir` from the page and every box from the geometry.

Defect pointers:

- `child-inside-container`: the child.
- `siblings-do-not-overlap`: the later sibling of the pair in section 4.4 geometry order, so for `/kicker` and `/title` it is `/title`. The message names the other.
- `text-fits-box`: the node that owns the part. The message names the part.
- `remembered-constants`: the text field. A field yields one defect per listed literal it contains, so one literal occurring twice is one defect and two different literals are two.
- `links-routed` and `links-avoid-boxes`: the link, `/links/<i>`. The message names the endpoint or obstacle pointers.
- `pipes-land`: the pipe, a Pipe node or Tee arm. The message names the side and the neighbor's pointer, for example `defect pipes-land /body/0/children/1/children/1/children/0: left neighbor /body/0/children/0 has no box across the pipe's center y 418.35`.
- `legend-consistency`: for a used kind with no legend entry, one defect per use, at the pointer of every Pipe, Tee arm or Tee (for its spine) using that kind. For a legend entry whose kind is never used, `/legend/i`. For a kind listed twice, one defect at each later entry `/legend/j`.

Defect messages name the pointer and the numbers involved. A `text-fits-box` size defect is `<part> <text> measured <w>x<h> in box <w>x<h>`, where `<part>` is the snake_case part name, `<text>` is the run's string as Rust `{:?}` prints it (double-quoted, with `"` and `\` escaped), and each number has exactly 2 decimals (`{:.2}`), the precision of the 0.01 px epsilon, for example `text-fits-box /body/0/children/1/children/0/children/0: tag_label "VLAN 1" measured 41.30x15.60 in box 38.00x15.60`.

Remembered constants. The literals and reasons match `drafting-diagrams/scripts/constants_lint.py` and `cue/stencil.cue`:

| Literal | Reason |
|---|---|
| `64512` | doc example ASN, not a requirement. Dedicated Interconnect takes any private ASN (RFC 6996); Partner Interconnect is fixed at 16550. |
| `130.211.0.0/22` | GFE health-check probe range. Applies only to backend types the health-check doc names. Not serverless NEG or Cloud Run. |
| `35.191.0.0/16` | GFE health-check probe range. Applies only to backend types the health-check doc names. Not serverless NEG or Cloud Run. |
| `10.8.0.0/28` | Serverless VPC Access connector range. Direct VPC egress does not use a connector; label it Private Google Access on the subnet instead. |

Matching follows the `\b` boundaries used in `cue/stencil.cue`. An occurrence counts only when the character before it and the character after it are not ASCII alphanumeric or `_`. `AS64512` does not match, and neither does `10.8.0.0/280`, while `ASN 64512` does. The check scans the authored text before any uppercase transform and does not use a regex dependency.

print-fit is not applicable without `--print-width` (`no print width`), and icon-matches-product on a page whose items have no products table (`grammar has no icon table`).

Exit codes: `stencil vet` and `stencil check` exit 1 when any report fails, including a failure because nothing was examined. They exit 0 only when every report passes.

## 7. CLI

```
stencil vet <json>
stencil render <json> --out-dir <dir> [--scale <1-4>] [--print-width <inches>]
stencil check <json> [--print-width <inches>]
stencil schema
stencil prime [<topic>]
stencil gallery <out-dir> [--examples <dir>]
```

| Command | Does | Output on stdout |
|---|---|---|
| `vet` | parse, `validate_page`, then, only when there is no violation, `remembered-constants`, `legend-consistency` and `icon-matches-product` | one line per violation, or one line per check and one line per defect; one summary line |
| `render` | parse and `validate_page` only (violations stop the command with exit 1; the model checks do not run, so a legend inconsistency does not stop a render), layout with `CosmicTextMeasurer`, SVG, PNG at `--scale` (default 2), measured JSON; creates `--out-dir` if missing and overwrites existing outputs | the three written paths, absolute |
| `check` | everything `render` does, held in memory without writing files, then all fourteen checks | one line per check, one line per defect, one summary line; or an `error` line and the summary line when layout or render fails with exit 1 |
| `schema` | prints `page_schema()` as pretty JSON | the schema |
| `prime` | prints the authoring briefing for an agent: `crates/stencil-cli/prime/base.md` with the vocabulary table rendered from `page_schema()`, so tag names, field names, bounds and enum values come from the model; at most 6,000 bytes. With a topic (`themes`, `links`, `blocks`, `layout`, `checks`, `cue`, `example`), that topic's text instead, each at most 4,000 bytes except `example`, which is `examples/g7.json` verbatim. An unknown topic writes one line to stderr naming the topics and exits 2 | the briefing or the topic |
| `gallery` | for every regular `.json` file directly inside `--examples` (default `examples`), in file-name order and at most 256 of them: parse and `validate_page` once, then for every theme in `--theme` order (center, dusk, wire) layout, SVG, PNG at scale 2 and measured JSON written to `<out-dir>/<stem>/<theme>/` as `render` writes them, followed by all twelve checks, print-fit not applicable. Then writes `<out-dir>/index.html` and `<out-dir>/gallery.json` | per render its failing check and defect lines, then `gallery <stem> <theme>: <check counts>`; the two index paths, absolute; one summary line |

Line formats, stable for scripts:

```
check child-inside-container: examined 33 relations, 0 defects
check text-fits-box: examined 40 text runs, 1 defect
defect text-fits-box /body/0/children/2/children/0/children/0/children/1: fact "BGP peering · link-local /29" measured 571.20x16.20 in box 560.00x16.20
check legend-consistency: examined 0 relations, FAILED: nothing examined
check links-routed: examined 0 links, not applicable: page has no links
stencil check: 14 checks, 3 passed, 2 failed, 9 not applicable
violation gap-out-of-range /body/0/gap: gap 65 is above 64
violation text-untrimmed /title: text starts or ends with whitespace
stencil vet: 2 violations, checks not run
check remembered-constants: examined 36 text fields, 0 defects
check legend-consistency: examined 8 relations, 0 defects
check icon-matches-product: examined 6 items, 0 defects
stencil vet: 0 violations, 3 checks, 3 passed, 0 failed
error document is not valid stencil JSON at line 3, column 7: missing field `kind`
stencil vet: document does not parse, checks not run
```

- A check line is `check <name>: examined <n> <unit(n)>, <d> defect` when d is 1 and `defects` otherwise, 0 included. A check that examined nothing prints `FAILED: nothing examined` in place of the defect count, and a not-applicable report prints `not applicable: <reason>` there instead.
- A defect line is `defect <check> <pointer>: <message>`, and a violation line is `violation <rule> <pointer>: <message>`, with the rule in kebab-case as in section 1.3. An empty pointer prints as `""`.
- The `vet` summary always starts with the violation count, `1 violation` or `<n> violations`, 0 included. A `render` or `check` summary carries a violation count only when violations stopped the run; otherwise `check` prints `stencil check: <n> checks, <p> passed, <f> failed`, followed by `, <a> not applicable` when a report did not apply. A document with violations or a parse failure prints `checks not run` in place of the check counts.
- `render` and `check` print violations and parse errors in the same formats, followed by the summary line `stencil <command>: …`.
- `check` prints the fourteen check lines in `CheckName` declaration order (child-inside-container, siblings-do-not-overlap, text-fits-box, remembered-constants, legend-consistency, links-routed, links-avoid-boxes, pipes-land, iso-labels-clear, iso-links-clear, iso-link-ends, iso-links-apart, print-fit, icon-matches-product), and `vet` prints its three in the same relative order.
- With `--print-width`, `render` prints the print-fit check line and its defect lines after the three paths, once the files are written, and exits 1 when the report fails. Each check's defect lines follow its check line directly, in the order the `CheckReport` lists them.
- On success `render` prints the three absolute paths, one per line, in the order SVG, PNG, measured JSON, and nothing after them.
- `gallery` prints, for each render, the check and defect lines of the checks that failed (none when every check passed or did not apply) followed by `gallery <stem> <theme>: <n> checks, <p> passed, <f> failed[, <a> not applicable]`. A document that does not parse or has violations prints its `error` or `violation` lines and `gallery <stem>: not rendered`; a render that fails with exit 1 (`MissingGlyph`) prints its `error` line and `gallery <stem> <theme>: not rendered`. After the renders come the absolute paths of `index.html` and `gallery.json`, then `stencil gallery: <r> renders of <e> examples in <t> themes, <f> failed`, where r counts the renders whose three files were written and f counts the renders that failed a check or were not rendered.
- `index.html` is a static page with no script: one anchor per theme (`<a href="#center">`) leading to one `<section id="<theme>">` per theme, which lists every example with its kicker and title read from the document, the PNG as a thumbnail linking to the full PNG, links to the SVG and the measured JSON, and the check counts line. Every link is relative to the gallery directory, so the page works from a download, a zip or a static host. `gallery.json` holds the same data: `themes`, `examples` (each with `name`, `source`, `title`, `kicker` and one entry per theme holding `theme`, `passed`, `summary` and, when the render was written, `files` with the relative `png`, `svg` and `measured` paths), `renders` and `failed`. A document that does not load is listed under its file name with every render not rendered. Neither file carries a timestamp, so two runs over the same examples write the same bytes.
- A `MissingGlyph` layout failure (exit 1) prints `error <LayoutError display>` on stdout, followed by `stencil <command>: checks not run`. `render` writes no files in this case, because layout runs before any write.

Exit codes:

| Code | Meaning | Examples |
|---|---|---|
| 0 | clean | every check passed; render wrote its files; schema or prime printed; `--help` or `--version` printed; gallery wrote every render and every render passed |
| 1 | defects in the document | invalid JSON, a serde type error, a vet violation, a failing check, a zero-examined check, `MissingGlyph`, content that overflows; for gallery, any example with one of these, while the other renders and both index files are still written |
| 2 | could not run | bad arguments, unknown subcommand or unknown prime topic, unreadable input file, output directory not creatable or not writable, an examples directory that cannot be read, holds no `.json` document or holds more than 256 (a gallery with nothing to render is never a clean run), bundled font verification failure, resvg rejecting generated SVG, `TextNotRendered`, a canvas above the PNG pixel budget (section 5.3), any internal fault in the table below |

Every error variant maps to one code. The mapping is an exhaustive `match` with no wildcard arm, so a new variant does not compile until it has a code. The one exception is clap's `ErrorKind`, which is `#[non_exhaustive]`: its match names `DisplayHelp` and `DisplayVersion` and sends every other kind to 2 through a wildcard arm.

| Error | Code |
|---|---|
| clap error of kind `DisplayHelp` or `DisplayVersion` (returned by `try_parse` for `--help` and `--version`); the rendered text (`err.render()`) is written to the `stdout` argument of `run`, never through `err.print()` | 0 |
| clap error of any other kind, unreadable input file (an input over `INPUT_BYTES_MAX` included), `--out-dir` not creatable or not writable, any output write failure (a directory at an output name included) | 2 |
| gallery: an examples directory that cannot be read (`ReadExamples`), holds no `.json` file (`NoExamples`) or holds more than 256 (`ExamplesExceeded`) | 2 |
| CLI failures outside the document: an input path with no file stem (`InputStem`), an output path that is the input file (`OutputIsInput`), the second parse of a vetted input into `serde_json::Value` (`DocumentValue`), serializing the measured JSON or the schema (`Serialize`) | 2 |
| `ModelError::Json`, `ModelError::Invalid`, `LayoutError::Invalid` | 1 |
| `LayoutError::Measure` with source `MeasureError::MissingGlyph` | 1 |
| `LayoutError::Measure` with source `EmptyText`, `InvalidStyle`, `InvalidMaxWidth` or `Backend` | 2 |
| `LayoutError::Taffy`, `LayoutError::NonFinite` | 2 |
| `FontError`, any variant, and `RenderError::Fonts` | 2 |
| `RenderError::ScaleOutOfRange`, `GeometryMismatch`, `Svg`, `TextNotRendered`, `TextCountExceeded`, `FontNotResolved`, `PixmapAllocation`, `PngEncode` | 2 |
| `PrimeError`, any variant: the briefing is built from the binary's own schema and texts, so a failure is an internal fault | 2 |
| a failing `CheckReport`, zero examined included; a not-applicable report does not fail, and a run is clean only when at least one report passed | 1 |

An input file that reads but is not UTF-8 is a `ModelError::Json` (exit 1) at the line and column of the first invalid byte, with the message `input is not valid UTF-8`, because RFC 8259 requires JSON text to be UTF-8. "Unreadable" means an I/O failure only.

`render` refuses an input whose canonical path equals the canonical path of one of its three outputs (`stencil render d/x.svg --out-dir d`), so a render never overwrites its own input. Before anything is staged, `render` returns `WriteOutput` (exit 2) when one of the three final names is a directory (`fs::symlink_metadata`, so a symlink to a directory is replaced like any other symlink), because a rename onto a directory fails only after the earlier renames have replaced their outputs. The three outputs are then written to temporary files named `.<name>.<process id>.<sequence>.tmp` in the output directory, where the sequence is a process-wide counter, each created new, and renamed over the final names only after all three writes succeed. A failed write removes the temporary files this run created and leaves existing outputs untouched, and a symlink at an output name is replaced, not followed. The three renames are not one atomic step: a rename that still fails after the directory check (for example, because another process created a directory at an output name in between) leaves the outputs renamed before it replaced. A run killed while writing can leave its temporary files behind. They are never deleted by a later run; one whose process id and sequence give the same name fails that write with exit 2 instead of overwriting the file.

`read_input` reads at most `pipeline::INPUT_BYTES_MAX` = 64 MiB (67,108,864 bytes). A longer input, `/dev/zero` included, is an unreadable input file: `ReadInput` with an I/O error of kind `FileTooLarge`, exit 2. A vetted page is far smaller, 4096 nodes holding a few texts of at most 400 scalar values each.

The CLI decides a `LayoutError::Measure` by its `source`, never by the wrapper alone. `EmptyText` cannot occur after vet, which rejects empty text, and `InvalidMaxWidth` cannot occur for a negative or NaN width because the measure closure clamps every width at 0 before adding the epsilon (section 3). Reaching either is an internal fault, as are the other exit 2 variants in this table.

Errors for exit 2 go to stderr with the underlying error chain. The CLI never prints a pass for a check that did not run. If layout or render fails, `check` reports the failure and exits with the code the failure maps to. It does not report the geometry checks as skipped.

## 8. Fonts and icons

### 8.1 Fonts

Family: Inter, SIL Open Font License 1.1. Four static instances from the Inter 4.1 release archive (`extras/ttf/`), committed unmodified under `assets/fonts/` by the asset pre-step (section 8.3):

| File | Weight | Used by | SHA-256 |
|---|---|---|---|
| `Inter-Regular.ttf` | 400 | lede, card_product, note_legend, legend_text, foot, block_body | in `FONTS.md` |
| `Inter-SemiBold.ttf` | 600 | fact, ask, tag_sub | in `FONTS.md` |
| `Inter-Bold.ttf` | 700 | kicker, title, zone_label, gcp_bar, perimeter_label, card_function, tag_label, legend_label | in `FONTS.md` |
| `Inter-ExtraBold.ttf` | 800 | badge | in `FONTS.md` |

- `assets/fonts/OFL.txt` holds the license text from the same archive. `assets/fonts/FONTS.md` holds the table above with the SHA-256 column filled in and the release version and archive URL. `BUNDLED_FONTS` copies the hashes, and a test compares them with the compiled bytes.
- Static instances are used instead of the variable font, so each weight is a separate face with fixed metrics and cosmic-text and usvg select the same face. `docs/api-notes/cosmic-text.md` recommends bundling `InterVariable.ttf` alone; the spec does not follow it, for this reason.
- The files are not subset, renamed or otherwise modified, so the OFL's clauses on modified versions do not apply. The SVG names the family and does not embed the font. The license permits bundling the files with the software, and `OFL.txt` ships next to them.
- Helvetica Neue, the face the HTML stencil uses, is a system `.ttc` that may not be redistributed, so it is not an option. Google Sans stays excluded for the brand reason recorded in the stencil provenance.
- Only these four faces are ever loaded, into the cosmic-text database and into the usvg database alike. A character Inter lacks is a `MissingGlyph` defect, never a fallback.

### 8.2 Icons

The 23 icons are copied byte for byte from `drafting-diagrams/stencil/icons/` to `assets/icons/`, together with `PROVENANCE.md` from `drafting-diagrams/stencil/`. That file records their source (the two ZIP archives linked from cloud.google.com/icons), the stated terms, and the conclusion that the icons may be used unaltered on labeled product cards in architecture and technical figures. A marketing or co-branded surface needs a Partner Marketing Hub request first. Stencil never recolors, resizes by rewriting, crops, re-exports or minifies an icon. It places the original bytes in a data URI and scales them with the `<image>` element's width and height.

`stencil-render` embeds the files with `include_bytes!`. A test compares each embedded file with this table:

| Local file | Archive path | SHA-256 |
|---|---|---|
| `agents.svg` | `Category Icons/Agents/SVG/Agents-512-color.svg` | `02b04606fa62689b6add0235489e58c9dfb19b63ba54f6799e51547619e76b46` |
| `ai-ml.svg` | `Category Icons/AI _ Machine Learning/SVG/AIMachineLearning-512-color.svg` | `59b77ac8d8df60438120c83a65fcb68486e576186beba7f37dcb7bb484a0f2f0` |
| `bigquery.svg` | `Unique Icons/BigQuery/SVG/BigQuery-512-color.svg` | `6c088a2a7afbfcab01918c959aad90d76835093cfc2ec8314bf7ba25b85aaae8` |
| `cloud-run-flat.svg` | `Unique Icons/Cloud Run/SVG/CloudRun-512-color-rgb.svg` | `42a910e5d9f5a8685227f91623bf50c8c9259c556a41eb9a97157fd1e78b0cea` |
| `cloud-run.svg` | `Unique Icons/Cloud Run/SVG/CloudRun-512-color-rgb.svg` | `42a910e5d9f5a8685227f91623bf50c8c9259c556a41eb9a97157fd1e78b0cea` |
| `cloud-sql.svg` | `Unique Icons/Cloud SQL/SVG/CloudSQL-512-color.svg` | `4e0d3d2049a67f64e5a2c88f836bf2d1786f809a72d82279b18906ec9d6b35c9` |
| `cloud-storage.svg` | `Unique Icons/Cloud Storage/SVG/Cloud_Storage-512-color.svg` | `df42fc4b652cb133a4150bea7988aa965651d2c52fdfd141b6e8a359552eec8e` |
| `compute-engine.svg` | `Unique Icons/Compute Engine/SVG/ComputeEngine-512-color-rgb.svg` | `5cf9e57e4f99e125e45f9db269f3d5d1fa0dfd187aebfe00f3f97b9caf488af2` |
| `compute.svg` | `Category Icons/Compute/SVG/Compute-512-color.svg` | `6551c59efcd0582dd4b7286ab5c031567468ad0b70921cc02fa4601d4e4af28e` |
| `containers.svg` | `Category Icons/Containers/SVG/Containers-512-color.svg` | `809477486d9ed5bfbdecacd262c9082306186a90f89e620ef4ecfee300d02459` |
| `data-analytics.svg` | `Category Icons/Data Analytics/SVG/DataAnalytics-512-color.svg` | `b32c8b4702fb90e9669e841e0410687471b7ca97ad236527f6dad8e3d90c87d9` |
| `databases.svg` | `Category Icons/Databases/SVG/Databases-512-color.svg` | `f9ecdabe95c8c30cd6adabcb3e521f7fe462eacd63ce19021222a2a1b41a7266` |
| `devops.svg` | `Category Icons/DevOps/SVG/DevOps-512-color.svg` | `b08a5231acec916701acec3a2e0003cc3a4a29ddf3fc66f24f3c90e51d40587d` |
| `gke.svg` | `Unique Icons/GKE/SVG/GKE-512-color.svg` | `6a33654c499bc4c38012661875a1a8c1bf2cdf01be8fdcc90d1c6504657878ec` |
| `hybrid.svg` | `Category Icons/Hybrid & Multicloud/SVG/HybridMulticloud-512-color.svg` | `66389f10f6f3d8a4a05b05ce883543783ed4e407f4c696c5777bb09671a72a23` |
| `integration.svg` | `Category Icons/Integration Services/SVG/IntegrationServices-512-color.svg` | `f4a14bb1f46b6b8ec2dd8bd69d8e37e9fac8f161801b781cf6e1f41772fc2101` |
| `networking.svg` | `Category Icons/Networking/SVG/Networking-512-color-rgb.svg` | `c88d1b7d3bcba21ba984d92a7762e1f940b70d922f83449370870f76caaf5787` |
| `observability.svg` | `Category Icons/Observability/SVG/Observability-512-color.svg` | `f22fe17c2e91d65d3109e499bb3b5d8d60216f678bf9fa41c541de7d8254216a` |
| `scc.svg` | `Unique Icons/Security Command Center/SVG/SecurityCommandCenter-512-color.svg` | `d7cadf3c5bd7d4fea4bab88d72d3dff276f29728c86d1b8c5d99bede796ca542` |
| `security-identity.svg` | `Category Icons/Security Identity/SVG/SecurityIdentity-512-color.svg` | `4e233138e1170653522e4261779d51bb8f1383255845aba893bbb2381fa48e0e` |
| `serverless.svg` | `Category Icons/Serverless Computing/SVG/ServerlessComputing-512-color.svg` | `971fec343a13520b4f1edb49f34d33f5611ca427a9b09e36c4537b07e9d62721` |
| `storage.svg` | `Category Icons/Storage/SVG/Storage-512-color.svg` | `d99325d1f951ec5ddff47e1d787a90acd819449ee38c6ce20a981e803f5d7ed1` |
| `vertex-ai.svg` | `Unique Icons/Vertex AI/SVG/VertexAI-512-color.svg` | `17922247f3110026fd637c531d0604c67a00c9a58a6e152030f2242b9fd48a8e` |

`cloud-run.svg` and `cloud-run-flat.svg` are the same file under two names, which the upstream provenance also records.

### 8.3 Asset pre-step

stencil-text and stencil-render embed the assets with `include_bytes!`, so neither crate compiles until the files exist, and the tests of stencil-model, stencil-text, stencil-render and stencil-cli read the g7 fixtures. One person lands the assets and the g7 fixtures in a single commit on the default branch before crate work fans out. No crate implementer adds or changes files under `assets/`, `examples/g7.json` or `examples/golden/`.

1. Download `https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip`.
2. Copy `extras/ttf/Inter-Regular.ttf`, `Inter-SemiBold.ttf`, `Inter-Bold.ttf` and `Inter-ExtraBold.ttf` unmodified to `assets/fonts/`, and the archive's `LICENSE.txt` to `assets/fonts/OFL.txt`.
3. Write `assets/fonts/FONTS.md`: the section 8.1 table with each file's SHA-256 (`shasum -a 256`), the release version 4.1 and the archive URL.
4. Copy the 23 icons from `drafting-diagrams/stencil/icons/` to `assets/icons/` and `drafting-diagrams/stencil/PROVENANCE.md` to `assets/icons/PROVENANCE.md`. Confirm every icon's SHA-256 equals the section 8.2 table.
5. Confirm with a throwaway probe that both font databases read the four files as this spec expects. cosmic-text's re-exported `fontdb` and usvg's re-exported `fontdb` each load the files with `load_font_data` and must report family `Inter` for every face and weights 400, 600, 700 and 800. The cosmic-text scout checked only Regular, SemiBold and Bold, so ExtraBold at weight 800 is unconfirmed until this step. If any face reports another family (for example `Inter ExtraBold` from the legacy name table) or another weight, the pre-step stops and this spec is revised before fan-out.
6. Write `examples/g7.json` as the section 9.3 JSON block, byte for byte, with a trailing newline. stencil-model, stencil-text, stencil-render, stencil-cli and the golden test all read this one file, and a change to it is made in section 9.3 first.
7. Copy the existing Chrome headless render of the g7 gold to `examples/golden/g7-gold-chrome.png`, or regenerate it with the recipe in section 9.4. Confirm it is 2640 by 1800 px (`sips -g pixelWidth -g pixelHeight` or `file`).

The probe is not committed. `verify_bundled_fonts` and the stencil-text tests check the same facts once the crate exists.

Two more fixtures are created during crate work, each by one owner:

- `schema/stencil.schema.json`: the stencil-model implementer generates the first version from `page_schema()` (`serde_json::to_string_pretty` plus a trailing newline) and commits it with the stencil-model schema test. The stencil-cli `schema` test is written after that commit is on the default branch. It reads the committed file and never writes it. After the first version, the file changes only together with the section 1.2 types.
- `examples/stress-dense.json`: the stencil-layout implementer writes it exactly as listed under stencil-layout in section 10. No other crate reads it.

## 9. MVP scope and the g7 golden

### 9.1 In scope

The eight tags in section 1, layout by taffy, measurement by cosmic-text over bundled Inter, SVG, PNG and measured JSON output, the eight checks, and the five CLI commands.

### 9.2 Cut list

| Cut | Where it stands |
|---|---|
| Connection router, and `from`/`to` links between nodes | Superseded by section 11.2, which adds routed links with a `links-avoid-boxes` check. A pipe stays a laid-out element in a gutter. |
| CUE evaluation in Rust | `cue export` produces the JSON, and CUE-side rules (tint pairing, cards without pn, fact or ask) stay in `cue/stencil.cue`. |
| HTML output | The PNG is the deliverable. The SVG is its source. |
| Cross-row column alignment, spanning zone backgrounds, expanded cards with chips, step badges in tags | Section 2.10. |
| User pills, availability-zone badges, MIG boxes, `.opt` badges | Future tags. |
| Nesting-order check (gcp frame, then VPC, then region, then subnet) | Candidate check after MVP. |
| Fonts other than Inter, embedded fonts in SVG | Section 8.1. |
| Multiple pages per document | One Page per file. |
| CUE export of g7 compared with `examples/g7.json` | `examples/g7.json` is maintained by hand, and no Rust test runs `cue`. The comparison belongs to `cue/check.sh`, which fails when the `cue` binary is missing, and is added there together with the `cue/g7.cue` update in section 9.3. No file in the repository pins the `cue` version. |

### 9.3 examples/g7.json

The block below is the g7 document in the vocabulary of sections 1 to 12. `examples/g7.json` is written in the core vocabulary of section 13 (Box, Item, `line` and `tint`, `"grammar": "gcp"`), migrated field for field by section 13.13's table, and lays out to the same geometry.

This is the g7 customer canvas as data. `examples/g7.json` is maintained by hand. For `cue export ./cue -e customer --out json` to produce this document, `cue/g7.cue` and `cue/stencil.cue` need an update outside the MVP (section 9.2): `grow` and `justify` on `#Container`, no Col wrapper inside the VPC zone, and the VLAN column split into two halves as the gold does.

```json
{
  "title": "Four lines. Two metros. Two regions. Failover sits between the regions.",
  "kicker": "Dedicated Interconnect 99.99% · two metros, two regions",
  "lede": "VLAN IDs, EAD, and BGP sit on the hops. A reader following the lines sees four attachments, not one bundled cable.",
  "foot": "Illustrative topology · each metro has one VLAN in each EAD",
  "canvas": "customer",
  "body": [
    {
      "tag": "Row",
      "gap": 8,
      "grow": [0, 0, 1],
      "children": [
        {
          "tag": "Col",
          "grow": [1, 1],
          "children": [
            {
              "tag": "Zone",
              "kind": "onprem-a",
              "label": "On-prem · Metro 1",
              "children": [
                { "tag": "Pcard", "icon": "hybrid", "fn": "On-prem router 1", "pn": "port toward Google" },
                { "tag": "Pcard", "icon": "hybrid", "fn": "On-prem router 2", "pn": "port toward Google" }
              ]
            },
            {
              "tag": "Zone",
              "kind": "onprem-b",
              "label": "On-prem · Metro 2",
              "children": [
                { "tag": "Pcard", "icon": "hybrid", "fn": "On-prem router 3", "pn": "port toward Google" },
                { "tag": "Pcard", "icon": "hybrid", "fn": "On-prem router 4", "pn": "port toward Google" }
              ]
            }
          ]
        },
        {
          "tag": "Col",
          "grow": [1, 1],
          "children": [
            {
              "tag": "Col",
              "gap": 12,
              "justify": "center",
              "children": [
                { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "VLAN 1", "sub": "EAD 1 · BGP" },
                { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "VLAN 2", "sub": "EAD 2 · BGP" }
              ]
            },
            {
              "tag": "Col",
              "gap": 12,
              "justify": "center",
              "children": [
                { "tag": "Pipe", "dir": "h", "kind": "pink", "label": "VLAN 3", "sub": "EAD 1 · BGP" },
                { "tag": "Pipe", "dir": "h", "kind": "pink", "label": "VLAN 4", "sub": "EAD 2 · BGP" }
              ]
            }
          ]
        },
        {
          "tag": "Zone",
          "kind": "gcp",
          "label": "Google Cloud",
          "children": [
            {
              "tag": "Zone",
              "kind": "vpc",
              "label": "Transit VPC · one network, two regions",
              "children": [
                {
                  "tag": "Zone",
                  "kind": "region-a",
                  "label": "Region A",
                  "children": [
                    { "tag": "Pcard", "icon": "networking", "fn": "Cloud Router A", "pn": "private ASN · RFC 6996" },
                    { "tag": "Fact", "text": "BGP peering · link-local /29 · keepalive and hold from the Cloud Router BGP-timer doc" }
                  ]
                },
                { "tag": "Pipe", "dir": "v", "kind": "dash", "label": "failover · Region A ↔ Region B" },
                {
                  "tag": "Zone",
                  "kind": "region-b",
                  "label": "Region B",
                  "children": [
                    { "tag": "Pcard", "icon": "networking", "fn": "Cloud Router B", "pn": "same private ASN as Region A" },
                    { "tag": "Fact", "text": "Advertise the Google VIP ranges named on the Private Google Access doc" }
                  ]
                }
              ]
            }
          ]
        }
      ]
    }
  ],
  "legend": [
    { "kind": "blue", "text": "Metro 1 ↔ Region A" },
    { "kind": "pink", "text": "Metro 2 ↔ Region B" },
    { "kind": "dash", "text": "region failover, not a fifth line" }
  ]
}
```

### 9.4 Golden test plan

The reference is the Chrome headless render of `drafting-diagrams/stencil/golds/g7-customer.html` at device scale 2 and window width 1320. It is committed as `examples/golden/g7-gold-chrome.png`, 2640 by 1800 px with trailing white space included. The recipe to regenerate it is the drafting-diagrams Step 3 render command.

The test lives in `crates/stencil-cli/tests/golden_g7.rs` and runs the library pipeline on `examples/g7.json` with `CosmicTextMeasurer`.

1. Render. Assert that the SVG, PNG and measured JSON are produced. Assert that the PNG width is 2640, equal to the reference width, and that the PNG height is `ceil(h * 2)`, where `h` is the canvas height rounded by section 5.1, the value written as `canvas.height` in the measured JSON (section 5.3, step 5).
2. Checks with exact examined counts, derived by hand from the document above:

   | Check | Examined | Derivation |
   |---|---|---|
   | child-inside-container | 33 | 6 page-level nodes, 3 legend entries, 24 body nodes |
   | siblings-do-not-overlap | 32 | root 6 children (15 pairs), legend 3 (3), Row 3 (3), on-prem Col 2 (1), two on-prem zones (1 each), VLAN Col 2 (1), two VLAN halves (1 each), VPC zone 3 (3), two regions (1 each) |
   | text-fits-box | 40 | runs per section 2.11: kicker BadgeText and Text 2, title 1, lede 1, legend LegendLabel and LegendText 3 by 2, foot 1, zone Label 6 (gcp included), cards FunctionName and ProductName 6 by 2, Fact nodes 2, VLAN TagLabel and TagSub 4 by 2, failover TagLabel 1 |
   | remembered-constants | 36 | title, kicker, lede, foot, 3 legend texts, 6 zone labels, 6 fn, 6 pn, 2 facts, 5 pipe labels, 4 pipe subs |
   | legend-consistency | 8 | 5 pipes, 3 legend entries |
   | pipes-land | 8 | four VLAN pipes by two sides, the on-prem Col on the left and the gcp zone on the right; the failover pipe's column container is the VPC Zone, not a Col, so it is not examined |

   Every report passes, and the two link checks are not applicable. If one of these counts changes, the test fails, and the new count is updated in the test with its derivation.
3. Geometry assertions on the measured geometry. These encode the gold's structure, and each one holds in `g7-gold-chrome.png` except the metro zone heights, which the known differences below list:
   - The canvas width is 1320, and the canvas height is between 560 and 720.
   - The on-prem Col starts at x = 20 and is between 185 and 230 wide. The gold column is 200.
   - The VLAN Col starts 8 px right of the on-prem Col and is between 120 and 170 wide. The gold column is 150.
   - The gcp Zone starts 8 px right of the VLAN Col and ends at x = 1300.
   - The VLAN pipe centers increase strictly in y from VLAN 1 to VLAN 4. VLAN 1 and 2 lie inside the upper half Col, VLAN 3 and 4 inside the lower half Col, and the upper half ends at or above the start of the lower half.
   - Each VLAN pipe spans the full width of its half Col, and its two wires differ in length by at most 0.01 px.
   - The failover pipe's top is at or below Region A's bottom and its bottom is at or above Region B's top. Its center x is within 0.5 px of the VPC content box's center x.
   - Region A and Region B have the same x and width, equal to the VPC content box width.
   - The on-prem Col splits the Row height the way the VLAN Col does (`grow [1, 1]`, gap 8), so Metro 1 and the upper half share their top and height, and so do Metro 2 and the lower half, within 0.01 px. Each VLAN pipe's center y lies within the vertical extent of its metro zone: VLAN 1 and 2 in Metro 1, VLAN 3 and 4 in Metro 2. With no wraps the gcp zone is 6 (border) + 35.6 (bar) + 30 (body padding) + 388.2 (VPC) = 459.8 tall, and it is the Row's tallest child. The VPC is 4 + 20 + 14.4 + 8 + 127.6 (Region A) + 8 + 70.6 (failover pipe v: 8 + 12 + 30.6 + 12 + 8) + 8 + 127.6 (Region B). Region A is 3 + 24 + 22.4 (label band) + 46 (card) + 8 + 24.2 (one-line Fact). Each metro zone and each gutter half is therefore (459.8 - 8) / 2 = 225.9 tall, and the cards sit at the top of their zone. Without the on-prem weights each metro zone keeps its 149.4 px content height, Metro 2 ends above the lower half's pipes, and pipes-land reports VLAN 3 and VLAN 4.
4. Visual artifact. The test composes `target/golden/g7-side-by-side.png`, with the reference on the left and the stencil render on the right. It uses `resvg::tiny_skia::Pixmap::decode_png` and `draw_pixmap`, through stencil-cli's `resvg` dev-dependency (section 4.1). No pixel metric is asserted, because the known differences below would dominate any pixel or SSIM score instead of structure. A person reviews the side-by-side at 200 percent zoom whenever the golden test's geometry changes.

Known differences between the stencil render and the gold, which a reviewer of the side-by-side does not report:

- Font face: Inter against Helvetica Neue (section 8.1).
- Region fills: Region A blue and Region B pink against the gold's two neutral gray regions (section 2.4).
- On-prem fills: Metro 1 blue and Metro 2 pink against the gold's two neutral warm-gray zones (section 2.4).
- Legend: stencil draws a 26 by 2 swatch, the kind label and the entry text for each entry, with no separator between label and text (section 2.2). The gold writes each entry as one string with no swatch and an `=` separator, such as "Solid blue = Metro 1 ↔ Region A", and labels the `dash` kind "Dashed" where the section 2.2 table says "Dashed blue".
- Metro zone height: stencil gives each on-prem zone half the Row height so it sits level with its gutter half. The gold keeps both zones at content height, Metro 1 about 152 px tall, so its VLAN 3 and VLAN 4 point at the empty space below Metro 2.
- Foot: g7's `foot` is one string drawn as one left-aligned text leaf. The gold's foot has two segments, one aligned left and one aligned right against the Google Cloud frame's right edge. The Page model has a single `foot` text and no way to express the right-aligned segment; a split foot is a model change outside the MVP.
- Wording: the kicker, lede and foot of `examples/g7.json` differ from the gold's text. The gold is a layout reference, not a text reference.

## 10. Test plan

Each behavior change comes with a test that fails before the change. Each positive test that accepts at a boundary has a paired negative test that rejects just past it.

### stencil-model

- Parse `examples/g7.json`, serialize and parse again, and assert equality. Assert that `width` defaults to 1280 when absent.
- Negative parses, each returning `ModelError::Json`: an unknown field on Zone (`"colour"`), an unknown field on Pipe, an unknown field on Page, unknown tag `"Box"`, unknown icon `"bigtable"`, unknown zone kind, a Pipe without `kind`, and a Tee with three arms.
- A Tee arm written with `"tag": "Pipe"` parses. An arm with `"tag": "Zone"` is rejected.
- An optional field written as `null` (`"pn": null`, `"gap": null`) parses as absent and validates against the schema. `"width": null` is a `ModelError::Json`.
- For every `VetRule`, one document accepted at the limit and one rejected just past it: gap 64 against 65, width 640 against 639 and 2560 against 2561, 400 against 401 scalars, 256 against 257 children, 256 against 257 body nodes (`children-too-many` at `/body`), 16 against 17 legend entries, depth 24 against 25 (section 1.3 depth, body elements at depth 1), 4096 against 4097 nodes with Tee arms counted (a page far past the limit still gives `body_nodes` length 4097 and one `nodes-exceeded`), `grow` length equal against off by one, weight 100 against 101, text `"a"` against `" a"`, text with `\n`, and a Tee arm `v`. Each rejection asserts the violation's pointer and message against the section 1.3 table, and `VetRule::as_str` against its first column. `validate_page` returns all violations of a document with three distinct faults, in document order as section 4.2 defines it, using a document whose keys are written in an order different from the struct order (for example `kicker` before `title`, both untrimmed).
- Walk bounds (section 1.3): a legend of 20 untrimmed entries gives one `legend-too-long` and `text-untrimmed` for `/legend/0/text` to `/legend/16/text` only. A Row of one child with 10,000 `grow` weights of 101 gives one `grow-length-mismatch` and 257 `grow-out-of-range`, the last at `/…/grow/256`. A Row of 200,000 Pcards gives `body_nodes` length 4097 ending at `/body/0/children/4095`.
- Schema: `page_schema()` serialized equals the committed `schema/stencil.schema.json`, which makes drift a test failure. `examples/g7.json` validates against the schema with the `jsonschema` crate. A copy with an unknown Zone field fails schema validation, and so does a copy with a missing `tag`.
- `remembered-constants`: each of the four literals fires in each kind of text field (title, lede, foot, legend text, zone label, fn, pn, fact, ask, note, pipe label, sub, hub). `AS64512`, `164512`, `10.8.0.0/280` and `135.191.0.0/16` do not fire. The examined count equals `text_fields(page).len()`. A field holding two different literals yields two defects, and a field holding one literal twice yields one.
- `legend-consistency`: a used kind missing from the legend, a legend kind never used, a duplicate legend kind, a Tee spine kind counted, and Tee arm kinds counted. A kind missing from the legend and used by two Pipes yields two defects at the two Pipe pointers, and a duplicate legend kind yields one defect at the later entry. A page with no pipes and an empty legend reports examined 0, and `passed()` is false. A page with one Pipe and a 20-entry legend reports examined 18: one use and the first 17 entries.
- `TextStyleName`: for every variant, `TEXT_STYLES[v as usize].name == v`, `text_style()` returns that entry, and `as_str()` equals the section 2.9 name.
- `CheckReport::passed` is false for examined 0 with no defects, and for examined 1 with one defect. `CheckName::unit` returns the singular for 1 and the plural for 0 and 2, for every check.
- `FixedMetricsMeasurer`: the worked example (104.0 by 15.6), wrapping at a width that splits two words, a word longer than the max width (width exceeds max, one line), `Some(0.0)` giving the widest word, letter spacing adding `n * em * size`, `MissingGlyph` for a configured character, `EmptyText`, `InvalidStyle` for size 5 and 97 and for an infinite line height, and `InvalidMaxWidth` for -1 and NaN.

### stencil-text

- Each `BUNDLED_FONTS` entry's bytes hash to its `sha256`, and the hashes equal `assets/fonts/FONTS.md`.
- `verify_bundled_fonts` passes. Each face reports family `Inter` and weights 400, 600, 700 and 800. A unit test copies `BUNDLED_FONTS`, swaps the `bytes` of two entries of different weight, passes the copy to `verify_fonts` and gets `WeightMismatch`.
- Coverage: every character in every text field of `examples/g7.json`, uppercased where the style uppercases, has a glyph in the weight its style uses, `·` and `↔` included.
- `MissingGlyph` for U+4E00 and U+1F600.
- Idempotence: for each style in `stencil_model::text::TEXT_STYLES` and a set of strings, measuring again at `Some(width_px + WRAP_EPSILON_PX)` gives the same line count and width.
- For strings whose only break opportunities are U+0020 (no hyphen, slash or other UAX #14 break class), `Some(0.0)` equals the widest space-separated word measured with `None`. Separately, `"On-prem router 1"` in `card_function` at `Some(0.0)` gives four lines, `On-`, `prem`, `router` and `1`.
- Letter spacing: a 20-character string with no ligature or contextual-alternate candidates (`ABCDEFGHIJKLMNOPQRST`) at 0.08 em and 11 px is 17.6 px wider (within 0.5 px) than at 0.
- Determinism: two measurements of the same input are bit-identical.

### stencil-layout

All tests except the wrap-epsilon regression use `FixedMetricsMeasurer::default()`. Its 0.5 em advances are exact in f32, so the expected numbers are exact.

- Page: the root is 1320 wide. `/title` starts at x = 20. `/kicker` is 6 px above `/title`. The canvas height equals the last root child's bottom plus 20. A kicker longer than the page width wraps and its text part stays inside `/kicker`.
- Pcard: the icon is 28 by 28. The card is 46 tall with icon and one-line fn and pn, and 44 tall with icon and fn only. It grows by the fact box height plus 4 when `fact` is set. Without an icon the text starts at the left padding. In a card 300 wide with an icon and a fact whose max-content width is 500, the text column is 300 - 3 - 20 - 28 - 10 = 239 wide and the fact wraps to more than one line.
- Fact: a fact whose max-content width exceeds its parent's content width wraps, and the Fact node stays inside its parent.
- Geometry order: for g7 the pointers of `PageGeometry.nodes` begin `""`, `/kicker`, `/title`, `/lede`, `/body`, `/body/0` and end `/legend`, `/legend/0`, `/legend/1`, `/legend/2`, `/foot`. In a document with a Tee followed by a sibling, the Tee's two arms come directly after the Tee and before the sibling. The body portion equals `body_nodes(page)` pointer for pointer.
- Parts: for every NodeTag, the part names and their order match section 2.11, and exactly the listed parts carry a `TextRun`. Title, Lede, Foot and Note carry one `Text` part equal to their border box.
- Zone: the first child's y is zone y + border + padding + label line height + 8, checked for region-a (1.5, 12, 14.4) and perimeter (2.5, 12, 15.6). The gcp bar is 35.6 tall, and children start inside the body padding.
- Row and Col: an absent `grow` on a Row gives equal widths to two Pcards. An absent `grow` on a Col inside a taller Row (a Row whose other child is a taller Pcard stack) leaves each of its two Zone children at its content height, and a Col with `grow [1, 1]` in the same Row splits the height equally. A Pipe h in a Row keeps its max-content width. `grow [0, 0, 1]` gives all free width to the third child. `justify: center` centers two pipes in a taller Col.
- Pipe: h in a Col spans the Col width. Its two wires are equal within 0.01 px and each is at least 14. v in a Col is centered and at least 36 tall. A `deny` v pipe's wires are each at least 16 tall, and every other kind's at least 12. v in a Row stretches to the Row height. A tag in a gutter narrower than its max-content wraps to two lines and the wires stay at 14.
- Tee: the spine spans the grid height minus 36. The hub is centered horizontally and sits in the middle row. The arms occupy rows 1 and 3 in column 2. The Tee is at least 118 wide.
- Text runs: every text leaf has a `TextRun` measured at final width. TagLabel, TagSub and HubText runs report `TextAlign::Center`, and every other run `TextAlign::Start`.
- Wrap-epsilon regression. The measurer is `FixedMetricsMeasurer { advance_em, missing_glyphs: Vec::new() }` with `advance_em = 0.501 + k as f32 * 0.002` for k from 0 to 99. These advances are not exact in f32, unlike the default 0.5, which can never show the bug. For each advance and each n from 1 to 40, the page body is one Row with `grow: [0]` holding one Col holding one Pipe h (kind `blue`, no `sub`) whose label is `VLAN ` followed by n `x` characters. The Row weight 0 makes the Col its max-content width, and the pipe stretches to it, which is the section 2.7 tag round trip. Every one of the 4000 layouts must give a `TagLabel` run with `line_count` 1, a `TagLabel` part 15.6 tall and a `Tag` part 30.6 tall (15.6 + 12 padding + 3 border), within 0.01 px. A taffy 0.14.0 probe of this structure with the section 3.1 arithmetic wrapped 163 of the 4000 without the epsilon in the closure and 0 with it.
- pipes-land: g7 without the on-prem Col's `grow` fails with two defects, at VLAN 3 and VLAN 4, each naming the left side and the on-prem Col. g7 as written passes with 8 pipe ends examined. A Row holding only a Col of pipes is not applicable with `no pipe has a neighbor`, and a page with no pipe is not applicable with `page has no pipes`. A page whose pipe has no geometry node fails. A Tee between two cards in a Row examines four arm ends and passes. A Pipe v in a Col below a Row of two cards and above one card examines two ends and misses above, because its center x falls in the gap between the cards and the Row's own box does not count.
- Errors: a measurer returning `MissingGlyph` gives `LayoutError::Measure` with the text's pointer. An invalid page gives `LayoutError::Invalid`.
- Checks on hand-built `PageGeometry`: overlap by 0.02 px is a defect and touching edges are not; a child 0.02 px outside its parent is a defect; a text run 0.02 px wider than its part is a defect; geometry with only the root gives examined 0 and fails for each geometry check.
- Checks on laid-out documents: a Pcard with a single 120-character word in a Row of three at width 640 produces `child-inside-container` defects for the three cards and no `text-fits-box` defect, because the min-content floor of section 2.5 widens the card to the word. A `/title` of one 200-character word at width 640 produces one `text-fits-box` defect at `/title` and no `child-inside-container` defect, because the root stretches the title leaf to the content width with no min-content floor. A small document of five nodes has hand-derived examined counts.
- Dense stress (`examples/stress-dense.json`, written by the stencil-layout implementer). Layout completes, all five checks and `pipes-land` pass, `pipes-land` examining 22 pipe ends (two Pipe h per Row with a card on each side, and the Pipe v between Row 1 and Row 2; the Tee arms have no Row ancestor), and `body_nodes(page).len()` is 37. A test compares the file, parsed as a `serde_json::Value`, with the document below built field for field, so any drift fails. The document is exactly:
  - Page: title `Fifteen Cloud Run services in five rows behind one service perimeter`, kicker `Service perimeter · Cloud Run`, lede `Five rows of cards and pipes inside a service perimeter.`, canvas `internal`, no `foot`, no `width`, and legend entries `gray` `internal call`, `blue` `request path`, `pink` `reply path`, `dash` `failover` and `deny` `blocked egress`, in that order.
  - `body` holds one Zone, kind `gcp`, label `Google Cloud`. Its one child is a Zone of kind `perimeter`, label `Service perimeter`. Its one child is a Col with no `gap`, `grow` or `justify` and seven children in this order: Row 1, the Pipe v, Row 2, Row 3, the Tee, Row 4, Row 5.
  - Row r, for r from 1 to 5, has no `gap`, `grow` or `justify` and five children: Pcard, Pipe h, Pcard, Pipe h, Pcard. Card c, for c from 1 to 3, has icon `cloud-run`, fn `Service r.c` with r and c as digits (`Service 2.3`), pn `Cloud Run` and fact `Reads its config from a bucket in the same project`. The first Pipe h has kind `blue` and label `step r.1`, the second kind `gray` and label `step r.2`. Neither has a `sub`.
  - The Pipe v has kind `dash` and label `failover`.
  - The Tee has kind `deny`, hub `egress`, and arms Pipe h `blue` with label `allowed` and Pipe h `pink` with label `reply`.
  - Node count: gcp zone 1, perimeter zone 1, Col 1, Rows 5, Row children 25, Pipe v 1, Tee 1, Tee arms 2, which is 37. Every PipeKind is used and listed once, so legend-consistency passes, and no text holds a remembered constant. The facts wrap to two lines in the default measurer, so wrapping is exercised in every card.

### stencil-render

- SVG parses with usvg. There is one `<g data-id>` per geometry node, in the same order with the same pointers. The only `http` substring is the namespace URI.
- Every `<text>` has `xml:space="preserve"` and `font-family="Inter"` with no fallback list. `letter-spacing` is present exactly for the styles with nonzero spacing.
- Icons: each data URI decodes to bytes whose SHA-256 equals the section 8.2 table. A render of a Pcard for each of the 23 icons parses.
- Colors: for each ZoneKind the rect fill, stroke, stroke width and dasharray match section 2.4. For each PipeKind the wire color and dasharray match section 5.2, and a Tee of that kind draws its spine, and a legend entry of that kind its swatch `<line>`, in the same color and dasharray. A `deny` Tee's hub has stroke #F4C7C3 and text fill #C5221F.
- Numbers: no attribute value has more than 2 decimals, and `-0` never appears. `format_number` gives `Integer(0)` for -0.0 and -0.001, `Integer(1320)` for 1320.0, `Integer(652)` for 652.004 and `Decimal(652.4)` for 652.4.
- PNG: dimensions are `ceil(c * scale)` for scales 1 and 4, where `c` is the canvas size rounded by section 5.1. A geometry whose height rounds down (for example 652.004) gives the PNG height from the rounded value. `DeviceScale::new(0)` and `new(5)` are rejected. Two renders are byte-identical. The pixel budget accepts 8192 by 16384 at scale 1 and rejects 8192 by 16385, and rejects zero, negative, NaN and infinite extents. A 1320 by 60000 SVG at scale 2 returns `PixmapAllocation { width: 2640, height: 120000 }` without allocating. An `<image>` whose href is the path of a red SVG file renders white, because only `data:` hrefs resolve. The file is SVG, not PNG, because resvg is built without `raster-images`: a PNG href never decodes whichever resolver runs, while usvg decodes an SVG sub-image itself, so only an SVG file makes the test fail when a file-reading resolver is installed.
- `TextNotRendered`: `render_png` on a one-line SVG whose `<text>` names family `Helvetica Neue`, with `expected_text_elements` 1, fails with count 1. The same SVG naming `Inter` passes with 1, returns `TextNotRendered` with count 1 when given 2, and returns `TextCountExceeded` when given 0. The `Helvetica Neue` SVG given 0 returns `FontNotResolved` with 1 lookup, and so does an `Inter` line holding U+4E00 given 1, because the fallback selector substitutes no face. For g7, `SvgDocument::text_elements` equals the number of `<text>` elements in the SVG string and the sum of `line_count` over all runs.
- Legend bound: a 16-entry page's geometry extended with nodes `/legend/16` to `/legend/19`, rendered against the page with 20 entries, returns `GeometryMismatch` with expected `/<absent>` and found `/legend/17`, because the document side stops after `/legend/16`.
- Measured-vs-rendered parity, one case per style in `TEXT_STYLES`. The measurement contract holds only in one direction, and this test checks that direction. A string containing `·` and `↔` (uppercased where `NamedTextStyle::uppercase` is true) is measured with `CosmicTextMeasurer`, written as a one-line SVG at x = 0 with `y = baseline_px` using the section 5.2 text attributes, and rendered at scale 4 on white with `expected_text_elements` 1. The test asserts:
  - no non-white pixel has x greater than `(width_px + 1) * 4`;
  - no non-white pixel has y greater than `(height_px + 1) * 4`;
  - at least one non-white pixel falls inside the measured box.

  resvg reports ink bounds and cosmic-text reports advance widths, so the two are never compared for equality.
- Measured JSON: `document` is value-equal to the input. The length of `nodes` equals the geometry node count. Every `id` resolves with `Value::pointer`. Every box number has at most 2 decimals. A walk over every number in the output `Value` (not a search of the serialized string, where `1320.05` contains `.0`) finds that each number with no fractional part is an integer `Number` (`is_i64` or `is_u64`) and none is negative zero. `kind` is present on exactly the Zone, Pipe, Tee and LegendEntry nodes. The serialized g7 output has the keys of every object in ascending byte order (`canvas`, `document`, `nodes` at the top, and `kicker` before `title` inside `document`), which fails if any crate in the test build turns on serde_json's `preserve_order`.

### stencil-cli

- `vet examples/g7.json` exits 0 and prints the three model checks with examined 36, 8 and 6.
- `vet` on a copy containing `64512` in a pipe sub exits 1 and prints the defect with its pointer.
- `vet` on a document with no pipes and an empty legend exits 1 and prints `FAILED: nothing examined`.
- `vet` on a document with one vet violation prints its `violation` line and `stencil vet: 1 violation, checks not run`, and exits 1.
- `vet` on malformed JSON exits 1. `vet`, `check` and `render` on a missing file exit 2 with empty stdout, and `render` creates no out-dir. `vet /dev/zero` exits 2 naming the 67,108,864-byte limit, and an input of exactly that many bytes (g7 padded with spaces) is read and vets clean. `vet` and `check` on a 4097-node document exit 1 with `violation nodes-exceeded /body: more than 4096 nodes`. An unknown subcommand or flag exits 2. `--help` and `--version` exit 0, write the text to stdout and write nothing to stderr.
- `render` writes the three files to a temporary directory and prints exactly their three absolute paths in the order SVG, PNG, measured JSON. `--scale 5` exits 2. An `--out-dir` under a read-only directory exits 2.
- `check examples/g7.json` exits 0 and prints the fourteen lines in `CheckName` order: the five with the counts from section 9.4, both link checks as not applicable, pipes-land with 8 pipe ends and no defect, the four iso checks and print-fit as not applicable, icon-matches-product with 6 items, and the summary `14 checks, 7 passed, 0 failed, 7 not applicable`.
- `check` on a document whose text overflows exits 1 and names the pointer. The document has one Pipe and a matching legend entry, so child-inside-container is the only failing check. The Pipe sits in the body with no Row or Col sibling, so pipes-land is not applicable, and the summary is `12 checks, 5 passed, 1 failed, 6 not applicable`.
- `vet` and `check` on a file whose bytes are not UTF-8 exit 1 and print the `error` line with the line and column of the first invalid byte and `document does not parse, checks not run`.
- `render d/figure.svg --out-dir d` exits 2 and leaves the input unchanged. A symlink planted at `<out-dir>/g7.svg` is replaced by the rendered file, its target keeps its bytes, and no temporary file is left behind. With an old `g7.svg` and a directory at `<out-dir>/g7.png`, `render` exits 2, `g7.svg` keeps its old bytes, no `g7.measured.json` appears and no temporary file is left behind. Two temporary names from one process differ, and a file already at a temporary name fails the write and keeps its bytes.
- A vetted page of 160 stacked Pcards exits 2 from `render --scale 4` with a `PixmapAllocation` message and writes nothing, and renders at `--scale 1`.
- Two separate `stencil render` processes on g7 at the default scale write byte-identical SVG, PNG and measured JSON. `check` on a document with U+4E00 in a Pcard `fn` exits 1 (`MissingGlyph` through `LayoutError::Measure`) and prints an `error` line followed by `stencil check: checks not run`.
- `render` on a document whose legend omits a used kind writes its three files and exits 0, because render runs `validate_page` only.
- `schema` prints JSON equal to `schema/stencil.schema.json`.
- `gallery` over `examples/` into a temporary directory exits 0; `index.html` has a tab and a section for every theme and links the PNG, SVG and measured JSON of every example in every theme; `gallery.json` parses, lists every example with its title and kicker, and its render count, like the summary line, equals the number of examples times the number of themes.
- `gallery` over a directory holding one document whose text overflows and one clean document exits 1, prints the failing check with its defect, writes all six renders and both index files, and marks the three overflowing renders failed. A document that does not parse is listed as not rendered in every theme and exits 1. A directory with no `.json` file, or no directory at all, exits 2 with empty stdout and creates no output directory.
- The golden g7 test in section 9.4.

## 11. Themes, links and document blocks

This section extends the MVP with three features. Every rule in sections 1 to 10 still holds unless this section names the change. The router entry in the section 9.2 cut list is superseded by section 11.2.

### 11.1 Themes

`Page` gains an optional `theme` field. The CLI `--theme` flag overrides it. The theme decides colors only: layout, type sizes, icons and geometry are identical across themes, so `measured.json` is byte-identical for the same document under every theme, and a test asserts it.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Center,
    Dusk,
    Wire,
}
```

`center` is the Architecture Center stencil as rendered today, and its tables in `stencil-render/src/palette.rs` do not change. `dusk` is the dark theme. `wire` is a monochrome wireframe for design documents. `stencil-render` keys every color lookup by theme through one `Palette` value built from the theme, so no drawing code names a color literal.

Dusk palette:

| Role | Value |
|---|---|
| page background | `#0B1220` |
| text primary, text secondary | `#E6EDF7`, `#9AA7BD` |
| kicker, badge customer | `#5B9CFF`; badge fill `#16305C`, badge ink `#9CC3FF` |
| badge internal | fill `#2E1A4A`, ink `#D6B4FF` |
| card fill, card border | `#111A2E`, `#2A3550` |
| fact and ask box fill, ink | `#182238`, `#B7C2D6` |
| gcp frame border and bar | `#1A73E8`; bar ink `#FFFFFF`; gcp body fill `#0F172A` |
| vpc border | dashed `#6B7A99`, no fill |
| region-a fill and border | `#14213A`, `#2F4A7A` |
| region-b fill and border | `#2A1626`, `#6A2A47` |
| subnet fill and border | `#1D1836`, dashed `#4A3F7A` |
| onprem-a, onprem-b fill and border | `#14213A` / `#2A1626`, `#3A3532` |
| project fill and border | `#1F1B10`, `#5A4A1A` |
| optional fill and border | `#10203A`, dashed `#4284F3` |
| k8s fill | `#2A1626`, no border |
| perimeter fill and border | `#17130B`, dashed `#E37400`; label ink `#F2A44B` |
| zone label ink | `#B7C2D6` |
| wire gray, blue, pink | `#9AA7BD`, `#5B9CFF`, `#FF5C8A` |
| wire dash | dashed `#5B9CFF` |
| wire deny | dashed `#FF6B6B`; deny tag ink `#FF8A8A`, deny tag border `#5A2A2A` |
| tag fill, border, ink, sub ink | `#111A2E`, `#2A3550`, `#E6EDF7`, `#9AA7BD` |
| legend ink, foot ink | `#9AA7BD` |
| icon chip | `#FFFFFF`, radius 6, 36 by 36 behind every 28 by 28 icon |

The icon chip exists because the Google icons are drawn unaltered (section 8.2) and several of them are dark gray on transparent; on a dark card they would vanish. The chip is a rounded white square drawn under the icon, inside the icon part's box, which is 36 by 36 in every theme so geometry stays theme-independent. In `center` and `wire` the chip is drawn in the card fill color and is invisible.

Wire palette: page background `#FFFFFF`, every ink `#222222`, secondary ink `#555555`, every border `#222222` at 1.25 px, no fills anywhere except `#FFFFFF`. Zone borders: gcp 2 px solid, vpc, optional and perimeter dashed, subnet dotted, every other kind 1.25 px solid; the gcp bar is white with a 2 px bottom border and `#222222` ink. Wire kinds are told apart by line style alone: gray thin solid 1.25 px, blue solid 2 px, pink solid 2 px with round end dots drawn hollow, dash dashed, deny dotted. Legend swatches use the same styles, pink with its two hollow dots, so the swatch carries the meaning. Tags are white with a `#222222` border. Badges are white with a border. Icons keep their bytes and colors.

Open items for `wire`:

- Legend text is authored, so a wire page still reads "Solid blue" or "Dashed red" next to a black swatch. Layout text is theme-independent by design; a fix belongs in the document (neutral wording such as "request path") or in a future theme-aware legend label, not in the renderer.
- A link has no end dots, so a blue and a pink link draw identically in `wire`. Pink links need a second cue, for example hollow dots at the non-arrow ends.

### 11.2 Links

Any node may carry an `id`. `Page` gains `links`. A link is a routed orthogonal line between two nodes, drawn after layout. It is not a node, takes no layout space, and never moves anything.

```rust
pub const LINKS_MAX: usize = 256;
pub const LINK_VIA_MAX: usize = 8;
pub const LINK_SEGMENTS_MAX: usize = 12;
pub const ROUTER_GRID_LINES_MAX: usize = 512;

// On Row, Col, Zone, Pcard, Fact, Note, Pipe, Tee, Text, Callout, Frame:
#[serde(default, skip_serializing_if = "Option::is_none")]
#[schemars(regex(pattern = r"^[a-z0-9][a-z0-9-]{0,63}$"))]
pub id: Option<String>,

// On Page:
#[serde(default, skip_serializing_if = "Vec::is_empty")]
#[schemars(length(max = 256))]
pub links: Vec<Link>,

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub kind: PipeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub sub: Option<String>,
    #[serde(default)]
    pub arrow: Arrow,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_side: Option<Side>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_side: Option<Side>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 8))]
    pub via: Vec<PagePoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Arrow { None, #[default] End, Start, Both }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Side { Top, Right, Bottom, Left }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PagePoint { pub x: f32, pub y: f32 }
```

`Pipe` gains `arrow: Arrow` with default `None`. With an arrow at an end, the arrowhead replaces that end's dot; the wire ends at the arrowhead's base.

Vet rules, in `stencil-model`: `id` values are unique across the document (`id-duplicate`), every `from` and `to` resolves to a node id (`link-unknown-id`), `from` and `to` differ (`link-self`), a link's `via` points lie inside the page (`link-via-outside`), and `links` and `via` respect their limits. The legend rule of section 6 counts link kinds together with pipe kinds.

Routing, in `stencil-layout`, after `layout_page` and before the checks, producing `PageGeometry.links: Vec<LinkRoute>`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct LinkRoute {
    /// Position in `Page.links`; the link's pointer is `/links/<index>`.
    pub index: usize,
    pub kind: PipeKind,
    /// Geometry indices of the nodes named by `from` and `to`.
    pub from_node: usize,
    pub to_node: usize,
    /// Border-box polyline in page coordinates, first point on the from box edge, last on the to box edge. 2 to LINK_SEGMENTS_MAX + 1 points.
    pub points: Vec<PagePoint>,
    /// The tag box for label and sub, centered on the longest segment; None without a label.
    pub tag: Option<BoxRect>,
    /// Tag, TagLabel and [TagSub], boxed as in a pipe tag; empty without a label.
    pub parts: Vec<Part>,
    pub status: RouteStatus,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteStatus { Routed, Fallback }
```

- Attach points. Each end attaches at the midpoint of one side of its node's border box. With both `from_side` and `to_side` given, those sides. With neither and no `via`, the pair of facing sides (right to left, bottom to top, left to right, top to bottom) whose midpoints are closest, ties broken in the order right, bottom, left, top. Otherwise each end without a side takes the side whose midpoint is closest to what that end faces: the first via point for the from end, the last via point for the to end, or without `via` the other end's attach point; ties in the same order.
- Obstacles. The border boxes of every leaf node (Pcard, Fact, Note, Text, Callout, Frame, the tag of every Pipe and Tee, and the whole Tee), the label part of every Zone, and the page-level text nodes (`/kicker`, `/title`, `/lede`, each legend entry, `/foot`), except the from node, the to node, and any node that contains either endpoint. Container boxes are not obstacles: a link may cross a zone border, which is how an architecture figure shows a path entering a zone. A segment or grid intersection is blocked when it reaches into an obstacle's open interior by more than `GEOMETRY_EPSILON_PX`; running along an edge is allowed.
- Grid. The x set is every obstacle's left and right edge offset outward by 8 px, the from and to boxes' edges offset the same way, both attach x values and every via x; the y set likewise. Each set is capped at `ROUTER_GRID_LINES_MAX` per link; a link whose set exceeds the cap gets `RouteStatus::Fallback`.
- Search. A* over grid intersections with Manhattan moves between neighboring lines, cost 1 per pixel plus 40 per turn, with no move that reaches into an obstacle and no node inside one. The first move leaves the from box through its attach side and the last move enters the to box through its attach side, so a route never runs back across its own endpoint boxes. The result is simplified to its corner points. A route with more than `LINK_SEGMENTS_MAX` segments after simplification falls back. With `via`, the route passes each via point in order, each leg searched separately; the side rule applies to the first and last leg.
- Fallback. No route found, the grid cap exceeded, or too many segments: an L from the from attach point to the to attach point, horizontal leg first, with `status: Fallback`. Fallback is a defect in the `links-routed` check, never a silent success.
- Tag. Centered on the midpoint of the longest segment (the first on a tie), sized like a pipe tag (section 2.7) from the label and sub measured at max-content in `tag_label` and `tag_sub`, with the segment masked under it as for pipes.
- Determinism: the search is exhaustive over a bounded grid with a fixed tie-break (lower estimate, then lower x, then lower y, then the direction order right, down, left, up), so the route is a pure function of the geometry.

Checks, in section 6 terms, examined counts included:

| Check | Examined | Defect |
|---|---|---|
| `links-routed` | one per link | `status: Fallback` |
| `links-avoid-boxes` | one per segment and obstacle pair | a segment crosses an obstacle box, or a tag overlaps a node box other than a container or an ancestor of an endpoint |

Rendering: each route is a `<polyline>` (or `<path>` with `L` commands) in the wire color and line style of its kind, with an arrowhead as a filled triangle, 10 px long and 8 px wide, at each end the `arrow` value names, drawn with a `<marker>` per kind and theme. The tag is drawn exactly like a pipe tag. Links are drawn after every node, so they paint over zone fills and never under them. The measured JSON gains `links` with the routed points, tag box and status (section 5.4).

SVG structure: after the page group closes, one `<g data-id="/links/<i>" data-tag="Link" data-kind="<kind>">` per route in link order. Inside it, in order:

- A `<path d="M x y L x y ..." fill="none">` with the kind's stroke and `stroke-linejoin="round"`. At an arrowed end the path stops 10 px short of the attach point, or at the previous corner when the end segment is shorter, so the stroke ends under the arrowhead's base as a pipe wire does.
- One unstroked `<line>` per arrowhead from its base to its tip on the endpoint box edge, carrying `marker-end` with the `arrow-<theme>-<kind>` marker that pipes use. The marker set in `<defs>` covers every kind used by an arrowed Pipe, Tee arm or link.
- With a label, the tag `<rect>` (tag paint of the kind, radius 6) and the `tag_label` and `tag_sub` texts. The opaque tag fill masks the segment under it; the path itself is not split.

A `Fallback` route is drawn the same way; the `links-routed` check reports it. The routed tag sits on the longest segment's midpoint even when that midpoint falls on a zone's label or gcp bar, which `links-avoid-boxes` does not examine. Moving the tag along its segment clear of zone label parts is an open item for the router; `examples/onepager.json` sets `from_side` and `to_side` on its first link so its longest segment runs inside the zone body.

### 11.3 Document blocks

Three node tags for one-page design documents. They are leaf blocks: width from the container as for Fact, height from their wrapped text.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Text {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub heading: Option<String>,
    #[schemars(length(min = 1, max = 64))]
    pub body: Vec<String>,
    #[serde(default)]
    pub list: ListKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ListKind { #[default] Plain, Numbered, Bulleted }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Callout {
    pub kind: CalloutKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub title: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CalloutKind { Note, Risk, Decision, Open }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[serde(default = "frame_height_default")]
    #[schemars(range(min = 40, max = 1200))]
    pub height: u16,
}
```

Layout and paint:

- `Text`: 12 px padding, 1.25 px border in the card border color, radius 8, card fill. Heading 13 px bold with 6 px below it. Body lines 12 px regular at 1.45 line height, each line wrapped to the content width, 4 px apart; `numbered` prefixes `1.`, `2.`, ... in a 22 px hanging indent; `bulleted` prefixes a 4 px dot at the same indent. Each body line is its own text run in the measured output (`/body/…/body/i`), so `text-fits-box` examines it. A list line is a flex row: the Marker cell, 22 px wide and one body line tall, then the line's text leaf with `flex_grow` 1 and `flex_basis` 0, so wrapped lines keep the indent.
- `Callout`: as `Text` with a 4 px left accent bar and a tinted fill. The bar is an absolutely placed part at inset left 0, top 0 and bottom 0 inside the border, and the left padding is 12 + 4 = 16 px, so the bar takes no flow space and the content starts 12 px right of it. Accent and fill by kind in `center`: note `#1A73E8` on `#E8F0FE`, risk `#C5221F` on `#FCE8E6`, decision `#188038` on `#E6F4EA`, open `#B06000` on `#FEF7E0`. Dusk: the same accents on `#16305C`, `#3A1A1A`, `#143024`, `#3A2A10`. Wire: accent `#222222`, fill white, and the kind word in small capitals as a prefix to the title. Title 13 px bold, text 12 px regular.
- `Frame`: a wireframe placeholder. Dashed 1.25 px border, radius 4, no fill, two diagonal 1 px lines corner to corner in the secondary ink, and the label centered in a white (page background) chip so the diagonals do not cross the words. Height is authored; width comes from the container. The Frame is a flex column with padding 8 that centers the chip on both axes; its `size`, `min_size` and `max_size` heights all equal `height`, so neither a grow weight nor a Row's stretch changes it. The chip has padding 4/8 and `max_size.width` 100%, and holds the label in `zone_label`, centered, wrapping inside the Frame's content width.

Every block may carry `id` and be a link endpoint.

Paint, as implemented in `stencil-render/src/palette.rs`:

- The heading and Callout title use the `card_function` style (13 px bold); body lines, list numbers and Callout text use `block_body`; the Frame label uses `zone_label`. Inks follow those styles in each theme.
- A `bulleted` marker is a filled circle of radius 2 in the body ink, centered 5 px into the 22 px marker cell and on the cell's vertical center.
- The Callout box has the tint fill and the card border color at 1.25 px. The accent bar fills the Accent part and follows the inner curve of the rounded left corners (radius 8 minus the border), so it never covers the border stroke.
- The Frame border color is `#9AA0A6` in `center`, `#6B7A99` in `dusk` and `#222222` in `wire`. The diagonals are drawn first, between the inner corners pulled in to the rounded corner's inner curve, then the dashed border, then the label chip in the page background with radius 4.

Deviation, `wire` Callout: the kind word is not prefixed to the title. The prefix would be measured text that exists in one theme only, which changes geometry by theme and breaks the section 11.1 rule that measured JSON is identical across themes. A wire Callout shows its kind through the title the author writes; `examples/onepager.json` titles its callouts "Risk: ..." and "Decision: ...". A theme-independent kind label (drawn in every theme, measured once) is an open item.

### 11.4 CLI and CUE

`stencil render` and `stencil check` accept `--theme center|dusk|wire`, which overrides `Page.theme`. `stencil schema` includes the new types. `cue/stencil.cue` gains `theme`, `id`, `links`, `arrow` and the three block tags with the same constraints, and `cue/check.sh` gains negative cases for a duplicate id, an unknown link endpoint, a self link and a link kind missing from the legend.

### 11.5 Example and tests

`examples/onepager.json` is a one-page design document at width 1440: title, kicker and lede; a left column of `Text` blocks (Problem, Goals numbered, Non-goals, Interfaces) and two `Callout` blocks (a risk and a decision); a right column holding a `gcp` zone whose cards carry ids and are connected by six numbered `links` with arrows that trace one request through gateway, API, queue, worker and database, plus one `deny` link; and a bottom row of two `Frame` blocks for the console screens. The legend names every kind used.

Tests: `stencil-model` vets each new rule with a failing fixture; `stencil-layout` routes a link around one obstacle and asserts the polyline never enters it, routes with `via`, and produces `Fallback` for an endpoint fully enclosed by obstacles; `stencil-render` renders the same document under the three themes and asserts the measured JSON bytes are identical while the SVG bytes differ; `stencil-cli` renders `examples/onepager.json` under every theme with `check` exiting 0.

## 12. Isometric projection

`Page` gains an optional `projection` field, and the CLI `--projection` flag overrides it. An `iso` render draws the laid-out body as a 30 degree isometric floor plan: the flat layout lies on the floor, filled zones become slabs, a vpc becomes a dashed ring, leaf blocks become boxes, pipes lie on the slab they belong to, links run over the slabs to the blocks they join, and every label lies on the surface it names (section 12.4), the way a floor plan prints its names on the floor and a box carries its own on top. Layout under iso differs from flat only as section 12.4 rule 4 says: body type grows so sheared text reads, a zone keeps open floor under its name, and a tag that reads along y is a strip. The eight checks of sections 6 and 11.2 run on that layout. Every rule in sections 1 to 11 still holds unless this section names the change.

`flat` is the default, and a page without the field renders exactly as before this section: the Chrome golden comparison of section 9.4 and the center identity fixtures under `crates/stencil-render/tests/fixtures/` stay byte-identical.

### 12.1 Model

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Projection {
    #[default]
    Flat,
    Iso,
}

fn is_default_projection(projection: &Projection) -> bool {
    *projection == Projection::Flat
}

// On Page, directly after `theme`:
#[serde(default, skip_serializing_if = "is_default_projection")]
pub projection: Projection,
```

The field follows `theme`: absent means `flat`, and `flat` is never serialized, so the round trip of every existing example keeps its bytes. `schema/stencil.schema.json` is regenerated with the new optional property. No vet rule is added.

`CheckName` gains `IsoLabelsClear`, `as_str` `iso-labels-clear`, unit `pair` for 1 and `pairs` otherwise, and `IsoLinksClear` as its last variant, `as_str` `iso-links-clear`, unit `link leg` for 1 and `link legs` otherwise.

### 12.2 Projection

All projection code lives in `stencil_render::iso`. It reads `PageGeometry` only and never calls layout or a measurer.

```rust
pub mod iso;

// iso.rs
/// Slab thickness of a filled Zone, and the rise of each filled nested Zone over its parent.
pub const ISO_SLAB_THICKNESS_PX: f32 = 6.0;
/// Height of a leaf block (Pcard, Fact, Note, Text, Callout, Frame).
pub const ISO_BLOCK_HEIGHT_PX: f32 = 18.0;
/// cos 30 degrees, written out so every build uses the same f32.
pub const ISO_COS_30: f32 = 0.866_025_4;
/// sin 30 degrees.
pub const ISO_SIN_30: f32 = 0.5;
/// Smallest side margin of the projected body.
pub const ISO_MARGIN_PX: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint { pub x: f32, pub y: f32 }

/// A flat point at a height: one vertex of a link path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsoPoint { pub x: f32, pub y: f32, pub z: f32 }

/// The projected page. The pipeline builds it once for `measured_json` and
/// `iso_labels_clear`; `render_svg` builds an equal one itself, because the projection
/// is a pure function of the geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct IsoScene {
    /// The drawn canvas (section 12.2). `PageGeometry.canvas` stays the layout canvas.
    pub canvas: Size,
    /// Added to every projected point.
    pub offset: ScreenPoint,
    /// Added to the y of /legend, its entries and /foot.
    pub footer_shift: f32,
    /// One per body node that draws a solid, in geometry order.
    pub solids: Vec<Solid>,
    /// Every label on its plane (section 12.4): one per body node with plane members, in
    /// geometry order, then one per link with a tag, in link order.
    pub labels: Vec<Label>,
    /// One per `PageGeometry.links` entry: the routed polyline adjusted by section 12.3
    /// rule 8, laid over the slabs and cut back at its endpoint blocks (rule 7), in zoomed
    /// flat px.
    pub link_paths: Vec<Vec<IsoPoint>>,
    /// The kind of each link, in `link_paths` order.
    pub link_kinds: Vec<PipeKind>,
    /// The body zoom of rule 7.
    pub zoom: f32,
}

/// Carries the node's pointer and screen silhouette so that `iso_labels_clear` reads the
/// scene alone; the pointer makes it Clone but not Copy.
#[derive(Debug, Clone, PartialEq)]
pub struct Solid {
    /// Geometry index of the node.
    pub node: usize,
    pub pointer: NodePointer,
    pub shape: SolidShape,
    pub base_z: f32,
    /// ISO_SLAB_THICKNESS_PX for a filled slab, 0 for a ring zone and a surface,
    /// ISO_BLOCK_HEIGHT_PX for a block.
    pub height: f32,
    /// The six section 12.3 silhouette vertices in canvas px, offset included.
    pub silhouette: [ScreenPoint; 6],
    /// True when the faces are filled in every theme, so the solid hides what was drawn
    /// before it: a slab with height and every block except Note and Frame.
    pub opaque: bool,
    /// The node's border box in zoomed flat px.
    pub footprint: BoxRect,
}

impl Solid {
    /// The drawn edges of a slab (section 12.7): the four top-face edges, and for a slab
    /// with height the lower silhouette edges and the front vertical edge.
    pub fn slab_edges(&self) -> Vec<(ScreenPoint, ScreenPoint)>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidShape { Slab, Block, Surface }

// Label, PlaneMap and Axis are listed in section 12.4.

/// Asserts nodes[0] is the root and /body is present, then zooms and projects the body.
pub fn project_page(geometry: &PageGeometry) -> Result<IsoScene, RenderError>;
/// The geometry the projection draws (rule 7) and the zoom; the SVG writer draws from it.
pub fn zoomed_geometry(geometry: &PageGeometry) -> Result<(PageGeometry, f32), RenderError>;
/// `project_page` over a geometry `zoomed_geometry` returned.
pub fn project_zoomed(geometry: &PageGeometry, zoom: f32) -> Result<IsoScene, RenderError>;

/// NotApplicable with reason "projection is flat" for None; section 12.7 otherwise.
pub fn iso_labels_clear(scene: Option<&IsoScene>) -> CheckReport;
/// NotApplicable with reason "projection is flat" for None and "page has no links" for a
/// scene without links; section 12.7 otherwise.
pub fn iso_links_clear(scene: Option<&IsoScene>) -> CheckReport;
```

`project_page` returns `RenderError::GeometryMismatch` when `nodes[0]` is not `""` (expected `""`) or when no node has pointer `/body` (expected `/body`, found `/<absent>`).

Rules:

1. A flat point (x, y) at height z projects to `x' = (x - y) * ISO_COS_30 + offset.x` and `y' = (x + y) * ISO_SIN_30 - z + offset.y`, computed in f32 in that order. With this map a larger x runs down and to the right on screen, a larger y runs down and to the left, and a larger z runs straight up.
2. Only `/body` and its descendants are projected. `/kicker`, `/title` and `/lede` are drawn flat at their layout positions. `/legend`, its entries and `/foot` are drawn flat with `footer_shift` added to every y. The page root is the ground plane, z = 0, and draws nothing of its own. Projecting the page-level nodes would put the title on the ground behind the figure and slide the legend into it: text drawn horizontally from a point on the body's front edge enters the body's projection after about 10 px, because that edge descends at 30 degrees.
3. The extent set E holds, before any offset, what the body draws: the six silhouette vertices of every slab and block (section 12.3); both ends of every wire and spine and, at both ends of every pipe and spine, the cross-section of the widest ring a tube end can carry, a flange or a cone base, on the tube axis (the geometry does not record which ends are arrowed; the ring of an end lies inside the pipe box, so it does not move the extent); every vertex of every link path and the arrowhead vertices at both of its ends; and the four corners of every label (section 12.4). The `/body` border box draws nothing and is not in E; a body that draws nothing uses its four corners at z = 0. Let `min_x`, `max_x`, `min_y` and `max_y` bound E.
4. `offset.x = (canvas.width - (max_x - min_x)) / 2 - min_x` and `offset.y = body.y - min_y`, where `body.y` is the `/body` border box top. The drawn body is therefore centered across the canvas and starts at the flat body top.
5. `footer_shift = (max_y - min_y) - body.height`. `canvas.width = max(PageGeometry.canvas.width, (max_x - min_x) + 2 * ISO_MARGIN_PX)` and `canvas.height = PageGeometry.canvas.height + footer_shift`. The header keeps the width it was laid out at, so the drawn canvas is never narrower than the flat one. This is the only place the drawn canvas differs from `PageGeometry.canvas`.
6. The map sends a circle of radius r on a horizontal plane to an axis-aligned ellipse with semi-axes `r * sqrt(1.5)` across and `r * sqrt(0.5)` down, because the map's linear part M has M·Mᵀ = diag(1.5, 0.5).
7. Zoom. The body is scaled about the `/body` top-left corner by `zoom = clamp(ISO_FILL_FRACTION * canvas.width / W, 1, ISO_ZOOM_MAX)`, where `ISO_FILL_FRACTION` = 0.8, `ISO_ZOOM_MAX` = 1.6 and W is `cos 30` times the spread of `x - y` over the footprints of the body nodes that draw a solid (a Row or Col draws nothing and can span the page). Everything on the plane scales alike: every body node box, every link point, slab thickness, block height and every base_z are scaled in `zoom::zoomed` (scene px), while the parts a label draws keep their layout boxes and scale through the label's plane map (section 12.4 rule 1), so text and icons grow with the floor they lie on. A body at least 80 percent of the canvas wide is drawn at zoom 1, unchanged. `measured_json` keeps the layout `nodes`; only `projection` reflects the zoom.

### 12.3 Solids

A vpc zone is a ring: every theme draws it unfilled, so it has no slab of its own and draws its dashed border once, on its parent's top. Every other zone is a filled slab. The top of a node's nearest enclosing zone is that zone's `base_z + height`, and 0 without a Zone ancestor.

| NodeTag | Shape | base_z | Height | Faces and top-face drawing |
|---|---|---|---|---|
| Zone | Slab | top of the nearest enclosing zone | 6, or 0 for a vpc ring | three faces; a ring draws its top face only |
| Pcard | Block | top of the nearest enclosing zone | by shape: card 18, tile 6, tower 44, cylinder 36, stack 30 | a card draws three faces with its content on top; every other shape rises from its Footprint part with the icon on top and the text on the floor in front (rule 8) |
| Fact, Note, Text, Callout, Frame | Block | top of the nearest enclosing zone | 18 | three faces; Callout draws its Accent part and Frame its two diagonals on the top face |
| Pipe (Tee arms included), Tee | Surface | top of the nearest enclosing zone | 2 × `ISO_TUBE_RADIUS_PX` = 10, the tube's diameter | tube, flanges or cones (Pipe); spine tube (Tee) |
| Row, Col | none | | | nothing: a container adds no depth and draws nothing |

The page-level nodes draw no solid. Nested filled zones stack: a top-level zone spans z 0 to 6, a filled zone inside it 6 to 12, and so on; a ring adds nothing, so the cards in a vpc inside a gcp zone stand at 6.

1. Faces. Every slab and block draws three faces, as `<polygon>` elements in this order: left, right, top. The left face stands on the flat bottom edge (y = bottom) and faces down and to the left on screen. The right face stands on the flat right edge (x = right) and faces down and to the right. The top face is the border box at `base_z + height`. A ring has no side faces and draws its top face only. The flat corner radius is not drawn: faces are square. A face that has neither fill nor stroke is not written, and every stroked face carries `stroke-linejoin="round"`, so no outline join ends in a miter spike.
2. Face paint. Fills and strokes come from the palette (section 12.6): `Palette::slab_faces` for a zone and `Palette::block_faces` for a block, from the node's flat fill (the zone fill of section 2.4, the gcp body fill, the card fill for a Pcard and a Text block, the fact fill for a Fact, the callout tint for a Callout) and its flat border. A node with no flat fill (vpc, Note, Frame) draws its faces with `fill="none"`; a Note has neither fill nor border and draws no faces, but still occupies its block height for its label.
3. Silhouette. The six silhouette vertices of a box with border box (x0, y0, x1, y1), base z0 and top z1 are, in order: (x0, y0, z1), (x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0), (x0, y1, z1). Their projection is the convex hexagon that the three visible faces cover exactly.
4. Top-face drawing. The Callout Accent part and the Frame diagonals are projected onto the top face as polygons and lines at `base_z + height`, in their flat paint. The gcp Bar part is not drawn: a band along the back edge is the heaviest mass on the page and carries no meaning the label chip and the brand blue side faces do not.
5. Pipe. The wire is a tube of radius `ISO_TUBE_RADIUS_PX` = 5 lying on the Pipe's base_z, its axis at base_z + 5, from the start dot center to the end dot center; at an arrowed end the tube stops at the cone's base. The tube is screen geometry from the flat endpoints (`iso::tube`): a cross-section is the circle of the radius in the vertical plane across the run, projected as 24 points; the body is the parallelogram between the two silhouette offsets of that circle, the extremes across the projected run, split along the axis into a lit upper half and a shaded lower half. The far cap is drawn under the body as its rounded end and the near cap over it; the near end is the one the run leaves toward +x or +y, whose face the viewer sees. A dot end carries a flange ring, a tube of `FLANGE_RADIUS_PX` = 7 and `FLANGE_LENGTH_PX` = 4 on the same axis, in the dot's style: none draws nothing, hollow draws an outlined ring, filled a filled one. A ring at the far end is painted before the body, and the body then starts at the ring's near face and draws no far cap, so the tube is seen coming out of the ring; a ring at the near end is painted after the body, over its end. A deny line carries no ring: a hollow ring at its start read as a broken C. Its arrowed end carries, after the cone, a stop plate (`tube::stop_plate`): a wall across the run at the tip, `STOP_PLATE_HALF_WIDTH_PX` = 10 each side, `STOP_PLATE_THICKNESS_PX` = 3 along the run, from the floor to `STOP_PLATE_RISE_PX` = 4 above the tube's axis, all at the zoom; its face toward the cone is painted in the cap tone and its top in the wire color, or the top outlined and the face filled under a hollow style. A deny link draws the same plate at each arrowed end. An arrowed end carries a cone of `CONE_LENGTH_PX` = 12 and base radius `CONE_RADIUS_PX` = 8.5 whose tip lies where the flat arrowhead's tip lies; its base circle is drawn over the cone only when the base faces the viewer. A tube is filled when the theme shades faces and the line is solid: the body halves are the wire color and that color `TUBE_LIT_STEP` = 12 points lighter, the near cap and a visible cone base `TUBE_CAP_STEP` = 16 points darker. A theme whose face steps are all zero draws every tube hollow, as the page background outlined in the wire color; a dashed or deny line draws a hollow tube whose outline carries its pattern; a cone is always filled. Every piece is a `<polygon>`, so no `<ellipse>` and no `<line>` remain in a pipe. The tag pill lies on the plane at the tube's top, base_z + 10 (section 12.4), and masks the tube's middle. A Pipe with `"form": "band"` (`PipeForm`, default `tube`) is a band instead: a flat wide arrow lying on the plane at base_z, for a wide flow across a row. Its body is the rectangle of half-width `BAND_HALF_WIDTH_PX` = 10 between the dot centers, stopping at each arrowhead's base, and an arrowed end carries the flat triangle of `BAND_HEAD_LENGTH_PX` = 24 and half-width `BAND_HEAD_HALF_WIDTH_PX` = 18 with its tip where the flat arrowhead's tip lies; a dot end is square. It is painted like a tube, filled in the wire color or hollow with the line's pattern, each piece a `<polygon>`. A band's solid has no height, so its tag pill lies on the plane at base_z. The flat writer draws a band as the wire of section 2.7. Under iso a pipe end reaches what it joins (`pipe_ends::extend_pipe_ends`, after the slots of section 13.8 move): each dot part moves outward along the run until its center lies on the near edge of the landing box: the attach box (an item's footprint when it stands on one, else its bounds) of the node nearest along the run among those in the neighbour's subtree (section 6), the neighbour included, that may carry a pipe and whose attach box spans the pipe's center on the cross axis, or, when the pipe names that end's target, the near edge of the target's attach box; an end whose neighbour subtree holds no such node, an end that would land on an item's floor text among them, stays; an end never moves inward, and flat moves nothing. Under iso `pipes-land` reads the attach boxes for the cross-axis rule of section 6 and adds the run axis: the dot center lies on the landing box's near edge, else `<side> end stops D px short of NODE (attach box edge at x E)`; and an end that names no target and lands on a zone whose subtree holds an item is `<side> end lands on the edge of zone NODE and names no target; name the device it reaches`, since a trunk that stops at a zone's edge beside the devices in it reads as a line to nowhere (a named zone target is the author's choice and passes). A tube or band therefore never stops a Row gap short of the slab or solid it joins.
6. Tee. The spine is a tube down the projected spine center line, from the spine box's top to its bottom, built and painted as a Pipe's tube with a cap at each end. Under iso the spine moves to stand on the right edge of the Tee's left neighbour, and its arms' start dots with it. The hub lies on the plane at the tube's top, with `ISO_TAG_CLEARANCE_PX` of margin above and below it under iso so the arms' tags stay clear of it (section 12.4).
8. Shapes. An Item stands as the `Shape` of section 13.1: its own `shape`, else the `shape` of its icon's row in the kind's product table, else the kind's `shape`, else card (`products::item_shape`). The gcp `product` kind and plain's `service`, `store` and `external` name `block`, so under iso an item stands as a block of a fixed footprint with its text on the floor unless its icon's row or the author says otherwise; `"shape": "card"` keeps the raised card with its content on top. The gcp table makes databases and analytics cylinders, storage stacks, compute and containers towers, networking and hybrid tiles; the `person` kind of gcp and plain stands as a figure and the `device` kind as a laptop, with `"shape": "phone"` on the item for a phone. A card is laid out and drawn as before. Every other shape is laid out under iso as a column: a square `Footprint` part (`shape_footprint_px`: 64 tile and block, 44 tower, 56 cylinder and stack, 44 figure, 56 laptop, 36 phone; `shape_height_px`: 6 tile, 18 block, 44 tower, 36 cylinder, 30 stack, 56 figure, 40 laptop, 44 phone) holding the Icon when the item has one, then the text column centered under it after a gap, with `ISO_FLOOR_TEXT_RESERVE_PX` = 64 kept clear below the text for the strip a raised solid in the next row covers. Its solid rises from the footprint (`Solid.footprint`), with `Solid.form` the shape, `Solid.silhouette` the six vertices of the footprint box and `Solid.outline` the convex outline the checks and the link cut-back read: the six vertices for a box form, for a round form the hull of the base and top ellipses, each the circle inscribed in the footprint mapped by rule 6 of section 12.2 (`ellipse_radii`). A tile, a block and a tower draw the three faces of rule 1 at their heights. A cylinder draws its visible side as two `<path>` halves, left and right of the front line, in the left and right face paints, then its top `<ellipse>` in the top paint; a stack draws three such cylinders of (height - 2 gaps) / 3 with `STACK_GAP_PX` = 2 between them, bottom to top. A round solid's shadow is its base ellipse, lowered and blurred like a block's. The sprites (`iso::sprites`) are composed of these primitives inside the footprint. A figure is a torso cylinder of 0.62 of the footprint side, centered, up to 0.56 of the height, then a 2 px neck and a head sphere filling the rest, drawn as a `<circle>`; a figure is painted as a dark silhouette in the theme's secondary ink (`Palette::figure_paint`: the top 14 points lighter, the right side 12 points darker), not in the card's fill; its outline is the hull of the torso ellipses and the head, and its shadow the torso's base ellipse. A laptop is a 3 px base plate over the footprint and a 4 px deep screen slab standing on the plate's back edge up to the height, with a screen panel on the slab's front face inset 3 px and 28 points darker than the right face; its outline is the hull of both silhouettes and its shadow the plate's. A phone is a 6 px deep slab of 0.6 of the footprint's width standing across its middle, facing +y, with the same panel; its outline and shadow are the slab's. Links attach to the footprint, not the item's box, and never to its bottom side unless the author says so, because the text lies there; the item's Text part is an obstacle for its own links.
7. Links. A link runs over the terrain: the footprints of the slabs with height. Its routed polyline of section 11.2 is lifted whole to the top of the highest slab it runs over (`drape::route_z`: every segment is sampled at the midpoint of each span between its footprint-edge crossings, and the route's first point), so it lies on the highest terrain it crosses and floats over lower floor by at most that difference; there are no risers and no pass-unders. Collinear points are merged. Each end is then cut back to where the path enters the silhouette of its endpoint block on screen: an end on a block's bottom or right side lies on the silhouette already and stays; an end on a top or left side reaches a face the viewer cannot see, and without the cut its last stretch would be drawn across the block's top face. A zone endpoint is not cut. An end that still lies outside the silhouette after the cut, as on a figure, laptop or phone whose hull is narrower than the footprint the route ends on, is extended along its last stretch until it meets the silhouette, by at most `LAND_REACH_PX` = 64 zoomed px (`drape::land_end_at`); an end that would not meet it within reach stays, and `iso-link-ends` reports it. `IsoScene.link_paths` holds the result. The writer draws a solid link under a theme that shades faces as a tube per leg of radius `link_tube_radius(kind)` at the zoom (4.5 for solid slot 1, 3 for dash, 2.5 for every other line, the same in every theme), built and painted as a Pipe's tube of rule 5 with a shadow under each leg, a round joint at each corner (a `<circle>` of the radius on the tube's axis, in the wire color), and a cone of `iso_link_arrowhead_length(kind)` (18 for blue, 15 for dash, 14 for gray, pink and deny) and base radius 1.9 times the tube's at each arrowed end; the path is shortened by the cone's length along the last horizontal stretch first. A dashed or deny link, and every link under a theme whose face steps are all zero, is one `<path>` stroked with its kind's stroke and pattern along the path lifted by the tube radius, since a hollow tube would double a patterned line, ending in the same cone. An iso SVG writes no `<marker>`, because a marker draws its triangle unprojected. A link is drawn after every node, as in flat, so it paints over faces; its tag lies on the tube's top, the path height plus twice the radius at the zoom, under the midpoint of its longest leg (section 12.4).
8. Link adjustments. Before the path is laid over the terrain, the routed polyline changes in two ways, both iso only (the flat route and `PageGeometry` are unchanged). First, when its ends sit on two facing sides of the attach boxes (an item's footprint when it stands on one, else its bounds; right and left, or bottom and top, with the second box beyond the first) that share at least `ISO_STRAIGHT_SHARED_MIN_PX` = 16 across the gap, the route becomes one leg through the target's center, held inside the shared span by a margin at each end of `ISO_LINK_CORNER_CLEARANCE_PX`, plus the block's height where the end sits on a box form's hidden top or left side, since the cut of rule 7 lands such an end that much nearer the front corner; a span with no room between its margins, or a leg that enters a block, is left as routed. Side midpoints of two blocks that nearly line up would otherwise leave a short jog that the projection turns into a kink. Then a route of three legs whose first and last run the same way, a Z, has its step moved to the first of `STEP_FRACTIONS` = 1/2, 3/4, 1/4 of the run between its ends at which the moved route enters no block and the step, once cleared of the zone edges, keeps `ISO_LINK_CLEARANCE_PX` from every zone edge beside it and from every leg of another link along its axis, side by side or end to end (`route::step_at`, `step_clear`, `route::near_parallel`; each link is adjusted knowing the earlier links as adjusted and the later ones as routed); the router leaves the step at the end of the target's stub, where it reads as a kink at the block. A route through `via` points keeps the author's corners (`SolidInputs.vias`). Second, each inner leg (neither the first nor the last) that runs parallel to a zone edge closer than `ISO_LINK_CLEARANCE_PX` = 24, over an overlapping extent, moves to the nearest line at 24 from each parallel edge beside it, provided the two legs around it keep their direction, no moved leg enters a block, and the leg stays over the same zones; otherwise it stays, and `iso-links-clear` reports it. Third, ends that share a side of one node spread along that side (`end_offsets`): the ends on each (node, side) pair, the side read from the leg that leaves or reaches the end, sit in link order `bundle_spacing` apart, their tube radii at the zoom plus `ISO_LINK_GAP_PX` = 4 and never less than `LINK_CONE_RADIUS_SCALE` times the two radii, so two cones on one node do not overlap. On a visible side the group is centered on the mean of its attach points; on a block's top or left side the cut of rule 7 lands an arrival the block's height nearer the front corner than its attach point, so there the group is centered half the block's height behind that mean. The spacing between neighbours is also never less than either link's tag half-height plus the other's tube radius and the air, so a tag lying on one tube does not cover the other (`bundle_spacing`, `link_width`). A route of three or more corners moves only the leg at each shifted end, with the corner after it (`route::shift_end_leg`), so the leg beside the corner only changes length. A route of one leg, and every member of a bundle, is shifted whole (`route::offset_polyline`), by the offset of an end on a hidden side if it has one, else of the end that shares a side; a bundle is the set of links whose adjusted routes coincide corner for corner (`shared_route_places`, read from the routes adjusted at the base clearance), and its members are laid out from one shared route whose inner legs keep the bundle's widest offset added to the zone clearance; a lane too narrow for the wider clearance takes the base one. A shift that would leave an end off its attach box or send a leg through a block is dropped, and `iso-links-apart` reports the pair. The tags of a bundle sit at `(position + 0.5) / size` of the longest leg instead of its middle (`longest_segment_point`), so the pills do not stack.

### 12.4 Labels on their planes

Under iso nothing stands upright. Every part a node draws beyond its faces, its label, icon, fact boxes or tag, lies on the node's plane, and solids rise from their footprints, so a figure reads as one object rather than a drawing with stickers. One `Label` is made per body node that has plane members, and one per link with a tag.

| Owner | Members (`plane_member`) | Plane z | Axis |
|---|---|---|---|
| Box (slab) | the Label run | slab top | x |
| Item, card | Icon with its chip, FactBox, BuiltBox, AskBox, FunctionName, ProductName, Fact, Built, Ask | block top | x |
| Item, other shape | two labels: the Icon with its chip (`LabelParts::Icon`, contained) at the solid's top; every other member (`LabelParts::Text`) at the floor the solid stands on | solid top; floor | x |
| Fact, Note | Text | block top | x |
| Text | Heading, every Marker and BodyLine | block top | x |
| Callout | Heading, Text | block top | x |
| Frame | LabelChip, Label | block top | x |
| Pipe, Tee arm | Tag, TagLabel, TagSub | the tube's top, base_z + 10; base_z for a band | x for a horizontal pipe, y for a vertical one, or the pipe's `axis` |
| Tee | Hub, HubText | the spine tube's top, base_z + 10 | x |
| Link with a label | Tag, TagLabel, TagSub | the tube's top: the path height plus twice `link_tube_radius` at the zoom | x on a leg along flat x, y on a leg along flat y, or the link's `axis` |

```rust
pub enum Axis { X, Y }

/// Layout px to screen px: (a x + c y + e, b x + d y + f), an SVG matrix(a b c d e f).
pub struct PlaneMap { pub a: f32, pub b: f32, pub c: f32, pub d: f32, pub e: f32, pub f: f32 }

impl PlaneMap {
    pub fn plane(z: f32, zoom: f32, origin: (f32, f32), offset: ScreenPoint) -> PlaneMap;
    pub fn turned_about(self, pivot: (f32, f32)) -> PlaneMap;
    pub fn apply(&self, x: f32, y: f32) -> ScreenPoint;
    pub fn corners(&self, bounds: BoxRect) -> [ScreenPoint; 4];
}

pub struct Label {
    pub owner: NodePointer,
    pub node: Option<usize>,
    pub z: f32,
    pub axis: Axis,
    pub map: PlaneMap,
    pub flat: BoxRect,
    pub corners: [ScreenPoint; 4],
    pub marks: Vec<[ScreenPoint; 4]>,
    pub opaque: bool,
    pub contained: bool,
    pub parts: LabelParts,
}

pub enum LabelParts { All, Icon, Text }
```

1. Plane map. `PlaneMap::plane(z, zoom, origin, offset)` zooms about the body origin (rule 7 of section 12.2) and then projects at height z (rule 1): `a = zoom cos 30`, `b = zoom sin 30`, `c = -zoom cos 30`, `d = zoom sin 30`, `e = (1 - zoom)(origin.x - origin.y) cos 30 + offset.x`, `f = (1 - zoom)(origin.x + origin.y) sin 30 - z + offset.y`. Every entry is written with `format_number`.
2. Frames. The members stay in layout px in `NodeGeometry.parts` and `LinkRoute.parts`; `zoom::zoomed` leaves them untouched while it scales everything else into scene px. `Label.flat` is the union of the member boxes; `corners` is `flat` through the map; `marks` holds each text run's ink box through the map. The ink box is `(part.x + align offset, part.y, metrics.width_px, metrics.height_px)`, the align offset `(part.width - metrics.width_px) / 2` for `TextAlign::Center` and 0 otherwise. Corners and marks are parallelograms, top-left first then clockwise in layout terms.
3. Axis y. A run that reads along y is laid out as a strip: `measure_in_layout` wraps it at the height the layout offers and reports its size with width and height swapped; a vertical pipe's tag pill becomes a row of strips, the label left of the sub, with its padding turned (8 across, 6 down); a link tag on a leg along flat y has every part box passed through `turned_box` about the tag center. `label_axis` reads the axis back from the parts: y when any text member's box is taller than wide and narrower than its text, with the pivot at the center of the members' union. The map is then `plane(z).turned_about(pivot)`, a quarter turn that takes layout right to flat -y and layout down to flat +x, so the text reads up-right; `local_part` turns a strip back through `unturned_box` to the layout box it is drawn in. The author overrides the default with `axis` on a Pipe or Link.
4. Type and floor. Under iso `text_style_for` grows every body style by `ISO_TYPE_SCALE` = 1.3, a zone label by `ISO_ZONE_LABEL_SCALE` = 1.25 and the frame bar label by `ISO_FRAME_LABEL_SCALE` = 1.4, a tier above the zone names; chrome and the legend keep their flat styles. Spacing grows more than the type: every container padding, card padding and gap by `ISO_SPACE_SCALE` = 1.75, a pipe's wire minimum by `ISO_WIRE_SCALE` = 3 with `ISO_TAG_CLEARANCE_PX` = 16 of wire kept on each side of its tag. A raised block covers, on screen, the strip of floor its height spans behind and left of it (a point at z = h projects like the floor point moved by (-h, -h)), so a container's back and left padding grow by `ISO_BLOCK_HEIGHT_PX` = 18, and a zone label's leaf carries a bottom margin of the tallest solid any item in the zone stands as (`shape_height_px`, a card's 18 at least) plus `ISO_LABEL_CLEARANCE_PX` = 8, since a solid in the zone's first row stands in front of the label on screen. Lines: a name, a zone or frame label and a tag label never wrap; a subtitle, fact, body line or tag sub keeps lines of at least `ISO_LINE_MIN_PX` = 120 unless the run is shorter, which `measure_in_layout` applies to the min-content request; a run never breaks around its middle dot (the spaces beside ` · ` become no-break spaces). Links: an end on a block is reached through a straight stub of `ISO_APPROACH_PX` = 40 outward from its side, or shorter, to the nearest obstacle ahead of it, so the search never starts inside a neighbour, and never past half the free run to the other end's attach box, so two facing stubs never overrun each other, the search runs between the stub ends with no leave or arrive direction at a stubbed end, and the body keeps `2 * ISO_APPROACH_PX` above the legend for a stub and its tag below the lowest block. A raised block covers the floor behind and left of its footprint by its height on screen (the top face at z lies over the floor at (x - z, y - z)), so under iso every block obstacle casts a second obstacle, its raised box grown by `block_height` toward -x and -y (`route::block_shadows`), and a route that clears the footprint in flat px no longer runs under the block's top face; `iso-links-clear` reports a leg that still crosses a block. Flat layout is byte-identical to the layout before this section; `measured_json` `nodes` differ between a flat and an iso layout of one page.
5. Drawing. Each label is one `<g data-plane="<z>" data-axis="x|y" transform="matrix(a b c d e f)">` written right after its owner's faces inside the owner's group, or after the path and arrowheads inside a link's group, holding its members in their layout boxes through the section 5.2 flat drawing: the icon chip of rule 6 under its image, fact and ask boxes, the tag pill, then the text runs. No tab and no text plate is drawn. A Box label takes the ink of section 12.6 rule 8. The number of `<text>` elements equals the flat render's. No other group carries a transform.
6. Icon chip. Under iso every theme draws the 36 px chip under an icon: `Palette::iso_icon_chip`, white with a 1 px card-border ring in center, the dusk white tile in dusk, white with a 1 px ink ring in wire; center also draws a shadow under it, the same rect 1.5 px lower in `#202124` at 0.18 opacity. Inside the plane group the chip and its image shear with the face, a decal on the box top.
7. Occlusion. A label paints with its solid, so a solid painted later in the painter order covers it the way it covers a face. The layout reserve of rule 4 keeps that from happening in a figure that passes `iso-labels-clear`, which reports every covered mark (section 12.7).
8. The kicker badge and every page-level run are drawn flat (section 12.2, rule 2) and are not labels.

### 12.5 Painter order and SVG

The painter order is section 4.4 geometry pre-order: each node's faces, then its descendants, siblings in geometry order, Tee arms directly after their Tee. This is a valid back-to-front order for the stencil tree for two reasons. Siblings in a Row or Col are separated along the container's main axis, and a box with the larger x or y is nearer the viewer. Children sit above their parent's slab and inside its footprint, so a parent face never covers a child. A sort by the x + y of each box's bottom-right corner is not used: a child's corner lies inside its parent's, so that key would paint every zone's top face over its own cards.

SVG structure of an iso render:

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="1124.5" height="789.1" viewBox="0 0 1124.5 789.1">
  <rect x="0" y="0" width="1124.5" height="789.1" fill="#FFFFFF"/>
  <g data-id="" data-tag="Page" data-projection="iso">
    <g data-id="/kicker" data-tag="Kicker"> … flat … </g>
    <g data-id="/body" data-tag="Body">
      <g data-id="/body/0" data-tag="Row">
        <g data-id="/body/0/children/0" data-tag="Zone" data-kind="onprem-a">
          <polygon points="…" fill="#ACCBF9"/> …
          <polygon points="…" fill="#D2E3FC" stroke="#D7CCC8" stroke-width="1.5" stroke-linejoin="round"/>
          <g data-plane="6.92" data-axis="x" transform="matrix(0.87 0.5 -0.87 0.5 340.55 31.64)">
            <text x="33.5" y="164.5" …>On-prem</text>
          </g>
        </g>
      </g>
    </g>
    <g data-id="/legend" data-tag="Legend"> … flat, shifted … </g>
  </g>
  <g data-id="/links/0" data-tag="Link" data-kind="blue">
    <path d="…"/> …
    <g data-plane="0" data-axis="x" transform="matrix(…)"> … the tag … </g>
  </g>
</svg>
```

The coordinates in this example are illustrative.

- The root `<svg>` and background `<rect>` use `IsoScene.canvas`. The page group carries `data-projection="iso"`; a flat render writes no such attribute.
- There is still one `<g data-id>` per geometry node, nested and ordered as in section 5.2. A body node's group holds its faces, top-face drawing and surface primitives, then its plane group (section 12.4 rule 5) when it has one.
- Link groups follow the page group in link order and hold the projected link path, the arrowhead polygons and, for a labeled link, the tag's plane group.
- The legend and foot are drawn flat and shifted; in `wire` each legend label names the line (rule 5 of section 12.6).

### 12.6 Shading

```rust
// palette.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face { Top, Left, Right }

/// Stroke widths of wires, links and legend swatches under iso (`Palette::iso_wire_width`).
pub const ISO_PRIMARY_WIRE_PX: f32 = 3.75;   // blue
pub const ISO_SECONDARY_WIRE_PX: f32 = 2.5;  // dash, pink, deny
pub const ISO_SERVICE_WIRE_PX: f32 = 2.0;    // gray in center and dusk
pub const ISO_WIRE_THIN_WIRE_PX: f32 = 1.25; // gray in wire
/// The gcp slab's top-face outline under iso in center and dusk.
pub const ISO_GCP_OUTLINE_PX: f32 = 1.5;

/// Fills and strokes of the three faces of an isometric solid. A None fill draws the face
/// unfilled; a None stroke draws it unstroked.
pub struct FacePaint {
    pub top: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub top_stroke: Option<Stroke>,
    pub side_stroke: Option<Stroke>,
}

impl Palette {
    /// The palette of a theme under a projection; `Flat` gives `Palette::new`.
    pub fn for_projection(theme: Theme, projection: Projection) -> Self;
    /// HSL lightness step of a face in percentage points; None in wire (no shading).
    pub fn face_lightness_step(self, face: Face) -> Option<i8>;
    /// The fill of `face` for a node whose flat fill is `base`: base itself for Top, base
    /// shaded by the step for Left and Right, and the page background for every face in wire.
    pub fn face_fill(self, base: &'static str, face: Face) -> Option<String>;
    /// The faces of a zone slab that has `level` filled zones under it.
    pub fn slab_faces(self, kind: ZoneKind, level: usize) -> Option<FacePaint>;
    /// The faces of a leaf block with this flat fill and border.
    pub fn block_faces(self, fill: Option<&'static str>, border: Option<Stroke>) -> Option<FacePaint>;
    pub fn iso_icon_chip(self) -> BoxPaint;
    pub fn iso_icon_chip_shadow(self) -> Option<&'static str>;
    /// The legend label of a kind under iso when the flat label names a color the theme does
    /// not draw.
    pub fn iso_legend_label(self, kind: PipeKind) -> Option<&'static str>;
    pub fn iso_wire_width(self, kind: PipeKind) -> f32;
    /// Filled for a zone no zone encloses, an outline for a nested one (section 12.4).
    pub fn iso_label_ink(self, look: ContainerLook, tint: Option<u8>) -> &str;
}


/// `color` (`#RRGGBB`) with its HSL lightness moved by `step_points` percentage points.
/// None when `color` is not `#` followed by six hex digits.
pub fn shade(color: &str, step_points: i8) -> Option<String>;
/// `color` moved `fraction` of the way to `toward` in each sRGB channel.
pub fn mix(color: &str, toward: &str, fraction: f32) -> Option<String>;
```

The render builds its palette with `Palette::for_projection(page.theme, page.projection)`. Under `Flat` every method returns what it returned before this section, so the flat output is unchanged.

| Theme | Top | Left | Right |
|---|---|---|---|
| center | 0 | -8 | -16 |
| dusk | 0 | -4 | -8 |
| wire | no shading: every face is the page background `#FFFFFF` |

Slab paint (`slab_faces`), from the zone's flat fill F (the body fill for gcp) and flat border B:

| Theme | Top | Left, right | Top stroke | Side stroke |
|---|---|---|---|---|
| center | F | F shaded -8 and -16; gcp `#1A73E8` and `#1257B3` | gcp B at 1.5 px; a dashed B; a solid B on another kind is dropped, because the shaded sides carry the edge and a border in another hue shows as a seam | none |
| dusk | `mix(F, #7F93B8, 0.20 + 0.08 * level)` | left `mix(F, #7F93B8, top fraction - 0.04)`, right `mix(F, #7F93B8, top fraction - 0.08)`; gcp as center | solid B: a 1 px `#4A5B7E` rim; dashed B: B; gcp B at 1.5 px | none |
| wire | `#FFFFFF` when F exists | `#FFFFFF` when F exists | a solid B at 1.5 px; the vpc ring dotted in `#999999` at 1.5 px, so no zone edge shares the ink and dash of a link; other dashed B as B | B at 1.5 px when it is solid |

Block paint (`block_faces`), from the node's flat fill F and flat border B: center and wire use the face table above; dusk uses `mix(F, #7F93B8, f)` with f = 0.46 for the top, 0.32 for the left and 0.20 for the right, which puts a card top about 15 L* above the gcp floor. Every face is stroked with B, and in wire with the ink border when B is None.

1. `shade` parses the six hex digits into r, g and b in 0 to 1 (value / 255 in f64), converts to HSL with the standard formulas (hue from the largest channel, `rem_euclid(6.0)` for the red sector, saturation 0 when max equals min), sets `L = clamp(L + step_points / 100, 0, 1)`, converts back, maps each channel to `floor(channel * 255 + 0.5)` clamped to 0 to 255, and writes `#` and six uppercase hex digits. A step of 0 returns the input string unchanged, without a round trip. `mix` clamps the fraction to 0 to 1 and maps each channel to `floor(from + (to - from) * fraction + 0.5)`.
2. `face_fill`, `slab_faces` and `block_faces` return None when a color does not parse, and the SVG writer returns `RenderError::Svg` naming the node, so no library code unwraps. A unit test parses every color constant in `palette.rs`, so the None path is unreachable in practice.
3. Center and dusk draw slab sides unstroked. A stroked side face and the top face outline would draw two parallel lines 6 px apart along every front edge; the fill boundary alone reads as one edge. A dashed or dotted border is drawn once, on the top face, in every theme. The gcp slab carries the brand blue on its side faces and keeps a thin top outline, so a 3 px link never reads as a zone edge.
4. Dusk fills sit near 12 percent lightness, so shading them darker leaves blocks darker than the floor they stand on. Dusk instead moves every surface toward a light blue gray by level: the page, then the floor of each filled zone, then the blocks, each lighter than the one below it, with the side faces darker than their top.
5. In `wire`, "no face fills" means no shaded or colored fills. The faces are filled with the page background so hidden edges stay hidden and the scene reads as a line drawing. A node with no flat fill still draws its faces unfilled. Wire keeps its flat line styles, and its legend labels name the line, because every kind is drawn in one ink: `Thin line` (gray), `Solid line` (blue), `Ringed line` (pink), `Dashed line` (dash) and `Dotted line` (deny). The writer measures the new label in the label run's style and moves the description by the change in width, so the gap between them keeps its size.
6. Under iso the stroke weight ranks the kinds in every theme: blue (the primary) 3.75 px, dash, pink and deny 2.5 px (two thirds of it), gray 2 px in center and dusk, where its color sets it apart, and 1.25 px in wire (a third of the primary), where one ink leaves weight, pattern and arrowhead to tell them apart. Every link arrowhead is at least four times its stroke (section 12.3, rule 7).

Test vectors, all exact under rule 1:

| Color | -4 | -8 | -16 |
|---|---|---|---|
| `#D2E3FC` | `#BFD7FB` | `#ACCBF9` | `#85B3F7` |
| `#FFFFFF` | `#F5F5F5` | `#EBEBEB` | `#D6D6D6` |
| `#FAFBFC` | `#EDF1F4` | `#E0E7ED` | `#C7D2DD` |
| `#1A73E8` | `#166AD8` | `#1461C5` | `#104EA0` |
| `#14213A` | `#0F182B` | `#0A101C` | `#000000` |

### 12.7 Check

| Check | Crate | Unit examined | Defect when | Epsilon |
|---|---|---|---|---|
| `iso-labels-clear` | render | each unordered pair of labels; each (label, opaque solid) pair where the solid is painted after the label's owner, for a link tag every opaque block; each (label, slab) pair where the label is not opaque; each (label, link) pair where the label is not opaque; each (label, surface) pair where the label is not opaque, does not own the surface and does not own a block the surface touches; each (tag, link) and (tag, surface) pair where the tag does not own the link or surface; each link or pipe tag, for lying nearest its own connector; each contained label | two labels' corners overlap, or lie closer than `ISO_LABEL_GAP_PX` = 4; a mark of the label overlaps the solid's outline, or lies closer to it than `ISO_LABEL_CLEARANCE_PX` = 8; a visible edge of the slab crosses a mark; the link's tube, at its radius on the drawn path, overlaps a mark; a pipe's tube or band, or a tee's spine, overlaps a mark; a link's tube or a pipe passes under the tag; another connector lies nearer the tag's center than its own; a mark of a contained label leaves its block's silhouette, or the label has no block | 0.01 px |
| `iso-links-clear` | render | each leg of each link path, in flat px with its risers dropped and collinear points merged | the leg runs parallel to a zone footprint edge closer than `ISO_LINK_CLEARANCE_PX` over an overlapping extent; the leg turns back on the one before it; the last leg is shorter than two arrowheads of its kind; the path runs more than `ISO_LINK_DETOUR_RATIO` = 1.5 times the span between its ends plus two approach stubs; an inner leg is shorter than `ISO_LINK_JOG_DIAMETERS` = 2 diameters of the widest link tube on the page; the leg's tube outline overlaps the outline of an opaque block that is not one of the link's endpoints | 0.01 px |
| `iso-link-ends` | render | each end of each drawn link path | on a block, the end lies off the block's screen outline, or within `ISO_LINK_CORNER_CLEARANCE_PX` = 8 of a vertex of a box form's outline; on a zone or a surface, outside its footprint; the end's node has no solid | 0.01 px |
| `iso-links-apart` | render | each unordered pair of drawn link paths | two legs share a collinear stretch; two parallel legs run closer than the pair's tube radii plus `ISO_LINK_GAP_PX` over an overlapping extent; two ends on one node lie closer than `LINK_CONE_RADIUS_SCALE` times the two radii | 0.01 px |

`CheckName::unit` for `iso-labels-clear` is `pair` for 1 and `pairs` otherwise; for `iso-links-clear` `link leg` and `link legs`; for `iso-link-ends` `link end` and `link ends`; for `iso-links-apart` `link pair` and `link pairs`.

1. The report is `CheckReport::not_applicable(CheckName::IsoLabelsClear, "projection is flat")` for a flat render. An iso render with fewer than two labels and no (label, solid), (label, slab), (label, link), (label, surface), tag or contained pair examines 0 and fails, as section 6 requires.
2. Overlap is a separating-axis test over two convex polygons: the x and y axes plus the unit normal of every edge of either. Two polygons overlap when their projections on every axis overlap by more than `GEOMETRY_EPSILON_PX`. A crossing is Cyrus-Beck: the segment crosses when the part of it inside the mark, shrunk by the epsilon, has positive length, so a segment that only touches an edge passes.
3. Cover and clearance. A solid painted after a label's owner in the painter order of section 12.5 covers the label where its outline overlaps a mark, and crowds it where the outline comes closer than `ISO_LABEL_CLEARANCE_PX` to a mark (`… lies D px from NODE, closer than 8`; the distance between two convex polygons is the shortest vertex-to-edge distance either way). Two labels keep `ISO_LABEL_GAP_PX` of air (`… lies D px from label OWNER, closer than 4`). Link tags are painted after every solid, so for them every opaque block counts: a tag must not lie where a block stands. A zone's own children are painted after the zone, which is why rule 4 of section 12.4 reserves floor behind the zone's name.
4. Slab edges. `Solid::slab_edges` gives a slab's four top-face edges and, for a slab with height, its four lower silhouette edges and the front vertical edge. An edge piece covered on screen by an opaque solid painted later is hidden and does not count. A mark is the ink box of one text run; the icon chip and every tag pill are opaque boxes that hide a stroke under them, so an opaque label (a pipe, tee or link tag) is not tested against edges or link paths.
5. Defect pointer: for two labels, the owner of the later one in `IsoScene.labels` order; otherwise the label's owner. Messages name the label by its owner, its top-left screen corner with 2 decimals and its axis, for example `iso-labels-clear /body/0/children/0/children/1: label /body/0/children/0/children/1 at 112.40,260.80 along x overlaps label /body/0/children/0/children/0`, `… is covered by /body/0/children/0/children/1`, `… is crossed by an edge of slab /body/0/children/1`, `… is crossed by link /links/0` and `… leaves its block`. The numbers are illustrative.
6. Pairs are visited in label order, then solid order, then slab order, then link order, then contained labels, so the defect list is deterministic. The loops are bounded by NODES_MAX + LINKS_MAX labels and NODES_MAX solids.
7. Contained text. The parts of a Pcard, Fact, Note, Text, Callout or Frame are laid out to fit the block; each mark's four corners must lie inside the block's silhouette, else the message is `… leaves its block`. A contained label whose node has no block is examined and reported (`… has no block`): a probe that cannot run is a failure, not a pass.
7a. Drawn connectors. A link is tested as the tube it draws: each leg's body outline (`tube::body`) at the link's radius at the zoom on the drawn path, against every mark of a non-opaque label (`… is crossed by link /links/i`). A `Surface` solid's `outline` is the hull of what it draws, from `surface_outline`: a pipe's tube from dot center to dot center with a ring's radius at each end, a band's flat body, a tee's spine tube; a pipe without dot parts keeps its box. Every non-opaque label is tested against every surface it does not own (`… is crossed by pipe NODE`), except a surface whose outline overlaps the silhouette of the label's own block: a pipe landing on the block ends under its faces, which are painted after the pipe, so the content on the block's top is not crossed. An opaque tag is tested against every link tube and surface its owner does not own (`… is passed under by /links/i`, `… is passed under by pipe NODE`): a link under its own tag is the normal case. A link or pipe tag's center must lie nearest its own connector, a tee's arms counting as its own (`tag of OWNER lies nearer /links/j than its own path`, `… nearer pipe NODE …`); a link tag's own distance is to its drawn path, a pipe tag's to its surface outline.
8. `iso-links-clear` is `CheckReport::not_applicable(CheckName::IsoLinksClear, "projection is flat")` for a flat render and `"page has no links"` for a scene without links. Every defect points at `/links/<i>`, with the messages `leg N runs D px beside an edge of zone NODE, closer than 24`, `leg N turns back on the leg before it`, `last leg is L px, shorter than two arrowheads (A px)`, `link runs L px between ends S px apart, more than 1.5 times their span plus two stubs (A px)` (the Manhattan length of the drawn legs against the Manhattan distance between the drawn ends plus `2 * ISO_APPROACH_PX` at the zoom; the hero's via failover runs 1.54 times its span and passes on the stubs), `leg N is L px between two turns, shorter than J` (an inner leg under `ISO_LINK_JOG_DIAMETERS` diameters of the widest tube among the page's links, radii at the zoom), `leg N steps D px from the start|end, within a stub (S px)` (an inner leg between two legs that run the same way, with the run to the nearer end at most `ISO_APPROACH_PX` at the zoom while the two runs together reach two stubs, so the step had room elsewhere), `leg N runs back against leg M across a leg of D px, shorter than a stub (S px)` (a leg antiparallel to the leg two before it, joined by a leg under a stub: a hook past the target) and `leg N crosses block NODE on screen` (the leg's tube body outline, at the path's height and the link's radius, overlaps the screen outline of an opaque block other than the link's two endpoint nodes, read from `PageGeometry.links`), lengths with 2 decimals. The last-leg rule reads the end the default `arrow` draws; the scene does not carry the arrow value. Legs are visited in link order, and zones in solid order within a leg. The loops are bounded by LINKS_MAX links and NODES_MAX solids.

9. `iso-link-ends` is not applicable as `iso-links-clear` is. Each end of each drawn path is examined, start then end, in link order. Every defect points at `/links/<i>`, with the messages `end lies D px off the outline of NODE` (the screen distance from the projected end to the nearest outline edge, over `GEOMETRY_EPSILON_PX`), `end lies D px from a corner of NODE, under 8` (box forms only, card, tile, tower and block; round forms and sprites have hull outlines with many vertices), `end lies D px outside zone NODE` (the flat distance from the footprint) and `end node NODE has no solid`; `start` names the first point. An arrival through a top or left side is not a defect: the cut of section 12.3 rule 7 puts such an end on a visible top-face edge, where an arrow reads as touching the block, while an arrow into a visible face shows its cone base. The loops are bounded by LINKS_MAX links and NODES_MAX solids.
10. `iso-links-apart` is `CheckReport::not_applicable(CheckName::IsoLinksApart, "projection is flat")` for a flat render and `"page has fewer than two links"` short of a pair. Each unordered pair of drawn paths is examined, the pairs visited in link order with the later link last; every defect points at the later link and names the earlier one: `shares L px with /links/j` (the summed collinear overlap of parallel legs over `GEOMETRY_EPSILON_PX`), `runs D px beside /links/j, closer than T` (the closest parallel gap under the pair's threshold, radii at the zoom) and `ends D px from the end of /links/j on NODE, closer than T` (the screen distance between two ends on one node, T the two cone bases). Legs are read in flat px with collinear points merged, as in rule 8; ends on screen through `project_point`. The loops are bounded by LINKS_MAX squared pairs.

The eight checks of sections 6 and 11.2 run on the flat geometry in both projections.

### 12.8 Outputs

```rust
/// Section 5.4 shape; with Some(scene) the output also carries `projection`.
pub fn measured_json(
    document: &serde_json::Value,
    geometry: &PageGeometry,
    scene: Option<&IsoScene>,
) -> serde_json::Value;
```

`measured_json` takes the scene because `document` is the raw input, which does not hold a `--projection` override. With `None` it writes exactly the section 5.4 bytes. With a scene it adds one top-level key:

```json
"projection": {
  "canvas": { "height": 789.1, "width": 1124.5 },
  "footer_shift": 424.3,
  "kind": "iso",
  "labels": [
    { "axis": "x", "corners": [{ "x": 369.56, "y": 148.36 }, { "x": 429.06, "y": 182.71 }, { "x": 412.75, "y": 192.13 }, { "x": 353.25, "y": 157.78 }], "id": "/body/0/children/0", "z": 6.92 },
    { "axis": "y", "corners": [ … ], "id": "/links/0", "z": 0 }
  ],
  "offset": { "x": 90.7, "y": 60.8 }
}
```

The numbers in this example are illustrative.

- `labels` lists every label in painter order with its owner pointer, its axis, its plane height and the four screen corners of its parts' union, top-left first then clockwise in layout terms. Each `id` resolves in `document`, except `/links/<i>`, which resolves to the link.
- The top-level `canvas`, `document`, `nodes` and `links` are the layout ones (layout px, before the zoom). `projection.canvas` is the drawn canvas.
- Keys are in ascending byte order at every level (section 5.4), and every number goes through `format_number`.
- The projection is theme-independent, so the section 11.1 rule holds under iso: the measured JSON is byte-identical across themes for the same document and projection.

`render_svg(page, geometry)` keeps its signature. When `page.projection` is `Iso` it calls `project_page` and writes section 12.5, otherwise section 5.2. The PNG follows the SVG's `width` and `height`, so an iso PNG is `ceil(canvas * scale)` of `projection.canvas`, and the pixel budget of section 5.3 applies to that canvas.

### 12.9 CLI and CUE

| Command | Change |
|---|---|
| `render` | accepts `--projection flat` or `--projection iso`, which overrides `Page.projection` as `--theme` overrides `Page.theme` |
| `check` | accepts the same flag; runs fourteen checks, `iso-labels-clear`, `iso-links-clear`, `iso-link-ends` and `iso-links-apart` before print-fit and icon-matches-product |
| `vet` | unchanged; `--projection` is an unknown flag and exits 2 |

- `pipeline::render_page` calls `project_page` once when the effective projection is `Iso` and keeps the result in `RenderedPage.scene: Option<IsoScene>`, which `measured_json` and the check read.
- `pipeline::all_checks(page, grammar, geometry, scene: Option<&IsoScene>, print_width: Option<PrintWidth>) -> [CheckReport; 14]`, in `CheckName` order.
- `check` prints fourteen check lines. A flat page prints `check iso-labels-clear: examined 0 pairs, not applicable: projection is flat`, `check iso-links-clear: examined 0 link legs, not applicable: projection is flat`, `check iso-link-ends: examined 0 link ends, not applicable: projection is flat` and `check iso-links-apart: examined 0 link pairs, not applicable: projection is flat`, so the g7 summary is `stencil check: 14 checks, 7 passed, 0 failed, 7 not applicable`. Check stdout changes for every document; render outputs do not.
- `cue/stencil.cue` gains `projection?: "flat" | "iso"`.

### 12.10 Example and tests

`examples/hero-iso.json` is a cover-slide figure of 16 body nodes and six links at width 1600: an on-prem Box with the Edge router inside a Col with `justify: center`, and a gcp Box holding a Row of a vpc ring labeled `Shared VPC · europe-west4`, two centered Cols with the API gateway (Cloud Run) and Model serving (Vertex AI), and an apis Box labeled `Google APIs`. The ring holds a Row of two networking tiles, the Cloud Router the attachments land on and a Private Service Connect endpoint; the serverless products stand beside the ring, since they are not addresses in a VPC, and the apis Box holds Warehouse, a BigQuery product (section 13.3, rule 4). Every Row has `grow: [0, …]`, so every Box and item takes its content width, and the Col keeps the on-prem Box to the router and its margin instead of the Row's height and centers it. Every item carries a subtitle naming what it is, as the gcp hop-fact rule asks. VLAN 1 (solid tint 1) runs from the router's right side to the Cloud Router's left side, VLAN 2 (dash) through a via point in the gap between the sites; gray service calls run from the Cloud Router to the endpoint, the endpoint to the gateway, the gateway to the model and the model to the warehouse.

```json
{
  "title": "On-prem data reaches three managed services over two private paths.",
  "kicker": "Hybrid platform · cover figure",
  "lede": "Two Interconnect attachments carry on-prem traffic into the Shared VPC, which reaches the API gateway, model serving and the warehouse as managed services beside it.",
  "width": 1600,
  "canvas": "customer",
  "grammar": "gcp",
  "projection": "iso",
  "body": [
    {
      "tag": "Row",
      "gap": 64,
      "grow": [
        0,
        0
      ],
      "children": [
        {
          "tag": "Col",
          "justify": "center",
          "children": [
            {
              "tag": "Box",
              "kind": "onprem",
              "tint": 1,
              "label": "On-prem",
              "children": [
                {
                  "tag": "Item",
                  "id": "router",
                  "kind": "product",
                  "icon": "hybrid",
                  "title": "Edge router",
                  "subtitle": "on-prem · BGP"
                }
              ]
            }
          ]
        },
        {
          "tag": "Box",
          "kind": "gcp",
          "label": "Google Cloud",
          "children": [
            {
              "tag": "Row",
              "gap": 64,
              "grow": [
                0,
                0,
                0,
                0
              ],
              "children": [
                {
                  "tag": "Box",
                  "kind": "vpc",
                  "label": "Shared VPC · europe-west4",
                  "children": [
                    {
                      "tag": "Row",
                      "gap": 48,
                      "grow": [
                        0,
                        0
                      ],
                      "children": [
                        {
                          "tag": "Item",
                          "id": "attachments",
                          "kind": "product",
                          "icon": "networking",
                          "title": "Cloud Router",
                          "subtitle": "VLAN attachments"
                        },
                        {
                          "tag": "Item",
                          "id": "psc",
                          "kind": "product",
                          "icon": "networking",
                          "title": "Service endpoint",
                          "subtitle": "Private Service Connect"
                        }
                      ]
                    }
                  ]
                },
                {
                  "tag": "Col",
                  "justify": "center",
                  "children": [
                    {
                      "tag": "Item",
                      "id": "gateway",
                      "kind": "product",
                      "icon": "cloud-run",
                      "title": "API gateway",
                      "subtitle": "Cloud Run"
                    }
                  ]
                },
                {
                  "tag": "Col",
                  "justify": "center",
                  "children": [
                    {
                      "tag": "Item",
                      "id": "model",
                      "kind": "product",
                      "icon": "vertex-ai",
                      "title": "Model serving",
                      "subtitle": "Vertex AI"
                    }
                  ]
                },
                {
                  "tag": "Box",
                  "kind": "apis",
                  "label": "Google APIs",
                  "children": [
                    {
                      "tag": "Item",
                      "id": "warehouse",
                      "kind": "product",
                      "icon": "bigquery",
                      "title": "Warehouse",
                      "subtitle": "BigQuery"
                    }
                  ]
                }
              ]
            }
          ]
        }
      ]
    }
  ],
  "legend": [
    {
      "line": "solid",
      "tint": 1,
      "text": "primary attachment"
    },
    {
      "line": "dash",
      "text": "failover attachment"
    },
    {
      "line": "gray",
      "text": "service call"
    }
  ],
  "links": [
    {
      "from": "router",
      "to": "attachments",
      "line": "solid",
      "tint": 1,
      "label": "VLAN 1",
      "to_side": "left"
    },
    {
      "from": "router",
      "to": "attachments",
      "line": "dash",
      "label": "VLAN 2",
      "via": [
        {
          "x": 300,
          "y": 480
        }
      ]
    },
    {
      "from": "attachments",
      "to": "psc",
      "line": "gray"
    },
    {
      "from": "psc",
      "to": "gateway",
      "line": "gray"
    },
    {
      "from": "gateway",
      "to": "model",
      "line": "gray"
    },
    {
      "from": "model",
      "to": "warehouse",
      "line": "gray"
    }
  ]
}
```

Its `iso-labels-clear` count is 406: 18 labels (4 Box names, the icon and the text of six items, 2 link tags) give 153 label pairs, the (label, later opaque solid) pairs of section 12.7 rule 3, the non-opaque labels against 4 slabs and 6 link tubes, the 2 tags against the 5 links they do not own and for lying nearest their own path, and the contained item labels; the page has no pipes, so no surface pairs. Its `iso-links-clear` count is 13 legs, the approach stubs included; `iso-link-ends` examines 12 ends and `iso-links-apart` 15 pairs. The page has no pipes, so `pipes-land` is not applicable. Without `projection` the document passes the flat checks with `iso-labels-clear` and `iso-links-clear` not applicable.

`examples/onprem-iso.json` is the on-prem site under the plain grammar at width 1700 with `"projection": "iso"`: two firewall zones, four VLAN zones of blocks with the core switch between the inside VLANs, the Edge firewall a card of its content width centered in its column, two gutter slots each holding two pipes named by `from` and `to`: the staff and server trunks from the core switch into the firewall's left side, and the DMZ trunk and a deny pipe from the VLAN 30 zone on its right side, the deny arrow at the firewall ending in a stop plate; and one link, the transit VLAN, from the firewall's top side to the edge router in the transit VLAN zone. It is the pipe fixture: every tube, flange and cone of rule 5 is in its center SVG, and `check` passes every check that applies to it. Its gallery PNG stands in the README.

`examples/people-iso.json` is the sprite figure under the plain grammar at width 1200: a Row weighted 3 to 1 of a system Box holding a group Box with a Row of a `person` (Operator), a `device` (Admin console, a laptop) and a `service` (API, a block), and a centered group Box of customers holding a Row, gap 32, of a `device` with `"shape": "phone"` (Shop app) and a `person` (Customer); the operator links to the laptop, the customer to the phone, the laptop and the phone to the API. Every sprite of rule 8 is in its center SVG, `check` passes every check that applies to it, and its gallery PNG stands in the README.

`examples/transit-iso.json` is the narrow-zone, long-name figure under gcp at width 1800, the top Row growing its gcp Box: an onprem Box (a networking tile and a person) beside a centered gutter Col holding one slot with a Pipe named `from` the Edge router and `to` the transit hub, labeled Dedicated Interconnect with both VLAN attachments in its sub and an end arrow, then a gcp Box with a Col at `justify: start` holding the Network Connectivity Center hub, which is global and so sits outside every region (section 13.3 rule 4), a narrow tinted region (Google Cloud VMware Engine) and a second narrow region of two items (Certificate Authority Service above Database Migration Service), with one gray link from the hub to Migration. The hub stands at the top of the Row so its footprint overlaps the router's on the cross axis, which is what the slot of section 13.8 aims at. It exercises the pipe tube with a ring and a cone, the zone label reserve under a tower and floor text wrapped to three lines;

Tests:

- stencil-model: `"projection": "iso"` and `"flat"` parse and an absent field gives `Flat`; `"oblique"` is a `ModelError::Json`; a page with `Flat` serializes without the field, so the g7 round trip is unchanged; the schema test passes against the regenerated file and a copy with `"projection": "oblique"` fails schema validation; `CheckName::IsoLabelsClear.as_str()` is `iso-labels-clear` and `unit` gives `pair` and `pairs`; `CheckName::IsoLinksClear.as_str()` is `iso-links-clear` and `unit` gives `link leg` and `link legs`.
- stencil-render, projection: with zero offset, (100, 0, 0) projects to (86.60254, 50), (0, 100, 0) to (-86.60254, 50) and (0, 0, 18) to (0, -18). For a one-zone document laid out with `FixedMetricsMeasurer`, `canvas.width` equals the formula of section 12.2 rule 5, the drawn body's left and right margins are equal and at least 20 px, and the smallest y equals the `/body` top.
- stencil-render, solids: a filled zone inside a filled zone has base_z 6; a Pcard in that inner zone has base_z 12 and height 18; a Pipe in a Col in the Row of the body has base_z 0 and height 2 × `ISO_TUBE_RADIUS_PX`; a Row produces no solid. A vpc inside a gcp zone is a slab with base_z 6, height 0, four edges and `opaque` false, and a Pcard in it stands at 6.
- stencil-render, zoom: a lone card in a zone of content width is zoomed by `ISO_ZOOM_MAX`, its block height and footprint scale by it, and its label's corners are `ISO_ZOOM_MAX` times as far from the body origin as at zoom 1, because the plane map carries the zoom; a body of any width gets a zoom between 1 and the cap, and a zoom of 1 returns an equal geometry.
- stencil-render, links: a link from a card in one top-level zone to the bottom of a card in another lies at 6 times the zoom throughout and ends on the zoomed route's end point. The same link into the left side ends more than 1 px short of that card. Every hero link lies at one height: the two VLANs and the first service call at 6 times the zoom, the call into the apis slab at 12. The hero's VLAN 1 is one straight leg although its layout route has a jog, and its tag's center lies on it. A jog between facing sides that share a span becomes one leg, and not when a block stands in the way or the sides share nothing; an inner leg 10 px from a zone edge moves to 24 px on the side it came from; a leg in a gap narrower than 48 px stays. The hero's dashed link is one stroked path with a single move and a cone, and its solid primary a tube of polygons with joint circles and no path. A link from a region zone inside a gcp zone to a card in that region ends at 12 and lies at 6 or 12 throughout. A path over two slabs of 6 and a nested one of 12 lies at 12 throughout, one between the first two at 6, and one over the ground between them at 0; it has one `<g data-id>` per geometry node and, inside the group of every node or link that has a label, one `<g data-plane>` whose matrix equals the label's map, written after the faces or the path; no `<marker>`; every g7 pipe group holds at least six `<polygon>` elements (two caps, two body halves, a flange or cone per end) and no `<ellipse>`, and its tag lies at base_z + 2 × `ISO_TUBE_RADIUS_PX`; `text_elements` equals the flat render's; each zone group's first `<polygon>` precedes every descendant group; a vertical pipe's plane group carries `data-axis="y"`.
- stencil-render, sprites: a figure of footprint 36 between 6 and 50 centers a torso of 0.62 of the side with its head top at 50; a laptop's screen stands on the footprint's back edge 4 px deep and a phone's slab across the footprint's middle at half its width; a front panel's first corner is the slab's left edge inset 3 px at the top inset 3 px and its lower corners lie below its upper ones; every sprite outline is convex. Shadows: a figure casts an ellipse, a laptop and a phone a polygon.
- stencil-layout, links: under iso a link from the first to the third item of a Row of a person, a device, a card and a phone is routed and avoids every box; without the stub cut-back it fell back to a straight line through the middle items.
- stencil-render, band: a band pipe with `arrow: both` between two cards has a surface solid of height 0, its tag at base_z, and three `<polygon>` elements in its group, the body and two heads; a band body along x spans 10 px either side of the axis and its head 18 px either side of its base, 24 px behind the tip.
- stencil-render, tubes: a cross-section of radius 5 across an x run projects 24 points whose flat y offset and z offset lie on the circle; a body along x has its lit half above the axis and a silhouette half-width of 5 × √1.5 on screen, and a tube of no length has no body; the far end of a run toward +x or +y faces the viewer and one toward −x or −y does not; a cone toward +x hides its base and one toward −x shows it; a flange straddles its center on the tube axis.
- stencil-render, shading: the five test vectors above; a step of 0 returns the input; `shade("#12345", -8)` and `shade("red", -8)` are None; every color constant in `palette.rs` parses; in `wire` every face polygon's fill is `#FFFFFF` or `none`. Under dusk the gcp floor, the on-prem top and a level 1 region top are lighter than the page, the region lighter than the on-prem top, a card top lighter than both floors, its left face darker than its top and its right face darker than its left, and a solid-bordered slab has a top stroke. In center and dusk the gcp slab's sides are `#1A73E8` and `#1257B3`, unstroked, and its outline is 1.5 px, thinner than a wire.
- stencil-render, links clear: a V of 600 px between ends 100 px apart is a detour, a 6 px inner leg is a jog, and a hand-built leg over the two-zone page's right block is reported with that block unless the block is one of the link's endpoints.
- stencil-render, bundles: two links between the same two blocks, routed along one line, are drawn two cone bases apart along the sides they share, the pair hanging toward the back from the attach point on the right block's hidden left side, with their tags at a quarter and three quarters of the leg; two links that share a start side but not a route move their start legs apart; a second path stacked on the first produces the shared-stretch message and the two shared-end messages, a path 3 px beside it the crowding message, and the check is not applicable under flat and short of two links.
- stencil-render, deny: the on-prem deny pipe's group holds a hollow tube, a hollow cone and the plate's top and face, and no ring; a deny link between two blocks draws its cone and the plate.
- stencil-render, labels: touching pills are a gap defect and pills 4 px apart pass; a link under another owner's tag is reported and under its own is not; a pipe's surface through floor text, a tag nearer another pipe than its own, and a contained label with no block each produce their message; the hero examines 242 pairs and the platform 237.
- stencil-render, check: every hero link end lands on its node (`iso-link-ends` examines 8 and passes), hand-built ends off a block's outline, on its corner, outside a zone and without a solid produce their four messages, and a link from the people figure starts on the figure's hull inside its footprint; the hero passes `iso-labels-clear` with 0 defects; hand-built scenes produce each of the five defect kinds of section 12.7 with its message: two labels overlapping, a mark covered by a later block, a slab edge crossing a mark, a link crossing a mark, and a contained label leaving its block.
- stencil-render, measured JSON: for the hero, `projection.labels` has 14 entries with `id`, `axis`, `corners` and `z` and no `billboards` key, and the whole output is byte-identical under the three themes while the SVG bytes differ.
- stencil-cli: `check examples/hero-iso.json` under `--theme center`, `dusk` and `wire` exits 0 with `check iso-labels-clear: examined 188 pairs, 0 defects`, `check iso-links-clear: examined 11 link legs, 0 defects`, the pipes-land and print-fit not-applicable lines `check iso-link-ends: examined 8 link ends, 0 defects`, `check iso-links-apart: examined 6 link pairs, 0 defects` and `stencil check: 14 checks, 12 passed, 0 failed, 2 not applicable`. With `--projection flat` it prints the two iso not-applicable lines and `12 checks, 8 passed, 0 failed, 4 not applicable`. `render examples/hero-iso.json` under each theme writes three files, and the PNG width is `ceil(projection.canvas.width * 2)`. `render examples/g7.json --projection iso` writes an iso SVG whose measured JSON `nodes` differ from the flat run's (section 12.4 rule 4) on the same canvas width. `vet --projection iso` exits 2.

## 13. Core and grammars

This section splits the vocabulary into a core and grammars. The core is what every figure shares: layout containers, a generic container (`Box`) and a generic named leaf (`Item`), facts, document blocks, pipes, tees, links, lanes, the legend, themes and checks. A grammar is a data module that gives a domain its kinds: which container kinds exist, how each is drawn, where each may sit, which item kinds exist and which icons they carry, and the domain rules that CUE enforces. The Google Cloud vocabulary of sections 1 to 12 becomes the first grammar, `gcp`, and a domain-neutral grammar, `plain`, ships beside it. Themes become data files whose roles are generic, so every theme applies to every grammar, and numbered tint slots replace the color-named kinds.

Every rule in sections 1 to 12 still holds unless this section names the change. Where this section and an earlier one disagree, this section wins, and implementation step (a) of section 13.15 edits the earlier tables and JSON blocks (sections 1.1, 1.2, 1.3, 2.2, 2.4, 2.5, 2.11, 5.2, 9.3, 10, 11.1, 11.4, 12.6 and 12.10) in the same change that migrates the files they describe, so the spec and the repository agree at every commit.

Superseded by this section:

- The `Zone` and `Pcard` tags, the `ZoneKind` and `PipeKind` enums and the Pcard fields `fn`, `pn`, `fact` and `ask` (section 1). `Box`, `Item`, grammar kinds, `line` and `Fact` entries replace them (section 13.1).
- The `Theme` enum of section 11.1 and the dusk and wire palette tables there. A theme is a data file (section 13.4), and `dusk` takes new values (section 13.5).
- The legend label table of section 2.2 and the wire color table of section 5.2. Labels follow section 13.1 rule 7, colors follow the theme.
- The per-kind table of section 2.4. Layout reads padding, radius, border width and label style from the grammar (section 13.2), and colors come from the theme.
- The dusk mixing rules of section 12.6 (`ISO_SURFACE_TARGET`, the lifts and the rim). Every theme shades faces by lightness steps (section 13.4).
- The wire legend relabel of section 12.6 rule 5, which applied under iso only. It now applies in both projections to every theme whose labels differ from the canonical ones (section 13.4, rule 9).
- Section 12.3 rule 7, "an iso SVG writes no `<marker>` and no `<defs>`". An iso SVG still writes no `<marker>`; it writes one `<defs>` when the theme has a block shadow (section 13.11).
- The definition of a word in section 3. Break opportunities are listed in section 13.11.

### 13.1 Core vocabulary

Core node tags: `Row`, `Col`, `Lanes` (section 13.6), `Box`, `Item`, `Fact`, `Note`, `Text`, `Callout`, `Frame`, `Pipe` and `Tee`. `Link` and `LegendEntry` are page-level objects, as before. `Zone` becomes `Box` and `Pcard` becomes `Item`; `Note` is unchanged.

| Retired | Core replacement |
|---|---|
| `Zone` with `kind` from `ZoneKind` | `Box` with `kind` from the page's grammar, plus `tint` |
| `Pcard` | `Item` with `kind` from the grammar (`product` in gcp) |
| Pcard `fn`, `pn` | Item `title`, `subtitle` |
| Pcard `fact`, `ask` | Item `facts`: a list of Fact entries, each with a `source` of `doc`, `built` or `ask` |
| `kind` on Pipe, Tee, Link and LegendEntry (`PipeKind`) | `line`: `solid`, `dash`, `deny` or `gray`, plus `tint` |
| `blue`, `pink` | `"line": "solid"` with `"tint": 1` or `2` |
| `region-a`, `region-b`, `onprem-a`, `onprem-b` | `"kind": "region"` or `"onprem"` with `"tint": 1` or `2` |

Field summary additions and changes (section 1.1):

| Tag | Field | Type | Required | Default |
|---|---|---|---|---|
| Page | grammar | `gcp`, `plain` or a path ending in `.json` | no | `gcp` |
| Page | theme | a built-in theme name or a path ending in `.json` | no | `center` |
| Page | theme_overrides | object, a partial theme (section 13.4 rule 8) | no | absent |
| Page | chrome | `full` or `none` | no | `full` |
| Box | kind | a container kind of the grammar | yes | |
| Box | tint | integer 1 to 8 | no | the kind's default (rule 2) |
| Box | label, children | as Zone | yes | |
| Item | kind | an item kind of the grammar | yes | |
| Item | icon | IconName, from the kind's icon pack | no | absent |
| Item | title | text | yes | |
| Item | subtitle | text | no | absent |
| Item | facts | list of Fact entries, 0 to 8 | no | empty |
| Fact (node and entry) | text | text | yes | |
| Fact (node and entry) | source | `doc`, `built` or `ask` | no | `doc` |
| Pipe | line | Line | yes | |
| Pipe | tint | integer 1 to 8 | no | rule 2 |
| Pipe | from, to | node id | no | absent |
| Tee | line | Line | yes | |
| Tee | tint | integer 1 to 8 | no | rule 2 |
| Link | line | Line | yes | |
| Link | tint | integer 1 to 8 | no | rule 2 |
| Link | order | integer 1 to 256 | no | absent (section 13.6) |
| LegendEntry | line | Line | yes | |
| LegendEntry | tint | integer 1 to 8 | no | rule 2 |
| LegendEntry | form | PipeForm (section 12.7) | no | `tube` |

Line: `gray`, `solid`, `dash`, `deny`.

Rust types. These replace their section 1.2 and 11.2 definitions in `crates/stencil-model/src/document.rs`; types not shown keep their definitions.

```rust
pub const TINT_SLOTS: u8 = 8;
/// The slot names of the built-in center theme, in slot order. Layout measures every
/// legend label with these names (rule 7), so geometry never depends on the theme.
pub const TINT_NAMES: [&str; 8] = [
    "blue", "pink", "teal", "amber", "violet", "green", "orange", "cyan",
];
pub const FACTS_MAX: usize = 8;
pub const LANES_MAX: usize = 32;
pub const LINK_ORDER_MAX: u16 = 256;
/// A grammar kind name: a Box or Item `kind`.
pub const KIND_PATTERN: &str = r"^[a-z][a-z0-9-]{0,31}$";
pub const BUILTIN_GRAMMARS: [&str; 2] = ["gcp", "plain"];
/// Designed built-ins first (section 13.5), then the imported tier (section 13.9).
pub const BUILTIN_THEMES: [&str; 13] = [
    "center", "paper", "dusk", "clear", "clear-dark", "wire",
    "tokyo-night", "solarized-light", "solarized-dark", "material-dark",
    "gruvbox-dark", "dracula", "nord",
];
pub const GRAMMAR_REFERENCE_PATTERN: &str = r"^(gcp|plain|[^\u0000-\u001F]{1,395}\.json)$";
pub const THEME_REFERENCE_PATTERN: &str = r"^(center|paper|dusk|clear|clear-dark|wire|tokyo-night|solarized-light|solarized-dark|material-dark|gruvbox-dark|dracula|nord|[^\u0000-\u001F]{1,395}\.json)$";
/// Largest grammar or theme file the CLI reads, in bytes.
pub const DATA_FILE_BYTES_MAX: usize = 65_536;

fn is_default_chrome(chrome: &Chrome) -> bool {
    *chrome == Chrome::Full
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[schemars(length(min = 1, max = 400))]
    pub title: String,
    #[schemars(length(min = 1, max = 400))]
    pub kicker: String,
    #[schemars(length(min = 1, max = 400))]
    pub lede: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub foot: Option<String>,
    #[serde(default = "page_width_default")]
    #[schemars(range(min = 640, max = 2560))]
    pub width: u32,
    pub canvas: Canvas,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = GRAMMAR_REFERENCE_PATTERN))]
    pub grammar: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = THEME_REFERENCE_PATTERN))]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_overrides: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "is_default_projection")]
    pub projection: Projection,
    #[serde(default, skip_serializing_if = "is_default_chrome")]
    pub chrome: Chrome,
    #[schemars(length(min = 1, max = 256))]
    pub body: Vec<Node>,
    #[schemars(length(max = 16))]
    pub legend: Vec<LegendEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 256))]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Chrome {
    #[default]
    Full,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tag")]
pub enum Node {
    Row(Row),
    Col(Col),
    Lanes(Lanes),
    Box(BoxNode),
    Item(Item),
    Fact(Fact),
    Note(Note),
    Pipe(Pipe),
    Tee(Tee),
    Text(Text),
    Callout(Callout),
    Frame(Frame),
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Line {
    Gray,
    Solid,
    Dash,
    Deny,
}

/// Any container. `kind` names one of the grammar's container kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoxNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

/// Any named leaf. `kind` names one of the grammar's item kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Item {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconName>,
    #[schemars(length(min = 1, max = 400))]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 8))]
    pub facts: Vec<FactEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum FactSource {
    /// Read from the live documentation when the figure was authored.
    #[default]
    Doc,
    /// An as-built name read off the running system: a bucket, a VLAN ID, a project id.
    Built,
    /// An open question for the reader.
    Ask,
}

fn is_default_fact_source(source: &FactSource) -> bool {
    *source == FactSource::Doc
}

/// A fact inside an Item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactEntry {
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
    #[serde(default, skip_serializing_if = "is_default_fact_source")]
    pub source: FactSource,
}

/// The Fact node: a fact standing on its own in a container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
    #[serde(default, skip_serializing_if = "is_default_fact_source")]
    pub source: FactSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pipe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    pub dir: PipeDir,
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub sub: Option<String>,
    #[serde(
        default = "pipe_arrow_default",
        skip_serializing_if = "is_pipe_arrow_default"
    )]
    pub arrow: Arrow,
    /// Under iso, a round tube on the pipe's plane (the default) or a band, a flat wide
    /// arrow lying on it (section 12.3 rule 5).
    #[serde(default, skip_serializing_if = "PipeForm::is_tube")]
    pub form: PipeForm,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub to: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum PipeForm {
    #[default]
    Tube,
    Band,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tee {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub hub: String,
    pub arms: [TeeArm; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub sub: Option<String>,
    #[serde(default)]
    pub arrow: Arrow,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_side: Option<Side>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_side: Option<Side>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 8))]
    pub via: Vec<PagePoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 256))]
    pub order: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegendEntry {
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

/// The tint a Box is painted with (rule 2); `None` for a kind that takes no tint.
pub fn box_tint(kind: &ContainerKind, tint: Option<u8>) -> Option<u8>;
/// The tint a line is painted with: a solid or dash line's tint or slot 1, none for gray
/// and deny.
pub fn line_tint(line: Line, tint: Option<u8>) -> Option<u8>;
/// The output keys of rule 6.
pub fn box_key(kind: &str, tint: Option<u8>) -> String;
pub fn line_key(line: Line, tint: Option<u8>) -> &'static str;
/// The canonical legend label of rule 7, for example "Solid blue".
pub fn legend_label(line: Line, tint: Option<u8>) -> &'static str;
```

`Line::ALL` and `Line::as_str` replace the `PipeKind` ones. `Node::tag_name` returns `Box` and `Item` for the new variants. The Rust type is `BoxNode` so the standard library's `Box` stays unshadowed; the serialized tag is `Box`.

Rules:

1. `tint` is optional on Box, Pipe (Tee arms included), Tee, Link and LegendEntry. Vet rejects a value outside 1 to 8 with rule `tint-out-of-range`, pointer `/…/tint`, message `tint <n> is outside 1 to 8`. serde reads any `u8`, so 0 and 9 to 255 parse and reach vet, as `gap` does.
2. Effective tint. A Box takes its own `tint`, or the default tint its container kind declares (section 13.2), when the kind is tintable; a kind that is not tintable ignores `tint`. A `solid` or `dash` line (Pipe, Tee spine, Link, LegendEntry) without `tint` is slot 1. `tint` on a `gray` or `deny` line has no effect. `box_tint` and `line_tint` return None for an ignored tint, the renderer ignores it, and legend consistency keys the line without it; the core CUE schema reports such a field as `_tintWithoutEffect` so an author learns it does nothing.
3. A dash line takes its slot's wire color, as center's dash took the blue wire. The dash pattern tells it apart from the solid line of the same slot.
4. Fact entries. An Item's `facts` are drawn in list order under its title and subtitle (section 13.7). The standalone `Fact` node takes the same `source`, and a Fact node without one is a `doc` fact drawn exactly as section 2.6 draws a Fact today.
5. Output vocabulary. The SVG, the measured JSON and the check messages keep the identifiers they use today, so the identity proofs of section 13.14 are a plain byte comparison and every consumer of the measured JSON keeps working: `NodeTag` keeps `Zone` for a Box and `Pcard` for an Item (`data-tag` and the measured `tag`), the title and subtitle runs keep the part names `FunctionName` and `ProductName`, and a Box or line writes its key (rule 6) as `data-kind` and the measured `kind`. Renaming the output vocabulary to the core names is a separate later change with its own fixture regeneration; it is listed as an open item.
6. Keys. The key of a Box is its kind, followed by `-a` to `-h` for effective tint 1 to 8 when the kind is tintable, so gcp's `region` with tint 1 is `region-a` and `onprem` without tint is `onprem`. The key of a line keeps the pre-section-13 names for slots 1 and 2:

   | Line | Effective tint | Key |
   |---|---|---|
   | gray, deny | none | `gray`, `deny` |
   | solid | 1, 2 | `blue`, `pink` |
   | solid | 3 to 8 | `solid-3` to `solid-8` |
   | dash | 1 | `dash` |
   | dash | 2 to 8 | `dash-2` to `dash-8` |

   Arrow marker ids are `arrow-<theme name>-<line key>`. The keys are identifiers, not color claims: `blue` under the paper theme is drawn in paper's slot 1 wire.
7. Canonical legend labels. Layout measures the LegendLabel run of each legend entry with the canonical label of its (line, effective tint), in every theme: `Solid gray` for gray, `Dashed red` for deny, `Solid <name>` for solid and `Dashed <name>` for dash, where `<name>` is `TINT_NAMES[tint - 1]`. For slots 1 and 2 these are the section 2.2 labels (`Solid blue`, `Solid pink`, `Dashed blue`), so center geometry does not move. What the renderer draws in place of the canonical label is section 13.4, rule 9.
8. Legend consistency (section 6) keys every use and every legend entry on (line, effective tint, form): a Pipe or Tee arm with `form: band` and a legend entry with `form: band` key on the band, every other use on the tube. A used key with no entry, an entry whose key is never used, and a key listed twice are the three defects, with messages that name the key in document terms, for example `Pipe line solid tint 2 has no legend entry` or `Pipe line solid tint 4 form band has no legend entry`. The core CUE schema appends `-band` to such a key. The examined count is unchanged: one per use plus one per examined legend entry. Section 13.7 adds one exception, for figures with `chrome: none`.
9. `NodeGeometry.kind` becomes `Option<String>` and holds the key of rule 6; `NodeGeometry` gains `tint: Option<u8>`, the effective tint, which the measured JSON does not write (rule 5). `LinkRoute` gains `line: Line` in place of `kind` and `tint: Option<u8>`.

Vet rules added to section 1.3. `validate_page` takes the resolved grammar: `validate_page(page: &Page, grammar: &Grammar) -> Vec<Violation>`.

| Rule | Condition | Pointer | Message |
|---|---|---|---|
| `tint-out-of-range` | `tint` is 0 or above 8 | `/…/tint` | `tint <n> is outside 1 to 8` |
| `grammar-unknown` | `grammar` is neither a `BUILTIN_GRAMMARS` name nor a string ending in `.json` | `/grammar` | `grammar "<value>" is neither a built-in grammar nor a .json path` |
| `theme-unknown` | `theme` is neither a `BUILTIN_THEMES` name nor a string ending in `.json` | `/theme` | `theme "<value>" is neither a built-in theme nor a .json path` |
| `kind-unknown` | a Box kind is not a container kind of the grammar, or an Item kind not an item kind | `/…/kind` | `kind "<kind>" is not a <container or item> kind of grammar <name>` |
| `kind-parent-not-allowed` | the nearest Box ancestor's kind, or `page` when there is none, is not in the kind's `parents` (Row, Col and Lanes are transparent) | `/…/kind` | `<kind> cannot sit in <parent kind>` |
| `icon-outside-pack` | an Item carries `icon` and its kind's icon pack is `none` | `/…/icon` | `item kind <kind> takes no icon` |
| `pipe-target-unknown` | a Pipe `from` or `to` names no node id | `/…/from` or `/…/to` | `pipe target "<id>" names no node id` |
| `pipe-targets-equal` | a Pipe's `from` and `to` are the same id | `/…/to` | `pipe from and to both name "<id>"` |
| `pipe-target-on-tee-arm` | a Tee arm carries `from` or `to` | `/…/arms/<i>/from` or `/…/arms/<i>/to` | `a Tee arm cannot name a target` |
| `facts-too-many` | an Item has more than 8 facts | `/…/facts` | `<n> facts, above 8` |
| `lanes-too-many` | a Lanes node has more than 32 children | `/…/children` | `<n> lanes, above 32` |
| `link-order-outside-lanes` | a Link with `order` does not join two different children of one Lanes node | `/links/<i>/order` | `an ordered link joins two lanes of one Lanes node` |
| `link-order-duplicate` | two ordered links of one Lanes node share an `order` | `/links/<j>/order` | `order <n> is already used by /links/<i>` |
| `lanes-in-iso` | the page has a Lanes node and `projection` is `iso` | `/projection` | `a page with Lanes cannot be drawn in iso` |

`VetRule` gains one variant per row, with `as_str` as in the first column. Because the grammar a page names is known only after parsing, `parse_page(json_text)` becomes serde parsing alone, and callers run `validate_page(&page, &grammar)` once `page.grammar` is resolved; `pipeline::load_document` does both, and `grammar-unknown` is checked before resolution. Pipe ids join the id namespace of section 11.2, so `id-duplicate` covers them. `kind-parent-not-allowed` follows the section 1.3 walk bounds; it reads the nearest Box ancestor from `NodeEntry` parents.

Document order (section 4.2): an Item gives `title`, `subtitle`, then each fact's `text`; a Page gives `title`, `kicker`, `lede`, `foot`, `width`, `canvas`, `grammar`, `theme`, `theme_overrides`, `projection`, `chrome`, `body`, `legend`, `links`. `text_fields` includes every fact entry, at `/…/facts/<i>/text`.

Layout. A Box reads everything layout needs from its container kind: the role (a `frame` lays out as the section 2.4 gcp zone with bar and body; every other role as the non-gcp zone), the border width, padding, radius and label style. An Item lays out as the section 2.5 Pcard. Neither reads the tint or the theme, so geometry depends on the document and the grammar alone. A drawn border width never changes a box: layout reserves the grammar's width whatever the theme draws, as the wire theme already does.

### 13.2 Grammars

A grammar is a data module. Its source is a CUE file under `cue/grammars/`, which declares a `grammar` value against `#Grammar` and adds the domain's rules in CUE. The data part is exported to JSON (`cue export ./cue/grammars:<name> -e grammar --out json`) and committed as `crates/stencil-model/grammars/<name>.json`, which the Rust side embeds and reads; Rust never evaluates CUE (section 9.2). `cue/check.sh` re-exports every built-in grammar and fails when the export differs from the committed JSON, as it does for g7.

`Page.grammar` names a built-in grammar or a `.json` grammar file (an export of a CUE grammar), resolved like a theme reference (section 13.4 rule 1). Absent means `gcp`, so every existing figure keeps its meaning. A grammar given by path carries its data only; its CUE rules apply when the figure is authored in CUE against that grammar's package.

The renderer draws roles, never kinds. Each container kind declares one of four roles. The `frame` role names the outermost container, the gcp frame; it is unrelated to the `Frame` node tag of section 11.3, the wireframe placeholder, whose theme role is called `placeholder` for that reason. Every theme paints roles and tones (section 13.4), so a theme needs no knowledge of a grammar's kinds:

| Role | Drawn as | Layout |
|---|---|---|
| frame | the outermost system or cloud: a filled bar holding the label over a filled body, with a border | section 2.4's gcp construction: bar and body parts, padding 0 on the frame |
| boundary | a network, trust or security edge: the border in the kind's pattern, filled with its tone when the tone has a fill; under iso a boundary with no drawn fill is a ring (section 12.3) | section 2.4's non-gcp construction |
| group | a locality, cluster or site: filled with its tint when tinted, else with its tone | as boundary |
| tile | an ownership scope (project, folder, account): filled with its tone | as boundary |

Group and tile draw alike in every built-in theme; the role tells the grammar's rules and a future theme which containers scope ownership and which gather things in a place.

`#Grammar` lives in `cue/grammar.cue`:

```cue
package stencil

import (
	"list"
	"strings"
)

#KindName: =~"^[a-z][a-z0-9-]{0,31}$"
#Tone:     "neutral" | "warm" | "cool" | "soft" | "strong" | "highlight" | "emphasis" | "accent"

#Grammar: {
	name: #KindName
	containers: [...#ContainerKind] & list.MinItems(1) & list.MaxItems(32)
	items: [...#ItemKind] & list.MinItems(1) & list.MaxItems(32)
	// Literals no text field may carry, checked by remembered-constants.
	remembered: [...#Remembered] & list.MaxItems(64)
}

#ContainerKind: {
	name: #KindName
	role: "frame" | "boundary" | "group" | "tile"
	// Every role but frame names a tone; a frame paints from the theme's frame role.
	tone?:    #Tone
	tintable: bool
	// The slot a tintable kind takes when the Box sets none; absent leaves it untinted.
	default_tint?: int & >=1 & <=8
	border: {
		pattern: "solid" | "dashed" | "dotted" | "none"
		// Layout reserves this width; 0 exactly when pattern is none.
		width: number & >=0 & <=4
	}
	padding: number & >=0 & <=32
	radius:  number & >=0 & <=16
	// plain: zone_label; accent: perimeter_label; bar: gcp_bar, frame only.
	label: "plain" | "accent" | "bar"
	// Container kinds a Box of this kind may sit in, or "page" for the top level.
	parents: [...#KindName] & list.MinItems(1)
}

#ItemKind: {
	name: #KindName
	// The bundled icon pack the kind draws from; none takes no icon.
	icons: "gcp" | "none"
	// The icon-to-name table of icon-matches-product; empty when icons is none.
	products: [...#IconProducts]
	parents: [...#KindName] & list.MinItems(1)
}

#IconProducts: {
	icon:  #Icon
	class: "product" | "category"
	names: [...string & !="" & strings.MaxRunes(64)] & list.MinItems(1)
}

#Remembered: {
	literal: string & !="" & strings.MaxRunes(64)
	reason:  string & !="" & strings.MaxRunes(400)
}
```

The Rust mirror, in `crates/stencil-model/src/grammar.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Grammar {
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub name: String,
    #[schemars(length(min = 1, max = 32))]
    pub containers: Vec<ContainerKind>,
    #[schemars(length(min = 1, max = 32))]
    pub items: Vec<ItemKind>,
    #[schemars(length(max = 64))]
    pub remembered: Vec<Remembered>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Frame,
    Boundary,
    Group,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Neutral,
    Warm,
    Cool,
    Soft,
    Strong,
    Highlight,
    Emphasis,
    Accent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BorderPattern {
    Solid,
    Dashed,
    Dotted,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KindBorder {
    pub pattern: BorderPattern,
    #[schemars(range(min = 0.0, max = 4.0))]
    pub width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LabelStyle {
    Plain,
    Accent,
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContainerKind {
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub name: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
    pub tintable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub default_tint: Option<u8>,
    pub border: KindBorder,
    #[schemars(range(min = 0.0, max = 32.0))]
    pub padding: f32,
    #[schemars(range(min = 0.0, max = 16.0))]
    pub radius: f32,
    pub label: LabelStyle,
    #[schemars(length(min = 1))]
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IconPack {
    Gcp,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IconClass {
    /// From the archive's Unique Icons: stands for one product.
    Product,
    /// From the archive's Category Icons: stands for a product family.
    Category,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IconProducts {
    pub icon: IconName,
    pub class: IconClass,
    #[schemars(length(min = 1))]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemKind {
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub name: String,
    pub icons: IconPack,
    pub products: Vec<IconProducts>,
    #[schemars(length(min = 1))]
    pub parents: Vec<String>,
    /// Under iso, the solid an item of this kind stands as when neither the item nor its
    /// icon's row names one; absent is card.
    pub shape: Option<Shape>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Remembered {
    #[schemars(length(min = 1, max = 64))]
    pub literal: String,
    #[schemars(length(min = 1, max = 400))]
    pub reason: String,
}

/// The embedded built-in of that name, parsed and validated.
pub fn builtin_grammar(name: &str) -> Option<Result<Grammar, GrammarError>>;
/// serde_json parse followed by validate_grammar.
pub fn parse_grammar(json_text: &str, origin: &str) -> Result<Grammar, GrammarError>;
pub fn validate_grammar(grammar: &Grammar) -> Vec<GrammarViolation>;
pub fn grammar_schema() -> schemars::Schema;

impl Grammar {
    pub fn container(&self, kind: &str) -> Option<&ContainerKind>;
    pub fn item(&self, kind: &str) -> Option<&ItemKind>;
}
```

`GrammarViolation` and `GrammarError` have the shape of the theme ones in section 13.4 (pointer into the grammar document, a `GrammarRule`, a message; `Json` and `Invalid` variants with an `origin` naming the file or built-in).

Grammar rules, in `validate_grammar`, all reported in field order: container and item names are unique across both lists (`grammar-name-duplicate`); every parent is a container kind of the grammar or `page` (`grammar-parent-unknown`); at least one kind lists `page` (`grammar-no-top-level`); a `frame` kind has no tone and label `bar`, and every other kind has a tone and a label other than `bar` (`grammar-role-mismatch`); `default_tint` only on a tintable kind (`grammar-default-tint-untintable`); border width 0 exactly when the pattern is `none` (`grammar-border-mismatch`); an item kind with icons `none` has no products, and within one item kind each icon appears at most once (`grammar-icon-table`); `remembered` literals are distinct (`grammar-remembered-duplicate`). A grammar file that fails stops `vet`, `render`, `check` and `gallery` before layout, printed and mapped to exit codes exactly as a theme failure is (section 13.4 rule 5).

What a grammar decides and what it does not:

- Vet enforces the grammar's kind lists and nesting (`kind-unknown`, `kind-parent-not-allowed`, `icon-outside-pack`, section 13.1) in Rust, so a JSON figure gets them without CUE.
- Layout reads role, border width, padding, radius and label style from the kind.
- The renderer reads role, tone, tintability and border pattern from the kind and every color from the theme.
- `remembered-constants` scans text fields for the grammar's `remembered` literals, and is not applicable (reason `grammar has no remembered constants`) when the list is empty. `icon-matches-product` reads the item kinds' `products` tables (section 13.10).
- The domain rules that need more than a list (tint pairing, products outside a VPC, the hop-fact rule) live in the grammar's CUE file and run under `cue vet`, as the CUE rules of section 9.2 always have.

### 13.3 Built-in grammars

gcp. The vocabulary of sections 1 to 12 as a grammar. Its container kinds reproduce section 2.4 exactly, so every gcp figure lays out as before:

| Kind | Role | Tone | Tint | Border | Padding | Radius | Label | Parents |
|---|---|---|---|---|---|---|---|---|
| gcp | frame | | no | solid 3 | 0 | 10 | bar | page |
| vpc | boundary | strong | no | dashed 2 | 10 | 8 | plain | gcp, project, perimeter |
| region | group | neutral | yes, default 1 | solid 1.5 | 12 | 8 | plain | gcp, vpc, perimeter, project |
| subnet | group | cool | no | dashed 1.5 | 12 | 8 | plain | region, vpc, project |
| onprem | group | warm | yes, no default | solid 1.5 | 12 | 8 | plain | page, optional |
| project | tile | highlight | no | solid 1.5 | 12 | 8 | plain | page, gcp, vpc, perimeter, project |
| optional | group | emphasis | no | dashed 2 | 12 | 8 | plain | page, gcp, vpc, region, subnet, project, perimeter |
| k8s | group | soft | no | none 0 | 12 | 8 | plain | gcp, vpc, region, subnet, project, perimeter, optional |
| perimeter | boundary | accent | no | dashed 2.5 | 12 | 10 | accent | gcp, vpc, project |
| apis | group | neutral | no | solid 1.5 | 12 | 8 | plain | gcp, project, perimeter |

The parents include every nesting in today's examples (page holds gcp, onprem and project; gcp holds vpc, project and perimeter; vpc holds region, project and perimeter; perimeter holds region; project holds subnet, optional and k8s) and the nestings the prime layout topic describes. subnet keeps its dashed border: center draws it dashed today and the identity proof holds center to it; wire's dotted subnet was a wire-only choice and goes (section 13.5). k8s keeps no border and its pink-tint fill comes from the `soft` tone, not from slot 2, so a k8s Box is not tinted and stays outside the pairing rule, as `#TintOf` had it.

gcp has three item kinds. `product`: icons `gcp`, shape block, parents every container kind and `page`, and the 23-row products table of section 13.10. `person` (shape figure) and `device` (shape laptop): icons `none`, an empty table, the same parents. Its `remembered` list is the four literals of section 6.

`cue/grammars/gcp.cue` carries the gcp rules, every one a vet-time defect under `cue vet`:

1. Remembered constants: the four literals stay in `#NoRememberedConstant` in `core.cue`, because the core `#Text` applies it inside every node and a grammar package cannot reach into it. CUE therefore rejects them under every grammar, while the Rust check reads the grammar's `remembered` list and is not applicable under plain; a grammar with its own literals adds a CUE rule over its text fields.
2. The hop-fact rule: every `product` item carries a `subtitle` or a fact whose source is `doc` or `ask` (`_itemsWithoutSubtitleOrFact`, which replaces `_pcardsWithoutPnFactOrAsk`). A `built` fact does not satisfy it: as-built names need no live doc, and they say nothing about what the product is.
3. Tint pairing, moved from color names to slot numbers: a `solid` pipe beside or inside a tinted Box (a `region`, or an `onprem` with a tint) of another slot is the defect it was for blue and pink (`_pipeBesideZoneOfOtherTint`, `_otherTintInsideZone`). `dash`, `gray` and `deny` stay outside the rule, as `dash` was before: the g7 failover pipe sits between Region A and Region B.
4. Products outside a VPC: a `product` item whose `subtitle` names Cloud Storage, BigQuery, Pub/Sub, Artifact Registry or Cloud Logging at word boundaries, or whose `icon` is `cloud-storage` or `bigquery`, is a defect when a `vpc` Box is among its ancestors: `_productInsideVpc.<title>: "product" & "sits inside a vpc; draw it in an apis box"`. These are Google APIs reached over Private Google Access, never addresses in a VPC. A serverless product, whose `subtitle` names Cloud Run, Cloud Run functions, Cloud Functions, App Engine or Vertex AI or whose `icon` is `cloud-run`, `vertex-ai` or `serverless`, runs outside the customer's VPC and is reached over Private Service Connect, an internal load balancer or Direct VPC egress: inside a `vpc` it is `_serverlessInsideVpc.<title>`, "sits inside a vpc; a serverless product is reached over Private Service Connect; draw it beside the vpc". A managed data product with a private IP, whose `subtitle` names Cloud SQL, AlloyDB or Memorystore or whose `icon` is `cloud-sql`, lives in a Google-managed producer network that the customer's VPC reaches over private services access peering or a Private Service Connect endpoint: inside a `vpc` it is `_managedDataInsideVpc.<title>`, "sits inside a vpc; a managed data product lives in a producer network reached over private services access or Private Service Connect; draw it beside the vpc and name the attachment". A global resource, whose `subtitle` names Network Connectivity Center, has no region (Cloud Armor and Cloud DNS also have regional policies and zones, so they are not listed): inside a `region` Box it is `_globalInsideRegion.<title>`, "sits inside a region; a global resource sits outside every region".
5. The `apis` kind holds them: a Box for Google APIs reached over Private Google Access, drawn inside the gcp frame and outside every vpc. Its parents keep it out of a vpc in Rust vet already (`kind-parent-not-allowed`); the CUE rule `_apisOutsideGcp` adds that it has a `gcp` Box among its ancestors, which a parents list cannot say when a project or perimeter sits between.

Step (d) of section 13.15 corrects two examples that broke rule 4: `hero-iso.json` drew Warehouse (icon `bigquery`) inside the Shared VPC ring, and `hybrid-ai.json` drew the feature store (`BigQuery · vectors`), the checkpoint bucket (`Cloud Storage · dual-region`), the audit logs (`Cloud Logging · org sink`) and cost and usage (`Billing export · BigQuery`) inside its vpc. Each moves into an `apis` Box outside the vpc inside the gcp frame: in hybrid-ai the feature store and the checkpoint bucket sit in one beside the vpc within the service perimeter, and the audit logs and cost and usage in one inside the evidence project, which sits beside the perimeter. `cue/check.sh` vets every example against the gcp `#Page`, so the rules examine them.

plain. A domain-neutral grammar for system diagrams, with no icons required and no domain rules beyond the core:

| Kind | Role | Tone | Tint | Border | Padding | Radius | Label | Parents |
|---|---|---|---|---|---|---|---|---|
| system | frame | | no | solid 3 | 0 | 10 | bar | page |
| boundary | boundary | strong | no | dashed 2 | 10 | 8 | plain | page, system, group, tile |
| group | group | neutral | yes, no default | solid 1.5 | 12 | 8 | plain | page, system, boundary, group, tile |
| tile | tile | highlight | no | solid 1.5 | 12 | 8 | plain | page, system, boundary, tile |

Item kinds `service`, `store`, `external`, `person` and `device`, each with icons `none`, an empty products table and every container kind and `page` as parents; `service`, `store` and `external` have shape block, `person` figure and `device` laptop. `remembered` is empty, so remembered-constants is not applicable under plain, and so is icon-matches-product. `cue/grammars/plain.cue` declares the data and no extra rules.

Other domains, sketched to show the model carries them. None ships in this section; each would be a CUE file and an exported JSON like gcp and plain.

`org` (an organization chart):

| Declares | Value |
|---|---|
| containers | `company` frame; `division` tile (tone highlight); `team` group (tone neutral, tintable, so teams pair with tinted reporting lines) |
| items | `person` and `role`, icons none |
| lines | solid for reporting, dash for dotted-line reporting |
| CUE rules | a person sits in exactly one team; every team has one item marked lead (a fact with source doc naming the role) |

`onprem-network`:

| Declares | Value |
|---|---|
| containers | `site` frame; `zone` boundary (tone strong, a firewall zone); `vlan` group (tintable, default 1); `rack` tile |
| items | `router`, `switch`, `firewall`, `server`, `storage`, icons none (a network icon pack would be a new bundled pack) |
| lines | solid tinted per VLAN, deny for a blocked path, dash for a backup path |
| CUE rules | a pipe between two vlan boxes passes a firewall item or carries a deny line; VLAN ids appear as built facts |

`sequence` (uses Lanes, section 13.6):

| Declares | Value |
|---|---|
| containers | `system` frame around the lanes; `group` boundary around several lane heads |
| items | `actor`, `service`, `store`, icons none |
| lines | ordered links: solid for a call, dash for a reply, deny for a rejected call |
| CUE rules | every ordered link's `to` is a lane head; a reply follows the call it answers in `order`; each lane head is an item |

`c4` (C4 model, container level):

| Declares | Value |
|---|---|
| containers | `software-system` frame; `boundary` boundary (enterprise boundary); `container-group` group |
| items | `person`, `container`, `database`, `external-system`, icons none; the C4 element text maps to title (name), subtitle (technology) and a doc fact (description) |
| lines | solid for synchronous calls, dash for asynchronous, each labeled with the protocol in `sub` |
| CUE rules | every item has a subtitle (its technology); external systems sit outside the software-system frame |

### 13.4 Themes as data

A theme is a JSON document that sets every role the renderer reads: surfaces, inks, the container frame and eight container tones, the tag, fact and ask boxes, the four line kinds with their patterns and end dots, drawn stroke widths, the icon chip, the isometric face shading and slab thickness, the block shadow and the eight tint slots. No drawing code names a color literal; `stencil-render` reads every paint from a `Theme` value. A theme never changes layout, type sizes or icons.

Theme roles are generic. A theme knows roles and tones, never a grammar's kinds, so every theme applies to every grammar: the grammar maps each container kind to a role and a tone (section 13.2), and the theme paints the role and tone. For gcp the mapping is the table in section 13.3; read through center it gives exactly the colors of section 2.4:

| gcp kind | Role, tone | Center fill | Center border |
|---|---|---|---|
| gcp | frame | `frame.body_fill` `#FAFBFC`, bar `#1A73E8` | `frame.border` `#1A73E8` |
| vpc | boundary, strong | none | `#5F6368` |
| region | group, neutral, tinted | the slot's fill (`#D2E3FC`, `#FCE4EC`) | `#BDC1C6` |
| subnet | group, cool | `#EDE7F6` | `#9AA0A6` |
| onprem | group, warm, tinted when set | the slot's fill, or `#EFEBE9` untinted | `#D7CCC8` |
| project | tile, highlight | `#FFF8E1` | `#FFE082` |
| optional | group, emphasis | `#F8FBFF` | `#4284F3` |
| k8s | group, soft | `#FCE4EC` | none |
| perimeter | boundary, accent | `#FFFBF5`, label `#B06000` | `#E37400` |
| apis | group, neutral | `#EDF3F0` | `#BDC1C6` |

A container's paint, in every theme: the fill is the slot fill of its effective tint when it has one, else its tone's fill, else none; the border color is its tone's border (a frame's is `frame.border`), drawn in the grammar's pattern when the pattern is not `none`; the label ink is the slot ink when tinted, else the tone's `label_ink` when set, else `ink.zone_label`, and a frame's label is `frame.bar_ink`. A tinted group therefore keeps its tone's border, as center's region keeps `#BDC1C6` whatever its slot; the slot's `border` is used where a container has a tint and its tone has no border.

`#Theme` lives in `cue/theme.cue`. It is closed at every level, so an unknown role is a vet error. A theme file vets with `cue vet -c -d '#Theme' ./cue <file>.json`.

```cue
package stencil

import "list"

// Uppercase #RRGGBB.
#Color:   =~"^#[0-9A-F]{6}$"
#Pattern: "solid" | "dashed" | "dotted"
#Dot:     "filled" | "hollow" | "none"
// A drawn stroke width in px. Layout reserves the grammar's widths whatever is drawn.
#Width: number & >=0.5 & <=4
// HSL lightness step of an isometric face, in percentage points.
#Step:    int & >=-40 & <=40
#Opacity: number & >=0 & <=1

#Stroke: {
	color:   #Color
	width:   #Width
	pattern: #Pattern
}

#Swatch: {
	fill: #Color
	ink:  #Color
}

#Accent: {
	accent: #Color
	fill:   #Color
}

#ToneRole: {
	fill?:      #Color
	border?:    #Color
	label_ink?: #Color
}

#Tint: {
	name:   =~"^[a-z]{1,12}$"
	fill:   #Color
	border: #Color
	ink:    #Color
	wire:   #Color
}

#Theme: {
	name: =~"^[a-z][a-z0-9-]{0,31}$"
	// color: tints are told apart by color, legend labels name the color, and the
	// separation thresholds apply. line: tints are told apart by end dots and pattern,
	// legend labels name the line, and the separation thresholds do not apply.
	tint_cue: "color" | "line"
	page:     #Color
	ink: {
		primary:    #Color // title, item title, block body, list bullets
		secondary:  #Color // lede, item subtitle
		zone_label: #Color // an untinted container's label, a Frame block's label
	}
	kicker: #Color
	badge: {
		customer: #Swatch
		internal: #Swatch
		border?:  #Stroke
	}
	card: {
		fill:   #Color
		border: #Stroke
	}
	fact: #Swatch
	ask:  #Swatch
	tag: {
		fill:    #Color
		border:  #Stroke
		ink:     #Color
		sub_ink: #Color
	}
	legend: {
		label_ink: #Color
		text_ink:  #Color // legend text and Note kind legend
	}
	foot: #Color
	callout: {
		note:     #Accent
		risk:     #Accent
		decision: #Accent
		open:     #Accent
	}
	// The Frame block of section 11.3, a wireframe placeholder.
	placeholder: {
		border:   #Color
		diagonal: #Color
	}
	// The flat chip under every icon; absent draws none.
	icon_chip?: #Color
	// The frame role.
	frame: {
		border:     #Color
		frame_fill: #Color
		bar_fill:   #Color
		bar_ink:    #Color
		bar_rule?:  #Stroke
		body_fill:  #Color
	}
	tones: {
		neutral:   #ToneRole
		warm:      #ToneRole
		cool:      #ToneRole
		soft:      #ToneRole
		// strong never fills: a container of this tone is a ring under iso.
		strong: {
			border?:    #Color
			label_ink?: #Color
		}
		highlight: #ToneRole
		emphasis:  #ToneRole
		accent:    #ToneRole
	}
	containers: {
		// Draw every container border but a frame's at this width; absent draws the
		// grammar's width.
		draw_width?:       #Width
		frame_draw_width?: #Width
		// A border for containers whose grammar pattern is none; absent draws none.
		borderless_outline?: #Stroke
	}
	lanes: lifeline: #Stroke
	gray: {
		color:   #Color
		width:   #Width
		pattern: #Pattern
		dot:     #Dot
	}
	// Solid and dash lines take their color from the tint slot's wire.
	solid: {
		width: #Width
		dots:  [...#Dot] & list.MinItems(8) & list.MaxItems(8)
	}
	dash: {
		width:   #Width
		pattern: #Pattern
		dot:     #Dot
	}
	deny: {
		color:      #Color
		width:      #Width
		pattern:    #Pattern
		dot:        #Dot
		tag_border: #Color
		tag_ink:    #Color
	}
	tints: [...#Tint] & list.MinItems(8) & list.MaxItems(8)
	iso: {
		faces: {
			top:   #Step
			left:  #Step
			right: #Step
		}
		slab_thickness: number & >=2 & <=16
		// The frame slab's side faces; absent shades the frame body fill like any slab.
		frame_sides?: {
			left:  #Color
			right: #Color
		}
		frame_outline: #Width
		// none: a solid container border is not drawn on the slab, the shaded sides carry
		// the edge. outline: drawn at edge_width on the top face and the sides.
		solid_edges: "none" | "outline"
		edge_width:  #Width
		// A ring (a container with no drawn fill); absent draws its flat border.
		ring?: #Stroke
		// The outline of a block whose flat drawing has no border; absent draws none.
		block_outline?: #Stroke
		labels: {
			frame: #Color
			zone:  #Color
		}
		chip: {
			fill:  #Color
			ring?: #Color
			shadow?: {
				color:   #Color
				opacity: #Opacity
				dy:      number & >=0 & <=4
			}
		}
		// A soft shadow under every opaque block (section 13.11); absent draws none.
		shadow?: {
			color:   #Color
			opacity: #Opacity
			blur:    number & >0 & <=8
			dy:      number & >=0 & <=8
		}
		widths: {
			primary:   #Width // solid slot 1
			secondary: #Width // solid slots 2 to 8, dash, deny
			gray:      #Width
		}
	}
}
```

The Rust mirror lives in `crates/stencil-model/src/theme.rs`, re-exported from the crate root. `schema/theme.schema.json` is generated from it by schemars, committed, and compared in a test as `schema/stencil.schema.json` is. Field declaration order is the CUE order above, so a serialized theme lists its roles in that order. Every struct carries `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]` and `#[serde(deny_unknown_fields)]`, every enum `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]` and `#[serde(rename_all = "lowercase")]`, and every `Option` field `#[serde(default, skip_serializing_if = "Option::is_none")]`; the range attributes follow the CUE bounds.

```rust
pub const COLOR_PATTERN: &str = r"^#[0-9A-F]{6}$";
pub const THEME_NAME_PATTERN: &str = r"^[a-z][a-z0-9-]{0,31}$";
pub const TINT_NAME_PATTERN: &str = r"^[a-z]{1,12}$";
/// The widest a drawn legend label may run past its canonical label (rule 9): the
/// legend's column gap, so a relabeled entry never reaches the next one.
pub const LEGEND_RELABEL_SLACK_PX: f32 = 16.0;

#[serde(transparent)]
pub struct Color(#[schemars(regex(pattern = COLOR_PATTERN))] pub String);

pub enum TintCue { Color, Line }
pub enum LinePattern { Solid, Dashed, Dotted }
pub enum DotStyle { Filled, Hollow, None }
pub enum SolidEdges { None, Outline }

pub struct ThemeStroke { pub color: Color, pub width: f32, pub pattern: LinePattern }
pub struct Swatch { pub fill: Color, pub ink: Color }
pub struct Accent { pub accent: Color, pub fill: Color }
pub struct ToneRole { pub fill: Option<Color>, pub border: Option<Color>, pub label_ink: Option<Color> }
pub struct Tint {
    #[schemars(regex(pattern = TINT_NAME_PATTERN))]
    pub name: String,
    pub fill: Color,
    pub border: Color,
    pub ink: Color,
    pub wire: Color,
}

pub struct Inks { pub primary: Color, pub secondary: Color, pub zone_label: Color }
pub struct BadgeRole { pub customer: Swatch, pub internal: Swatch, pub border: Option<ThemeStroke> }
pub struct CardRole { pub fill: Color, pub border: ThemeStroke }
pub struct TagRole { pub fill: Color, pub border: ThemeStroke, pub ink: Color, pub sub_ink: Color }
pub struct LegendRole { pub label_ink: Color, pub text_ink: Color }
pub struct CalloutRole { pub note: Accent, pub risk: Accent, pub decision: Accent, pub open: Accent }
pub struct PlaceholderRole { pub border: Color, pub diagonal: Color }
pub struct FrameRole {
    pub border: Color,
    pub frame_fill: Color,
    pub bar_fill: Color,
    pub bar_ink: Color,
    pub bar_rule: Option<ThemeStroke>,
    pub body_fill: Color,
}
pub struct Tones {
    pub neutral: ToneRole,
    pub warm: ToneRole,
    pub cool: ToneRole,
    pub soft: ToneRole,
    pub strong: ToneRole,
    pub highlight: ToneRole,
    pub emphasis: ToneRole,
    pub accent: ToneRole,
}
pub struct ContainerRole {
    pub draw_width: Option<f32>,
    pub frame_draw_width: Option<f32>,
    pub borderless_outline: Option<ThemeStroke>,
}
pub struct LanesRole { pub lifeline: ThemeStroke }
pub struct GrayRole { pub color: Color, pub width: f32, pub pattern: LinePattern, pub dot: DotStyle }
pub struct SolidRole { pub width: f32, pub dots: [DotStyle; 8] }
pub struct DashRole { pub width: f32, pub pattern: LinePattern, pub dot: DotStyle }
pub struct DenyRole {
    pub color: Color,
    pub width: f32,
    pub pattern: LinePattern,
    pub dot: DotStyle,
    pub tag_border: Color,
    pub tag_ink: Color,
}
pub struct FaceSteps { pub top: i8, pub left: i8, pub right: i8 }
pub struct FrameSides { pub left: Color, pub right: Color }
pub struct LabelInks { pub frame: Color, pub zone: Color }
pub struct ChipShadow { pub color: Color, pub opacity: f32, pub dy: f32 }
pub struct ChipRole { pub fill: Color, pub ring: Option<Color>, pub shadow: Option<ChipShadow> }
pub struct BlockShadow { pub color: Color, pub opacity: f32, pub blur: f32, pub dy: f32 }
pub struct IsoWidths { pub primary: f32, pub secondary: f32, pub gray: f32 }
pub struct IsoRole {
    pub faces: FaceSteps,
    pub slab_thickness: f32,
    pub frame_sides: Option<FrameSides>,
    pub frame_outline: f32,
    pub solid_edges: SolidEdges,
    pub edge_width: f32,
    pub ring: Option<ThemeStroke>,
    pub block_outline: Option<ThemeStroke>,
    pub labels: LabelInks,
    pub chip: ChipRole,
    pub shadow: Option<BlockShadow>,
    pub widths: IsoWidths,
}

pub struct Theme {
    #[schemars(regex(pattern = THEME_NAME_PATTERN))]
    pub name: String,
    pub tint_cue: TintCue,
    pub page: Color,
    pub ink: Inks,
    pub kicker: Color,
    pub badge: BadgeRole,
    pub card: CardRole,
    pub fact: Swatch,
    pub ask: Swatch,
    pub tag: TagRole,
    pub legend: LegendRole,
    pub foot: Color,
    pub callout: CalloutRole,
    pub placeholder: PlaceholderRole,
    pub icon_chip: Option<Color>,
    pub frame: FrameRole,
    pub tones: Tones,
    pub containers: ContainerRole,
    pub lanes: LanesRole,
    pub gray: GrayRole,
    pub solid: SolidRole,
    pub dash: DashRole,
    pub deny: DenyRole,
    pub tints: [Tint; 8],
    pub iso: IsoRole,
}

impl Tones {
    pub fn get(&self, tone: Tone) -> &ToneRole;
}

/// serde_json parse followed by validate_theme. `origin` names the file or built-in.
pub fn parse_theme(json_text: &str, origin: &str) -> Result<Theme, ThemeError>;
/// Structural rules serde does not express (rule 3). Empty means loadable.
pub fn validate_theme(theme: &Theme) -> Vec<ThemeViolation>;
/// Deep-merges `overrides` onto `base` (rule 8), then parses and validates the result.
pub fn apply_overrides(
    base: &Theme,
    overrides: &serde_json::Map<String, serde_json::Value>,
) -> Result<Theme, ThemeError>;
/// The contrast and separation rows of rule 10, in row order.
pub fn theme_quality(theme: &Theme) -> ThemeReport;
pub fn theme_schema() -> schemars::Schema;

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeViolation {
    /// RFC 6901 pointer into the theme document, or into `/theme_overrides` of the page.
    pub pointer: NodePointer,
    pub rule: ThemeRule,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeRule {
    ColorMalformed,
    WidthOutOfRange,
    StepOutOfRange,
    OpacityOutOfRange,
    SlabThicknessOutOfRange,
    ShadowOutOfRange,
    NameMalformed,
    TintNameMalformed,
    TintNameDuplicate,
    StrongToneFilled,
    OverrideNotObject,
    LegendLabelTooWide,
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("theme {origin} is not valid theme JSON at line {line}, column {column}: {message}")]
    Json { origin: String, line: usize, column: usize, message: String },
    #[error("theme {origin} violates {} rule(s)", .violations.len())]
    Invalid { origin: String, violations: Vec<ThemeViolation> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeReport {
    pub theme: String,
    pub rows: Vec<QualityRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualityRow {
    pub class: QualityClass,
    /// For example "ink.primary on page" or "wires deuteranopia".
    pub subject: String,
    /// The contrast ratio or the minimum delta E; None when not applicable.
    pub value: Option<f64>,
    pub threshold: f64,
    /// For a separation row, the closest pair of slot names.
    pub closest: Option<(String, String)>,
    pub not_applicable: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityClass {
    Contrast,
    Separation,
}
```

Rules:

1. A theme reference is resolved once per run, before layout. A `BUILTIN_THEMES` name selects the embedded file of that name (sections 13.5 and 13.9). Any other value ends in `.json` (vet guarantees it) and is a path: `--theme` resolves against the current directory, `Page.theme` against the directory of the input document. The CLI reads at most `DATA_FILE_BYTES_MAX` bytes, the same bounded read as `read_input`. `--theme` overrides `Page.theme`, and an absent reference is `center`.
2. Built-in themes are embedded with `include_str!` from `crates/stencil-render/themes/<name>.json` and `crates/stencil-render/themes/imported/<name>.json`, and `stencil_render::themes::builtin_theme(name) -> Option<Result<Theme, ThemeError>>` parses one on demand. Library code never unwraps the result; a test parses and validates every built-in, and `cue/check.sh` vets every built-in file against `#Theme`, so a broken built-in fails CI, never a user's run.
3. Structural rules, in `validate_theme`, return every violation in field order: every `Color` matches `COLOR_PATTERN` (`theme-color-malformed`); every width is 0.5 to 4 (`theme-width-out-of-range`); every face step is -40 to 40; every opacity is 0 to 1; `slab_thickness` is 2 to 16; a block shadow's `blur` is above 0 and at most 8 and its `dy` 0 to 8; `name` matches `THEME_NAME_PATTERN`; each tint name matches `TINT_NAME_PATTERN` and the eight names are distinct; `tones.strong` sets no fill (`theme-strong-tone-filled`). `ThemeRule::as_str` gives the kebab-case names with a `theme-` prefix.
4. The legend label width is the one structural rule that needs a measurer, so it lives in stencil-layout: `pub fn theme_legend_labels(theme: &Theme, measurer: &mut dyn TextMeasurer) -> Result<Vec<ThemeViolation>, MeasureError>` measures, in the `legend_label` style, every label rule 9 draws for this theme and the canonical label of the same (line, tint), and reports each drawn label wider than its canonical label by more than `LEGEND_RELABEL_SLACK_PX`, at `/tints/<i>/name` or at `/tint_cue`, with rule `theme-legend-label-too-wide`. The CLI runs it on every theme it loads, with `CosmicTextMeasurer`.
5. A theme that fails a structural rule, or a file that is not valid theme JSON, stops `vet`, `render`, `check` and `gallery` before layout. The CLI prints `error <ThemeError display>` followed by one `violation <rule> <pointer>: <message>` line per violation and `stencil <command>: checks not run`, and exits 1: the theme file is authored input like the document. An unreadable theme file (missing, a directory, over the byte limit) exits 2, as an unreadable input does.
6. Quality rows (rule 10) never gate `render` or `check`. They are reported by `stencil theme check` and enforced on the built-ins by tests (section 13.14). A user may render with a theme that fails contrast; the theme check tells them so.
7. Validation runs in both places. `cue vet -c -d '#Theme'` checks a theme file against the closed definition, and the Rust loader checks the same structure through serde (`deny_unknown_fields` at every level, fixed-length `[Tint; 8]` and `[DotStyle; 8]`) and `validate_theme`. Where the two disagree, the Rust types win and `cue/theme.cue` is updated, as for the document schema.
8. Overrides. `Page.theme_overrides` is a JSON object merged onto the resolved base theme before validation. Objects merge key by key, recursively; a string, number, boolean or array replaces the base value. `tints` is the one exception: in an override it is an object whose keys are slot numbers `"1"` to `"8"`, and each value merges into that slot, so `{"tints": {"3": {"wire": "#00796B"}}}` recolors one wire. The merge descends only where the base holds an object, so it is bounded by the theme's depth (4). The merged JSON is then parsed with `parse_theme`, so an unknown key, a wrong type or a bad value is a `ThemeError` whose pointers start with `/theme_overrides`. A non-object value where the base holds an object is `theme-override-not-object`. The CUE side types `theme_overrides` as an open struct of `_` and leaves the check to the loader, because a partial of a closed definition would duplicate `#Theme` field by field.
9. Legend wording. The renderer draws each legend label from the theme. Under `tint_cue: color` a label is `<Pattern> <name>`: `Solid gray` for gray, `<Pattern> red` for deny, `Solid <tint name>` for solid and `<Pattern> <tint name>` for dash, where `<Pattern>` is `Solid`, `Dashed` or `Dotted` from the role's pattern. Under `tint_cue: line` a label names the line: `Thin line` for gray, `Solid line`, `Ringed line` or `Plain line` for a solid slot whose dot is filled, hollow or none, `Dashed line` for dash, and `<Pattern> line` for deny. When the drawn label differs from the canonical label of section 13.1 rule 7, the writer measures the drawn label in the label run's style and moves the description by the change in width, as section 12.6 rule 5 did for wire under iso; the gap between label and description keeps its size. Rule 4 bounds the change, so a relabeled entry never reaches the next entry and the last entry of a row stays inside the page's 20 px padding. For center every drawn label equals its canonical label, and no entry moves.
10. Quality rows, computed by `theme_quality` in row order:

    | Class | Subject | Threshold |
    |---|---|---|
    | contrast | text on its ground (WCAG 2.2 contrast ratio): `ink.primary`, `ink.secondary`, `ink.zone_label`, `kicker`, `foot`, `legend.label_ink` and `legend.text_ink` on `page`; `ink.primary` and `ink.secondary` on `card.fill`; `tag.ink`, `tag.sub_ink` and `deny.tag_ink` on `tag.fill`; `fact.ink` on `fact.fill`; `ask.ink` on `ask.fill`; each badge ink on its fill; `frame.bar_ink` on `frame.bar_fill`; `ink.zone_label` on `frame.body_fill` and on every tone fill the theme sets; each tone `label_ink` on its tone's fill; each tint ink on its fill; `ink.primary` on each callout fill; each iso tab ink on its fill | 4.5 |
    | contrast | non-text on its ground: the gray wire, the deny wire and each of the eight tint wires against `page`, `frame.body_fill`, every tone fill the theme sets and every tint fill; `frame.border` against `page`; each callout accent against its fill | 3.0 |
    | separation | minimum pairwise CIE76 delta E over the eight tint wires, under normal vision and under deuteranopia, protanopia and tritanopia (Machado, Oliveira and Fernandes 2009, severity 1.0, applied in linear sRGB, clipped to 0 to 1, then CIELAB D65) | 20 normal, 12 each dichromacy |
    | separation | the same over the eight tint fills | 8 normal, 5 each dichromacy |

    The separation rows are not applicable under `tint_cue: line` (reason `tints are told apart by line`): that theme draws every slot in one ink by design, and its slots differ by end dot. A theme that sets no tone fill still examines its page, card and tint rows, so no theme examines zero rows. The checker of the palette research (`accessible/scripts/palette-check.py`) is the reference for the numbers; a test compares `theme_quality` with recorded values from it for center and dusk to 2 decimals.
11. Rendering reads the theme through `Palette`, which holds the resolved `Theme` and the projection: `Palette::new(theme: &Theme, projection: Projection) -> Palette<'_>`. Every method returns colors borrowed from the theme. `shade`, `mix`, `Face` and `FacePaint` keep their section 12.6 definitions; `face_lightness_step` reads `iso.faces`, `slab_faces` and `block_faces` follow section 12.6 with every constant replaced by its role (`iso.frame_sides`, `iso.frame_outline`, `iso.solid_edges` and `iso.edge_width`, `iso.ring`, `iso.block_outline`) and every gcp rule applied to the frame role; `slab_faces` takes the container's resolved paint (fill, border color, the grammar's pattern) instead of a `ZoneKind`, and `iso_wire_width` gives `iso.widths.primary` for solid slot 1, `iso.widths.gray` for gray and `iso.widths.secondary` for everything else. The constants of section 12.6 (`ISO_PRIMARY_WIRE_PX` and the rest) are removed from `palette.rs`; their values live in the theme files.
12. Isometric slab thickness is a theme role, so the drawn iso canvas depends on it. `ISO_SLAB_THICKNESS_PX` is removed; `project_page` takes the thickness and the ring flags: `project_page(geometry: &PageGeometry, solids: &SolidInputs)`, where `SolidInputs` holds the slab thickness and, per Box, whether it is a ring. A Box is a ring (section 12.3's vpc) when its tone is `strong` and it has no effective tint. The `strong` tone never has a fill (rule 3), so which Boxes are rings depends on the document and the grammar, never on the theme. Every designed built-in uses 6. The section 11.1 and 12.8 invariance is restated: the measured JSON `canvas`, `nodes` and `links` are byte-identical under every theme, and `projection` is byte-identical under every theme with the same slab thickness. The iso link arrowhead lengths of section 12.3 rule 7 stay theme-independent and key on the line key: 18 for `blue`, 15 for every dash key, 14 for every other key.
13. Text runs keep their center color from `stencil_layout::styles` in `TextRun.color`, as today; the writer replaces it with the theme ink. A test asserts that the center theme's inks equal `stencil_layout::styles`, so the two never drift.

### 13.5 Built-in themes

Six designed themes ship as data files under `crates/stencil-render/themes/`, and seven imported ones under `themes/imported/` (section 13.9). Every one applies to every grammar. The designed six, in the gallery's order:

| Name | File | Source | Use for |
|---|---|---|---|
| center | `center.json` | today's center values, plus the new roles below | customer slides and documents on white; the default |
| paper | `paper.json` | editorial-light from the palette research | print and long documents: warm paper, one saturated blue |
| dusk | `dusk.json` | dusk-deep from the palette research, surface ladder widened | dark slides and screens |
| clear | `clear.json` | accessible-light from the palette research | audiences with color-vision deficiency, light |
| clear-dark | `clear-dark.json` | accessible-dark from the palette research | the same, dark |
| wire | `wire.json` | today's wire values | design docs and reviews: one ink, kinds told apart by line |

The palette research is the `stencil-palettes-2026-09-30` run: five candidate palettes, each checked against the thresholds of section 13.4 rule 10 by its own checker and by an independent one. The judges' findings are folded in here: dusk-deep's surface ladder was too tight and is widened below; material-tonal and catppuccin-nord are not adopted; the accessible dark palette's warm slots (ochre, vermillion, olive) lean brown, which is accepted because they pass every threshold and the alternative searched by the research gave up worst-case separation for hue.

Mapping a research palette onto `#Theme`. Every research role maps to one theme path; the roles the research does not have are derived by the rules in the second table, and the same rules derive them for every research palette.

| Research key | Theme path |
|---|---|
| `page_bg` | `page`; `frame.frame_fill` |
| `ink` | `ink.primary`; `legend.label_ink` |
| `ink_secondary` | `ink.secondary`; `ink.zone_label` |
| `line` | `placeholder.diagonal` |
| `card_fill`, `card_border` | `card.fill`, `card.border.color` |
| `tag_fill`, `tag_border`, `tag_ink`, `tag_sub_ink` | `tag.fill`, `tag.border.color`, `tag.ink`, `tag.sub_ink` |
| `fact_fill`, `fact_ink` | `fact.fill`, `fact.ink` |
| `badge_fill`, `badge_ink` | `badge.customer` |
| `kicker_ink`, `legend_ink`, `foot_ink` | `kicker`, `legend.text_ink`, `foot` |
| `gcp_bar_fill`, `gcp_bar_ink`, `gcp_frame_border`, `gcp_body_fill` | `frame.bar_fill`, `frame.bar_ink`, `frame.border`, `frame.body_fill` |
| `vpc_border` | `tones.strong.border`; `placeholder.border` |
| `apis_fill`, `apis_border` | `tones.neutral` |
| `onprem_fill`, `onprem_border` | `tones.warm` |
| `subnet_fill`, `subnet_border` | `tones.cool` |
| `k8s_fill` | `tones.soft.fill` (no border) |
| `project_fill`, `project_border` | `tones.highlight` |
| `optional_fill`, `optional_border` | `tones.emphasis` |
| `perimeter_fill`, `perimeter_border`, `perimeter_ink` | `tones.accent` fill, border and `label_ink` |
| `wire_gray` | `gray.color` |
| `wire_dash` | not mapped: a dash line draws its slot's wire (section 13.1 rule 3). It equals slot 1 in editorial-light and accessible; dusk-deep's gray-blue dash (`#8F98BD`) is dropped so a dashed slot 1 line matches its solid partner, as in center |
| `wire_deny`, `deny_tag_ink`, `deny_tag_border` | `deny.color`, `deny.tag_ink`, `deny.tag_border` |
| `icon_chip` | `icon_chip` in light themes; dark themes use `#FFFFFF`, because the chip exists so that the unaltered Google icons, several of them dark gray on transparent, stay visible on a dark card (section 11.1), and the research's dark chips defeat that |
| `tints` | `tints`, names lowercased, hex uppercased |

The research keys are gcp kind names because the research was run against the gcp vocabulary; the tone each maps to is the one the gcp grammar gives that kind (section 13.3), so a research palette drawn through gcp gives the colors its authors checked, and the same theme draws plain and every other grammar through the same tones.

| Derived role | Rule |
|---|---|
| widths and patterns | center's: card and tag borders 1.5, every line 2 px; gray and solid lines solid, dash and deny dashed; every dot filled; no `containers` overrides (container widths and patterns come from the grammar) |
| `lanes.lifeline` | `ink.secondary`, 1 px, dashed |
| `ask` | slot 4: its fill and ink |
| `badge.internal` | slot 5. When the customer badge is filled with slot 1's wire (dusk), the internal badge is slot 5's wire with the page as ink; otherwise slot 5's fill and ink |
| `callout` | note: slot 1 wire on slot 1 fill; risk: the deny wire on `mix(page, deny wire, r)`; decision: slot 6 wire on slot 6 fill; open: slot 4 wire on slot 4 fill; r is 0.22 on a light page and 0.18 on a dark one (L* of `page` below 50) |
| `iso.faces` | top 0, left -8, right -16 on a light page (center's steps); 0, -4, -8 on a dark page (the steps section 12.6 gave dusk) |
| `iso.slab_thickness` | 6 |
| `iso.frame_sides` | left `frame.border`, right that color shaded -12, as center's `#1A73E8` and `#1257B3` |
| `iso.frame_outline`, `solid_edges`, `edge_width` | 1.5, `none`, 1.5 |
| `iso.labels` | frame: `frame.bar_fill` on a light page, the slot 1 tint ink on a dark one; zone: `ink.secondary` on a light page, `ink.primary` on a dark one |
| `iso.chip` | light page: `#FFFFFF` ringed in `card.border.color`, with the `ink.primary` shadow at 0.18 opacity 1.5 px down; dark page: `#FFFFFF`, no ring, no shadow |
| `iso.shadow` | light page: `ink.primary` at 0.18 opacity, blur 2.5, dy 3; dark page: `#000000` at 0.5, blur 3, dy 3 |
| `iso.widths` | primary 3.75, secondary 2.5, gray 2 |
| `tint_cue` | `color` |

Center. Every value of today's center palette carries over unchanged: the colors of sections 2.4 to 2.9, 5.2 and 11.3 (section 2.4's per-kind colors through the tones, as the table of section 13.4 shows), the iso values of section 12.6 and `palette.rs` (faces 0, -8, -16; frame sides `#1A73E8` and `#1257B3`; frame outline 1.5; solid edges `none`; tabs `#1A73E8` and `#5F6368` with white ink; chip `#FFFFFF` ringed `#DADCE0` with the `#202124` shadow at 0.18 and 1.5 px; widths 3.75, 2.5 and 2), and no `icon_chip`, so center draws no flat chip. Slots 1 and 2 are today's tints: blue fill `#D2E3FC`, wire `#1A73E8`; pink fill `#FCE4EC`, wire `#C2185B`; both with border `#BDC1C6` and ink `#5F6368`. New center values, needed because the roles are new:

| Role | Value | How it was chosen |
|---|---|---|
| slot 3 teal | fill `#E1FCFD`, border `#BDC1C6`, ink `#004845`, wire `#007D78` | wires and inks from editorial-light slots 3 to 8; fills fit by a bounded search against the separation thresholds with slots 1 and 2 held at today's values |
| slot 4 amber | fill `#FAF9CA`, border `#BDC1C6`, ink `#5A3900`, wire `#A36E14` | as above |
| slot 5 violet | fill `#EFEBFF`, border `#BDC1C6`, ink `#4B3275`, wire `#593894` | as above |
| slot 6 green | fill `#E5F7E3`, border `#BDC1C6`, ink `#26481B`, wire `#497938` | as above |
| slot 7 orange | fill `#FCDBCD`, border `#BDC1C6`, ink `#6E2B12`, wire `#9D3E1B` | as above |
| slot 8 cyan | fill `#BFDDE8`, border `#BDC1C6`, ink `#18445E`, wire `#005276` | as above |
| `tones.warm.fill` | `#EFEBE9` | an untinted onprem had no fill before; the pale of the on-prem border hue, zone label 5.11:1 |
| `tones.neutral.fill` | `#EDF3F0` | a region is always tinted, so the neutral fill shows only on apis (new) and plain groups; a cool green-gray apart from the frame body `#FAFBFC`, zone label 5.38:1 |
| `lanes.lifeline` | `#9AA0A6`, 1 px, dashed | the frame and subnet gray |
| `iso.shadow` | `#202124` at 0.2, blur 2.5, dy 3 | added in step (e) |

With these, center examines 223 quality rows and passes 222. The one failing row is `ask.ink on ask.fill`, `#B06000` on `#FEF7E0` at 4.34:1. Changing either color would change center output, which section 13.14 holds byte-identical, so the row stays failed and the center test names it as the only expected failure. The separation minima are wires 29.55 normal and 13.16 worst case (deuteranopia), fills 8.53 and 5.38.

Paper takes editorial-light as mapped, with tint names `blue`, `rose`, `teal`, `ochre`, `violet`, `green`, `clay`, `petrol`. Its page is `#FCFBF8`, frame body `#F7F6F2`, card `#FFFFFF`.

Dusk takes dusk-deep as mapped, with the surface ladder widened. The research ladder put page, frame body, the tone fills and the card within 8 L* of each other (page 4.4, body 7.5, zones 6.4 to 10.9, card 12.5), so nested surfaces read as one. Each rung now keeps the research hue and chroma (CIELAB a and b) at a new L*, at least 4 above the rung below:

| Rung | Roles | Research | Dusk | L* |
|---|---|---|---|---|
| page | `page` | `#0D0F17` | `#0D0F17` | 4.4 |
| frame body | `frame.body_fill` | `#131623` | `#161926` | 9.0 |
| tones | `tones.cool`, `warm`, `highlight`, `emphasis`, `soft`, `accent` and `neutral` fills (research subnet, onprem, project, optional, k8s, perimeter, apis) | `#171B2A`, `#18181F`, `#151928`, `#11141E`, `#141E2B`, `#1C1812`, `#141C26` | `#1E2231`, `#222229`, `#1E2232`, `#1F222D`, `#192331`, `#26221C`, `#1B232E` | 13.4 to 13.5 |
| card | `card.fill`, `tag.fill` | `#1B2031`, `#191E2F` | `#272B3D` | 17.9 |
| fact box | `fact.fill` | `#1F2640` | `#2E3450` | 22.4 |

The fact box moves with the card so it stays above the card it sits in. The tint fills (L* 17 to 25) are governed by the separation rows, not the ladder. With the widened ladder dusk passes all 223 rows; its lowest non-text contrast is 3.16 (the gray wire on the slot 1 fill). The face steps shade every surface darker than its own top, and the ladder keeps each top lighter than the one it stands on, which section 12.6 rule 4 achieved by mixing toward a blue gray; the mix, the lifts and the rim are no longer needed.

Clear and clear-dark take the accessible palettes, with the slots reordered so each slot keeps its hue family across themes: blue, pink, teal, ochre, indigo, green, vermillion, olive (the research order was blue, pink, teal, vermillion, indigo, ochre, olive, green). Separation is a minimum over all pairs, so the order does not change any row. The vermillion slot is named `orange`, because `Solid vermillion` would exceed the relabel slack of section 13.4 rule 4. Both pass all 223 rows.

Wire carries today's values: page and every fill `#FFFFFF`, ink `#222222`, secondary ink `#555555`; `containers.draw_width` 1.25 and `frame_draw_width` 2, with a 2 px bar rule; every tone border `#222222`, every tone fill `#FFFFFF` where center's tone has one; `borderless_outline` `#222222` at 1.25 solid, so a k8s box keeps the outline wire draws today; gray 1.25 solid, every other line 2 px, dash dashed, deny dotted; `tint_cue: line`; every tint slot fill `#FFFFFF`, border and wire `#222222`, ink `#555555`, with the canonical names; solid dots by slot filled, hollow, none, filled, hollow, none, filled, hollow, so slot 1 draws as today's blue and slot 2 as today's pink, and wire tells at most three solid slots apart; lifeline `#555555` 1 px dashed; iso faces 0, 0, 0 (every face takes its flat fill, which is the page white), solid edges `outline` at 1.5, ring dotted `#999999` at 1.5, block outline `#222222` at 1.25, tabs `#222222` with white ink, chip white ringed `#222222`, widths 3.75, 2.5 and 1.25, no shadow. Wire passes its 215 contrast rows, and its 8 separation rows are not applicable. Two wire outputs change. Its flat legend labels read `Solid line` and so on in place of `Solid blue` (section 13.4 rule 9), which closes the first open item of section 11.1. Its subnet border is dashed like every other dashed container, because the pattern now comes from the grammar; today's wire drew subnet dotted.

### 13.6 Lanes

`Lanes` is a core container for figures that read along time: columns that share one vertical axis, with the messages between them drawn as horizontal arrows in order. It is what a sequence grammar is built on (section 13.3).

```rust
pub const LANE_GAP_DEFAULT_PX: u16 = 32;
/// Smallest height of one message row in the band.
pub const LANE_ROW_MIN_PX: f32 = 36.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lanes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 64))]
    pub gap: Option<u16>,
    /// The lane heads, left to right: 1 to 32 nodes, usually Items.
    #[schemars(length(min = 1, max = 32))]
    pub children: Vec<Node>,
}
```

1. Each child of a Lanes node is a lane head. A Link with `order` whose `from` and `to` are two different heads of the same Lanes node is a message of that node; vet rejects any other ordered link (`link-order-outside-lanes`) and two messages of one node with the same order (`link-order-duplicate`).
2. Layout. A Lanes node is a flex column with two parts, `Heads` and `Band`. `Heads` is a flex row with gap `gap` (default 32) whose children are the heads, each with weight 1 as in a Row with no `grow`, so the lanes are equal columns. `Band` is a leaf below it, as wide as the node, whose height is the sum of the message rows: messages are sorted by `order`, then by link index, and row k is `max(LANE_ROW_MIN_PX, tag height + 8)` tall, where the tag is sized from the link's `label` and `sub` as a link tag is (section 11.2); a message without a label has no tag and its row is `LANE_ROW_MIN_PX` tall. A Lanes node with no message has a band of height 0.
3. Messages are routed by layout, not by the A* router. Message k is a two-point `LinkRoute` from the center x of its `from` head to the center x of its `to` head, at y = band top + the heights of rows 0 to k-1 + half of row k, with status `Routed`. Its tag is centered on the segment's midpoint, as section 11.2 places a link tag. `arrow` places the arrowheads as for any link.
4. Lifelines. Each head draws a lifeline: part `Lifeline` of the Lanes node, repeated once per head in head order (`lifeline/<i>` in the measured JSON), a vertical line from the head's bottom center to the band's bottom, in `lanes.lifeline`. Lifelines are not obstacles, so links-avoid-boxes ignores a message crossing them.
5. Section 2.11 gains the row: Lanes, parts Heads, Band and Lifeline per head, none carrying a TextRun. `NodeTag` gains `Lanes`. Lanes is transparent for grammar nesting, like Row and Col, and a column container for pipes (section 2.7).
6. Checks count messages as links: links-routed examines each (always `Routed`), links-avoid-boxes examines each segment against the obstacles of section 11.2, and legend consistency counts each as a use. text-fits-box examines each message tag's runs.
7. A page with a Lanes node cannot be iso (`lanes-in-iso`): the projection has no rule for a time axis.

### 13.7 Facts with a source, and figure chrome

A fact carries its `source`. `doc` (the default) is a value read from the live documentation when the figure was authored, the hop-fact value of earlier sections. `built` is an as-built name read off the running system: a bucket name, a VLAN ID, a project id; it carries no live-doc requirement. `ask` is an open question for the reader, as Pcard `ask` was.

Layout and paint by source, for each entry of an Item's `facts` in list order after the title and subtitle, and for a Fact node:

| Source | Item parts | Box | Text style and prefix | Paint |
|---|---|---|---|---|
| doc | `FactBox`, `Fact` | margin-top 4, padding 4/8, radius 4, filled | `fact`, no prefix | `fact.fill`, `fact.ink` |
| built | `BuiltBox`, `Built` | the same, unfilled | `fact`, prefix `• ` (U+2022 and a space) | no fill, `fact.ink` |
| ask | `AskBox`, `Ask` | the same, filled | `ask`, prefix `Ask: ` | `ask.fill`, `ask.ink` |

The prefix is part of the measured and rendered text, as `Ask: ` was. The bullet and the unfilled box tell an as-built line from a fact, since the bundled faces have no monospace. A Fact node keeps its one `Text` part and takes the box paint, style and prefix of its source. Section 2.11's Item row (the Pcard row) becomes: [Icon], Text, FunctionName, [ProductName], then the parts of each fact entry, with FunctionName, ProductName and each Fact, Built and Ask run carrying a `TextRun`. In the measured JSON the first entry of each source uses the bare part names (`fact_box`, `fact`, `built_box`, `built`, `ask_box`, `ask`) and every later entry of that source adds `/<i>`, its index in `facts`, as `body_line/<i>` does; a migrated Pcard has at most one doc fact and one ask, so its keys do not change. Under iso every fact part lies on the block top with the icon, in its flat layout (section 12.4). `PartName` gains `BuiltBox` and `Built`. The stencil-text coverage test adds U+2022 in the SemiBold face.

The hop-fact rule (a product on a hop carries a fact read from the live doc, or an explicit ask) is a gcp grammar rule (section 13.3, rule 2) and counts `doc` and `ask` facts only. Remembered constants scan every fact entry like every text field.

`Page.chrome` is `full` (default) or `none`. A figure captioned by the document it sits in sets `none`. Then:

1. Layout omits the badge, `/kicker`, `/title` and `/lede`: the root's children are `/body`, then `/legend` when the legend is not empty, then `/foot` when set. The geometry order of section 4.4 drops the three pointers, and the body starts at the root padding.
2. `title`, `kicker` and `lede` stay required. They label the figure in the gallery index and in `gallery.json`, remembered constants scan them, and they are not drawn. A captioned figure that wants none of them still writes a one-word title.
3. A figure with `chrome: none` whose uses (every Pipe, Tee arm, Tee spine and Link) share exactly one (line, effective tint) may have an empty legend. Legend consistency then reports the uses as examined and clean: examined is the number of uses, no defect, so the report passes. With two or more keys, or with any legend entry, section 13.1 rule 8 applies unchanged. A `chrome: full` figure keeps the full rule: every figure that names itself also names its lines.

### 13.8 Pipes aimed at named nodes

`Pipe.from` and `Pipe.to` name nodes by id. They tell layout which boxes the pipe joins, so a gutter can line up with what it connects without the matching grow lists of section 9.4.

1. The slot of a Pipe is its parent when the parent is a Row or Col whose children are all Pipes or Tees, and otherwise the Pipe itself. The move axis is the Pipe's cross axis: y for `h`, x for `v`.
2. The aim span of a Pipe with targets is the intersection of the named nodes' extents on the move axis (attach boxes: an item's footprint when it stands on one under iso, else its border box, so a pipe aims at the solid it lands on and not at its floor text), or the one named node's extent when only one is set. When the two extents do not overlap, the aim span is the `to` node's extent, and pipes-land reports the `from` side.
3. The aim of a slot is the midpoint of the union of the aim spans of the Pipes in it that have targets. A slot holding no Pipe with targets does not move.
4. After taffy computes the layout and the absolute positions are known, and before the text re-measure and link routing, layout translates every slot with an aim along its move axis so the slot's center lies on the aim, clamped so the slot stays inside its parent's content box. The translation moves the slot's box, its descendants and their parts by the same amount and changes no size. Slots are processed in geometry order, each once; a slot holds only pipes and tees, so no slot lies inside another.
5. Nothing else moves, and taffy is not run again. A translated slot that overlaps a sibling is reported by siblings-do-not-overlap, and one clamped short of its aim by pipes-land.
6. `pipes-land` (section 6) examines, for a Pipe with targets, one end per named target instead of the neighbor on that side: the `from` target is the left (h) or upper (v) end, and `to` the right or lower end. The end lands when the pipe's center on the move axis lies within the named node's extent on that axis, the epsilon included, and the node lies on its side: its center on the run axis is before the pipe's center for `from` and after it for `to`. A side without a target keeps the neighbor rule. Messages name the target: `from target /body/0/children/0/children/1 has no box across the pipe's center y 418.35` and `to target /body/0/children/2 lies on the left of the pipe`.
7. A Tee arm cannot carry a target (vet rule `pipe-target-on-tee-arm`): the Tee's grid places its arms.

Example. g7's gutter could be written as one Col holding two slot Cols, the first with VLAN 1 and VLAN 2 (`"from": "metro-1"`, the id of the Metro 1 Box) and the second with VLAN 3 and VLAN 4 (`"from": "metro-2"`), with no grow list on either Col: each slot then centers on its metro Box. `examples/g7.json` keeps its grow lists, so its geometry does not change.

### 13.9 base16 import and the imported tier

`stencil theme import --base16 <scheme.yaml> [--name <name>] -o <theme file>` maps a base16 color scheme onto `#Theme` by fixed rules, writes the theme file and a report next to it, and prints the report. The result is a theme like any other, so it applies to every grammar.

Reading. The importer reads a restricted line format, not general YAML, so the CLI takes no YAML dependency: every line matching `^\s*(base[0-9A-Fa-f]{2})\s*:\s*"?#?([0-9A-Fa-f]{6})"?\s*(#.*)?$` sets one color, the key's hex digits case-insensitive. It accepts both the tinted-theming `palette:` layout (keys indented under `palette:`) and the legacy flat layout (keys at top level, values with or without `#`). An optional `system:` line must say `base16` or `base24`. base24 is a superset: its eight extra keys, `base10` to `base17`, are read and ignored. A key from `base00` to `base0F` that is missing or given twice is an error, as is a file over 65,536 bytes; the command then prints `error base16 <path>: <message>`, writes nothing and exits 1. An unreadable file exits 2.

A `--name`, or a file stem, that does not match `THEME_NAME_PATTERN` (a leading digit, more than 32 characters) is a base16 error too: nothing is written and the message asks for `--name`.

Mapping. `r` is 0.18 when `base00` has L* below 50 (a dark scheme) and 0.22 otherwise. `mix(a, b, f)` is the section 12.6 function, `a` moved `f` of the way to `b`. "The higher contrast of x and y on z" picks by WCAG contrast ratio against z, the first on a tie. `surface` is `base02` when its L* lies within 14 of `base00`'s, else `mix(base00, base02, 14 / d)` where `d` is that L* difference. base16 defines `base02` as the selection background and the seven pinned schemes put it 9 to 32 L* from the page; solarized puts it at mid gray, where a card drawn in it outweighs every container and holds its secondary ink at 2:1. The cap leaves tokyo-night, material-dark, dracula and nord on `base02` and lifts gruvbox-dark and both solarized themes less far. `base07` is not used as an ink on its own: nord sets it to a teal accent, so every ink this section picks between a light and a dark candidate picks between `ink.primary` and `base00`.

| Theme path | Value |
|---|---|
| `name` | `--name`, or the file stem lowercased with every character outside `a-z0-9-` replaced by `-` |
| `tint_cue` | `color` |
| `page`, `frame.frame_fill` | `base00` |
| `frame.body_fill`, `fact.fill` | `base01` |
| the fill of every tone but `strong` and `accent` | `mix(base01, surface, 0.5)` |
| `card.fill`, `tag.fill` | `surface` |
| `card.border` and `tag.border` colors; the `neutral`, `warm`, `cool` and `strong` tone borders; `placeholder.border` | `base03` |
| `foot`, `placeholder.diagonal` | `base04` |
| `ink.secondary`, `ink.zone_label`, `tag.sub_ink`, `fact.ink`, `legend.text_ink` | `base05` |
| `ink.primary`, `tag.ink`, `legend.label_ink` | the higher contrast of `base06` and `base07` on `base00` |
| `kicker`, `frame.border`, `frame.bar_fill`, the `emphasis` tone border | `base0D` |
| `frame.bar_ink` | the higher contrast of `base00` and `ink.primary` on `base0D` |
| the `highlight` tone border | `base0A` |
| the `accent` tone | fill `mix(base00, base09, r)`, border `base09`, label ink the higher contrast of `base09` and `ink.primary` on that fill |
| the `soft` tone | the shared tone fill, no border |
| `gray.color` | `base04` |
| `deny` | color `base08`, tag border `mix(base00, base08, 0.5)`, tag ink the higher contrast of `base08` and `ink.primary` on `tag.fill` |
| dash | slot 1's wire, which is `base0D` (section 13.1 rule 3) |
| each tint slot | wire: the accent in the table below; fill `mix(base00, accent, r)`; border `mix(base00, accent, 0.5)`, the accent at half strength; ink the higher contrast of `ink.primary` and `base00` on the fill |
| `badge.customer`, `badge.internal`, `ask` | slot 1, slot 2 and slot 4, each its fill and ink |
| `callout` | note slot 1, decision slot 6, open slot 4 (wire on fill); risk `base08` on `mix(base00, base08, r)` |
| `icon_chip`, `iso.chip.fill` | `#FFFFFF` |
| `lanes.lifeline` | `base04`, 1 px, dashed |
| `iso.faces` | top 0, left `-s`, right `-2s`, where `s` is the L* difference between `base00` and `base02`, rounded and clamped to 4 to 8 |
| `iso.slab_thickness` | 6 when the L* difference between `base00` and `base01` is at least 4, else 8, so a slab whose top barely differs from the page shows a thicker shaded side |
| `iso.labels` | frame: `base0D` on a light scheme, the bar ink on a dark one; zone: `base03` on a light scheme, `ink.primary` on a dark one |
| `iso.chip` ring, shadow | ring `base03`, no shadow |
| `iso.shadow` | dark scheme: `#000000` at 0.5, blur 3, dy 3; otherwise `ink.primary` at 0.18, blur 2.5, dy 3 |
| widths, patterns, dots, `iso.frame_sides`, `iso.frame_outline`, `solid_edges`, `edge_width`, `iso.widths` | as the derived roles of section 13.5 |

Tint slots. Slot 1 stays the blue, and slots keep the hue families of `TINT_NAMES` where base16 has them: cyan takes teal's slot 3, and base16, which has no violet and only one cyan, puts its red in violet's slot 5 and its brown in the second cyan's slot 8:

| Slot | base16 key | Usual base16 role | Slot name |
|---|---|---|---|
| 1 | `base0D` | blue (functions) | blue |
| 2 | `base0E` | magenta or purple (keywords) | pink |
| 3 | `base0C` | cyan (support, regex) | cyan |
| 4 | `base0A` | yellow (classes) | yellow |
| 5 | `base08` | red (variables) | red |
| 6 | `base0B` | green (strings) | green |
| 7 | `base09` | orange (constants) | orange |
| 8 | `base0F` | brown (deprecated, embedded) | brown |

Slot 5 and the deny line both take `base08`, so a solid slot 5 line and a deny line share a color. The deny line keeps its dashed pattern and its tag, which carry the distinction, as they do in the accessible palettes, where the research found deny close to vermillion under dichromacy; an author who needs both reaches for another slot. Slot names are fixed by slot, not read from the scheme, and they are short enough for the relabel slack of section 13.4 rule 4.

Report. The importer runs `theme_quality` on the result and writes `<theme file stem>.report.txt` beside the theme file, holding exactly the lines `stencil theme check <name>` prints for it (section 13.12): the summary line names the theme, never the path it was written to, so a report reads the same wherever it is written and equals the output of `stencil theme check` on the built-in of that name. It always writes both files once the scheme reads. It prints the report lines, which name every failing row with its roles, then the two written paths. Exit 0 when every row passes; exit 1 when any row fails. Output is deterministic: the theme JSON is `serde_json::to_string_pretty` of the `Theme` in declaration order plus a trailing newline, so a re-import of the same scheme writes the same bytes.

The imported tier. Seven schemes are committed under `crates/stencil-render/themes/imported/`, each as `<name>.json`, `<name>.base16.yaml` (the pinned source, byte for byte) and `<name>.report.txt`, with `SOURCES.md` listing for each the source URL, the repository commit and the SHA-256 of the YAML. All seven come from `github.com/tinted-theming/schemes` at commit `d70255b752ac8328ee3d549c72a1a55ce5fc794f`, path `base16/<file>`:

| Theme name | Source file | SHA-256 |
|---|---|---|
| tokyo-night | `tokyo-night-dark.yaml` | `98e88daa15855821c156441c1db963e97fbf89d40912b10d11ee2226c9f48755` |
| solarized-light | `solarized-light.yaml` | `9e9ab6cfab64904e85250ae097a52dfd2de7bde320d28a64997e86b1fe42e306` |
| solarized-dark | `solarized-dark.yaml` | `22d8250ac9958985dddcff9f438435a5c0429a672ff86f6fc984332c991bc5a9` |
| material-dark | `material-darker.yaml` | `b427528303ad3b625e0b40b721f7c72f09c9732c831140dcbc53dc0861c5e0ef` |
| gruvbox-dark | `gruvbox-dark.yaml` | `b17930d08392c161d609ba5490d9f52c174ae38321557192d871d5d9fe61d6cc` |
| dracula | `dracula.yaml` | `f6f5a7f3a28a3c305a021328b32a8849ca76cfce3da340dc2c1e69f3cbd8bc17` |
| nord | `nord.yaml` | `bf0620d47f2326576d9f7f28302c9acd67fb4c753c5d496986d34687027c717b` |

`material-dark` is built from `material-darker.yaml`; the upstream `material.yaml` is also dark and is not imported.

None of the seven passes the thresholds of section 13.4 rule 10. A prototype of this mapping, run with the palette research checker, measured these separation minima (CIE76; the worst dichromacy named):

| Theme | Wires, normal (need 20) | Wires, worst dichromacy (need 12) | Fills, normal (need 8) | Fills, worst dichromacy (need 5) |
|---|---|---|---|---|
| tokyo-night | 3.98 | 1.50 protanopia | 1.35 | 0.36 protanopia |
| solarized-light | 18.59 | 3.42 protanopia | 5.75 | 0.34 deuteranopia |
| solarized-dark | 18.59 | 3.42 protanopia | 3.70 | 0.69 deuteranopia |
| material-dark | 17.41 | 3.61 deuteranopia | 4.43 | 0.85 deuteranopia |
| gruvbox-dark | 14.51 | 5.50 protanopia | 3.69 | 0.82 protanopia |
| dracula | 34.56 | 6.75 protanopia | 7.46 | 1.24 protanopia |
| nord | 13.94 | 6.55 protanopia | 2.77 | 1.12 protanopia |

The separation failures belong to the source accents: eight terminal colors chosen for syntax highlighting collapse in pairs under dichromacy (yellow and green, blue and magenta, red and brown), and no mapping that takes the accents as wires can separate them. tokyo-night's base16 port also assigns lavender to `base08` and two cyans to `base0A` and `base0D`, so its slot names do not describe its colors. Several contrast rows fail as well, for example `ink.secondary on card.fill` at 3.52:1 in solarized-dark and 3.47:1 in solarized-light, where `base05` sits on the capped `surface` (2.01:1 on `base02` itself); each report lists them. The imported tier therefore ships with its failures recorded rather than repaired: a repair that moved the accents would stop being a fixed mapping and would change each scheme's look. The tests hold the record exactly instead (section 13.14), the prime themes topic says the imported tier fails the colorblind thresholds and points at `stencil theme check <name>`, and the gallery renders only the six designed themes.

### 13.10 Checks

`CheckName` gains `PrintFit` and `IconMatchesProduct`, appended after `IsoLinksClear`, so the CLI prints twelve check lines in this order: child-inside-container, siblings-do-not-overlap, text-fits-box, remembered-constants, legend-consistency, links-routed, links-avoid-boxes, pipes-land, iso-labels-clear, iso-links-clear, print-fit, icon-matches-product. `vet` runs the three model checks, remembered-constants, legend-consistency and icon-matches-product, in that order. Two model checks now read the grammar: `remembered_constants(page, grammar)` scans for the grammar's `remembered` literals (section 13.2), and `icon_matches_product(page, grammar)` reads its products tables. `REMEMBERED_CONSTANTS` moves into the gcp grammar's data, with the same four literals and reasons.

| Check | Crate | Unit examined | Defect when | Epsilon |
|---|---|---|---|---|
| `print-fit` | layout | each text run (every `TextRun` of every node part and every link tag), as text-fits-box counts them | the run would print below 8 pt at the print width: `size_px * 72 / (canvas_px / inches)` is below 8, where `canvas_px` is the drawn canvas width (`IsoScene.canvas.width` under iso, `PageGeometry.canvas.width` otherwise) and `inches` the `--print-width` value | 0.01 pt |
| `icon-matches-product` | model | each Item whose kind has a non-empty products table and that has an `icon` or a `subtitle` | the `subtitle` names a product that has a product icon and the item carries a different icon; or the `subtitle` names a product with no product icon and the item carries a product icon | |

| Check | Count 1 | Any other count, 0 included |
|---|---|---|
| `print-fit` | text run | text runs |
| `icon-matches-product` | item | items |

print-fit:

1. Without `--print-width` the report is `CheckReport::not_applicable(CheckName::PrintFit, "no print width")`. With it, a page always has text runs (the title at least, or under `chrome: none` the body), so examined 0 fails as section 6 requires.
2. `--print-width <inches>` takes a decimal from 0.5 to 200 on `render` and `check`. A value outside that range, not finite or not a number is a clap error, exit 2.
3. `pub fn print_fit(geometry: &PageGeometry, canvas_width_px: f32, print_width: Option<PrintWidth>) -> CheckReport`, where `PrintWidth` is a newtype over f32 whose constructor enforces rule 2. The pipeline passes the drawn canvas width.
4. Defect pointer: the node that owns the run, or `/links/<i>` for a link tag. Message: `<part> <text> prints at <p> pt, below 8 pt (<s> px on a <c> px canvas at <w> in)`, every number with 2 decimals, for example `print-fit /kicker: badge_text "CUSTOMER" prints at 7.64 pt, below 8 pt (10.00 px on a 1320.00 px canvas at 14.00 in)`.
5. `render` runs print-fit when `--print-width` is set, after it has written the three files: it prints the three paths, then the print-fit check line and its defect lines, and exits 1 when the report fails. The files stay written, as a gallery render that fails a check stays written.

icon-matches-product:

1. The table is grammar data: each item kind's `products` (section 13.2) lists icons with their class and the product names each may stand for. The class follows the archive path in section 8.2: `Unique Icons/` is `product` and `Category Icons/` is `category`. The gcp `product` kind carries this table, one row per `IconName` in `IconName::ALL` order:

   | Icon | Class | Names |
   |---|---|---|
   | agents | category | Vertex AI Agent Builder, Vertex AI Agent Engine, Agent Engine, Agentspace, Gemini Enterprise, Dialogflow |
   | ai-ml | category | Vertex AI, Gemini, Document AI, Vision AI, Speech-to-Text, Text-to-Speech, Translation AI, Natural Language AI, Cloud TPU |
   | bigquery | product | BigQuery |
   | cloud-run-flat | product | Cloud Run, Cloud Run functions |
   | cloud-run | product | Cloud Run, Cloud Run functions |
   | cloud-sql | product | Cloud SQL |
   | cloud-storage | product | Cloud Storage |
   | compute-engine | product | Compute Engine |
   | compute | category | Compute Engine, Cloud Run, Cloud Run functions, App Engine, Cloud Functions, Batch, Bare Metal Solution, Google Cloud VMware Engine |
   | containers | category | Google Kubernetes Engine, GKE, Cloud Run, Artifact Registry |
   | data-analytics | category | BigQuery, Dataflow, Dataproc, Pub/Sub, Looker, Dataplex, Cloud Data Fusion, Cloud Composer, Datastream, Dataform |
   | databases | category | Cloud SQL, AlloyDB, Spanner, Firestore, Bigtable, Memorystore, Database Migration Service |
   | devops | category | Cloud Build, Artifact Registry, Cloud Deploy, Infrastructure Manager |
   | gke | product | Google Kubernetes Engine, GKE |
   | hybrid | category | Google Distributed Cloud, Cloud Interconnect, Dedicated Interconnect, Partner Interconnect, Cross-Cloud Interconnect |
   | integration | category | Pub/Sub, Application Integration, Workflows, Eventarc, Apigee, API Gateway, Cloud Tasks, Cloud Scheduler |
   | networking | category | Cloud Load Balancing, Cloud CDN, Cloud DNS, Cloud NAT, Cloud Router, Cloud VPN, Cloud Interconnect, Network Connectivity Center, Private Service Connect, Virtual Private Cloud |
   | observability | category | Cloud Logging, Cloud Monitoring, Cloud Trace, Cloud Profiler, Error Reporting |
   | scc | product | Security Command Center |
   | security-identity | category | Security Command Center, Cloud KMS, Secret Manager, Identity and Access Management, Identity-Aware Proxy, Cloud Armor, VPC Service Controls, Certificate Authority Service, Sensitive Data Protection |
   | serverless | category | Cloud Run, Cloud Run functions, Cloud Functions, App Engine, Workflows, Eventarc |
   | storage | category | Cloud Storage, Filestore, Persistent Disk, Hyperdisk, Backup and DR Service, Storage Transfer Service, Google Cloud NetApp Volumes |
   | vertex-ai | product | Vertex AI |

2. `pub fn named_product(table: &[IconProducts], subtitle: &str) -> Option<&str>` finds every name of the table in the subtitle, ASCII case-insensitive, at the word boundaries of the remembered-constants rule (section 6), and returns the longest; on a tie, the one that starts first. `Vertex AI Agent Engine` therefore names Vertex AI Agent Engine, not Vertex AI, and `Cloud Run jobs` names Cloud Run. The scan is bounded by the table (at most 32 item kinds of at most 64 rows, enforced by `validate_grammar`) times the 400-scalar text limit, and uses no regex dependency.
3. The product icons of a name are the `product`-class rows that list it. An item is examined when its kind has a non-empty products table and it has an `icon` or a `subtitle`. It is a defect when the subtitle names a product with product icons and the item's icon is set and not one of them, or when the subtitle names a product with no product icon and the item's icon is a `product`-class icon. Category icons are allowed for any product with no product icon. An item with no subtitle, or whose subtitle names nothing in the table, is examined and clean, as is one that names a product with a product icon and carries no icon.
4. When no item of the page has a kind with a products table (every plain figure), the report is not applicable with the reason `grammar has no icon table`.
5. Defect pointer: the item. Messages: `subtitle names Vertex AI, whose icon is vertex-ai; the item carries ai-ml`, with every product icon listed and joined by ` or ` (`cloud-run or cloud-run-flat`), and `subtitle names Cloud Logging, which has no product icon; the item carries the product icon bigquery`.
6. Run over the examples, the check finds three real mismatches, corrected in step (d): `hybrid-ai.json` `/body/0/children/2/children/0/children/0/children/0/children/0/children/2/children/0` (`Vertex AI Registry` on `ai-ml`, becomes `vertex-ai`) and `/body/0/children/2/children/0/children/1/children/0/children/4` (`Billing export · BigQuery` on `data-analytics`, becomes `bigquery`), and `network-hub-spoke.json` `/body/0/children/2/children/1/children/1/children/0/children/0/children/2/children/1` (`Cloud Run · Direct VPC` on `serverless`, becomes `cloud-run`). Icon boxes keep their size, so the geometry is unchanged; the `hybrid-ai.json` pointers change when step (d) moves those items into an apis Box.

New check lines after section 13 lands, for the examples as corrected in step (d), with no `--print-width`:

| Example | Summary |
|---|---|
| g7 | `stencil check: 12 checks, 7 passed, 0 failed, 5 not applicable` (icon-matches-product examines 6 items) |
| hero-iso | `stencil check: 12 checks, 10 passed, 0 failed, 2 not applicable` (4 items; pipes-land and print-fit not applicable) |
| hybrid-ai, network-hub-spoke, stress-dense | `stencil check: 12 checks, 7 passed, 0 failed, 5 not applicable` |
| onepager | `stencil check: 12 checks, 8 passed, 0 failed, 4 not applicable` (7 items) |

`vet examples/g7.json` prints three check lines (remembered-constants 36 text fields, legend-consistency 8 relations, icon-matches-product 6 items) and `stencil vet: 0 violations, 3 checks, 3 passed, 0 failed`. `check examples/g7.json --print-width 14` fails print-fit with one defect, the 10 px badge at 7.64 pt (examined 40 text runs; the 11 px kicker and foot print at 8.40 pt), and `--print-width 16` passes, the badge at 8.73 pt.

### 13.11 Break opportunities and isometric polish

Break opportunities. A long bucket name such as `acme-prod.analytics.raw-events` is one word to a measurer that breaks only at spaces, so its min-content width can widen an item past its column. Section 3's definition of a word changes for both measurers: a line may break after a run of U+0020, after a `-` or `/` that is followed by an ASCII letter, and after a `.` that is followed by an ASCII letter. A word is the text between two consecutive opportunities, and the min-content floor of sections 2.1 and 2.5 is the widest word. A dot between digits (`10.8.0.0`) and a hyphen before a digit (`-29`) stay unbreakable, so addresses and ranges never split.

1. `FixedMetricsMeasurer` breaks at exactly these opportunities. A break after `-`, `/` or `.` keeps the character on the earlier line, and no width is subtracted for it (it is not a trailing space). The worked example of section 3.1 is unchanged at max-content; at `Some(0.0)` `"On-prem router 1"` now gives `On-`, `prem`, `router` and `1`, as cosmic-text already does.
2. `CosmicTextMeasurer` keeps `Wrap::Word`, whose UAX #14 opportunities already include the hyphen and slash cases. For the dot case it shapes a copy of the string with U+200B ZERO WIDTH SPACE inserted after every `.` followed by an ASCII letter, and maps each layout run's byte range back to the original string through the table of inserted offsets, so `TextLine` ranges, the drawn text and the SVG never contain U+200B. The asset probe of section 8.3 step 5 gains one check before step (e) starts: each bundled face maps U+200B to a glyph other than 0 with zero advance. If a face does not, the measurer instead splits the string at those dots into segments, shapes each with `Wrap::Word`, and joins the lines greedily; the spec is revised before the change lands.
3. Cosmic-text also breaks at UAX #14 opportunities the fake does not have (after an em dash or a question mark, for example). The contract states the shared set; tests that compare the two use strings whose only opportunities are in that set.
4. No example has a dotted word, and cosmic-text already broke at their hyphens and slashes, so step (e) leaves every example's geometry unchanged; the geometry fixtures prove it. Layout tests that measure hyphenated strings with the fake are re-derived in step (e).

Isometric polish, carried from the judges of the hero figure:

1. Block shadow. When the theme sets `iso.shadow`, every block solid with `opaque` true (every Item, Fact, Text and Callout; not Note or Frame) draws a shadow first in its group: one `<polygon>` of the block's footprint projected at its base_z and moved `dy` px down on screen, with `fill` the shadow color, `fill-opacity` the opacity and `filter="url(#stencil-shadow)"`. The SVG writes, directly after the background `<rect>`, `<defs><filter id="stencil-shadow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="<blur>"/></filter></defs>`. The filter region is relative to the shadow polygon's bounding box, so 50 percent on each side covers three standard deviations of blur for every block wider than `6 * blur`. resvg rasterizes the blur on the CPU from the same inputs to the same pixels, so two renders stay byte-identical, and a test holds it. Shadows are not in the extent set E of section 12.2 rule 3: they lie under their blocks, and with the built-in values the blur reaches at most `3 * 3 + 3` = 12 px past a footprint, inside the 20 px margin; a theme file with a larger blur can reach the canvas edge, where it is clipped. A theme without `iso.shadow` writes neither the shadows nor the `<defs>`, and an iso SVG still writes no `<marker>`.
2. A link leaves its slab toward its target. Under iso only, after the adjustments of section 12.3 rule 8: let S be the slab (a Box that is not a ring) nearest the from node among its ancestors, when S does not contain the to node and the link has no authored `via` and no `from_side`. The exit edge of S is the side of S's footprint whose outer line is nearest the to node's border-box center, among the sides that center lies beyond (ties in the order right, bottom, left, top). When the adjusted route crosses S's footprint boundary more than once, or crosses it first on a side other than the exit edge, the link is re-routed: `stencil_layout::reroute_link(geometry, index, from_side, via)` runs the section 11.2 router for that link with `from_side` facing the exit edge and one via point on the line through the from attach point perpendicular to the exit edge, `ISO_LINK_CLEARANCE_PX` (24) beyond the edge. The re-routed polyline replaces the adjusted one when its status is `Routed`; otherwise the adjusted route stays and iso-links-clear reports what it finds. The flat route and `PageGeometry.links` do not change.
3. `reroute_link` is public in stencil-layout and pure: it reads the geometry and returns a `LinkRoute` without modifying anything.

Step (e) changes every iso render: hero-iso in every theme but wire gains block shadows, and any link that leaves its slab on the wrong edge is re-routed. Its center fixtures are regenerated in step (e) (the SVG, and the geometry fixture when a re-routed link moves a tag) and reviewed as an image at 200 percent, as section 9.4 asks for golden changes.

### 13.12 CLI, CUE and prime

```
stencil vet <json>
stencil render <json> --out-dir <dir> [--scale <1-4>] [--theme <theme>] [--projection flat|iso] [--print-width <inches>]
stencil check <json> [--theme <theme>] [--projection flat|iso] [--print-width <inches>]
stencil gallery <out-dir> [--examples <dir>]
stencil theme show <name>
stencil theme check <theme>
stencil theme import --base16 <scheme.yaml> [--name <name>] -o <theme file>
stencil theme preview <theme> -o <png> [--scale <1-4>]
stencil prime [<topic>]
stencil prime grammar <name>
```

`<theme>` is a built-in theme name or a path ending in `.json`. The grammar has no flag: it is part of what the document means, so it is set on the page, where a theme is a presentation choice the flag may override.

| Command | Change | Output on stdout |
|---|---|---|
| `vet` | parses, resolves and loads the grammar (section 13.2), runs the vet rules with it, then resolves and loads the theme and overrides (section 13.4 rules 1 to 5), then runs the three model checks | as before, three check lines; a grammar or theme failure prints its `error` and `violation` lines and `stencil vet: checks not run` |
| `render` | `--theme` takes any theme reference; `--print-width` runs print-fit after the files are written (section 13.10) | the three paths, then the print-fit lines when the flag is set |
| `check` | `--theme` as render; runs twelve checks; `--print-width` makes print-fit applicable | twelve check lines, defects, summary |
| `gallery` | renders every example under the six designed themes, in the order center, paper, dusk, clear, clear-dark, wire | as before; the summary counts six themes |
| `theme show` | prints a built-in theme's JSON exactly as embedded, so an author can copy it as the start of a theme file or an override; an unknown name exits 2 | the theme JSON |
| `theme check` | loads the theme (structural rules exit 1 as in section 13.4 rule 5), runs `theme_quality`, prints one line per row | row lines and a summary |
| `theme import` | section 13.9 | the report lines, then the two written paths, absolute |
| `theme preview` | renders the preview figure embedded at `crates/stencil-cli/preview/theme-preview.json` under the theme, with the kicker `theme preview · <theme name>`, to one PNG. The figure is a gcp page 1,100 px wide that uses every container kind, the eight tint slots as region fills, every line kind, a Tee, a routed link, a callout and a text block; a test runs `check` on it under every built-in theme. `mise run theme-docs` writes every built-in's preview to `docs/themes/<name>.png` at scale 1, the README's theme table embeds them, and CI regenerates and compares them byte for byte | the written path, absolute |
| `prime` | prints the core briefing (`prime/base.md`), which names the built-in grammars and points at `stencil prime grammar <name>` | the briefing |
| `prime grammar <name>` | prints `prime/grammars/<name>.md` for a built-in grammar, with its kind tables rendered from the grammar data as the vocabulary table is rendered from the schema; an unknown name writes one line to stderr naming the grammars and exits 2 | the grammar briefing |

Row lines, stable for scripts:

```
row contrast ink.primary on page: 16.10, at least 4.50, passed
row contrast ask.ink on ask.fill: 4.34, at least 4.50, FAILED
row separation wires deuteranopia: 13.16 between pink and green, at least 12.00, passed
row separation fills normal: not applicable: tints are told apart by line
stencil theme check center: 223 rows, 222 passed, 1 failed
stencil theme check wire: 223 rows, 215 passed, 0 failed, 8 not applicable
```

A theme with zero rows cannot occur (section 13.4 rule 10); if the row builder returned none, the summary would print `FAILED: nothing examined` and exit 1. Exit codes for `theme check`: 0 when every applicable row passed, 1 when a row failed or the theme is structurally invalid, 2 when the file cannot be read.

New error mappings for section 7's exhaustive table: `GrammarError` and `ThemeError`, any variant, exit 1; an unreadable grammar or theme file (`ReadGrammar`, `ReadTheme`) exits 2; a base16 scheme that does not read (`Base16`: a missing or repeated key, an oversize file) exits 1; an unreadable scheme exits 2; a failed write of the theme or report file exits 2.

`pipeline::load_document` returns the page with its resolved `Grammar` and `Theme`. `pipeline::all_checks(page, grammar, geometry, scene, print_width) -> [CheckReport; 12]`. `layout_page(page, grammar, measurer)` and `render_svg(page, grammar, theme, geometry)` take the grammar, and the renderer the theme, as arguments.

CUE. `cue/stencil.cue` splits:

- `cue/core.cue` (package `stencil`): the core `#Page`, `#Node` and every core tag, with `#Box` and `#Item` taking `kind: #KindName`; `#Line: "gray" | "solid" | "dash" | "deny"` as `line` on `#Pipe`, `#Tee`, `#Link` and `#LegendEntry`; `#TintSlot: int & >=1 & <=8` as `tint?` on `#Box` and the four line carriers; `#FactEntry` and `source?` on `#Fact`; `from?` and `to?` on `#Pipe`; `order?` on `#Link`; `#Lanes`; `grammar?`, `theme?` (the reference patterns), `theme_overrides?: {...}` and `chrome?: "full" | "none"` on `#Page`. The core checks stay here: ids, link endpoints, pipe targets (`_pipeTargetsUnknown`), ordered links, legend keys (`"\(line)-\(tint)"` for solid and dash, slot 1 when absent, the line alone for gray and deny; `_legendKeysAreUnique`; the `chrome: "none"` exception) and `_tintWithoutEffect` for a tint on a gray or deny line. The old `#Theme` enum is gone; `#Theme` names the theme definition.
- `cue/theme.cue` (package `stencil`): `#Theme` (section 13.4). `cue/grammar.cue` (package `stencil`): `#Grammar` (section 13.2).
- `cue/grammars/gcp.cue` (package `gcp`) and `cue/grammars/plain.cue` (package `plain`): each imports the core package, declares `grammar: stencil.#Grammar & {...}`, narrows `#Box.kind` and `#Item.kind` to its kind names, and adds its rules: for gcp the hop-fact rule, tint pairing over slot keys, `_productInsideVpc` and `_apisOutsideGcp` (section 13.3); for plain none. `_tintWithoutEffect` also names a tint on a Box whose kind is not tintable. A `cue.mod/module.cue` declares the module so the grammar packages can import the core; where CUE's package rules force a different file layout, `cue/README.md` records it.
- `cue/g7.cue` moves to `cue/figures/g7.cue` under package `gcp` and unifies with the gcp `#Page`.
- `cue/check.sh` vets the core and both grammar packages; exports both grammars and compares them with `crates/stencil-model/grammars/<name>.json`; exports g7 and compares it with `examples/g7.json`; vets every example against the gcp `#Page` and every built-in theme file against `#Theme`; and runs the negative cases, migrated to the new vocabulary (the pink cases become solid tint 2 cases) plus: tint 9 on a pipe, a tint on a deny pipe, a BigQuery product inside the vpc, an apis Box outside gcp, a Box of kind `zone`, a subnet at the top level, a pipe target naming no id, two ordered links with one order, a legend entry for solid tint 3 that no pipe uses, a theme file with seven tints and a theme file with an unknown role.

Prime:

- `prime/base.md` becomes the core briefing: the loop, the core vocabulary rendered from the schema (Box, Item, facts with their sources, lines, tints, Lanes, chrome, pipe targets), layout, the checks table with rows for `print-fit` (each text run, only with --print-width; prints below 8 pt; widen the print or shorten the figure) and `icon-matches-product` (each item with an icon or subtitle whose kind has an icon table; the subtitle names a product whose own icon is another, or a product icon on a product without one; use the product's icon or a category icon), themes, and one line per built-in grammar. The vocabulary sentence becomes "Every node object carries "tag"; the Page does not." `field_notes` adds `tint 1-8`, `source =doc`, `chrome =full`, `grammar =gcp; a built-in name or a .json path` and `theme =center; a built-in name or a .json path`. Under "Rules no check enforces" a line points sequence and timeline figures at Lanes with ordered links and `stencil prime grammar plain`.
- `prime/grammars/gcp.md` holds what was GCP-specific in the base briefing and the layout and cue topics: the kind table with meanings, the nesting, the tint pairing in slot terms, the apis kind and the products that never sit inside a VPC, and the rules no check enforces: "Every product on a hop carries a fact read from the live doc at authoring time (source doc), or an explicit ask; never a remembered value. A built fact holds as-built names (bucket names, VLAN IDs, project ids) and needs no live doc." "One audience per figure: canvas customer or internal." "Official product names in subtitle: Cloud Run, Cloud SQL, Pub/Sub." `prime/grammars/plain.md` lists the plain kinds.
- `prime/themes.md` is rewritten around tint slots: the slot table with center's names, the six designed themes with one line each, the imported tier with its warning, theme files, `stencil theme show` and `check`, and overrides with the `tints` slot-object form. The per-theme color table goes; `stencil theme show <name>` prints the values.
- `prime/layout.md`, `prime/cue.md`, `prime/links.md` (its example link becomes `"line": "solid", "tint": 1`, and it gains ordered links and Lanes), `prime/blocks.md` (callout tints by kind without naming colors) and `prime/checks.md` (the two new rows and their line formats) follow the core vocabulary. `stencil prime example` stays `examples/g7.json` verbatim.
- `BASE_BYTES_MAX` rises from 6,000 to 7,000 bytes and `TOPIC_BYTES_MAX` from 4,000 to 5,000, which also bounds each grammar briefing. Today's briefing is 6,000 bytes exactly and the themes and checks topics are within 20 bytes of their limit; moving the GCP text into `grammars/gcp.md` frees some of the base, and the raised limits leave room for the core additions. The prime tests enforce the new limits.

### 13.13 Migration

Every file that changes, by area. The step letters refer to section 13.15.

Documents (step a). Each of `examples/g7.json`, `examples/hero-iso.json`, `examples/hybrid-ai.json`, `examples/network-hub-spoke.json`, `examples/onepager.json` and `examples/stress-dense.json` is rewritten field for field and gains `"grammar": "gcp"` after `canvas`:

| Before | After |
|---|---|
| `{"tag": "Zone", "kind": "gcp", ...}` and every other zone kind | `{"tag": "Box", "kind": "gcp", ...}` |
| `"kind": "region-a"`, `"region-b"` | `"kind": "region", "tint": 1`, `"tint": 2` |
| `"kind": "onprem-a"`, `"onprem-b"` | `"kind": "onprem", "tint": 1`, `"tint": 2` |
| `{"tag": "Pcard", "icon": I, "fn": F, "pn": P, "fact": T, "ask": A}` | `{"tag": "Item", "kind": "product", "icon": I, "title": F, "subtitle": P, "facts": [{"text": T}, {"text": A, "source": "ask"}]}`, absent fields staying absent |
| `"kind": "blue"`, `"pink"` on a Pipe, Tee arm, Tee, Link or legend entry | `"line": "solid", "tint": 1`, `"tint": 2` |
| `"kind": "gray"`, `"dash"`, `"deny"` on the same | `"line": "gray"`, `"dash"`, `"deny"` |

`tint` is written directly after `kind` or `line`. The same rewrite applies to `cue/g7.cue` (moved to `cue/figures/g7.cue`), the negative cases in `cue/check.sh`, `cue/README.md`, the hand-built documents in the test helpers (`crates/*/tests/common/mod.rs`) and every test that names a tag, kind or field, and to the spec's own JSON blocks and tables (sections 1.1, 1.2, 2.2, 2.4, 2.5, 2.11, 5.2, 9.3, 10 and 12.10). `crates/stencil-render/tests/fixtures/` gains the identity fixtures of section 13.14.

Model (steps a, c, d, e): `crates/stencil-model/src/document.rs` (section 13.1 types, keys, labels; Lanes in step c; pipe targets in step d), `vet.rs` (the section 13.1 rules, each in the step that adds its field), `walk.rs` (fact entries in `text_fields`, Lanes in `body_nodes`), `checks.rs` (legend consistency on keys, remembered constants from the grammar, `CheckName` gains two variants in step d), new `grammar.rs` with `grammars/gcp.json` (step a) and `grammars/plain.json` (step c), new `theme.rs` (step b), new `products.rs` (`named_product`, `icon_matches_product`, step d), `lib.rs` (exports, `grammar_schema`, `theme_schema`, `validate_page` taking the grammar), `text.rs` (fake break opportunities, step e), `schema/stencil.schema.json` (regenerated at each step that changes a type), new `schema/grammar.schema.json` and `schema/theme.schema.json`.

Text (step e): `crates/stencil-text/src/measurer.rs` (U+200B insertion and offset mapping), tests `measure.rs` and `coverage.rs` (U+2022 lands in step a with the built facts).

Layout (steps a, c, d, e): `src/build.rs` (Box layout from the container kind, Item layout, fact parts by source, canonical legend labels, chrome; Lanes in step c), `src/lib.rs` (`layout_page` takes the grammar, `NodeGeometry.kind` as `String` and `tint`, `PartName::BuiltBox` and `Built`, `NodeTag::Lanes`, `LinkRoute.line` and `tint`), `src/compute.rs` (the slot translation of section 13.8, step d), `src/route.rs` (lane messages in step c, `reroute_link` in step e), `src/checks.rs` (`print_fit` and the pipes-land targets in step d), `src/styles.rs` (colors unchanged; the test of section 13.4 rule 13), new `src/theme_labels.rs` (`theme_legend_labels`, step b); every test file under `crates/stencil-layout/tests/`.

Render (steps a, b, b2, c, e): `src/palette.rs` (step a re-keys today's tables from `ZoneKind` and `PipeKind` to role, tone and tint and to line and tint; step b rewrites it over `Theme` and removes the constants), new `src/themes.rs` and `themes/center.json`, `paper.json`, `dusk.json`, `clear.json`, `clear-dark.json`, `wire.json` (step b) and `themes/imported/` (step b2), `src/svg.rs` and `src/svg/iso_writer.rs` (keys and roles in step a; relabel in step b; lifelines and lane messages in step c; shadows and `<defs>` in step e), `src/iso.rs` with `drape`, `route` and `shapes` (rings from the grammar in step a; slab thickness from the theme in step b; slab exit in step e), `src/measured.rs` (keys), `src/lib.rs`; tests `center_identity.rs`, `themes.rs`, `svg.rs`, `iso.rs`, `arrows.rs`, `blocks.rs`, `links.rs`, `icons.rs`, `measured.rs` and `common/mod.rs`.

CLI (steps a to e): `src/lib.rs` (flags, the `theme` subcommand, `prime grammar`), `src/pipeline.rs` (grammar and theme resolution, twelve checks, print width), `src/report.rs` (row lines), `src/exit.rs` (new error mappings), `src/gallery.rs` (six themes), `src/prime.rs` (budgets, field notes, grammar topic), new `src/theme_import.rs` (step b2), `prime/base.md`, `themes.md`, `layout.md`, `cue.md`, `links.md`, `blocks.md` and `checks.md`, new `prime/grammars/gcp.md` and `prime/grammars/plain.md`; tests `cli.rs`, `gallery.rs`, `golden_g7.rs`, `golden_onepager.rs`, `iso.rs`, `prime.rs`, `theme.rs` and new `theme_import.rs` and `grammar.rs`.

CUE (steps a to d): `cue/stencil.cue` splits into `cue/core.cue`, `cue/grammar.cue` and `cue/grammars/gcp.cue` (step a), `cue/theme.cue` (step b), `cue/grammars/plain.cue` (step c), new `cue/cue.mod/module.cue`, `cue/g7.cue` moved to `cue/figures/g7.cue`, `cue/check.sh`, `cue/README.md`.

Examples corrected (step d): `examples/hero-iso.json` (Warehouse into an apis Box; the via points and the section 12.10 counts re-derived), `examples/hybrid-ai.json` (four items into an apis Box; two icons), `examples/network-hub-spoke.json` (one icon). Section 12.10's JSON block and its counts are replaced in the same change. A new `examples/plain-system.json` (step c) draws a small system in the plain grammar with a Lanes node of three heads and four ordered messages, so the gallery renders the second grammar and the time axis in every theme.

Repository: `README.md` (grammars, six themes, the imported tier, `stencil theme`), `docs/gallery/*.png` regenerated by `mise run gallery-docs` whenever an example's center render or the one-pager's dusk or wire render changes (dusk and wire in step b; the new plain example in step c; hero-iso, hybrid-ai and network-hub-spoke in step d; hero-iso in step e), and `scripts/gallery-docs.sh` (adds the plain example).

### 13.14 Tests

Identity proofs. Before step (a) changes any code, the implementer runs `origin/main` over every example and commits, under `crates/stencil-render/tests/fixtures/`, two files per example: `<stem>.center.svg`, the center SVG in the example's own projection (g7, hybrid-ai and network-hub-spoke already have theirs; onepager, stress-dense and hero-iso are added, hero-iso in iso), and `<stem>.center.geometry.json`, the measured JSON with the top-level `document` key removed, serialized as `render` writes it. `document` echoes the input, which the migration rewrites, so it is the one key that cannot stay equal; `canvas`, `nodes`, `links` and `projection` must, and they can because the output vocabulary does not change (section 13.1 rule 5). `crates/stencil-render/tests/center_identity.rs` then asserts, for every example, that the migrated document renders under center to SVG bytes equal to the fixture and to measured JSON whose bytes, with `document` removed, equal the geometry fixture, and `golden_g7.rs` keeps its section 9.4 counts and geometry assertions on the migrated g7. These hold unchanged through steps (a), (b) and (c). Step (d) regenerates exactly the fixtures of the three examples it corrects (section 13.13), step (e) regenerates hero-iso's fixtures for the shadows and the slab exit and nothing else, and the pipe targets of step (d) and the break opportunities of step (e) regenerate nothing, which is their proof. Each regeneration is its own commit with the side-by-side PNGs attached to the pull request.

stencil-model:

- The migrated examples parse, round-trip and validate against the regenerated schema; a copy with `"tag": "Zone"`, `"tag": "Pcard"`, `"kind": "blue"` on a pipe or `"fn"` on an item is a `ModelError::Json`.
- Grammars: `gcp.json` and `plain.json` parse and pass `validate_grammar`; gcp's container kinds equal section 13.3's table field for field, and its border widths, paddings, radii and label styles equal section 2.4's; a copy with each grammar fault (a duplicate name, an unknown parent, no kind under `page`, a frame with a tone, a group without one, a default tint on an untintable kind, a solid border of width 0, an item with icons none and a products row, a repeated icon) is rejected with the named rule. `grammar_schema()` equals the committed `schema/grammar.schema.json`.
- Vet with a grammar: `kind-unknown` for a Box of kind `zone` and an Item of kind `service` under gcp; `kind-parent-not-allowed` for a subnet at the top level and for a vpc inside a region, and none for a subnet inside a project inside a Row (Row is transparent); `icon-outside-pack` for a plain `service` with an icon; `grammar-unknown` for `"gcpx"`. Every example vets clean under gcp.
- `tint`: 1 and 8 vet clean on every carrier, 0 and 9 give `tint-out-of-range`; `box_tint` for region (absent gives 1), onprem (absent gives none) and subnet (ignored); `line_tint`, `box_key`, `line_key` and `legend_label` against section 13.1's tables for every line and slot.
- Legend consistency: solid tint 2 used with only a solid tint 1 entry gives one defect at the pipe and one at the entry; a tint on a deny pipe and on its entry are the same key; an entry for solid with no tint matches a pipe with tint 1; a dash tint 2 needs its own entry. Under `chrome: none`, one key in use with an empty legend passes with examined equal to the uses; two keys with an empty legend fail; one key with a legend entry follows the normal rule.
- Facts: an Item's fact entries appear in `text_fields` after title and subtitle at `/…/facts/<i>/text`; remembered constants are found in a built fact; a Fact node without `source` serializes without it.
- Themes: every built-in parses and passes `validate_theme`; a copy of center with each structural fault (lowercase hex, width 5, step 41, opacity 1.5, slab thickness 1, blur 0, an uppercase tint name, two equal tint names, seven tints, a fill on the strong tone, an unknown key at the top and inside `iso`) is rejected with the named rule and pointer, all faults of one document reported together in field order. `apply_overrides`: one wire through the `tints` slot form changes that slot only; a nested object merges; an unknown key reports `/theme_overrides/...`; a string where the base holds an object is `theme-override-not-object`. `theme_schema()` equals `schema/theme.schema.json`.
- `theme_quality`: center gives 223 rows, 222 passed, and exactly one failure, `ask.ink on ask.fill` at 4.34; paper, dusk, clear and clear-dark give 223 rows and no failure; wire gives 215 passed and 8 not applicable; the center and dusk values match the research checker to 2 decimals (recorded in the test, with the checker's file named as the source). A hand-built theme with eight equal wires fails the four wire separation rows, and the same theme with `tint_cue: line` reports them not applicable. Center's inks equal `stencil_layout::styles`, and `TINT_NAMES` equals center's slot names.
- icon-matches-product: gcp's table has every icon once in `IconName::ALL` order, with the class of its section 8.2 archive path; `named_product` picks the longest match (`Vertex AI Agent Engine`, `Cloud Run functions`), respects word boundaries (`BigQueryX` names nothing) and ignores ASCII case; each defect kind fires once and its clean neighbor does not (subtitle BigQuery with icon bigquery, with no icon, with data-analytics; subtitle Pub/Sub with integration, with cloud-run); g7 examines 6 items clean; a plain page is not applicable; the three example mismatches of section 13.10 are found at their pointers before step (d) corrects them.
- Lanes and ordered links: `link-order-outside-lanes` for an ordered link between a head and a node outside the Lanes, `link-order-duplicate` for two messages with one order, `lanes-in-iso` for a Lanes page with `projection: iso`.

stencil-text:

- `"acme-prod.analytics.raw-events"` at `Some(0.0)` gives `acme-`, `prod.`, `analytics.`, `raw-` and `events`, and the lines' byte ranges index the original string, which contains no U+200B; `"10.8.0.0/28"` and `"v1.2"` stay one word; `"gs://acme/raw"` breaks after each `/` followed by a letter. The fake gives the same lines for the same strings.
- Coverage adds U+2022 in SemiBold, and the U+200B probe of section 13.11 runs as a test.

stencil-layout:

- Box: a gcp region with tints 1 and 8 and an onprem without tint lay out as section 2.4's region-a did; an apis Box lays out like a region; a plain `group` lays out with the plain grammar's padding and radius.
- Item facts: an Item with one doc fact, one built fact and one ask has the parts of section 13.7 in order, keyed `fact_box`, `fact`, `built_box`, `built`, `ask_box` and `ask`; a second doc fact at index 3 is keyed `fact_box/3` and `fact/3`; the Built run reads `• ` plus the value.
- Chrome: a `chrome: none` page has geometry nodes `""`, `/body`, `/legend` and its entries, `/foot`; the body starts at y 20; text-fits-box examines no kicker, title or lede run.
- Lanes: three heads and four messages give equal head columns at gap 32, a band of four rows each at least 36 px, message k at its row's middle from head center to head center, one lifeline per head from the head's bottom to the band's bottom; a labeled message with a sub widens its row to the tag height plus 8; links-routed passes the four, links-avoid-boxes passes across the lifelines, and legend consistency counts them.
- Pipe targets: a gutter Col of two slot Cols with no grow, beside a Col of two Boxes of different heights, whose pipes name the Boxes, centers each slot on its Box within 0.01 px, and pipes-land examines one end per target and passes; with the targets swapped it reports both; a slot whose aim lies past its parent is clamped to the parent's content box and pipes-land reports the miss; a pipe with only `to` keeps the neighbor rule on its left; no node outside the slots moves.
- print-fit: g7 at 14 in examines 40 runs with one defect at `/kicker` (7.64 pt), at 16 in passes, without the flag is not applicable; an iso page uses the drawn canvas width.
- `theme_legend_labels`: every built-in passes; a theme whose slot 2 is named `vermillion` fails at `/tints/1/name`.
- `reroute_link` returns the same route as layout for an unchanged request, and a route through a given via point otherwise.

stencil-render:

- The identity proofs above.
- Every color the center SVG writes for every gcp kind, slot and line key equals sections 2.4, 5.2 and 13.5; the theme name is the marker id prefix; `data-kind` carries the key.
- Each designed theme renders every example under its own grammar; the measured JSON `canvas`, `nodes` and `links` are byte-identical across the six themes and the seven imported ones, and `projection` across every theme with slab thickness 6; the SVG bytes differ between any two themes. The plain example renders under every theme, so every theme is exercised on a second grammar.
- Relabel: under wire the legend reads `Solid line`, `Ringed line`, `Dashed line`, `Thin line` and `Dotted line` in flat and iso, each description moved by the label's change in width; under paper a solid tint 2 entry reads `Solid rose`; under center no entry moves.
- Overrides: a page whose overrides recolor slot 3's wire draws that wire and nothing else differently from the base theme.
- A dusk iso render: every slab top is lighter than the surface it stands on, item tops lighter than the Box floors, left faces darker than tops and right faces darker than left.
- Shadows (step e): an iso render under center has one `<defs>` holding one `<filter id="stencil-shadow">` and one shadow polygon per opaque block, first in its group; wire has neither; two renders are byte-identical; the PNG of a lone item shows non-white pixels below the block's bottom silhouette edge and none 20 px past it.
- Slab exit (step e): a link from an item on a slab to an item off it, whose flat route leaves the slab on the far edge, is re-routed to leave through the edge nearest its target; one whose route already leaves through that edge keeps its route; one with an authored via or `from_side` is never re-routed.
- `project_page` with thickness 8 gives slabs 8 high and a taller canvas than with 6; a vpc is a ring under every theme.

stencil-cli:

- `check` on every example prints twelve lines in `CheckName` order and the summaries of section 13.10; `vet examples/g7.json` prints three check lines and `stencil vet: 0 violations, 3 checks, 3 passed, 0 failed`.
- `--theme` with each built-in name renders and checks g7; with a theme file path renders; with a missing path exits 2 with empty stdout; with a malformed file exits 1 with `error` and `violation` lines; `Page.theme` and `Page.grammar` resolve a path relative to the document's directory, and `--theme` relative to the current one; a grammar file with an unknown parent exits 1.
- `--print-width 14` on `check examples/g7.json` exits 1 with the one defect; on `render` writes the three files, prints the paths and the print-fit lines, and exits 1; `--print-width 0.4` and `--print-width nan` exit 2.
- `theme show center` prints `themes/center.json` byte for byte; `theme check center` prints 223 row lines, the ask failure and `stencil theme check center: 223 rows, 222 passed, 1 failed`, and exits 1; `theme check paper` exits 0; `theme check wire` prints 8 not-applicable rows and exits 0.
- `theme import`: every pinned scheme in `themes/imported/` re-imports to its committed theme file byte for byte and writes a report equal to the committed one row for row; the command exits 1 for each, as the committed reports say. A scheme missing `base0B` exits 1 and writes nothing; a base24 scheme imports with `base10` to `base17` ignored and gives the same theme as its first sixteen keys alone; a legacy flat-layout file with bare hex values imports.
- `gallery` writes six themes per example, and `index.html` has a section for each.
- `prime` is at most 7,000 bytes and each topic and grammar briefing at most 5,000; the base text contains "the Page does not" and one line pointing sequence and timeline figures at Lanes and `stencil prime grammar plain`; the vocabulary lists `Box`, `Item`, `Lanes`, `facts`, `source`, `line`, `tint`, `chrome` and `grammar`; the checks table lists `print-fit` and `icon-matches-product`; `prime grammar gcp` lists every gcp kind and `apis`; `prime grammar nope` exits 2.

CUE: `cue/check.sh` passes, with every negative case of section 13.12 rejected by its named error, and both grammar exports equal their committed JSON.

### 13.15 Implementation order

Each step is one pull request that leaves CI green.

a. Core model and gcp grammar extraction, with identity proofs. Commit the identity fixtures from `origin/main` first. Then the section 13.1 types (Box, Item, fact entries with sources, Line, tint, chrome, grammar on the Page), keys, canonical labels and vet rules; the grammar types, `validate_grammar` and the gcp grammar as CUE and exported JSON; layout and vet reading the grammar; legend consistency on keys; remembered constants from the grammar; the CUE split into core, grammar and gcp; every document and test of section 13.13 migrated; the spec's earlier sections edited to match. Themes do not exist yet: `Page.theme` keeps the section 11.1 three-name enum and `theme_overrides` is not yet a field (it is an unknown field until step b), and `palette.rs` keeps today's three palettes, re-keyed by role, tone and tint and by line and tint, which is how the identity proof holds before themes are data. Step (b) changes `theme` to the reference string of section 13.1 and adds `theme_overrides`; the three names keep their meaning.
b. Themes as data and the six designed built-ins: `theme.rs`, `cue/theme.cue`, the six theme files, `themes.rs` and the `Palette` rewrite over `Theme`, theme resolution and overrides, `theme_legend_labels`, the relabel rule, `stencil theme show` and `check`, the six-theme gallery, the prime themes topic. Center stays byte-identical; dusk takes its new values and wire its new legend labels and dashed subnet, and their gallery PNGs are regenerated.

b2. The base16 importer and the imported tier: `stencil theme import`, the seven pinned schemes with their theme files, reports and `SOURCES.md`, and the re-import and report tests.

c. The plain grammar and Lanes: `plain.cue` and `plain.json`, `prime grammar`, the Lanes node with ordered links, lifelines and lane messages, its vet rules and theme role, and `examples/plain-system.json` with its fixtures and gallery PNGs.
d. Checks and CLI flags: `print-fit` with `--print-width`, `icon-matches-product` reading the grammar's table, twelve checks, three in `vet`; pipe targets with the slot translation and the pipes-land revision; the gcp CUE rules `_productInsideVpc` and `_apisOutsideGcp` and `check.sh` over every example; the example corrections of section 13.13 with their regenerated fixtures and gallery PNGs; the prime base and checks topics with the raised budgets. Sections 6, 7, 9.4, 10, 12.9 and 12.10 are edited in this step for twelve checks, the three checks of `vet` and the new summary lines.
e. Layout and render polish: the break opportunities in both measurers, after the U+200B probe; block shadows with the `<defs>` filter; the slab exit with `reroute_link`. Only hero-iso's center fixtures and gallery PNG are regenerated.

## Conventions

- TigerStyle, adapted: bounded loops over document content, assertions at public entry points and at every external-tool boundary (serde input, cosmic-text output, usvg output), specific names without abbreviations, and a test for every behavior.
- Comments explain why or a non-obvious how. They do not narrate how the code came to be.
- A check that examined nothing fails. A probe that could not run exits 2 and never reports a pass.
