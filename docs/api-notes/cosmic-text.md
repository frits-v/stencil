# cosmic-text 0.19 for FILE-fonts-only measuring and shaping

Pin: `cosmic-text = "0.19.0"` (verified against `cosmic-text-0.19.0` from crates.io, `rust-version = "1.89"`, transitively pulling `fontdb 0.23.0` and re-exporting it as `cosmic_text::fontdb`). Do not add a separate `fontdb` dependency: crates.io currently offers `fontdb 0.24.0`, and a version mismatch between a direct `fontdb` dependency and cosmic-text's re-export produces a type error on `Database` (`cosmic_text::fontdb::Database` and a top-level `fontdb::Database` from a different `fontdb` version are different types).

Default features are `["std", "swash", "fontconfig"]`. `fontconfig` only affects `Database::load_system_fonts()`, which this workflow never calls; it is a no-op on macOS and pulls in a pure-Rust config parser (no native linking) on Linux. `swash` is needed for `SwashCache` (rasterization to alpha masks / RGBA, or `get_outline_commands` for vector output) but not for measuring or emitting `<text>`/`<tspan>` elements with glyph advances alone. A pure measuring/shaping build can use `default-features = false, features = ["std"]`.

## FontSystem with only bundled font files, no system fonts

`FontSystem::new()` calls `load_system_fonts()` and does OS locale detection; avoid it entirely for a reproducible, FILE-fonts-only pipeline. Build a `fontdb::Database` by hand and never call `load_system_fonts` or `load_fonts_dir` on it:

```rust
use cosmic_text::{fontdb, FontSystem};

let mut db = fontdb::Database::new();
for path in ["fonts/Inter-Regular.ttf", "fonts/Inter-SemiBold.ttf", "fonts/Inter-Bold.ttf"] {
    let bytes = std::fs::read(path)?;
    db.load_font_data(bytes); // Source::Binary, owned Vec<u8>, no unsafe
}

let mut font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);
```

