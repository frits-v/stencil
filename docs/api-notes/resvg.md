# resvg 0.48 / usvg 0.48 / tiny-skia 0.12 API notes

Crates: `resvg = "0.48.1"`, `usvg = "0.48.1"`, `tiny-skia = "0.12.0"`. Verified
by pinning those versions in a scratch crate, reading
`~/.cargo/registry/src/.../{resvg,usvg,fontdb}-<version>/src/`, generating
`cargo doc`, and compiling and running a harness against the 23 icon SVGs in
`stencil/icons/` plus synthetic text and clip-path probes. Context7's
`/linebender/resvg` docs matched the pinned source for the render/scale/font
APIs used below; nothing here contradicts that lookup, but the font-resolver
and feature-flag findings needed the source and a running binary to confirm.

```toml
[dependencies]
resvg = "0.48.1"
tiny-skia = "0.12.0"
```

`usvg` does not need a direct dependency: `resvg` re-exports it as
`resvg::usvg` (and `resvg::tiny_skia` re-exports `tiny-skia` the same way, if
a direct `tiny-skia` dependency is skipped). The snippets below keep
`tiny-skia` as a direct dependency, matching the Cargo.toml above, and use
`resvg::usvg` for the parts `resvg` re-exports. Depending on `usvg`/`fontdb`
directly as well is only needed to force extra Cargo features onto the shared
instance (see Feature flags below).

## Rendering an SVG string to PNG at a device scale factor

```rust
use resvg::usvg::{self, Options};

fn render_at_scale(svg_text: &str, options: &Options, scale: f32) -> tiny_skia::Pixmap {
    let tree = usvg::Tree::from_str(svg_text, options).expect("parse SVG");
    let size = tree.size();
    let width = (size.width() * scale).round() as u32;
    let height = (size.height() * scale).round() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("non-zero size");
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
}
```

`Options::dpi` (default 96.0) controls unit conversion for absolute lengths
(`pt`, `mm`, etc. in the source SVG), not pixel density. Device scale is
purely the `Transform` passed to `resvg::render` together with the pixmap
dimensions computed from it; there is no separate "device scale factor" knob
on `Options` or `Tree`.

`Pixmap::save_png`/`encode_png` require the `png-format` feature, on by
default. `Pixmap::pixels()` exposes `&[PremultipliedColorU8]` for a
non-white-pixel check: fill the pixmap white before rendering, then treat any
pixel whose premultiplied RGB is not `(a, a, a)` for its own alpha as
"something was painted." A truly blank render (all-white SVG) and a solid-fill
render were both checked against this predicate as a self-test before trusting
it on the icon set.

## Icon compatibility: data-URI embedding vs. direct render

