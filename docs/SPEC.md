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

### 1.1 Field summary

| Tag | Field | Type | Required | Default |
|---|---|---|---|---|
| Page | title | text | yes | |
| Page | kicker | text | yes | |
| Page | lede | text | yes | |
| Page | foot | text | no | absent |
| Page | width | integer 640 to 2560 | no | 1280 |
| Page | canvas | `customer` or `internal` | yes | |
| Page | body | list of Node, 1 to 256 | yes | |
| Page | legend | list of LegendEntry, 0 to 16 | yes | |
| LegendEntry | kind | PipeKind | yes | |
| LegendEntry | text | text | yes | |
| Row, Col | gap | integer 0 to 64, px | no | 8 |
| Row, Col | grow | list of integers 0 to 100, one per child | no | see section 2.3 |
| Row, Col | justify | `start`, `center`, `end`, `space-between` | no | `start` |
| Row, Col | children | list of Node, 1 to 256 | yes | |
| Zone | kind | ZoneKind | yes | |
| Zone | label | text | yes | |
| Zone | children | list of Node, 1 to 256 | yes | |
| Pcard | icon | IconName | no | absent |
| Pcard | fn | text | yes | |
| Pcard | pn | text | no | absent |
| Pcard | fact | text | no | absent |
| Pcard | ask | text | no | absent |
| Fact | text | text | yes | |
| Note | kind | `kicker`, `h1`, `lede`, `legend`, `foot` | yes | |
| Note | text | text | yes | |
| Pipe | dir | `h` or `v` | yes | |
| Pipe | kind | PipeKind | yes | |
| Pipe | label | text | yes | |
| Pipe | sub | text | no | absent |
| Tee | kind | PipeKind | yes | |
| Tee | hub | text | yes | |
| Tee | arms | exactly two Pipe objects, each `dir: "h"` | yes | |

A text value is a string of 1 to 400 Unicode scalar values with no control characters (U+0000 to U+001F, U+007F). Leading or trailing whitespace is a violation.

An optional field (default `absent`, or `gap`, `grow` and `justify`) written as `null` means absent. serde reads `null` as `None`, and the generated schema admits `null` for every optional field, so the two agree. The measured JSON `document` carries the `null` through unchanged (section 5.4). `width` is not optional in this sense: `"width": null` is a parse error, and only an absent `width` takes the default. `cue export` never writes `null`.

PipeKind: `gray`, `blue`, `pink`, `dash`, `deny`.

ZoneKind: `gcp`, `vpc`, `region-a`, `region-b`, `subnet`, `onprem-a`, `onprem-b`, `project`, `optional`, `k8s`, `perimeter`.

IconName: the file stem of one of the 23 icons in `assets/icons/` (section 8).

### 1.2 Rust types

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

A LegendEntry node is a flex row, `AlignItems::CENTER`, gap 6, holding three parts. The swatch is 26 by 2 and is drawn as a wire: a `<line>` along its box's center line with `stroke-width="2"` and the section 5.2 color and dasharray for its kind. The label uses style `legend_label` with the fixed text for its kind. The text uses style `legend_text`.

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

`src/lib.rs` also exposes `pub mod pipeline`, the steps `run` composes, so the section 9.4 golden test drives the same code path without the argument parser: `read_input` (at most `INPUT_BYTES_MAX` bytes), `load_document` (parse, vet, then the input as a `serde_json::Value`), `output_names`, `render_page` (layout with `CosmicTextMeasurer`, SVG, PNG and measured JSON in memory), `model_checks`, `all_checks` (the eight reports in `CheckName` order), `write_outputs`, and the `Failure` enum that section 7 maps to exit codes.

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
      <text x="100.6" y="33.1" xml:space="preserve" font-family="Inter" font-size="11" font-weight="700" letter-spacing="0.88" fill="#1A73E8">TYPE 5 · DEDICATED INTERCONNECT 99.99% · SOW / DECK / EXEC</text>
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
| `links-avoid-boxes` | layout | each (segment, obstacle) pair of every link, with the link's section 11.2 obstacles, plus each (tag, node) pair for every geometry node that is not a strict ancestor of an endpoint | a segment enters an obstacle's interior by more than the epsilon, or the tag overlaps the node box with both overlap width and height above the epsilon | 0.01 px |
| `pipes-land` | layout | each pipe end that faces a neighbor: for every Pipe and Tee arm, one per side (left and right for h, above and below for v) that has a neighbor, as defined below | the pipe's center on the cross axis lies outside every box in the neighbor's subtree that is not a Row or Col | 0.01 px |

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

Exit codes: `stencil vet` and `stencil check` exit 1 when any report fails, including a failure because nothing was examined. They exit 0 only when every report passes.

## 7. CLI

```
stencil vet <json>
stencil render <json> --out-dir <dir> [--scale <1-4>]
stencil check <json>
stencil schema
stencil prime [<topic>]
```

