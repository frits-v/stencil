//! The section 5.3 PNG pipeline.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::{self, FontResolver, FontStretch, FontStyle, ImageHrefResolver, fontdb};
use stencil_model::NODES_MAX;
use stencil_text::{BUNDLED_FONTS, FontError, verify_bundled_fonts};

use crate::{DeviceScale, PngSize, RenderError, format_number};

const DEFAULT_FONT_FAMILY: &str = "Inter";

/// Largest pixmap render_png allocates, in pixels (section 5.3, step 5): 512 MiB of RGBA.
pub const PNG_PIXELS_MAX: u64 = 1 << 27;

/// Smallest ratio of a `PngSize::Width` to the canvas width; the largest is
/// `DeviceScale::MAX`. Below half size the 8 px foot and legend text draw 4 px tall.
pub const PNG_WIDTH_SCALE_MIN: f64 = 0.5;

/// Upper bound on the groups `count_text_nodes` visits. A vetted page writes one `<g>` per
/// geometry node: at most NODES_MAX body nodes plus the page-level nodes.
const TEXT_NODE_VISITS_MAX: usize = 16 * NODES_MAX;

/// Calls verify_bundled_fonts, parses the SVG with a usvg fontdb that holds only
/// BUNDLED_FONTS, asserts the tree holds exactly `expected_text_elements` text nodes
/// and that every font lookup resolved (section 5.3), renders at `scale` within
/// PNG_PIXELS_MAX, and encodes PNG.
pub fn render_png(
    svg: &str,
    expected_text_elements: usize,
    scale: DeviceScale,
) -> Result<Vec<u8>, RenderError> {
    render_png_sized(svg, expected_text_elements, PngSize::Scale(scale))
}

/// `render_png` at a device scale or at an exact width. A width renders the tree at
/// `width / canvas width`, so text is rasterized at that size rather than resampled.
pub fn render_png_sized(
    svg: &str,
    expected_text_elements: usize,
    size: PngSize,
) -> Result<Vec<u8>, RenderError> {
    verify_bundled_fonts()?;
    let font_database = bundled_font_database()?;

    let font_misses = Arc::new(AtomicUsize::new(0));
    let options = usvg::Options {
        fontdb: Arc::new(font_database),
        font_family: DEFAULT_FONT_FAMILY.to_string(),
        resources_dir: None,
        font_resolver: strict_font_resolver(Arc::clone(&font_misses)),
        image_href_resolver: data_only_image_resolver(),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(svg, &options).map_err(|error| RenderError::Svg {
        message: error.to_string(),
    })?;

    let found = count_text_nodes(tree.root())?;
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
    let lookups = font_misses.load(Ordering::SeqCst);
    if lookups > 0 {
        return Err(RenderError::FontNotResolved { lookups });
    }

    let extent = tree.size();
    let (width, height, scale_factor) = match size {
        PngSize::Scale(scale) => {
            let (width, height) = pixmap_size(extent.width(), extent.height(), scale)?;
            (width, height, f32::from(scale.get()))
        }
        PngSize::Width(width) => pixmap_size_at_width(width, extent.width(), extent.height())?,
    };
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

/// ceil(extent * scale) on both axes, computed in f64. The tree size is already rounded to
/// 2 decimals by section 5.1. tiny-skia's `Pixmap::new` accepts any size whose byte count
/// fits a usize, so this budget is the only guard before the allocation.
fn pixmap_size(width: f32, height: f32, scale: DeviceScale) -> Result<(u32, u32), RenderError> {
    let scale = f64::from(scale.get());
    let width_px = saturating_pixels((f64::from(width) * scale).ceil());
    let height_px = saturating_pixels((f64::from(height) * scale).ceil());
    within_budget(width_px, height_px)
}

/// `width_px` as asked and `ceil(height * width_px / width)`, with the factor the tree is
/// drawn at. `width` and `height` are a parsed tree's size, which usvg keeps finite and
/// positive. The width must lie within PNG_WIDTH_SCALE_MIN to DeviceScale::MAX times the
/// canvas width, rounded inward to whole px.
fn pixmap_size_at_width(
    width_px: u32,
    width: f32,
    height: f32,
) -> Result<(u32, u32, f32), RenderError> {
    let canvas_width = f64::from(width);
    let min = saturating_pixels((canvas_width * PNG_WIDTH_SCALE_MIN).ceil());
    let max = saturating_pixels((canvas_width * f64::from(DeviceScale::MAX)).floor());
    if width_px < min.max(1) || width_px > max {
        return Err(RenderError::PngWidthOutOfRange {
            width: width_px,
            min,
            max,
            canvas_width: format_number(width),
        });
    }
    let scale = f64::from(width_px) / canvas_width;
    let height_px = saturating_pixels((f64::from(height) * scale).ceil());
    let (width_px, height_px) = within_budget(width_px, height_px)?;
    Ok((width_px, height_px, scale as f32))
}

/// Both extents at least 1 px and their product within PNG_PIXELS_MAX.
fn within_budget(width_px: u32, height_px: u32) -> Result<(u32, u32), RenderError> {
    let pixels = u64::from(width_px) * u64::from(height_px);
    if width_px == 0 || height_px == 0 || pixels > PNG_PIXELS_MAX {
        return Err(RenderError::PixmapAllocation {
            width: width_px,
            height: height_px,
        });
    }
    Ok((width_px, height_px))
}

/// NaN and negative extents become 0 and extents above u32::MAX become u32::MAX; both land
/// outside the budget that `pixmap_size` checks next.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a float-to-int `as` cast saturates, and every saturated value fails the budget"
)]
fn saturating_pixels(extent: f64) -> u32 {
    extent as u32
}

