#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{ContainerLook, PageGeometry, TextRun, layout_page};
use stencil_model::text::FixedMetricsMeasurer;
use stencil_model::{Grammar, Page, builtin_grammar};
use stencil_render::{SvgDocument, render_svg};
use stencil_text::CosmicTextMeasurer;

pub const G7_JSON: &str = include_str!("../../../../examples/g7.json");

pub fn g7_document() -> Value {
    serde_json::from_str(G7_JSON).expect("g7.json is JSON")
}

pub fn g7_page() -> Page {
    serde_json::from_str(G7_JSON).expect("g7.json parses as a Page")
}

pub fn gcp() -> Grammar {
    builtin_grammar("gcp")
        .expect("gcp is a built-in grammar")
        .expect("the gcp grammar is valid")
}

/// The gcp grammar with `page` added to every kind's parents, so a render test can place
/// any container kind at the top level. Nesting is vet's concern and is tested there.
pub fn gcp_anywhere() -> Grammar {
    let mut grammar = gcp();
    for container in &mut grammar.containers {
        if !container.parents.iter().any(|parent| parent == "page") {
            container.parents.push("page".to_string());
        }
    }
    grammar
}

/// The grammar a test page names: plain, or gcp with every kind allowed at the top level.
pub fn grammar_for(page: &Page) -> Grammar {
    match page.grammar.as_deref() {
        Some("plain") => builtin_grammar("plain")
            .expect("plain is a built-in grammar")
            .expect("the plain grammar is valid"),
        _ => gcp_anywhere(),
    }
}

pub fn layout_with_cosmic_text(page: &Page) -> PageGeometry {
    let mut measurer = CosmicTextMeasurer::new().expect("bundled fonts load");
    layout_page(page, &grammar_for(page), &mut measurer).expect("layout succeeds")
}

pub fn layout_with_fixed_metrics(page: &Page) -> PageGeometry {
    layout_page(
        page,
        &grammar_for(page),
        &mut FixedMetricsMeasurer::default(),
    )
    .expect("layout succeeds")
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

/// The look and effective tint of a gcp Box of `kind` with `tint`, as layout records them.
pub fn zone(kind: &str, tint: Option<u8>) -> (ContainerLook, Option<u8>) {
    let grammar = gcp();
    let container = grammar.container(kind).expect("a gcp container kind");
    let look = ContainerLook {
        role: container.role,
        tone: container.tone,
        pattern: container.border.pattern,
        border_width: container.border.width,
        radius: container.radius,
        label: container.label,
    };
    (look, stencil_model::box_tint(container, tint))
}

/// The gcp Box kind and tint behind each output key the zone tests enumerate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKey {
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

impl ZoneKey {
    pub const ALL: [ZoneKey; 11] = [
        ZoneKey::Gcp,
        ZoneKey::Vpc,
        ZoneKey::RegionA,
        ZoneKey::RegionB,
        ZoneKey::Subnet,
        ZoneKey::OnpremA,
        ZoneKey::OnpremB,
        ZoneKey::Project,
        ZoneKey::Optional,
        ZoneKey::K8s,
        ZoneKey::Perimeter,
    ];

    /// The Box's `kind` and `tint`.
    pub fn kind_and_tint(self) -> (&'static str, Option<u8>) {
        match self {
            ZoneKey::Gcp => ("gcp", None),
            ZoneKey::Vpc => ("vpc", None),
            ZoneKey::RegionA => ("region", Some(1)),
            ZoneKey::RegionB => ("region", Some(2)),
            ZoneKey::Subnet => ("subnet", None),
            ZoneKey::OnpremA => ("onprem", Some(1)),
            ZoneKey::OnpremB => ("onprem", Some(2)),
            ZoneKey::Project => ("project", None),
            ZoneKey::Optional => ("optional", None),
            ZoneKey::K8s => ("k8s", None),
            ZoneKey::Perimeter => ("perimeter", None),
        }
    }

    /// The output key (section 13.1 rule 6), for example "region-a".
    pub fn as_str(self) -> &'static str {
        match self {
            ZoneKey::Gcp => "gcp",
            ZoneKey::Vpc => "vpc",
            ZoneKey::RegionA => "region-a",
            ZoneKey::RegionB => "region-b",
            ZoneKey::Subnet => "subnet",
            ZoneKey::OnpremA => "onprem-a",
            ZoneKey::OnpremB => "onprem-b",
            ZoneKey::Project => "project",
            ZoneKey::Optional => "optional",
            ZoneKey::K8s => "k8s",
            ZoneKey::Perimeter => "perimeter",
        }
    }

    /// A Box of this key holding one Fact.
    pub fn box_json(self) -> Value {
        let (kind, tint) = self.kind_and_tint();
        let mut node = json!({
            "tag": "Box",
            "kind": kind,
            "label": format!("Zone {}", self.as_str()),
            "children": [{ "tag": "Fact", "text": "fact" }]
        });
        if let Some(tint) = tint {
            node["tint"] = json!(tint);
        }
        node
    }
}

/// The line and tint behind each line key the wire tests enumerate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKey {
    Gray,
    Blue,
    Pink,
    Dash,
    Deny,
}

impl LineKey {
    pub const ALL: [LineKey; 5] = [
        LineKey::Gray,
        LineKey::Blue,
        LineKey::Pink,
        LineKey::Dash,
        LineKey::Deny,
    ];

    pub fn line(self) -> stencil_model::Line {
        match self {
            LineKey::Gray => stencil_model::Line::Gray,
            LineKey::Blue | LineKey::Pink => stencil_model::Line::Solid,
            LineKey::Dash => stencil_model::Line::Dash,
            LineKey::Deny => stencil_model::Line::Deny,
        }
    }

    pub fn tint(self) -> Option<u8> {
        match self {
            LineKey::Blue => Some(1),
            LineKey::Pink => Some(2),
            LineKey::Gray | LineKey::Dash | LineKey::Deny => None,
        }
    }

    /// The output key (section 13.1 rule 6), for example "blue".
    pub fn as_str(self) -> &'static str {
        stencil_model::line_key(self.line(), self.tint())
    }

    /// `object` with `line` and, for a tinted line, `tint` set.
    pub fn with_line(self, mut object: Value) -> Value {
        object["line"] = json!(self.line().as_str());
        if let Some(tint) = self.tint() {
            object["tint"] = json!(tint);
        }
        object
    }
}