| Command | Does | Output on stdout |
|---|---|---|
| `vet` | parse, `validate_page`, then, only when there is no violation, `remembered-constants` and `legend-consistency` | one line per violation, or one line per check and one line per defect; one summary line |
| `render` | parse and `validate_page` only (violations stop the command with exit 1; the two model checks do not run, so a legend inconsistency does not stop a render), layout with `CosmicTextMeasurer`, SVG, PNG at `--scale` (default 2), measured JSON; creates `--out-dir` if missing and overwrites existing outputs | the three written paths, absolute |
| `check` | everything `render` does, held in memory without writing files, then all eight checks | one line per check, one line per defect, one summary line; or an `error` line and the summary line when layout or render fails with exit 1 |
| `schema` | prints `page_schema()` as pretty JSON | the schema |
| `prime` | prints the authoring briefing for an agent: `crates/stencil-cli/prime/base.md` with the vocabulary table rendered from `page_schema()`, so tag names, field names, bounds and enum values come from the model; at most 6,000 bytes. With a topic (`themes`, `links`, `blocks`, `layout`, `checks`, `cue`, `example`), that topic's text instead, each at most 4,000 bytes except `example`, which is `examples/g7.json` verbatim. An unknown topic writes one line to stderr naming the topics and exits 2 | the briefing or the topic |

Line formats, stable for scripts:

```
check child-inside-container: examined 33 relations, 0 defects
check text-fits-box: examined 40 text runs, 1 defect
defect text-fits-box /body/0/children/2/children/0/children/0/children/1: fact "BGP peering · link-local /29" measured 571.20x16.20 in box 560.00x16.20
check legend-consistency: examined 0 relations, FAILED: nothing examined
check links-routed: examined 0 links, not applicable: page has no links
stencil check: 8 checks, 3 passed, 2 failed, 3 not applicable
violation gap-out-of-range /body/0/gap: gap 65 is above 64
violation text-untrimmed /title: text starts or ends with whitespace
stencil vet: 2 violations, checks not run
check remembered-constants: examined 36 text fields, 0 defects
check legend-consistency: examined 8 relations, 0 defects
stencil vet: 0 violations, 2 checks, 2 passed, 0 failed
error document is not valid stencil JSON at line 3, column 7: missing field `kind`
stencil vet: document does not parse, checks not run
```

- A check line is `check <name>: examined <n> <unit(n)>, <d> defect` when d is 1 and `defects` otherwise, 0 included. A check that examined nothing prints `FAILED: nothing examined` in place of the defect count, and a not-applicable report prints `not applicable: <reason>` there instead.
- A defect line is `defect <check> <pointer>: <message>`, and a violation line is `violation <rule> <pointer>: <message>`, with the rule in kebab-case as in section 1.3. An empty pointer prints as `""`.
- The `vet` summary always starts with the violation count, `1 violation` or `<n> violations`, 0 included. A `render` or `check` summary carries a violation count only when violations stopped the run; otherwise `check` prints `stencil check: <n> checks, <p> passed, <f> failed`, followed by `, <a> not applicable` when a report did not apply. A document with violations or a parse failure prints `checks not run` in place of the check counts.
- `render` and `check` print violations and parse errors in the same formats, followed by the summary line `stencil <command>: …`.
- `check` prints the eight check lines in `CheckName` declaration order (child-inside-container, siblings-do-not-overlap, text-fits-box, remembered-constants, legend-consistency, links-routed, links-avoid-boxes, pipes-land), and `vet` prints its two in the same relative order. Each check's defect lines follow its check line directly, in the order the `CheckReport` lists them.
- On success `render` prints the three absolute paths, one per line, in the order SVG, PNG, measured JSON, and nothing after them.
- A `MissingGlyph` layout failure (exit 1) prints `error <LayoutError display>` on stdout, followed by `stencil <command>: checks not run`. `render` writes no files in this case, because layout runs before any write.

Exit codes:

| Code | Meaning | Examples |
|---|---|---|
| 0 | clean | every check passed; render wrote its files; schema or prime printed; `--help` or `--version` printed |
| 1 | defects in the document | invalid JSON, a serde type error, a vet violation, a failing check, a zero-examined check, `MissingGlyph`, content that overflows |
| 2 | could not run | bad arguments, unknown subcommand or unknown prime topic, unreadable input file, output directory not creatable or not writable, bundled font verification failure, resvg rejecting generated SVG, `TextNotRendered`, a canvas above the PNG pixel budget (section 5.3), any internal fault in the table below |

Every error variant maps to one code. The mapping is an exhaustive `match` with no wildcard arm, so a new variant does not compile until it has a code. The one exception is clap's `ErrorKind`, which is `#[non_exhaustive]`: its match names `DisplayHelp` and `DisplayVersion` and sends every other kind to 2 through a wildcard arm.

| Error | Code |
|---|---|
| clap error of kind `DisplayHelp` or `DisplayVersion` (returned by `try_parse` for `--help` and `--version`); the rendered text (`err.render()`) is written to the `stdout` argument of `run`, never through `err.print()` | 0 |
| clap error of any other kind, unreadable input file (an input over `INPUT_BYTES_MAX` included), `--out-dir` not creatable or not writable, any output write failure (a directory at an output name included) | 2 |
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