`load_font_data` stores the font as `fontdb::Source::Binary`. The alternative, `db.load_font_file(path)`, stores it as `fontdb::Source::File(path)` and requires the `fs` fontdb feature (on by default via cosmic-text's `std` feature). Both work end to end: `FontSystem::get_font` promotes a `Source::File` face to `Source::SharedFile` on first use via `unsafe { self.db.make_shared_face_data(id) }` (memory-mapped), before constructing the shapeable `Font`. Verified by tracing a patched local build of `cosmic-text-0.19.0`: the `Source::File` rejection arm in `Font::new` (`font/mod.rs`) is unreachable through the normal `get_font` path; it only fires for direct low-level callers of `Font::new` that bypass `FontSystem`. For a handful of bundled font files read once at startup, `load_font_data(std::fs::read(path)?)` is simpler and has no unsafe in the call path; prefer it unless memory-mapping large font files matters for footprint.

Verify no system fonts leaked in:

```rust
assert_eq!(font_system.db().faces().count(), <expected count>);
```

## Attrs, Metrics, Buffer

```rust
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Weight};

let attrs = Attrs::new()
    .family(Family::Name("Inter"))
    .weight(Weight::SEMIBOLD); // constants: THIN 100 … BLACK 900, SEMIBOLD 600

let metrics = Metrics::new(16.0, 22.0);      // font_size px, line_height px
// or: Metrics::relative(16.0, 1.4)          // line_height = font_size * 1.4

let mut buffer = Buffer::new(&mut font_system, metrics);
let mut buf = buffer.borrow_with(&mut font_system);

buf.set_size(Some(400.0), None);             // max width for wrapping, no height clip
buf.set_text("some text", &attrs, Shaping::Advanced, None);
buf.shape_until_scroll(false);               // borrow_with form takes only `prune`
```

Without `borrow_with`, the plain form takes the font system explicitly: `buffer.shape_until_scroll(&mut font_system, false)`.

`Family::Name` must match the family string fontdb extracted from the font's name table, not the PostScript name. Verified per file:

| File | `face.families` | `face.weight` |
|---|---|---|
| Inter-Regular.ttf | `Inter` | 400 |
| Inter-SemiBold.ttf | `Inter` | 600 |
| Inter-Bold.ttf | `Inter` | 700 |
| InterVariable.ttf | `Inter Variable` | 400 (default instance) |

`Shaping::Basic` is a faster ASCII/Latin-1 path with no ligatures or bidi; `Shaping::Advanced` runs full harfrust shaping (ligatures, bidi, complex scripts, GPOS/GSUB). Use `Advanced` for anything that isn't guaranteed plain ASCII.

## Wrapping width vs. height clipping

`set_size(width, height)` takes two independent `Option<f32>`. `width` drives wrapping. `height` bounds `shape_until_scroll`, which stops shaping additional lines once accumulated height exceeds it — this is a viewport/scroll optimization, not a content limit. For measuring the full intrinsic extent of a text block, pass `None` for height. Passing `Some(height)` silently truncates the line count.

Verified with a 10-line wrapped paragraph at `Metrics::new(16.0, 22.0)`, width 100px:

```
set_size(Some(100.0), None)        -> layout_runs().count() == 10
set_size(Some(100.0), Some(40.0))  -> layout_runs().count() == 2
```

`Wrap::WordOrGlyph` is the default (word wrap, falling back to glyph-level breaks for a word that can't fit on its own line). `Wrap::None`, `Wrap::Glyph`, `Wrap::Word` are also available via `buf.set_wrap(...)`.

## Measuring: line count, max line width, total height

```rust
let mut line_count = 0usize;
let mut max_line_w = 0.0f32;
let mut total_height = 0.0f32;

for run in buf.layout_runs() {
    line_count += 1;
    max_line_w = max_line_w.max(run.line_w);
    total_height = total_height.max(run.line_top + run.line_height);
}
```

`LayoutRun` fields used here: `line_i` (original text line index), `line_y` (baseline offset), `line_top` (top-of-line offset), `line_height`, `line_w` (visual line width after wrapping), `glyphs: &[LayoutGlyph]`, `text: &str`, `rtl: bool`. With fixed `Metrics`, `total_height` also equals `line_count as f32 * metrics.line_height`, but summing `line_top + line_height` from the last run is correct even if per-span `Attrs::metrics` overrides line height for individual runs.

## Glyph positions for exact SVG x-offsets

`LayoutGlyph` fields relevant to placement:

```rust
pub start: usize,       // byte start of cluster in the original line
pub end: usize,         // byte end of cluster
pub font_size: f32,
pub x: f32,             // resolved X offset within the line (pen position after prior advances)
pub y: f32,             // resolved Y offset within the line
pub x_offset: f32,      // additional GPOS offset, in em units, added to x
pub y_offset: f32,      // additional GPOS offset, in em units, added to y
```

`glyph.x` already accounts for accumulated advances and kerning; `x_offset`/`y_offset` are extra per-glyph adjustments from GPOS (diacritic placement, mark attachment) expressed as a fraction of the em, and were 0.0 in all plain-Latin cases tested here. For a vector writer that wants sub-pixel-exact positions (no pixel-grid rounding), reproduce the same math `LayoutGlyph::physical()` uses internally, without its `scale` and `truncf` (pixel-snap for the raster cache):

```rust
let exact_x = run_left + glyph.x + glyph.font_size * glyph.x_offset;
let exact_y = run.line_y + glyph.y - glyph.font_size * glyph.y_offset;
```

`run_left` is 0.0 unless the caller offsets the whole buffer horizontally. `run.line_y` is the baseline; use it directly as the SVG `<text y="...">` value if emitting one `<tspan>` per run with per-glyph `x` deltas, or as the anchor for per-glyph `<text x="{exact_x}" y="{exact_y}">` elements.

If rasterizing instead of emitting vector glyph outlines, `LayoutGlyph::physical((offset_x, offset_y), scale)` returns a `PhysicalGlyph { cache_key, x: i32, y: i32 }` with pixel-grid-snapped integer coordinates, meant for `SwashCache` lookups, not for exact vector placement.

## Variable font weight axis

cosmic-text resolves the `wght` variation axis at three points, all confirmed against `InterVariable.ttf`:

1. Font matching (`font/system.rs`, `FontMatchKey::new`): when a candidate face has a `wght` axis whose range covers the requested `Attrs::weight`, that candidate is preferred over a non-matching static weight, via a `variable_weight_match` field ordered after `font_weight_diff` in `FontMatchKey`'s derived `Ord`. In a database that mixes static weights and the variable font, a request for an intermediate weight (say 650) can still resolve to the nearest static face if `font_weight_diff` is smaller than the variable face's diff from its own default instance. Bundling `InterVariable.ttf` alone avoids this ambiguity and gives every weight 100 to 900 from one file.
2. Metrics and shaping (`font/mod.rs`, `Font::new`): builds a variation `location` from `[(Tag::new(b"wght"), weight.0 as f32)]` via skrifa, so advance widths and metrics reflect the requested weight, not just the font's default instance.
3. Rasterization (`swash.rs`): clamps the requested weight to the axis's `min_value()..max_value()` and applies `normalized_coords` before rendering, so glyph outlines match the requested weight.

Verified end to end: shaping `"Weight axis test"` at `Metrics::new(16.0, 22.0)` against `InterVariable.ttf` alone produced different `line_w` per requested weight (350 -> 118.23px, 600 -> 123.34px, 800 -> 127.26px), confirming real per-weight advance-width interpolation, not a fixed default-instance width.

## Inter font (OFL)

Latest release: `v4.1`, published 2024-11-16.

- Release page: `https://github.com/rsms/inter/releases/tag/v4.1`
- Zip: `https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip` (33.7 MB)
- License: SIL Open Font License 1.1, `LICENSE.txt` at the zip root. Ship this file alongside the bundled fonts; OFL requires it to accompany redistributed copies.

Files inside the zip relevant here:

- `InterVariable.ttf` (root, ~880 KB) — variable font, upright, `wght` axis 100-900. Recommended: bundle this file alone. It covers every weight the SVG writer needs (Regular/SemiBold/Bold and anything in between) from a single 880 KB file, and cosmic-text's weight-axis handling (above) makes the static files redundant.
- `InterVariable-Italic.ttf` (root) — italic variable font, not needed unless italic text is in scope.
- `extras/ttf/Inter-Regular.ttf`, `extras/ttf/Inter-SemiBold.ttf`, `extras/ttf/Inter-Bold.ttf` (~411-420 KB each) — static weights, if a fixed three-weight set is preferred over the variable font. Combined size (~1.25 MB) exceeds the single variable file.
- `Inter.ttc` (root, font collection, all static weights in one file) and `extras/otf/*.otf`, `extras/woff-hinted/*.woff2` — not relevant to a Rust/cosmic-text pipeline.

## Blockers

- `/Users/frits/src/stencil` has no Cargo project yet (only `mise.toml`, `assets/`, `examples/` at the time of writing); nothing in the repo consumes this API yet, so these notes cannot be validated against the target crate's actual usage until that project exists.
- This file was written directly into the main checkout of `/Users/frits/src/stencil`, not a git worktree, per the task's explicit destination path. Per repository git-hygiene practice the main checkout should stay read-only; flagging rather than refusing since the path was given explicitly and no commit was made.
- Family-name strings (`Inter` for statics, `Inter Variable` for the variable font) are specific to this Inter release; if a different font or a future Inter release changes name-table strings, `Family::Name` matching must be re-verified rather than assumed.
