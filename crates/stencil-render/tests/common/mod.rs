#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{PageGeometry, TextRun, layout_page};
use stencil_model::Page;
use stencil_model::text::FixedMetricsMeasurer;
use stencil_render::{SvgDocument, render_svg};
use stencil_text::CosmicTextMeasurer;

pub const G7_JSON: &str = include_str!("../../../../examples/g7.json");

pub fn g7_document() -> Value {
    serde_json::from_str(G7_JSON).expect("g7.json is JSON")
}

pub fn g7_page() -> Page {
    serde_json::from_str(G7_JSON).expect("g7.json parses as a Page")
}

pub fn layout_with_cosmic_text(page: &Page) -> PageGeometry {
    let mut measurer = CosmicTextMeasurer::new().expect("bundled fonts load");
    layout_page(page, &mut measurer).expect("layout succeeds")
}

pub fn layout_with_fixed_metrics(page: &Page) -> PageGeometry {
    layout_page(page, &mut FixedMetricsMeasurer::default()).expect("layout succeeds")
}

pub struct Rendered {
    pub page: Page,
    pub geometry: PageGeometry,
    pub svg: SvgDocument,
}

pub fn render_g7() -> Rendered {
    let page = g7_page();
    let geometry = layout_with_cosmic_text(&page);
    let svg = render_svg(&page, &geometry).expect("g7 renders to SVG");
    Rendered {
        page,
        geometry,
        svg,
    }
}

pub fn render_document_with_fixed_metrics(document: Value) -> Rendered {
    let page: Page = serde_json::from_value(document).expect("test document parses as a Page");
    let geometry = layout_with_fixed_metrics(&page);
    let svg = render_svg(&page, &geometry).expect("document renders to SVG");
    Rendered {
        page,
        geometry,
        svg,
    }
}

/// A customer page whose body is `body` and whose legend is `legend`.
pub fn page_document(body: Value, legend: Value) -> Value {
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "canvas": "customer",
        "body": body,
        "legend": legend
    })
}

pub fn parse_xml(svg: &str) -> roxmltree::Document<'_> {
    roxmltree::Document::parse(svg).expect("SVG is well-formed XML")
}

/// The `<g>` whose data-id is `pointer`.
pub fn group<'a, 'input>(
    document: &'a roxmltree::Document<'input>,
    pointer: &str,
) -> roxmltree::Node<'a, 'input> {
    document
        .descendants()
        .find(|node| node.has_tag_name("g") && node.attribute("data-id") == Some(pointer))
        .unwrap_or_else(|| panic!("no <g data-id={pointer:?}>"))
}

/// Direct element children of `node` with this tag name, in order.
pub fn children_named<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Vec<roxmltree::Node<'a, 'input>> {
    node.children()
        .filter(|child| child.is_element() && child.has_tag_name(name))
        .collect()
}

/// Every TextRun in geometry order, parts in section 2.11 order.
pub fn text_runs(geometry: &PageGeometry) -> Vec<&TextRun> {
    geometry
        .nodes
        .iter()
        .flat_map(|node| node.parts.iter())
        .filter_map(|part| part.text.as_ref())
        .collect()
}

pub fn white_pixel(pixel: &resvg::tiny_skia::PremultipliedColorU8) -> bool {
    pixel.red() == 255 && pixel.green() == 255 && pixel.blue() == 255 && pixel.alpha() == 255
}

pub fn decode_png(bytes: &[u8]) -> resvg::tiny_skia::Pixmap {
    resvg::tiny_skia::Pixmap::decode_png(bytes).expect("PNG decodes")
}