This is the g7 customer canvas as data. `examples/g7.json` is maintained by hand. For `cue export ./cue -e customer --out json` to produce this document, `cue/g7.cue` and `cue/stencil.cue` need an update outside the MVP (section 9.2): `grow` and `justify` on `#Container`, no Col wrapper inside the VPC zone, and the VLAN column split into two halves as the gold does.

```json
{
  "title": "Four lines. Two metros. Two regions. Failover sits between the regions.",
  "kicker": "Type 5 · Dedicated Interconnect 99.99% · SOW / deck / exec",
  "lede": "Same topology as the internal canvas. VLAN IDs, EAD, and BGP sit on the hops. A stakeholder following the lines still sees four attachments, not one bundled cable.",
  "foot": "Customer canvas of g7 · same stencil · VLAN, EAD, BGP on both canvases",
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
- Foot: g7's `foot` is one string drawn as one left-aligned text leaf, "Customer canvas of g7 · same stencil · VLAN, EAD, BGP on both canvases". The gold splits it into "Customer canvas of g7 · same stencil" aligned left and "VLAN, EAD, BGP on both canvases" aligned right against the Google Cloud frame's right edge. The Page model has a single `foot` text and no way to express the right-aligned segment; a split foot is a model change outside the MVP.

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
  - Page: title `Dense stress figure`, kicker `Stress · dense layout`, lede `Five rows of cards and pipes inside a service perimeter.`, canvas `internal`, no `foot`, no `width`, and legend entries `gray` `internal call`, `blue` `request path`, `pink` `reply path`, `dash` `failover` and `deny` `blocked egress`, in that order.
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

- `vet examples/g7.json` exits 0 and prints the two model checks with examined 36 and 8.
- `vet` on a copy containing `64512` in a pipe sub exits 1 and prints the defect with its pointer.
- `vet` on a document with no pipes and an empty legend exits 1 and prints `FAILED: nothing examined`.
- `vet` on a document with one vet violation prints its `violation` line and `stencil vet: 1 violation, checks not run`, and exits 1.
- `vet` on malformed JSON exits 1. `vet`, `check` and `render` on a missing file exit 2 with empty stdout, and `render` creates no out-dir. `vet /dev/zero` exits 2 naming the 67,108,864-byte limit, and an input of exactly that many bytes (g7 padded with spaces) is read and vets clean. `vet` and `check` on a 4097-node document exit 1 with `violation nodes-exceeded /body: more than 4096 nodes`. An unknown subcommand or flag exits 2. `--help` and `--version` exit 0, write the text to stdout and write nothing to stderr.
- `render` writes the three files to a temporary directory and prints exactly their three absolute paths in the order SVG, PNG, measured JSON. `--scale 5` exits 2. An `--out-dir` under a read-only directory exits 2.
- `check examples/g7.json` exits 0 and prints the eight lines in `CheckName` order: the five with the counts from section 9.4, both link checks as not applicable, pipes-land with 8 pipe ends and no defect, and the summary `8 checks, 6 passed, 0 failed, 2 not applicable`.
- `check` on a document whose text overflows exits 1 and names the pointer. The document has one Pipe and a matching legend entry, so child-inside-container is the only failing check. The Pipe sits in the body with no Row or Col sibling, so pipes-land is not applicable, and the summary is `8 checks, 4 passed, 1 failed, 3 not applicable`.
- `vet` and `check` on a file whose bytes are not UTF-8 exit 1 and print the `error` line with the line and column of the first invalid byte and `document does not parse, checks not run`.
- `render d/figure.svg --out-dir d` exits 2 and leaves the input unchanged. A symlink planted at `<out-dir>/g7.svg` is replaced by the rendered file, its target keeps its bytes, and no temporary file is left behind. With an old `g7.svg` and a directory at `<out-dir>/g7.png`, `render` exits 2, `g7.svg` keeps its old bytes, no `g7.measured.json` appears and no temporary file is left behind. Two temporary names from one process differ, and a file already at a temporary name fails the write and keeps its bytes.
- A vetted page of 160 stacked Pcards exits 2 from `render --scale 4` with a `PixmapAllocation` message and writes nothing, and renders at `--scale 1`.
- Two separate `stencil render` processes on g7 at the default scale write byte-identical SVG, PNG and measured JSON. `check` on a document with U+4E00 in a Pcard `fn` exits 1 (`MissingGlyph` through `LayoutError::Measure`) and prints an `error` line followed by `stencil check: checks not run`.
- `render` on a document whose legend omits a used kind writes its three files and exits 0, because render runs `validate_page` only.
- `schema` prints JSON equal to `schema/stencil.schema.json`.
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
| `links-avoid-boxes` | one per segment and obstacle pair | a segment crosses an obstacle box, or a tag overlaps a node box other than an ancestor of an endpoint |

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

`Page` gains an optional `projection` field, and the CLI `--projection` flag overrides it. Projection is a render option. `layout_page`, `PageGeometry`, the seven checks of sections 6 and 11.2 and the measured JSON `nodes` are the same for `flat` and `iso`. An `iso` render draws the laid-out body as a 2:1 isometric scene: zones become slabs, leaf blocks become boxes, pipes and links lie on the slab they belong to, and every text run and icon stays upright and screen-aligned so type remains legible. Every rule in sections 1 to 11 still holds unless this section names the change.

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

`CheckName` gains `IsoLabelsClear` as its last variant, `as_str` `iso-labels-clear`, unit `pair` for 1 and `pairs` otherwise.

### 12.2 Projection

All projection code lives in `stencil_render::iso`. It reads `PageGeometry` only and never calls layout or a measurer.

```rust
pub mod iso;

