//! The section 5.3 PNG pipeline.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::{self, FontResolver, FontStretch, FontStyle, fontdb};
use stencil_text::{BUNDLED_FONTS, FontError, verify_bundled_fonts};

use crate::{DeviceScale, RenderError};

const DEFAULT_FONT_FAMILY: &str = "Inter";

/// Calls verify_bundled_fonts, parses the SVG with a usvg fontdb that holds only
/// BUNDLED_FONTS, asserts the tree holds exactly `expected_text_elements` text nodes
/// (section 5.3), renders at `scale`, and encodes PNG.
pub fn render_png(
    svg: &str,
    expected_text_elements: usize,
    scale: DeviceScale,
) -> Result<Vec<u8>, RenderError> {
    verify_bundled_fonts()?;
    let font_database = bundled_font_database()?;

    let font_misses = Arc::new(AtomicUsize::new(0));
    let options = usvg::Options {
        fontdb: Arc::new(font_database),
        font_family: DEFAULT_FONT_FAMILY.to_string(),
        resources_dir: None,
        font_resolver: strict_font_resolver(Arc::clone(&font_misses)),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(svg, &options).map_err(|error| RenderError::Svg {
        message: error.to_string(),
    })?;

    let found = count_text_nodes(tree.root());
    if found < expected_text_elements {
        return Err(RenderError::TextNotRendered {
            count: expected_text_elements - found,
        });
    }
    if found > expected_text_elements {
        return Err(RenderError::TextCountExceeded {
            expected: expected_text_elements,
            found,
        });
    }
    // A span whose family or glyph did not resolve inside a text node that still placed
    // other glyphs leaves the count intact, so the resolver's own tally is checked too.
    let misses = font_misses.load(Ordering::SeqCst);
    if misses > 0 {
        return Err(RenderError::TextNotRendered { count: misses });
    }

    let scale_factor = f32::from(scale.get());
    let size = tree.size();
    let width = pixel_extent(size.width(), scale_factor);
    let height = pixel_extent(size.height(), scale_factor);
    let mut pixmap =
        Pixmap::new(width, height).ok_or(RenderError::PixmapAllocation { width, height })?;
    // The SVG paints a white canvas rect; filling first also whitens the partial last
    // row or column that ceil adds when the canvas size has a fractional part.
    pixmap.fill(Color::WHITE);
    resvg::render(
        &tree,
        Transform::from_scale(scale_factor, scale_factor),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|error| RenderError::PngEncode {
        message: error.to_string(),
    })
}

/// ceil(extent * scale). The tree size is already rounded to 2 decimals by section 5.1.
fn pixel_extent(extent: f32, scale: f32) -> u32 {
    (extent * scale).ceil() as u32
}

/// A usvg database holding the four bundled faces and nothing else. System fonts are
/// never loaded and no generic family is pointed at Inter.
fn bundled_font_database() -> Result<fontdb::Database, RenderError> {
    let mut database = fontdb::Database::new();
    for (index, file) in BUNDLED_FONTS.iter().enumerate() {
        database.load_font_data(file.bytes.to_vec());
        if database.len() != index + 1 {
            return Err(RenderError::Fonts(FontError::Unparseable {
                file_name: file.file_name,
            }));
        }
    }
    Ok(database)
}

/// usvg's default selector appends the Serif generic to every query and its fallback
/// selector substitutes any loaded face that covers a glyph. This resolver does neither:
/// a family or glyph that does not resolve is counted in `misses` and yields no font.
fn strict_font_resolver(misses: Arc<AtomicUsize>) -> FontResolver<'static> {
    let fallback_misses = Arc::clone(&misses);
    FontResolver {
        select_font: Box::new(move |font, database| {
            let families: Vec<fontdb::Family<'_>> =
                font.families().iter().map(to_fontdb_family).collect();
            let query = fontdb::Query {
                families: &families,
                weight: fontdb::Weight(font.weight()),
                stretch: to_fontdb_stretch(font.stretch()),
                style: to_fontdb_style(font.style()),
            };
            let face = database.query(&query);
            if face.is_none() {
                misses.fetch_add(1, Ordering::SeqCst);
            }
            face
        }),
        select_fallback: Box::new(move |_character, _used_faces, _database| {
            fallback_misses.fetch_add(1, Ordering::SeqCst);
            None
        }),
    }
}

fn to_fontdb_family(family: &usvg::FontFamily) -> fontdb::Family<'_> {
    match family {
        usvg::FontFamily::Named(name) => fontdb::Family::Name(name),
        usvg::FontFamily::Serif => fontdb::Family::Serif,
        usvg::FontFamily::SansSerif => fontdb::Family::SansSerif,
        usvg::FontFamily::Cursive => fontdb::Family::Cursive,
        usvg::FontFamily::Fantasy => fontdb::Family::Fantasy,
        usvg::FontFamily::Monospace => fontdb::Family::Monospace,
    }
}

fn to_fontdb_stretch(stretch: FontStretch) -> fontdb::Stretch {
    match stretch {
        FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
        FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
        FontStretch::Condensed => fontdb::Stretch::Condensed,
        FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
        FontStretch::Normal => fontdb::Stretch::Normal,
        FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
        FontStretch::Expanded => fontdb::Stretch::Expanded,
        FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
        FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
    }
}

fn to_fontdb_style(style: FontStyle) -> fontdb::Style {
    match style {
        FontStyle::Normal => fontdb::Style::Normal,
        FontStyle::Italic => fontdb::Style::Italic,
        FontStyle::Oblique => fontdb::Style::Oblique,
    }
}

/// Counts `Node::Text` through `Node::Group` children only: not into a text node's
/// flattened group and not into an image's subtree. usvg drops a `<text>` whose font does
/// not resolve without an error, so a missing node is the only signal.
fn count_text_nodes(root: &usvg::Group) -> usize {
    let mut count = 0;
    let mut pending: Vec<&usvg::Group> = vec![root];
    while let Some(group) = pending.pop() {
        for node in group.children() {
            match node {
                usvg::Node::Group(child) => pending.push(child),
                usvg::Node::Text(_) => count += 1,
                usvg::Node::Path(_) | usvg::Node::Image(_) => {}
            }
        }
    }
    count
}
