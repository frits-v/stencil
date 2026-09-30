# taffy 0.14 API notes

Crate: `taffy = "=0.14.0"`. Verified by pinning that exact version in a scratch
crate and reading `~/.cargo/registry/src/.../taffy-0.14.0/src/`, then
compiling and running every snippet below (`cargo build` + `cargo run`
against `taffy-0.14.0`). Default features (`std`, `taffy_tree`, `flexbox`,
`flexbox_balance`, `grid`, `block_layout`, `float_layout`, `calc`,
`content_size`, `detailed_layout_info`) cover everything here; no feature
flags need enabling for flex, grid, or measure functions.

Context7's `/websites/rs_taffy` docs describe a different, incompatible
measure-closure shape (5 positional args returning `Size<f32>` directly, and
`min_size: Size<Dimension>`). That does not match 0.14: the signatures below
come from the pinned source and from a compiled, running program, not from
that lookup.

```toml
[dependencies]
taffy = "=0.14.0"
```

## Building a tree

```rust
use taffy::prelude::*;

let mut tree: TaffyTree<()> = TaffyTree::new();

let child = tree.new_leaf(Style {
    size: Size { width: Dimension::from_length(20.0), height: Dimension::from_length(20.0) },
    ..Default::default()
})?;

let root = tree.new_with_children(
    Style { display: Display::Flex, ..Default::default() },
    &[child],
)?;
```

`TaffyTree::new()` has a default capacity of 16 nodes. `new_leaf` takes a
`Style` and returns `TaffyResult<NodeId>`. `new_with_children` takes a
`Style` and a `&[NodeId]` slice of already-created children.

## Flex row/column, gap, padding, border, min sizes, percentage widths

```rust
use taffy::prelude::*;

fn build_flex_row(tree: &mut TaffyTree<()>) -> NodeId {
    let child_a = tree
        .new_leaf(Style {
            size: Size { width: Dimension::from_percent(0.5), height: Dimension::from_length(40.0) },
            min_size: Size { width: LengthPercentageAuto::from_length(80.0), height: LengthPercentageAuto::AUTO },
            ..Default::default()
        })
        .unwrap();

    let child_b = tree
        .new_leaf(Style {
            size: Size { width: Dimension::from_percent(0.5), height: Dimension::from_length(40.0) },
            ..Default::default()
        })
        .unwrap();

    tree.new_with_children(
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            gap: Size { width: LengthPercentage::from_length(12.0), height: LengthPercentage::ZERO },
            padding: Rect::length(10.0),
            border: Rect::length(2.0),
            size: Size { width: Dimension::from_percent(1.0), height: Dimension::AUTO },
            ..Default::default()
        },
        &[child_a, child_b],
    )
    .unwrap()
}
```

Field types in 0.14, confirmed against `src/style/mod.rs`:

- `size`: `Size<Dimension>`. `Dimension` is length, percent, or
  auto. Construct with `Dimension::from_length(px)`, `Dimension::from_percent(fraction)`
  (0.0-1.0, not 0-100), or `Dimension::AUTO`, or the free helpers `length(px)`
  / `percent(fraction)` / `auto()` from `taffy::prelude` (generic over the
  target type via `FromLength`/`FromPercent`/`TaffyAuto`).
- `min_size` and `max_size`: `Size<LengthPercentageAuto>`, not `Size<Dimension>`
  (`style/mod.rs` lines 643 and 646). Same
  constructors (`LengthPercentageAuto::from_length`, `::from_percent`,
  `::AUTO`), different type, because a minimum cannot itself be a
  content-based `auto` the way `size` can express `MinContent`/`MaxContent`.
- `gap`, `padding`, `border`: `Size<LengthPercentage>` / `Rect<LengthPercentage>`.
  `LengthPercentage` has no `auto` variant. `Rect::length(px)` and
  `Rect::zero()` set all four sides at once; build a `Rect { left, right,
  top, bottom }` literal for asymmetric values.
- `flex_direction`: `FlexDirection::Row` / `Column` / `RowReverse` /
  `ColumnReverse`.