/// Decodes `data:` hrefs, which is how the SVG writer embeds icons, and resolves every other
/// href to nothing, so parsing never reads a file.
fn data_only_image_resolver() -> ImageHrefResolver<'static> {
    ImageHrefResolver {
        resolve_data: ImageHrefResolver::default_data_resolver(),
        resolve_string: Box::new(|_href, _options| None),
    }
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
fn count_text_nodes(root: &usvg::Group) -> Result<usize, RenderError> {
    let mut count = 0;
    let mut pending: Vec<&usvg::Group> = vec![root];
    for _ in 0..TEXT_NODE_VISITS_MAX {
        let Some(group) = pending.pop() else {
            return Ok(count);
        };
        for node in group.children() {
            match node {
                usvg::Node::Group(child) => pending.push(child),
                usvg::Node::Text(_) => count += 1,
                usvg::Node::Path(_) | usvg::Node::Image(_) => {}
            }
        }
    }
    if pending.is_empty() {
        Ok(count)
    } else {
        Err(RenderError::Svg {
            message: format!("parsed SVG holds more than {TEXT_NODE_VISITS_MAX} groups"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale(value: u8) -> DeviceScale {
        DeviceScale::new(value).unwrap()
    }

    #[test]
    fn pixmap_size_is_the_ceiled_scaled_extent() {
        assert_eq!(pixmap_size(1320.0, 652.0, scale(2)).unwrap(), (2640, 1304));
        assert_eq!(pixmap_size(1320.0, 652.4, scale(1)).unwrap(), (1320, 653));
    }

    #[test]
    fn pixmap_size_accepts_the_budget_and_rejects_one_row_past_it() {
        assert_eq!(
            pixmap_size(8192.0, 16384.0, scale(1)).unwrap(),
            (8192, 16384)
        );
        assert!(matches!(
            pixmap_size(8192.0, 16385.0, scale(1)),
            Err(RenderError::PixmapAllocation {
                width: 8192,
                height: 16385
            })
        ));
        assert!(matches!(
            pixmap_size(4096.0, 8193.0, scale(2)),
            Err(RenderError::PixmapAllocation {
                width: 8192,
                height: 16386
            })
        ));
    }

    #[test]
    fn pixmap_size_rejects_empty_and_non_finite_extents() {
        for (width, height) in [
            (0.0, 10.0),
            (10.0, -5.0),
            (f32::NAN, 10.0),
            (10.0, f32::INFINITY),
            (f32::MAX, f32::MAX),
        ] {
            assert!(
                matches!(
                    pixmap_size(width, height, scale(4)),
                    Err(RenderError::PixmapAllocation { .. })
                ),
                "{width}x{height}"
            );
        }
    }

    #[test]
    fn a_png_width_keeps_the_width_and_derives_the_height_and_factor() {
        let (width, height, factor) = pixmap_size_at_width(2034, 1480.0, 1178.0).unwrap();
        assert_eq!((width, height), (2034, 1619));
        assert!((f64::from(factor) - 2034.0 / 1480.0).abs() < 1e-6);
        assert_eq!(
            pixmap_size_at_width(2034, 1480.0, 1178.0).unwrap(),
            (width, height, factor)
        );
    }

    #[test]
    fn a_png_width_lies_within_half_to_four_times_the_canvas() {
        assert_eq!(pixmap_size_at_width(740, 1480.0, 100.0).unwrap().0, 740);
        assert_eq!(pixmap_size_at_width(5920, 1480.0, 100.0).unwrap().0, 5920);
        for width in [0, 739, 5921] {
            assert!(
                matches!(
                    pixmap_size_at_width(width, 1480.0, 100.0),
                    Err(RenderError::PngWidthOutOfRange {
                        min: 740,
                        max: 5920,
                        ..
                    })
                ),
                "{width}"
            );
        }
    }

    #[test]
    fn a_png_width_keeps_the_pixel_budget() {
        assert!(matches!(
            pixmap_size_at_width(4096, 2048.0, 40000.0),
            Err(RenderError::PixmapAllocation {
                width: 4096,
                height: 80000
            })
        ));
    }

    /// Root plus `sibling_groups` groups, each kept by usvg because it has an id.
    fn tree_with_groups(sibling_groups: usize) -> usvg::Tree {
        let mut svg = String::from(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8" viewBox="0 0 8 8">"#,
        );
        for index in 0..sibling_groups {
            svg.push_str(&format!(
                r#"<g id="g{index}"><rect width="1" height="1"/></g>"#
            ));
        }
        svg.push_str("</svg>");
        usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap()
    }

    #[test]
    fn text_node_count_visits_up_to_the_group_bound() {
        let tree = tree_with_groups(TEXT_NODE_VISITS_MAX - 1);
        assert_eq!(count_text_nodes(tree.root()).unwrap(), 0);
    }

    #[test]
    fn text_node_count_past_the_group_bound_is_an_svg_error() {
        let tree = tree_with_groups(TEXT_NODE_VISITS_MAX);
        assert!(matches!(
            count_text_nodes(tree.root()),
            Err(RenderError::Svg { .. })
        ));
    }
}