Wrapper used for the data-URI path:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <image href="data:image/svg+xml;base64,{base64-of-icon-svg}" x="0" y="0" width="512" height="512"/>
</svg>
```

All 23 icons in `stencil/icons/` are `viewBox="0 0 512 512"` Illustrator
exports: an XML comment (`<!-- Generator: Adobe Illustrator ... -->`), a
`version="1.1"` attribute on the root `<svg>`, ids on the root and on
`<g>` groups (`standard_product_icon`/`Standard_Product_Icon`,
`bounding_box`, `art`), and 9 of the 23 carry a `<defs><style>` block with CSS
classes (`.st0 { fill: none; }`) instead of inline `fill` attributes. None use
`clip-path`.

| icon | direct renders | direct blank | wrapped renders | wrapped blank | warnings | notable features |
|---|---|---|---|---|---|---|
| agents | yes | no | yes | no | none | comment, id, version |
| ai-ml | yes | no | yes | no | none | + css classes |
| bigquery | yes | no | yes | no | none | + css classes |
| cloud-run-flat | yes | no | yes | no | none | + css classes |
| cloud-run | yes | no | yes | no | none | + css classes |
| cloud-sql | yes | no | yes | no | none | + css classes |
| cloud-storage | yes | no | yes | no | none | + css classes |
| compute-engine | yes | no | yes | no | none | + css classes |
| compute | yes | no | yes | no | none | comment, id, version |
| containers | yes | no | yes | no | none | comment, id, version |
| data-analytics | yes | no | yes | no | none | comment, id, version |
| databases | yes | no | yes | no | none | + css classes |
| devops | yes | no | yes | no | none | comment, id, version |
| gke | yes | no | yes | no | none | + css classes |
| hybrid | yes | no | yes | no | none | comment, id, version |
| integration | yes | no | yes | no | none | comment, id, version |
| networking | yes | no | yes | no | none | comment, id, version |
| observability | yes | no | yes | no | none | comment, id, version |
| scc | yes | no | yes | no | none | comment, id, version |
| security-identity | yes | no | yes | no | none | comment, id, version |
| serverless | yes | no | yes | no | none | comment, id, version |
| storage | yes | no | yes | no | none | comment, id, version |
| vertex-ai | yes | no | yes | no | none | comment, id, version |

23 of 23 icons rendered non-blank with zero warnings in both modes, at 2x
scale, PNG dimensions 1024x1024. A visual spot check (agents.svg through the
data-URI path) confirmed the vector output is crisp at 2x, not upscaled from a
1x raster.

Root-level id collisions between the wrapper document and the embedded icon
document (both use `art`, `bounding_box`, `standard_product_icon`) do not
matter for the data-URI `<image>` path: `usvg::Tree::from_data_nested` (used
internally to parse a `data:image/svg+xml` payload inside
`<image href="...">`) parses the embedded SVG as its own independent
`Tree`/id namespace, not merged into the parent document's. A `url(#...)`
reference inside one icon embedded this way can only ever resolve against ids
declared in that same icon.

This isolation is a property of the `<image>` embedding path specifically,
not of SVG in general: concatenating two of these icons directly into one
document (copying their `<g>` markup into a shared `<svg>` root rather than
wrapping each in its own `<image>`) puts both icons' ids into a single
namespace, and all 23 icons reuse the same three ids
(`standard_product_icon`/`Standard_Product_Icon`, `bounding_box`, `art`). None
of the 23 currently declare a `url(#...)` paint-server or clip-path
reference, so direct concatenation has not produced an observable collision
yet, but the id reuse means it is one `url(#gradient-id)` away from doing so.
Preferring the data-URI `<image>` path over inlining sidesteps this
entirely.

Nested `<image>` SVGs also inherit the parent's `fontdb` and `font_resolver`
(so a shared fontdb setup applies to icons embedded this way), but external
file-path image references inside a nested SVG are disabled
(`resolve_string` is forced to always return `None`), and `font_family`
falls back to usvg's default (`Times New Roman`) rather than the outer
`Options::font_family`, since `from_data_nested` rebuilds most of `Options`
from `Options::default()`. None of the 23 icons contain `<text>`, so this
does not affect them; it would matter if an icon set ever embedded labels.

A synthetic `clipPath`/`clip-path` probe (none of the 23 icons exercise this)
rendered correctly with no warnings, confirming resvg does support
`clip-path`, it is just unused by this icon set.

## Fonts: a fontdb built only from font files, and detecting a missing family

```rust
use resvg::usvg::{self, Options};
use std::sync::Arc;

let mut db = resvg::usvg::fontdb::Database::new();
db.load_font_file("/path/to/Inter-Regular.ttf")?;

let mut options = Options::default();
options.fontdb = Arc::new(db);
```

`fontdb::Database::new()` starts empty: no system fonts are scanned unless
`load_system_fonts()` is called. With only Inter loaded, `<text
font-family="Inter">` rendered correctly and logged no warning. `<text
font-family="Nonexistent Family XYZ">` rendered fully blank (no glyphs) and
usvg's default font selector logged `No match for '"Nonexistent Family XYZ"'
font-family.` at `log::Level::Warn` via the `log` crate. Wiring a `log::Log`
implementation and checking for `blank == true && a "No match for" warning
was logged` is a workable detection signal for the default resolver, but it
has a gap described below.

### The generic-family footgun

usvg's `FontResolver::default_font_selector` (`usvg::text::FontResolver`)
appends `fontdb::Family::Serif` to every font query as a last-resort generic,
regardless of what family the `<text>` element requested:

```rust
// from usvg's default_font_selector, roughly:
name_list.push(fontdb::Family::Serif);
let id = fontdb.query(&fontdb::Query { families: &name_list, .. });
```

`fontdb::Database::new()` maps the `serif` generic to the literal name
`"Times New Roman"` by default, which will not match a font-file-only
database unless something explicitly points it there. But if the fontdb is
ever built with `db.set_serif_family("Inter")` (a natural thing to do on a
single-font database, to give unstyled text somewhere to land), a missing
family silently renders in Inter: verified by re-running the missing-family
case with `set_serif_family("Inter")` set, which rendered non-blank with zero
warnings. Log-message detection cannot see this case, because
`fontdb.query()` legitimately returns `Some` via the generic-family match; no
warning is emitted.

The mechanism that does catch it is a custom `Options::font_resolver`. It has
to replicate `default_font_selector`'s weight/stretch/style mapping (`usvg`
re-exports `FontFamily`, `FontStretch`, `FontStyle` at the crate root, so no
direct `svgtypes` dependency is needed), and must not append the Serif
generic:

```rust
use resvg::usvg::{self, FontResolver};
use std::sync::{Arc, Mutex};

fn to_fontdb_family(f: &usvg::FontFamily) -> fontdb::Family<'_> {
    match f {
        usvg::FontFamily::Named(s) => fontdb::Family::Name(s),
        usvg::FontFamily::Serif => fontdb::Family::Serif,
        usvg::FontFamily::SansSerif => fontdb::Family::SansSerif,
        usvg::FontFamily::Cursive => fontdb::Family::Cursive,
        usvg::FontFamily::Fantasy => fontdb::Family::Fantasy,
        usvg::FontFamily::Monospace => fontdb::Family::Monospace,
    }
}