`LengthPercentage::ZERO` and `LengthPercentageAuto::AUTO` are associated
consts; `length()`/`percent()`/`auto()` free functions also work anywhere a
concrete target type can be inferred.

### Percentage-width gotcha (verified by running)

A node's own percentage `size` resolves against its parent's *definite*
size, and a flex/grid root has no parent — so leaving the root's own `size`
at the default `Dimension::AUTO` and relying only on the `available_space`
argument to `compute_layout` is not enough for descendants' percentages to
resolve against that available space.

Measured: with the root style at `size: Size::AUTO` and
`compute_layout(root, Size { width: AvailableSpace::Definite(600.0), .. })`,
a child with `width: Dimension::from_percent(1.0)` came out **116px** wide
(shrink-to-fit: it fell back to the sum of its own children's resolved
sizes, because 50%-width grandchildren without a percentage-resolvable
parent width resolved to 0 and only `min_size` saved one of them). Setting
the root style's own `size.width` to `Dimension::from_length(600.0)`
(matching the `compute_layout` available space) made the same percentage
child resolve to the full **600px**, and its 50% grandchildren correctly
split the remaining space (282px each after padding/border/gap).

Rule: for percentage widths to resolve as expected under a fixed root
width, set that width as a definite `Dimension::from_length(...)` on the
root's own `Style`, not only as the `available_space` passed to
`compute_layout`.

## Grid: template rows/columns for a Tee shape

The snippet below demonstrates the grid API on a generic 4-row by 3-column
shape. It is not stencil's Tee. Stencil's Tee is a 2-column by 3-row grid,
`[length(14), fr(1)]` by `[fr(1), auto, fr(1)]`, with the spine at
`grid_row: Line { start: line(1), end: line(4) }`; build it from SPEC section
2.8, not from this code.

A Tee: one spine column spanning the full grid height, and two arm rows
that each place a leaf on either side of the spine (e.g. a
plumbing-tee-style connector with branches at two heights). Built as a
4-row x 3-column grid — rows alternate arm-height and flexible spacer,
columns are flexible-left / fixed-spine / flexible-right:

```rust
use taffy::prelude::*;

fn build_tee_grid(tree: &mut TaffyTree<()>) -> NodeId {
    let spine = tree
        .new_leaf(Style {
            grid_row: Line { start: line(1), end: span(4) },
            grid_column: line(2),
            ..Default::default()
        })
        .unwrap();

    let arm1_left = tree.new_leaf(Style { grid_row: line(1), grid_column: line(1), ..Default::default() }).unwrap();
    let arm1_right = tree.new_leaf(Style { grid_row: line(1), grid_column: line(3), ..Default::default() }).unwrap();
    let arm2_left = tree.new_leaf(Style { grid_row: line(3), grid_column: line(1), ..Default::default() }).unwrap();
    let arm2_right = tree.new_leaf(Style { grid_row: line(3), grid_column: line(3), ..Default::default() }).unwrap();

    tree.new_with_children(
        Style {
            display: Display::Grid,
            size: Size { width: Dimension::from_length(600.0), height: Dimension::from_length(400.0) },
            grid_template_columns: vec![fr(1.0), length(40.0), fr(1.0)],
            grid_template_rows: vec![length(60.0), fr(1.0), length(60.0), fr(1.0)],
            gap: Size { width: LengthPercentage::from_length(4.0), height: LengthPercentage::from_length(4.0) },
            ..Default::default()
        },
        &[spine, arm1_left, arm1_right, arm2_left, arm2_right],
    )
    .unwrap()
}
```

- `grid_template_columns` / `grid_template_rows`: `Vec<GridTemplateComponent<S>>`
  (aliased `GridTrackVec` internally). Track sizing functions: `length(px)`,
  `percent(fraction)`, `fr(weight)`, `auto()`, `min_content()`, `max_content()`,
  `fit_content(length_percentage)`, `minmax(min, max)`.