// iso.rs
/// Slab thickness of a Zone, and the rise of each nested Zone over its parent.
pub const ISO_SLAB_THICKNESS_PX: f32 = 6.0;
/// Height of a leaf block (Pcard, Fact, Note, Text, Callout, Frame).
pub const ISO_BLOCK_HEIGHT_PX: f32 = 18.0;
/// cos 30 degrees, written out so every build uses the same f32.
pub const ISO_COS_30: f32 = 0.866_025_4;
/// sin 30 degrees.
pub const ISO_SIN_30: f32 = 0.5;
/// Left margin of the projected body and the sum of both side margins.
pub const ISO_MARGIN_PX: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint { pub x: f32, pub y: f32 }

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
    /// Painter order (section 12.5): body nodes in geometry order, then link tags in link order.
    pub billboards: Vec<Billboard>,
    /// One per `PageGeometry.links` entry: the z of the plane the link lies on.
    pub link_planes: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Solid {
    /// Geometry index of the node.
    pub node: usize,
    pub shape: SolidShape,
    pub base_z: f32,
    /// ISO_SLAB_THICKNESS_PX for a slab, ISO_BLOCK_HEIGHT_PX for a block, 0 for a surface.
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidShape { Slab, Block, Surface }

#[derive(Debug, Clone, PartialEq)]
pub struct Billboard {
    /// The owning node's pointer, or `/links/<i>` for a link tag.
    pub owner: NodePointer,
    /// Geometry index of the owner; None for a link tag.
    pub node: Option<usize>,
    pub role: BillboardRole,
    /// The flat box the billboard redraws, in layout px.
    pub flat: BoxRect,
    pub z: f32,
    /// The drawn box, in canvas px, offset included.
    pub screen: BoxRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillboardRole { Content, Tag }

/// Asserts nodes[0] is the root and /body is present, then projects the body.
pub fn project_page(geometry: &PageGeometry) -> Result<IsoScene, RenderError>;

/// NotApplicable with reason "projection is flat" for None; section 12.7 otherwise.
pub fn iso_labels_clear(scene: Option<&IsoScene>) -> CheckReport;
```

`project_page` returns `RenderError::GeometryMismatch` when `nodes[0]` is not `""` (expected `""`) or when no node has pointer `/body` (expected `/body`, found `/<absent>`).

Rules:

1. A flat point (x, y) at height z projects to `x' = (x - y) * ISO_COS_30 + offset.x` and `y' = (x + y) * ISO_SIN_30 - z + offset.y`, computed in f32 in that order. With this map a larger x runs down and to the right on screen, a larger y runs down and to the left, and a larger z runs straight up.
2. Only `/body` and its descendants are projected. `/kicker`, `/title` and `/lede` are drawn flat at their layout positions. `/legend`, its entries and `/foot` are drawn flat with `footer_shift` added to every y. The page root is the ground plane, z = 0, and draws nothing of its own. Projecting the page-level nodes would put the title on the ground behind the figure and slide the legend into it: text drawn horizontally from a point on the body's front edge enters the body's projection after about 10 px, because that edge descends at 30 degrees.
3. The extent set E holds, before any offset: the four corners of the `/body` border box at z = 0; the six silhouette vertices of every slab and block (section 12.3); both ends of every wire, spine and link segment and every arrowhead vertex; the extreme points of every dot ellipse; and the four corners of every billboard's screen box. Let `min_x`, `max_x`, `min_y` and `max_y` bound E.
4. `offset.x = ISO_MARGIN_PX - min_x` and `offset.y = body.y - min_y`, where `body.y` is the `/body` border box top. The projected body therefore starts at the page margin on the left and at the flat body top.
5. `footer_shift = (max_y - min_y) - body.height`. `canvas.width = max(PageGeometry.canvas.width, (max_x - min_x) + 2 * ISO_MARGIN_PX)` and `canvas.height = PageGeometry.canvas.height + footer_shift`. The header keeps the width it was laid out at, so the drawn canvas is never narrower than the flat one. This is the only place the drawn canvas differs from `PageGeometry.canvas`.
6. The map sends a circle of radius r on a horizontal plane to an axis-aligned ellipse with semi-axes `r * sqrt(1.5)` across and `r * sqrt(0.5)` down, because the map's linear part M has M·Mᵀ = diag(1.5, 0.5).

### 12.3 Solids

A node's zone depth is the number of Zone ancestors it has. The nearest slab top of a node is `ISO_SLAB_THICKNESS_PX * (d + 1)`, where d is the zone depth of its nearest Zone ancestor, or 0 when it has no Zone ancestor.

| NodeTag | Shape | base_z | Height | Faces and top-face drawing |
|---|---|---|---|---|
| Zone | Slab | `6 * zone depth` | 6 | three faces; gcp also draws its Bar part as a band on the top face |
| Pcard, Fact, Note, Text, Callout, Frame | Block | nearest slab top | 18 | three faces; Callout draws its Accent part and Frame its two diagonals on the top face |
| Pipe (Tee arms included), Tee | Surface | nearest slab top | 0 | wire, dots or arrowheads (Pipe); spine (Tee) |
| Row, Col | none | | | nothing: a container adds no depth and draws nothing |

The page-level nodes draw no solid. Nested zones stack: a top-level zone spans z 0 to 6, a zone inside it 6 to 12, and so on.

1. Faces. Every slab and block draws three faces, as `<polygon>` elements in this order: left, right, top. The left face stands on the flat bottom edge (y = bottom) and faces down and to the left on screen. The right face stands on the flat right edge (x = right) and faces down and to the right. The top face is the border box at `base_z + height`. The flat corner radius is not drawn: faces are square.
2. Face paint. The top face takes the node's flat fill: the zone fill of section 2.4 (for gcp the body fill), the card fill for a Pcard and a Text block, the fact fill for a Fact, the callout tint for a Callout. The left and right faces take the same fill shaded by the palette (section 12.6). Every face is stroked with the node's flat border (color, width and line style), and a node with no flat border draws no stroke. A node with no flat fill (vpc, Note, Frame) draws its faces with `fill="none"`; a Note has neither fill nor border and draws no faces, but still occupies its block height for its billboard.
3. Silhouette. The six silhouette vertices of a box with border box (x0, y0, x1, y1), base z0 and top z1 are, in order: (x0, y0, z1), (x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0), (x0, y1, z1). Their projection is the convex hexagon that the three visible faces cover exactly.
4. Top-face drawing. The gcp Bar part, the Callout Accent part and the Frame diagonals are projected onto the top face as polygons and lines at `base_z + height`, in their flat paint.
5. Pipe. The wire is one `<line>` from the start dot center to the end dot center, both at the Pipe's base_z, with the section 5.2 stroke of its kind. It runs under the tag billboard, which masks its middle, so no gap opens between a wire end and an upright tag. At an arrowed end the line stops at the arrowhead base, as in section 2.7. A dot is an `<ellipse>` at the projected dot center with `rx` 4.9 and `ry` 2.83 (r = 4, rule 6 of section 12.2, rounded by section 5.1), painted as the flat dot. An arrowhead is a `<polygon>` of the three projected vertices of the flat arrowhead, filled with the wire color.
6. Tee. The spine is a `<line>` along the projected spine center line at the Tee's base_z. The hub is a billboard.
7. Links. A link lies on the plane at the nearest slab top of the innermost Zone that contains both endpoints, and on the ground (z = 0) when no Zone contains both; `IsoScene.link_planes` holds that z. The routed polyline of section 11.2 is projected point by point and drawn as a `<path>` with the flat stroke. Arrowheads are projected `<polygon>` elements as for pipes. An iso SVG writes no `<marker>` and no `<defs>`, because a marker draws its triangle unprojected. A link is drawn after every node, as in flat, so it paints over faces; its tag is a billboard.

### 12.4 Billboards

A billboard is part of the flat drawing, drawn upright and screen-aligned at a projected position. One billboard is made per node, not per text run: a flat line pitch of 15.6 px projects to 7.8 px of screen height with a 13.5 px shear to the left, so two runs of one node placed independently would overlap each other.

| Owner | Role | Flat box and content | Anchor | z |
|---|---|---|---|---|
| Zone, kind other than gcp | Content | the Label run | top-left | slab top + 18 |
| Zone, kind gcp | Content | a chip: the Label run's box grown by 7 top and bottom and 16 left and right, filled with the gcp bar fill at radius 4, then the Label run | top-left | slab top + 18 |
| Pcard | Content | Icon (with its icon chip), FactBox, AskBox and the FunctionName, ProductName, Fact and Ask runs that are present | top-left | block top |
| Fact, Note | Content | the Text run | top-left | block top |
| Text | Content | Heading, every Marker and every BodyLine | top-left | block top |
| Callout | Content | Heading and Text runs | top-left | block top |
| Frame | Content | LabelChip and the Label run | top-left | block top |
| Pipe, Tee arm | Tag | the Tag part with its TagLabel and TagSub runs | center | base_z |
| Tee | Tag | the Hub part with its HubText run | center | base_z |
| Link with a label | Tag | the link Tag with its runs | center | link plane |

1. Flat box. A run's box is `(part.x + align offset, part.y, metrics.width_px, metrics.height_px)`, where the align offset is `(part.width - metrics.width_px) / 2` for `TextAlign::Center` and 0 otherwise. The ink width, not the part width, matters for a zone label, whose Label part spans the zone. Every other member contributes its part box. The billboard's flat box is the union of its members' boxes.
2. Anchor. A top-left billboard's screen box has its top-left corner at the projection of the flat box's top-left corner at z. A center billboard's screen box is centered on the projection of the flat box's center at z. In both, the screen box has the flat box's width and height. Text groups anchor top-left because a run drawn horizontally from there moves away from the block's back edge, which descends at 30 degrees. Tags anchor on the center because the wire passes through the projected center.
3. Lift. A zone's billboard sits 18 px above its slab top, at the height of the block tops of its children. At the slab top, the label band's 8 px gap projects to 4 px while the first child block rises 18 px, so the label would cover that block's top face in every zone whose first child is a block.
4. Drawing. Every member is drawn with its section 5.2 flat drawing, translated by `(screen.x - flat.x, screen.y - flat.y)`. Coordinates stay absolute, and no `<g>` carries a transform. The number of `<text>` elements is the same as in the flat render of the same page.
5. The gcp chip exists because the bar band descends at 30 degrees on screen while its label runs horizontally, so white label ink would leave the band within about 30 px. In `wire` the chip is white with a 1.25 px ink border, like a wire tag.
6. The kicker badge and every page-level run are drawn flat (section 12.2, rule 2) and are not billboards.

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
          <polygon points="…" fill="#ACCBF9" stroke="#D7CCC8" stroke-width="1.5"/> …
        </g>
      </g>
    </g>
    <g data-id="/legend" data-tag="Legend"> … flat, shifted … </g>
  </g>
  <g data-id="/links/0" data-tag="Link" data-kind="blue"> … </g>
  <g data-layer="billboards">
    <g data-billboard="/body/0/children/0" data-role="content"> … </g>
  </g>
</svg>
```

The coordinates in this example are illustrative.

- The root `<svg>` and background `<rect>` use `IsoScene.canvas`. The page group carries `data-projection="iso"`; a flat render writes no such attribute.
- There is still one `<g data-id>` per geometry node, nested and ordered as in section 5.2. A body node's group holds its faces, top-face drawing and surface primitives, and no text.
- Link groups follow the page group in link order and hold the projected path and arrowhead polygons.
- The billboard layer is last: one `<g data-billboard="<owner>" data-role="…">` per billboard, with role `content` or `tag`, in `IsoScene.billboards` order. Billboards paint over everything, and `iso-labels-clear` reports where that hides a face.

### 12.6 Shading

```rust
// palette.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face { Top, Left, Right }

impl Palette {
    /// HSL lightness step of a face in percentage points; None in wire (no shading).
    pub fn face_lightness_step(self, face: Face) -> Option<i8>;
    /// The fill of `face` for a node whose flat fill is `base`: base itself for Top, base
    /// shaded by the step for Left and Right, and the page background for every face in wire.
    pub fn face_fill(self, base: &'static str, face: Face) -> Option<String>;
}

/// `color` (`#RRGGBB`) with its HSL lightness moved by `step_points` percentage points.
/// None when `color` is not `#` followed by six hex digits.
pub fn shade(color: &str, step_points: i8) -> Option<String>;
```

| Theme | Top | Left | Right |
|---|---|---|---|
| center | 0 | -8 | -16 |
| dusk | 0 | -4 | -8 |
| wire | no shading: every face is the page background `#FFFFFF`, stroked in `#222222` at 1.25 px |

1. `shade` parses the six hex digits into r, g and b in 0 to 1 (value / 255 in f64), converts to HSL with the standard formulas (hue from the largest channel, `rem_euclid(6.0)` for the red sector, saturation 0 when max equals min), sets `L = clamp(L + step_points / 100, 0, 1)`, converts back, maps each channel to `floor(channel * 255 + 0.5)` clamped to 0 to 255, and writes `#` and six uppercase hex digits. A step of 0 returns the input string unchanged, without a round trip.
2. `face_fill` returns None when `base` does not parse, and the SVG writer returns `RenderError::Svg` naming the color, so no library code unwraps. A unit test parses every color constant in `palette.rs`, so the None path is unreachable in practice.
3. Dusk uses smaller steps because its fills sit near 12 percent lightness: a -16 step clamps them to black and the side faces lose their hue.
4. In `wire`, "no face fills" means no shaded or colored fills. The faces are filled with the page background so hidden edges stay hidden and the scene reads as a line drawing. A node with no flat fill still draws its faces unfilled. Blocks keep their flat line style; slab outlines follow the wire zone borders of section 11.1.

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
| `iso-labels-clear` | render | each unordered pair of billboards, plus each (billboard, block) pair where the block is not the billboard's owner | two billboard screen boxes overlap, or a billboard screen box overlaps a block's projected silhouette | 0.01 px |

`CheckName::unit` for `iso-labels-clear` is `pair` for 1 and `pairs` otherwise.

1. The report is `CheckReport::not_applicable(CheckName::IsoLabelsClear, "projection is flat")` for a flat render. An iso render with fewer than two billboards and no (billboard, block) pair examines 0 and fails, as section 6 requires.
2. Overlap is a separating-axis test over the two convex shapes: the x and y axes, plus the unit normals of the six silhouette edges for a block. Two shapes overlap when their projections on every axis overlap by more than `GEOMETRY_EPSILON_PX`. For two billboards this reduces to overlap width and height both above the epsilon, as in `siblings-do-not-overlap`.
3. Leaf blocks own no descendants, so the exemption covers only a node's billboard over its own block. A zone's billboard over a block inside that zone is a defect: section 12.4 rule 3 exists so that it does not happen.
4. Defect pointer: for two billboards, the owner of the later one in `IsoScene.billboards` order; for a billboard and a block, the billboard's owner. Messages give the screen box with 2 decimals, for example `iso-labels-clear /body/0/children/0/children/1: content billboard 112.40,260.80 179.00x28.00 overlaps content billboard /body/0/children/0/children/0` and `iso-labels-clear /body/0/children/1: tag billboard 79.20,164.90 60.87x30.60 covers block /body/0/children/0/children/1`. The numbers in these two messages are illustrative.
5. Pairs are visited in billboard order, then block order, so the defect list is deterministic. The loops are bounded by NODES_MAX + LINKS_MAX billboards and NODES_MAX blocks.

The seven checks of sections 6 and 11.2 run on the flat geometry in both projections.

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
  "billboards": [
    { "height": 14.4, "id": "/body/0/children/0", "role": "content", "width": 45.2, "x": 20, "y": 110.9 },
    { "height": 30.6, "id": "/body/0/children/1/children/0", "role": "tag", "width": 60.87, "x": 169.9, "y": 225.5 }
  ],
  "canvas": { "height": 789.1, "width": 1124.5 },
  "footer_shift": 424.3,
  "kind": "iso",
  "offset": { "x": 90.7, "y": 60.8 }
}
```

The numbers in this example are illustrative.

- `billboards` lists every billboard in painter order with its owner pointer and its screen box. Each `id` resolves in `document`, except `/links/<i>`, which resolves to the link.
- The top-level `canvas`, `document`, `nodes` and `links` are the flat ones, so `nodes` is byte-identical between a flat and an iso render of the same page. `projection.canvas` is the drawn canvas.
- Keys are in ascending byte order at every level (section 5.4), and every number goes through `format_number`.
- The projection is theme-independent, so the section 11.1 rule holds under iso: the measured JSON is byte-identical across themes for the same document and projection.

`render_svg(page, geometry)` keeps its signature. When `page.projection` is `Iso` it calls `project_page` and writes section 12.5, otherwise section 5.2. The PNG follows the SVG's `width` and `height`, so an iso PNG is `ceil(canvas * scale)` of `projection.canvas`, and the pixel budget of section 5.3 applies to that canvas.

### 12.9 CLI and CUE

| Command | Change |
|---|---|
| `render` | accepts `--projection flat` or `--projection iso`, which overrides `Page.projection` as `--theme` overrides `Page.theme` |
| `check` | accepts the same flag; runs eight checks, `iso-labels-clear` last |
| `vet` | unchanged; `--projection` is an unknown flag and exits 2 |

- `pipeline::render_page` calls `project_page` once when the effective projection is `Iso` and keeps the result in `RenderedPage.scene: Option<IsoScene>`, which `measured_json` and the check read.
- `pipeline::all_checks(page, geometry, scene: Option<&IsoScene>) -> [CheckReport; 8]`, in `CheckName` order.
- `check` prints eight check lines. A flat page prints `check iso-labels-clear: examined 0 pairs, not applicable: projection is flat`, so the g7 summary becomes `stencil check: 8 checks, 5 passed, 0 failed, 3 not applicable`. Check stdout changes for every document; render outputs do not.
- `cue/stencil.cue` gains `projection?: "flat" | "iso"`.

### 12.10 Example and tests

`examples/hero-iso.json` is a cover-slide figure of 13 body nodes: one on-prem zone with one card, a gutter Col with two pipes, and a gcp zone holding a vpc, a region and a Row of three cards, the first two joined by an arrowed link. The Row gap of 32 leaves room for the arrow. The cards carry no `pn` and no fact, and the pipes sit at the two ends of the gutter (`justify: space-between`), because tall content stacked along y crowds upright labels: a flat pitch of p gives only p / 2 of screen height. The implementer writes the file exactly as below.

```json
{
  "title": "On-prem data reaches three managed services over two private paths.",
  "kicker": "Hybrid platform · cover figure",
  "lede": "One on-prem site, one Google Cloud region, two Interconnect attachments.",
  "width": 1040,
  "canvas": "customer",
  "projection": "iso",
  "body": [
    {
      "tag": "Row",
      "gap": 8,
      "grow": [0, 0, 1],
      "children": [
        {
          "tag": "Zone",
          "kind": "onprem-a",
          "label": "On-prem",
          "children": [
            { "tag": "Pcard", "icon": "hybrid", "fn": "Edge router" }
          ]
        },
        {
          "tag": "Col",
          "justify": "space-between",
          "children": [
            { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "VLAN 1" },
            { "tag": "Pipe", "dir": "h", "kind": "dash", "label": "VLAN 2" }
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
              "label": "Shared VPC",
              "children": [
                {
                  "tag": "Zone",
                  "kind": "region-a",
                  "label": "europe-west4",
                  "children": [
                    {
                      "tag": "Row",
                      "gap": 32,
                      "children": [
                        { "tag": "Pcard", "id": "gateway", "icon": "cloud-run", "fn": "API gateway" },
                        { "tag": "Pcard", "id": "warehouse", "icon": "bigquery", "fn": "Warehouse" },
                        { "tag": "Pcard", "icon": "vertex-ai", "fn": "Model serving" }
                      ]
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
    { "kind": "blue", "text": "primary attachment" },
    { "kind": "dash", "text": "failover attachment" }
  ],
  "links": [
    { "from": "gateway", "to": "warehouse", "kind": "blue", "arrow": "end" }
  ]
}
```

Its `iso-labels-clear` count is 81: 10 billboards (4 zones, 4 cards, 2 pipe tags; the link has no label) give 45 billboard pairs, and 10 billboards against 4 blocks less the 4 own-block pairs give 36. Without `projection` the document passes all seven flat checks. Sections 12.2 to 12.7 applied to its flat geometry give no `iso-labels-clear` defect even with each card's run boxes widened to their part boxes; without the lift of section 12.4 rule 3 they give four, one per zone whose label sits above a card.

Tests:

- stencil-model: `"projection": "iso"` and `"flat"` parse and an absent field gives `Flat`; `"oblique"` is a `ModelError::Json`; a page with `Flat` serializes without the field, so the g7 round trip is unchanged; the schema test passes against the regenerated file and a copy with `"projection": "oblique"` fails schema validation; `CheckName::IsoLabelsClear.as_str()` is `iso-labels-clear` and `unit` gives `pair` and `pairs`.
- stencil-render, projection: with zero offset, (100, 0, 0) projects to (86.60254, 50), (0, 100, 0) to (-86.60254, 50) and (0, 0, 18) to (0, -18). For a one-zone document laid out with `FixedMetricsMeasurer`, `canvas.width` equals the formula of section 12.2 rule 5, the smallest billboard or silhouette x equals 20 within 0.01 px, and the smallest y equals the `/body` top.
- stencil-render, solids: a zone inside a zone has base_z 6; a Pcard in that inner zone has base_z 12 and height 18; a Pipe in a Col in the Row of the body has base_z 0; a Row produces no solid. A link between two cards in one region has plane 18 in the hero; a link between cards in two top-level zones has plane 0.
- stencil-render, flat identity: `render_svg` of g7, `hybrid-ai` and `network-hub-spoke` with the field absent equals the existing center fixtures (the existing test), and with `"projection": "flat"` written in the document the SVG bytes are equal to the absent case. `measured_json(…, None)` is byte-identical to the section 5.4 output.
- stencil-render, SVG: an iso render parses with usvg; it has one `<g data-id>` per geometry node and one `<g data-billboard>` per billboard, in order; no `<marker>` and no `<defs>`; every dot is an `<ellipse>` with `rx="4.9"` and `ry="2.83"`; the billboard layer is the last child of `<svg>`; `text_elements` equals the flat render's; each zone group's first `<polygon>` precedes every descendant group.
- stencil-render, shading: the five test vectors above; a step of 0 returns the input; `shade("#12345", -8)` and `shade("red", -8)` are None; every color constant in `palette.rs` parses; in `wire` every face polygon's fill is `#FFFFFF` or `none`.
- stencil-render, check: a region-a zone holding a Row (gap 32) of two Pcards with icon and one-word `fn` examines 7 pairs (3 billboards give 3 pairs, and 3 billboards against 2 blocks less 2 own give 4) and passes. The same cards in a Col with gap 8 examine 7 and give two defects at the second card: its billboard overlaps the first card's billboard and covers the first card's block. The flat render of either returns the not-applicable report. A hand-built `IsoScene` with two billboards overlapping by 0.02 px is a defect, and touching edges are not.
- stencil-render, measured JSON: for the hero, `nodes` is byte-identical between flat and iso, `projection.billboards` has 10 entries, and the whole output is byte-identical under the three themes while the SVG bytes differ.
- stencil-cli: `check examples/hero-iso.json` under `--theme center`, `dusk` and `wire` exits 0 with `check iso-labels-clear: examined 81 pairs, 0 defects` and `stencil check: 8 checks, 8 passed, 0 failed`. With `--projection flat` it prints the not-applicable line and `8 checks, 7 passed, 0 failed, 1 not applicable`. `render examples/hero-iso.json` under each theme writes three files, and the PNG width is `ceil(projection.canvas.width * 2)`. `render examples/g7.json --projection iso` writes an iso SVG whose measured JSON `nodes` equal the flat run's. `vet --projection iso` exits 2. The existing expectations of section 10 and 11.5 that name `7 checks` become `8 checks` with one more not applicable: the g7 summary in `cli.rs`, the overflow summary (`8 checks, 4 passed, 1 failed, 3 not applicable`), the onepager summary in `golden_onepager.rs` (`8 checks, 7 passed, 0 failed, 1 not applicable`) and any assertion in `theme.rs` that names the check count.

## Conventions

- TigerStyle, adapted: bounded loops over document content, assertions at public entry points and at every external-tool boundary (serde input, cosmic-text output, usvg output), specific names without abbreviations, and a test for every behavior.
- Comments explain why or a non-obvious how. They do not narrate how the code came to be.
- A check that examined nothing fails. A probe that could not run exits 2 and never reports a pass.