fn to_fontdb_stretch(s: usvg::FontStretch) -> fontdb::Stretch {
    match s {
        usvg::FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
        usvg::FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
        usvg::FontStretch::Condensed => fontdb::Stretch::Condensed,
        usvg::FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
        usvg::FontStretch::Normal => fontdb::Stretch::Normal,
        usvg::FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
        usvg::FontStretch::Expanded => fontdb::Stretch::Expanded,
        usvg::FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
        usvg::FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
    }
}

fn to_fontdb_style(s: usvg::FontStyle) -> fontdb::Style {
    match s {
        usvg::FontStyle::Normal => fontdb::Style::Normal,
        usvg::FontStyle::Italic => fontdb::Style::Italic,
        usvg::FontStyle::Oblique => fontdb::Style::Oblique,
    }
}

let misses: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
let misses_for_closure = Arc::clone(&misses);

options.font_resolver = FontResolver {
    select_font: Box::new(move |font, fontdb| {
        let names: Vec<_> = font.families().iter().map(to_fontdb_family).collect();
        let query = fontdb::Query {
            families: &names,
            weight: fontdb::Weight(font.weight()),
            stretch: to_fontdb_stretch(font.stretch()),
            style: to_fontdb_style(font.style()),
        };
        // No Serif (or any generic) append here, unlike the default selector.
        let id = fontdb.query(&query);
        if id.is_none() {
            misses_for_closure.lock().unwrap().push(
                font.families().iter().map(|f| f.to_string()).collect::<Vec<_>>().join(", "),
            );
        }
        id
    }),
    select_fallback: FontResolver::default_fallback_selector(),
};
```

Run against the same missing-family case, this resolver reported the miss
(`recorded_misses = ["Nonexistent Family XYZ"]`) and rendered blank, because
it never appended the Serif generic. This is the structural detection point:
querying at the API level, inside the resolver that usvg actually calls,
rather than string-matching whatever the default resolver happens to log.

Skipping the weight/stretch/style replication is its own footgun, distinct
from the generic-family one: a resolver that hardcodes
`weight: fontdb::Weight::NORMAL` instead of `fontdb::Weight(font.weight())`
will match `font-weight="bold"` against an Inter Regular-only database,
verified by rendering `font-weight="bold"` under such a resolver: it produced
non-blank output and recorded zero misses, i.e. it silently substituted the
wrong weight and the detection resolver never saw a miss to report.

`select_font` and `select_fallback` are two separate substitution paths.
`select_font` runs once per `<text>` run to pick the family; the resolver
above only guards that path. `select_fallback` runs per-glyph, when a chosen
font's charmap does not cover a specific character, and by default searches
the whole `fontdb` for any face (regardless of requested family) that does
cover it, logging `Fallback from {a} to {b}.` at warn. With a single loaded
font this path is inert; with two or more it is live and needs the same
API-level guard. Setting `select_fallback` to `Box::new(|_, _, _| None)`
disables it outright: a character outside the loaded font's charmap is then
simply dropped from the render (verified: an ASCII string with one
CJK codepoint appended still rendered the ASCII part and produced no
substitution for the missing glyph), rather than picked up from another
loaded face.

### The method stencil uses: counting text nodes

Stencil keeps the default resolver and detects a dropped `<text>` by
counting `usvg::Node::Text` nodes in the parsed tree (SPEC section 5.3).
When a `<text>` element's family does not resolve, usvg 0.48 drops the
element without an error and pushes no `Node::Text`, so a missing family
shows up as one node fewer than the number of `<text>` elements written.
Verified with usvg 0.48.1 on a fontdb holding only the bundled Inter
faces: a one-line SVG naming `Inter` parsed to 1 text node, and the same
SVG naming `Helvetica Neue` parsed to 0. The generic-family footgun above
still applies: the count works only while no generic family is pointed at
Inter (`set_serif_family` and its siblings are never called), because the
Serif fallback would then resolve the missing family. The custom resolver
in this section is an alternative stencil does not build.

## Feature flags

`resvg`'s and `usvg`'s default features: `svgz`, `text`, `system-fonts`,
`memmap-fonts`, plus `raster-images` (resvg) / `writer` (usvg). For an
SVG-only, font-file-only pipeline, `raster-images` (`gif`, `image-webp`,
JPEG/PNG decoding) is dead weight unless the source SVGs embed raster
`<image>` data; the 23 icons here are pure vector paths and do not need it.

`usvg`'s `system-fonts` feature is defined as `fontdb/fs + fontdb/fontconfig`
together: turning it off removes `fontdb::Database::load_font_file` and
`load_fonts_dir` (gated by `fontdb/fs`) along with system font discovery
(`fontdb/fontconfig`), because usvg bundles the two. To keep file loading
while dropping `system-fonts`, add a direct `fontdb` dependency requesting
only `fs`; Cargo's feature unification applies it to the single shared
`fontdb` instance used by `usvg`:

```toml
[dependencies]
resvg = { version = "0.48.1", default-features = false, features = ["text"] }
tiny-skia = "0.12.0"
fontdb = { version = "0.24.0", default-features = false, features = ["fs"] }
```

This combination compiles and `load_font_file` works. It does not, however,
make `load_system_fonts()` unavailable: `fontdb::Database::load_system_fonts`
is gated only by the `fs` feature (needed for file loading regardless), and
on macOS and Windows its font-directory scanning does not depend on the
`fontconfig` feature at all: `fontconfig` only switches which discovery
mechanism `load_system_fonts()` uses on Linux (fontconfig library vs. a
hardcoded directory list), it never removes the method. Verified by building
with `system-fonts` fully disabled via the config above and calling
`load_system_fonts()` directly: it compiled and ran without error. There is
no Cargo-feature-level way to keep `load_font_file` while making
`load_system_fonts` unavailable or inert; the only guarantee against an
accidental system-font load is not calling it, backed by the custom
`select_font` resolver above to catch the case where a generic-family alias
reopens the same hole.

## Blockers

None found against the current icon set or the font-file/scale-factor
requirements: all 23 icons render, both as direct trees and wrapped in a
`data:image/svg+xml;base64,` `<image>`, at 2x scale, with a font-file-only
fontdb and no silent system-font fallback (given the code-discipline
constraint above, not a Cargo-feature constraint).

Two non-blocking constraints to carry into the design:

- Missing-family detection must not rely on string-matching `log::warn!`
  output, because the default resolver's generic-family fallback (`Serif`
  appended to every query) can silently match a missing family if the
  fontdb's generic-family aliases are ever pointed at a loaded font. A
  custom `Options::font_resolver` is one fix; stencil instead counts
  `Node::Text` nodes with the default resolver and never sets a generic
  family (see "The method stencil uses: counting text nodes").
- `load_system_fonts()` cannot be structurally removed via Cargo features on
  macOS; avoiding it is an invariant to enforce in code (or via a lint/test
  on `Options` construction), not something the dependency graph can
  guarantee.