- `grid_row` / `grid_column`: `Line<GridPlacement<S>>`, a `{ start, end }`
  pair. `line(n)` places a track line (1-indexed, matching CSS Grid).
  `span(n)` places by span count from wherever the other end lands. A single
  line reference alone (just `line(n)` with the default other end) occupies
  one track; `Line { start: line(1), end: span(4) }` spans from line 1
  across 4 tracks, which covers the whole grid height here because this
  snippet's template has exactly 4 rows. In a 3-row template, such as the
  stencil Tee's `[fr(1), auto, fr(1)]`, `span(4)` creates an implicit fourth
  row; use `end: line(4)` or `end: line(-1)` there instead.
- The `grid` module also has `GridAutoFlow`, `grid_template_areas` (named
  areas) and `repeat()` for repeating track lists — not needed for a
  fixed-track Tee layout, but available for auto-placed or repeating grids.

Running this (inside a 600x400 root) placed the two arm rows at y=0 and
y=202 (after the two `fr(1.0)` spacer rows split the remaining height), and
the spine at x=280, spanning the full 400px height in the middle column —
confirmed by the printed `Layout` values, not just by inspection of the
style.

## Leaf nodes with a measure function

0.14's measure-closure signature (confirmed against
`TaffyTree::compute_layout_with_measure` in `src/tree/taffy_tree.rs`):

```rust
pub fn compute_layout_with_measure<MeasureFunction>(
    &mut self,
    node_id: NodeId,
    available_space: Size<AvailableSpace>,
    measure_function: MeasureFunction,
) -> Result<(), TaffyError>
where
    MeasureFunction: FnMut(LayoutInput, NodeId, Option<&mut NodeContext>, &Style) -> LayoutOutput;
```

This is a lower-level signature than a typical "measure text" closure:
it receives a `LayoutInput` (which bundles `known_dimensions`,
`available_space`, `parent_size`, `sizing_mode`, `run_mode`, and more) for
every childless node, with or without a context (the `(_, false)` dispatch
arm in `src/tree/taffy_tree.rs`; nodes with children go to the flex, grid or
block algorithm instead), and must return a full
`LayoutOutput` (size, scrollable-overflow rect, baselines, margin-collapse
info), not just a `Size<f32>`. `NodeContext` here is `TaffyTree`'s generic
context type parameter, i.e. whatever type the tree was created with
(`TaffyTree<MyContext>`) — it is not a fixed type named `NodeContext` in the
crate; that name is this doc's placeholder for the generic.

`taffy::compute_leaf_layout` is the bridge from that low-level shape down to
an ordinary "measure this leaf given known dimensions and available space"
closure:

```rust
pub fn compute_leaf_layout<MeasureFunction>(
    inputs: LayoutInput,
    style: &impl CoreStyle,
    resolve_calc_value: impl Fn(*const (), f32) -> f32,
    measure_function: MeasureFunction,
) -> LayoutOutput
where
    MeasureFunction: FnOnce(Size<Option<f32>>, Size<AvailableSpace>) -> Size<f32>;
```

`resolve_calc_value` only matters if the style uses CSS `calc()` expressions
(the `calc` feature, on by default); pass `|_, _| 0.0` when not using calc.

Full working pattern, context type `LabelContext` holding text to measure:

```rust
use taffy::prelude::*;
use taffy::{LayoutInput, LayoutOutput};

struct LabelContext {
    text: String,
}

fn measure_label(
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    node_context: Option<&mut LabelContext>,
) -> Size<f32> {
    if let (Some(width), Some(height)) = (known_dimensions.width, known_dimensions.height) {
        return Size { width, height };
    }
    let Some(ctx) = node_context else { return Size::ZERO };
    let char_width = 8.0;
    let line_height = 20.0;
    let natural_width = ctx.text.len() as f32 * char_width;
    let max_width = match available_space.width {
        AvailableSpace::Definite(w) => w,
        AvailableSpace::MaxContent => f32::INFINITY,
        AvailableSpace::MinContent => 0.0,
    };
    let width = known_dimensions.width.unwrap_or(natural_width.min(max_width).max(char_width));
    let lines = (natural_width / width.max(1.0)).ceil().max(1.0);
    let height = known_dimensions.height.unwrap_or(lines * line_height);
    Size { width, height }
}

let mut tree: TaffyTree<LabelContext> = TaffyTree::new();
let label = tree.new_leaf_with_context(
    Style::default(),
    LabelContext { text: "spine label".to_string() },
)?;

tree.compute_layout_with_measure(
    label,
    Size { width: AvailableSpace::Definite(600.0), height: AvailableSpace::MaxContent },
    |layout_input: LayoutInput, _node_id: NodeId, node_context: Option<&mut LabelContext>, style: &Style| -> LayoutOutput {
        taffy::compute_leaf_layout(
            layout_input,
            style,
            |_val, _basis| 0.0,
            |known_dimensions, available_space| measure_label(known_dimensions, available_space, node_context),
        )
    },
)?;
```

`compute_layout` (no `_with_measure`) is exactly this with a stub measure
function baked in (`|_, _| Size::ZERO` for every leaf), so any tree that
mixes measured and unmeasured leaves must use
`compute_layout_with_measure` for the whole tree; there's no way to opt a
single leaf out of the tree-wide measure closure.

`new_leaf_with_context(style, context) -> TaffyResult<NodeId>` attaches a
context value to a leaf; `new_leaf(style)` (no context) is for leaves with
no measure-time data (fixed-size boxes, spacers).

## compute_layout with a fixed root width

```rust
tree.compute_layout(
    root,
    Size { width: AvailableSpace::Definite(600.0), height: AvailableSpace::MaxContent },
)?;
```

`AvailableSpace` has three variants: `Definite(f32)`, `MinContent`,
`MaxContent`. A fixed diagram width with unconstrained height (grow to fit
content) is `Definite(width)` / `MaxContent`. See the percentage-width
gotcha above: `Definite(600.0)` here does not by itself make the root
report `width: 600.0` unless the root's own `Style.size.width` also says so
(or the root has no other size-determining content, in which case the
available space is what it falls back to — confirmed in the test program,
the un-styled root did end up 600px wide by fallback, but its *percentage
children* did not resolve against that 600 until the root's own style also
had a definite width).

## Reading the final layout

```rust
let layout: &taffy::Layout = tree.layout(node_id)?;
// layout.location -> Point<f32>, relative to this node's parent
// layout.size     -> Size<f32>
```

Confirmed from `src/tree/taffy_tree.rs`: `TaffyTree::layout()` doc comment
states plainly `unrounded_layout(&self, node) -> &Layout` "Returns this node
layout with unrounded values **relative to its parent**." `layout()` itself
returns the rounded (by default) equivalent, same coordinate convention.

Verified by computing a real tree and checking a grandchild's numbers by
hand: a grid cell reported `location: { x: 0, y: 202 }` while its grid
parent reported `location: { x: 0, y: 64 }` (that parent's own position
inside a flex column above it). Summing parent chains gave `{ x: 0, y: 266
}`, which matched a second, independent computation done by re-deriving
the same value from the tree structure. `Layout` carries no absolute
field — absolute position for rendering must be computed by walking
`NodeId` parents and summing `location` at each level:

```rust
fn absolute_location(tree: &TaffyTree<LabelContext>, node: NodeId) -> Point<f32> {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut current = Some(node);
    while let Some(id) = current {
        let layout = tree.layout(id).unwrap();
        x += layout.location.x;
        y += layout.location.y;
        current = tree.parent(id);
    }
    Point { x, y }
}
```

`tree.parent(node_id) -> Option<NodeId>` walks one level up; there is no
built-in "absolute layout" helper in the crate, so this walk (or an
equivalent recursive tree-walk done once after `compute_layout` and cached)
is required for any renderer.

### Rounding

`TaffyTree::new()` enables rounding by default (`use_rounding: true` in the
tree's `TaffyConfig`), so `tree.layout(node)` returns pixel-snapped
(rounded) values — every example output in this file is an integer for
that reason. `tree.unrounded_layout(node) -> &Layout` (no `Result`, always
present) returns the raw sub-pixel values. Toggle with
`tree.disable_rounding()` / `tree.enable_rounding()`. For an SVG-emitting
diagram engine that wants sub-pixel precision (or wants to do its own
rounding at the very end, after summing parent offsets, rather than at each
level), call `disable_rounding()` before `compute_layout` and read
`unrounded_layout` instead of `layout`.

## Alignment

Confirmed against `src/style/alignment.rs` and `src/style/mod.rs` in the
pinned source, and by compiling the snippet below against `taffy-0.14.0`.
`Style` is generic in 0.14, so a `Style` literal not passed straight to
`new_leaf` or `new_with_children` needs the `: Style` annotation.

In 0.14 the alignment types are structs, not enums. Each holds a `keyword`
and a `safety: AlignmentSafety`, and the CSS values are UPPER_SNAKE_CASE
associated consts. `AlignItems::Stretch` and `JustifyContent::SpaceBetween`
do not compile.

| Style field | Type | Alias of |
|---|---|---|
| `align_items` | `Option<AlignItems>` | |
| `align_self` | `Option<AlignSelf>` | `AlignItems` |
| `justify_self` | `Option<AlignSelf>` | `AlignItems` |
| `justify_items` | `Option<AlignItems>` | |
| `justify_content` | `Option<JustifyContent>` | `AlignContent` |
| `align_content` | `Option<AlignContent>` | |

`AlignItems` consts: `START`, `END`, `FLEX_START`, `FLEX_END`, `SELF_START`,
`SELF_END`, `CENTER`, `BASELINE`, `STRETCH`, and `SAFE_` variants of the
position keywords. `AlignContent` (and so `JustifyContent`) consts: `START`,
`END`, `FLEX_START`, `FLEX_END`, `CENTER`, `STRETCH`, `SPACE_BETWEEN`,
`SPACE_EVENLY`, `SPACE_AROUND`, and `SAFE_` variants of the position keywords.

```rust
use taffy::prelude::*;

let card: Style = Style {
    display: Display::Flex,
    flex_direction: FlexDirection::Row,
    align_items: Some(AlignItems::CENTER),
    justify_content: Some(JustifyContent::SPACE_BETWEEN),
    ..Default::default()
};
let arm: Style = Style {
    align_self: Some(AlignSelf::CENTER),
    justify_self: Some(AlignSelf::STRETCH),
    ..Default::default()
};
```

Stencil's `Justify` maps to `JustifyContent` as follows. `START` is used
instead of `FLEX_START`; the two place items identically in the
non-reversed flex directions stencil uses.

| `Justify` | `JustifyContent` |
|---|---|
| `Start` | `JustifyContent::START` |
| `Center` | `JustifyContent::CENTER` |
| `End` | `JustifyContent::END` |
| `SpaceBetween` | `JustifyContent::SPACE_BETWEEN` |

Safety: every non-`SAFE_` const carries `AlignmentSafety::Unsafe`, the
default, and stencil uses only those. With `Unsafe`, a centered item that
is larger than its container stays centered and overflows both edges. With
`Safe`, it would fall back to start alignment and overflow only the end
edge. Stencil wants the unsafe behavior: the overflow is visible and the
child-inside-container check reports it. taffy treats `Safe` paired with
`STRETCH`, `BASELINE` or a `SPACE_` keyword the same as `Unsafe`.

A `None` `align_items` on a flex container behaves as stretch. Stencil
still sets it explicitly on every container, the root included.

## Blockers

None. Every feature the task named (flex row/column with gap, padding,
border, min sizes, percentage widths, grid template rows/columns for a Tee
spine + two arm rows, leaf measure functions with a generic node context,
`compute_layout` with a fixed root width, and reading final layout with a
parent-relative `location`) exists in 0.14.0 and is demonstrated above with
snippets taken from, or verified by running against, that exact pinned
version.
