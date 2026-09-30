#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unreachable
)]

//! Section 11.1: the theme decides colors only.

mod common;

use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_model::{Page, PipeKind, Theme, ZoneKind};
use stencil_render::palette::Palette;
use stencil_render::{DeviceScale, measured_json, render_png, render_svg};

const THEMES: [Theme; 3] = [Theme::Center, Theme::Dusk, Theme::Wire];

const EXAMPLES: [(&str, &str); 3] = [
    ("g7", include_str!("../../../examples/g7.json")),
    (
        "hybrid-ai",
        include_str!("../../../examples/hybrid-ai.json"),
    ),
    (
        "network-hub-spoke",
        include_str!("../../../examples/network-hub-spoke.json"),
    ),
];

fn theme_name(theme: Theme) -> &'static str {
    Palette::new(theme).theme_name()
}

/// `document` with its `theme` field set.
fn with_theme(mut document: Value, theme: Theme) -> Value {
    document["theme"] = json!(theme_name(theme));
    document
}

fn render_themed(document: Value, theme: Theme) -> common::Rendered {
    common::render_document_with_fixed_metrics(with_theme(document, theme))
}

/// A customer page with every ZoneKind, every PipeKind as a Pipe, a Tee and a legend entry,
/// a Pcard with an icon, fact and ask, and a Note of each legend-ish kind.
fn palette_document() -> Value {
    let zones: Vec<_> = ZoneKind::ALL
        .iter()
        .map(|kind| {
            json!({
                "tag": "Zone",
                "kind": kind.as_str(),
                "label": format!("Zone {}", kind.as_str()),
                "children": [{ "tag": "Fact", "text": "fact" }]
            })
        })
        .collect();
    let pipes: Vec<_> = PipeKind::ALL
        .iter()
        .map(|kind| json!({ "tag": "Pipe", "dir": "h", "kind": kind.as_str(), "label": kind.as_str(), "sub": "sub" }))
        .collect();
    let tees: Vec<_> = PipeKind::ALL
        .iter()
        .map(|kind| {
            json!({
                "tag": "Tee",
                "kind": kind.as_str(),
                "hub": "hub",
                "arms": [
                    { "tag": "Pipe", "dir": "h", "kind": kind.as_str(), "label": "arm one" },
                    { "tag": "Pipe", "dir": "h", "kind": kind.as_str(), "label": "arm two" }
                ]
            })
        })
        .collect();
    let legend: Vec<_> = PipeKind::ALL
        .iter()
        .map(|kind| json!({ "kind": kind.as_str(), "text": kind.as_str() }))
        .collect();
    let mut document = common::page_document(
        json!([
            { "tag": "Col", "children": zones },
            { "tag": "Col", "children": pipes },
            { "tag": "Col", "children": tees },
            {
                "tag": "Pcard",
                "icon": "bigquery",
                "fn": "Warehouse",
                "pn": "BigQuery",
                "fact": "a fact",
                "ask": "an ask"
            }
        ]),
        json!(legend),
    );
    document["foot"] = json!("foot");
    document
}

/// Values of `attribute` on every element named `name`.
fn attribute_values<'a>(
    document: &'a roxmltree::Document<'a>,
    name: &str,
    attribute: &str,
) -> Vec<&'a str> {
    document
        .descendants()
        .filter(|node| node.has_tag_name(name))
        .filter_map(|node| node.attribute(attribute))
        .collect()
}

#[test]
fn measured_json_is_identical_and_svg_differs_across_themes() {
    for (name, document_text) in EXAMPLES {
        let document: Value = serde_json::from_str(document_text).unwrap();
        let mut measured_outputs = Vec::new();
        let mut svg_outputs = Vec::new();
        for theme in THEMES {
            let mut page: Page = serde_json::from_str(document_text).unwrap();
            page.theme = theme;
            let geometry = common::layout_with_cosmic_text(&page);
            let svg = render_svg(&page, &geometry).unwrap();
            let measured = serde_json::to_vec_pretty(&measured_json(&document, &geometry)).unwrap();
            measured_outputs.push(measured);
            svg_outputs.push(svg.svg);
        }
        assert_eq!(
            measured_outputs[0], measured_outputs[1],
            "{name} center vs dusk"
        );
        assert_eq!(
            measured_outputs[0], measured_outputs[2],
            "{name} center vs wire"
        );
        assert_ne!(svg_outputs[0], svg_outputs[1], "{name} center vs dusk");
        assert_ne!(svg_outputs[0], svg_outputs[2], "{name} center vs wire");
        assert_ne!(svg_outputs[1], svg_outputs[2], "{name} dusk vs wire");
    }
}

fn hex_color(hex: &str) -> (u8, u8, u8) {
    let channel = |range| u8::from_str_radix(&hex[range], 16).unwrap();
    (channel(1..3), channel(3..5), channel(5..7))
}

#[test]
fn png_page_background_is_the_palette_page_background() {
    assert_eq!(Palette::new(Theme::Center).page_background(), "#FFFFFF");
    assert_eq!(Palette::new(Theme::Dusk).page_background(), "#0B1220");
    assert_eq!(Palette::new(Theme::Wire).page_background(), "#FFFFFF");
    for theme in THEMES {
        let mut page = common::g7_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let png = render_png(&svg.svg, svg.text_elements, DeviceScale::new(1).unwrap()).unwrap();
        let pixmap = common::decode_png(&png);
        let pixel = pixmap.pixel(2, 2).unwrap();
        let expected = hex_color(Palette::new(theme).page_background());
        assert_eq!(
            (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()),
            (expected.0, expected.1, expected.2, 255),
            "{}",
            theme_name(theme)
        );
    }
}

/// (fill, stroke, stroke-width, dasharray) of the dusk zone table, gcp excluded.
fn expected_dusk_zone_rect(
    kind: ZoneKind,
) -> (
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
) {
    match kind {
        ZoneKind::Gcp => unreachable!("gcp is drawn with paths"),
        ZoneKind::Vpc => ("none", Some("#6B7A99"), Some("2"), Some("6 5")),
        ZoneKind::RegionA => ("#14213A", Some("#2F4A7A"), Some("1.5"), None),
        ZoneKind::RegionB => ("#2A1626", Some("#6A2A47"), Some("1.5"), None),
        ZoneKind::Subnet => ("#1D1836", Some("#4A3F7A"), Some("1.5"), Some("6 5")),
        ZoneKind::OnpremA => ("#14213A", Some("#3A3532"), Some("1.5"), None),
        ZoneKind::OnpremB => ("#2A1626", Some("#3A3532"), Some("1.5"), None),
        ZoneKind::Project => ("#1F1B10", Some("#5A4A1A"), Some("1.5"), None),
        ZoneKind::Optional => ("#10203A", Some("#4284F3"), Some("2"), Some("6 5")),
        ZoneKind::K8s => ("#2A1626", None, None, None),
        ZoneKind::Perimeter => ("#17130B", Some("#E37400"), Some("2.5"), Some("6 5")),
    }
}

#[test]
fn dusk_zones_follow_the_section_11_1_table() {
    let rendered = render_themed(palette_document(), Theme::Dusk);
    let document = common::parse_xml(&rendered.svg.svg);
    for (index, kind) in ZoneKind::ALL.iter().enumerate() {
        let group = common::group(&document, &format!("/body/0/children/{index}"));
        let label = common::children_named(group, "text")[0];
        if *kind == ZoneKind::Gcp {
            let paths = common::children_named(group, "path");
            assert_eq!(paths[0].attribute("fill"), Some("#0F172A"));
            assert_eq!(paths[1].attribute("stroke"), Some("#1A73E8"));
            let rects = common::children_named(group, "rect");
            assert_eq!(rects[0].attribute("fill"), Some("#1A73E8"), "bar");
            assert_eq!(rects[1].attribute("fill"), Some("#0F172A"), "body");
            assert_eq!(label.attribute("fill"), Some("#FFFFFF"));
            continue;
        }
        let rect = common::children_named(group, "rect")[0];
        let (fill, stroke, stroke_width, dash) = expected_dusk_zone_rect(*kind);
        let name = kind.as_str();
        assert_eq!(rect.attribute("fill"), Some(fill), "{name}");
        assert_eq!(rect.attribute("stroke"), stroke, "{name}");
        assert_eq!(rect.attribute("stroke-width"), stroke_width, "{name}");
        assert_eq!(rect.attribute("stroke-dasharray"), dash, "{name}");
        let label_ink = if *kind == ZoneKind::Perimeter {
            "#F2A44B"
        } else {
            "#B7C2D6"
        };
        assert_eq!(label.attribute("fill"), Some(label_ink), "{name}");
    }
}

#[test]
fn dusk_wires_tags_and_deny_follow_the_section_11_1_table() {
    let rendered = render_themed(palette_document(), Theme::Dusk);
    let document = common::parse_xml(&rendered.svg.svg);
    let expected_wire = |kind: PipeKind| match kind {
        PipeKind::Gray => ("#9AA7BD", None),
        PipeKind::Blue => ("#5B9CFF", None),
        PipeKind::Pink => ("#FF5C8A", None),
        PipeKind::Dash => ("#5B9CFF", Some("6 5")),
        PipeKind::Deny => ("#FF6B6B", Some("6 5")),
    };
    for (index, kind) in PipeKind::ALL.iter().enumerate() {
        let (color, dash) = expected_wire(*kind);
        let pipe = common::group(&document, &format!("/body/1/children/{index}"));
        for wire in common::children_named(pipe, "line") {
            assert_eq!(wire.attribute("stroke"), Some(color));
            assert_eq!(wire.attribute("stroke-dasharray"), dash);
        }
        for dot in common::children_named(pipe, "circle") {
            assert_eq!(dot.attribute("fill"), Some(color));
        }
        let (tag_border, tag_ink) = if *kind == PipeKind::Deny {
            ("#5A2A2A", "#FF8A8A")
        } else {
            ("#2A3550", "#E6EDF7")
        };
        for pointer in [
            format!("/body/1/children/{index}"),
            format!("/body/2/children/{index}"),
        ] {
            let group = common::group(&document, &pointer);
            let tag = common::children_named(group, "rect")[0];
            assert_eq!(tag.attribute("fill"), Some("#111A2E"), "{pointer}");
            assert_eq!(tag.attribute("stroke"), Some(tag_border), "{pointer}");
            let texts = common::children_named(group, "text");
            assert_eq!(texts[0].attribute("fill"), Some(tag_ink), "{pointer}");
        }
        let pipe_texts = common::children_named(pipe, "text");
        assert_eq!(pipe_texts[1].attribute("fill"), Some("#9AA7BD"), "tag sub");
        let entry = common::group(&document, &format!("/legend/{index}"));
        let swatch = common::children_named(entry, "line")[0];
        assert_eq!(swatch.attribute("stroke"), Some(color));
    }
}

#[test]
fn dusk_page_card_and_text_inks_follow_the_section_11_1_table() {
    let rendered = render_themed(palette_document(), Theme::Dusk);
    let document = common::parse_xml(&rendered.svg.svg);
    let first_text_fill = |pointer: &str| {
        common::children_named(common::group(&document, pointer), "text")[0]
            .attribute("fill")
            .unwrap()
    };
    let kicker_texts = common::children_named(common::group(&document, "/kicker"), "text");
    assert_eq!(kicker_texts[0].attribute("fill"), Some("#9CC3FF"), "badge");
    assert_eq!(kicker_texts[1].attribute("fill"), Some("#5B9CFF"), "kicker");
    assert_eq!(first_text_fill("/title"), "#E6EDF7");
    assert_eq!(first_text_fill("/lede"), "#9AA7BD");
    assert_eq!(first_text_fill("/foot"), "#9AA7BD");
    let legend_texts = common::children_named(common::group(&document, "/legend/0"), "text");
    assert_eq!(legend_texts[0].attribute("fill"), Some("#E6EDF7"), "label");
    assert_eq!(legend_texts[1].attribute("fill"), Some("#9AA7BD"), "text");

    let card = common::group(&document, "/body/3");
    let rects = common::children_named(card, "rect");
    assert_eq!(rects[0].attribute("fill"), Some("#111A2E"), "card");
    assert_eq!(rects[0].attribute("stroke"), Some("#2A3550"), "card");
    assert_eq!(rects[1].attribute("fill"), Some("#FFFFFF"), "icon chip");
    assert_eq!(rects[2].attribute("fill"), Some("#182238"), "fact box");
    assert_eq!(rects[3].attribute("fill"), Some("#182238"), "ask box");
    let card_texts: Vec<_> = common::children_named(card, "text")
        .iter()
        .map(|text| text.attribute("fill").unwrap())
        .collect();
    assert_eq!(card_texts, ["#E6EDF7", "#9AA7BD", "#B7C2D6", "#B7C2D6"]);

    let fact = common::group(&document, "/body/0/children/1/children/0");
    assert_eq!(
        common::children_named(fact, "rect")[0].attribute("fill"),
        Some("#182238")
    );
    assert_eq!(first_text_fill("/body/0/children/1/children/0"), "#B7C2D6");
}

#[test]
fn badges_follow_the_theme_and_canvas() {
    let expected = [
        (Theme::Center, "customer", "#E8F0FE", None, "#174EA6"),
        (Theme::Center, "internal", "#F3E5F5", None, "#7B1FA2"),
        (Theme::Dusk, "customer", "#16305C", None, "#9CC3FF"),
        (Theme::Dusk, "internal", "#2E1A4A", None, "#D6B4FF"),
        (
            Theme::Wire,
            "customer",
            "#FFFFFF",
            Some("#222222"),
            "#222222",
        ),
        (
            Theme::Wire,
            "internal",
            "#FFFFFF",
            Some("#222222"),
            "#222222",
        ),
    ];
    for (theme, canvas, fill, stroke, ink) in expected {
        let mut document = palette_document();
        document["canvas"] = json!(canvas);
        let rendered = render_themed(document, theme);
        let svg = common::parse_xml(&rendered.svg.svg);
        let kicker = common::group(&svg, "/kicker");
        let badge = common::children_named(kicker, "rect")[0];
        let context = format!("{} {canvas}", theme_name(theme));
        assert_eq!(badge.attribute("fill"), Some(fill), "{context}");
        assert_eq!(badge.attribute("stroke"), stroke, "{context}");
        let badge_text = common::children_named(kicker, "text")[0];
        assert_eq!(badge_text.attribute("fill"), Some(ink), "{context}");
    }
}

#[test]
fn wire_zone_borders_follow_kind_and_fills_stay_white() {
    let rendered = render_themed(palette_document(), Theme::Wire);
    let document = common::parse_xml(&rendered.svg.svg);
    for (index, kind) in ZoneKind::ALL.iter().enumerate() {
        let group = common::group(&document, &format!("/body/0/children/{index}"));
        if *kind == ZoneKind::Gcp {
            let paths = common::children_named(group, "path");
            assert_eq!(paths[0].attribute("fill"), Some("#FFFFFF"));
            assert_eq!(paths[1].attribute("stroke"), Some("#222222"));
            assert_eq!(paths[1].attribute("stroke-width"), Some("2"));
            assert_eq!(paths[1].attribute("stroke-dasharray"), None);
            continue;
        }
        let rect = common::children_named(group, "rect")[0];
        let expected_dash = match kind {
            ZoneKind::Vpc | ZoneKind::Optional | ZoneKind::Perimeter => Some("6 5"),
            ZoneKind::Subnet => Some("2 3"),
            _ => None,
        };
        let expected_fill = if *kind == ZoneKind::Vpc {
            "none"
        } else {
            "#FFFFFF"
        };
        let name = kind.as_str();
        assert_eq!(rect.attribute("fill"), Some(expected_fill), "{name}");
        assert_eq!(rect.attribute("stroke"), Some("#222222"), "{name}");
        assert_eq!(rect.attribute("stroke-width"), Some("1.25"), "{name}");
        assert_eq!(rect.attribute("stroke-dasharray"), expected_dash, "{name}");
    }
}

#[test]
fn wire_gcp_bar_is_white_with_a_two_px_bottom_rule_and_dark_ink() {
    let rendered = render_themed(palette_document(), Theme::Wire);
    let document = common::parse_xml(&rendered.svg.svg);
    let group = common::group(&document, "/body/0/children/0");
    let bar = rendered
        .geometry
        .node(
            &stencil_model::pointer::NodePointer::root()
                .child("body")
                .index(0)
                .child("children")
                .index(0),
        )
        .unwrap()
        .part(stencil_layout::PartName::Bar)
        .unwrap()
        .bounds;
    let rects = common::children_named(group, "rect");
    assert_eq!(rects[0].attribute("fill"), Some("#FFFFFF"), "bar");
    let rule = common::children_named(group, "line")[0];
    assert_eq!(rule.attribute("stroke"), Some("#222222"));
    assert_eq!(rule.attribute("stroke-width"), Some("2"));
    let rule_y: f32 = rule.attribute("y1").unwrap().parse().unwrap();
    assert!((rule_y - (bar.bottom() - 1.0)).abs() <= 0.006);
    assert_eq!(rule.attribute("y1"), rule.attribute("y2"));
    let label = common::children_named(group, "text")[0];
    assert_eq!(label.attribute("fill"), Some("#222222"));
}

#[test]
fn wire_kinds_are_told_apart_by_line_style() {
    let rendered = render_themed(palette_document(), Theme::Wire);
    let document = common::parse_xml(&rendered.svg.svg);
    let expected = |kind: PipeKind| match kind {
        PipeKind::Gray => ("1.25", None),
        PipeKind::Blue | PipeKind::Pink => ("2", None),
        PipeKind::Dash => ("2", Some("6 5")),
        PipeKind::Deny => ("2", Some("2 3")),
    };
    for (index, kind) in PipeKind::ALL.iter().enumerate() {
        let (width, dash) = expected(*kind);
        let name = kind.as_str();
        let pipe = common::group(&document, &format!("/body/1/children/{index}"));
        let tee = common::group(&document, &format!("/body/2/children/{index}"));
        let entry = common::group(&document, &format!("/legend/{index}"));
        for group in [pipe, tee, entry] {
            for line in common::children_named(group, "line") {
                assert_eq!(line.attribute("stroke"), Some("#222222"), "{name}");
                assert_eq!(line.attribute("stroke-width"), Some(width), "{name}");
                assert_eq!(line.attribute("stroke-dasharray"), dash, "{name}");
            }
        }
        let dots = common::children_named(pipe, "circle");
        assert_eq!(dots.len(), 2, "{name}");
        let swatch_dots = common::children_named(entry, "circle");
        for dot in dots.iter().chain(&swatch_dots) {
            if *kind == PipeKind::Pink {
                assert_eq!(dot.attribute("fill"), Some("#FFFFFF"), "{name}");
                assert_eq!(dot.attribute("stroke"), Some("#222222"), "{name}");
            } else {
                assert_eq!(dot.attribute("fill"), Some("#222222"), "{name}");
                assert_eq!(dot.attribute("stroke"), None, "{name}");
            }
        }
        let expected_swatch_dots = if *kind == PipeKind::Pink { 2 } else { 0 };
        assert_eq!(swatch_dots.len(), expected_swatch_dots, "{name}");
        let tag = common::children_named(pipe, "rect")[0];
        assert_eq!(tag.attribute("fill"), Some("#FFFFFF"), "{name}");
        assert_eq!(tag.attribute("stroke"), Some("#222222"), "{name}");
    }
}

#[test]
fn wire_paints_only_white_fills_and_two_inks() {
    let rendered = render_themed(palette_document(), Theme::Wire);
    let document = common::parse_xml(&rendered.svg.svg);
    for name in ["rect", "path", "circle"] {
        for fill in attribute_values(&document, name, "fill") {
            assert!(
                ["#FFFFFF", "none", "#222222"].contains(&fill),
                "{name} fill {fill}"
            );
        }
    }
    for fill in attribute_values(&document, "text", "fill") {
        assert!(["#222222", "#555555"].contains(&fill), "text fill {fill}");
    }
    for stroke in ["rect", "path", "line", "circle"]
        .iter()
        .flat_map(|name| attribute_values(&document, name, "stroke"))
    {
        assert_eq!(stroke, "#222222");
    }
}

#[test]
fn icon_chip_sits_under_the_icon_in_dusk_and_wire_and_center_draws_none() {
    for theme in THEMES {
        let rendered = render_themed(palette_document(), theme);
        let document = common::parse_xml(&rendered.svg.svg);
        let card = common::group(&document, "/body/3");
        let icon = rendered
            .geometry
            .node(
                &stencil_model::pointer::NodePointer::root()
                    .child("body")
                    .index(3),
            )
            .unwrap()
            .part(stencil_layout::PartName::Icon)
            .unwrap()
            .bounds;
        let elements: Vec<_> = card.children().filter(|child| child.is_element()).collect();
        let image_position = elements
            .iter()
            .position(|element| element.has_tag_name("image"))
            .unwrap();
        let chips: Vec<_> = common::children_named(card, "rect")
            .into_iter()
            .filter(|rect| rect.attribute("width") == Some("36"))
            .collect();
        let name = theme_name(theme);
        let Some(chip_fill) = Palette::new(theme).icon_chip() else {
            assert!(chips.is_empty(), "{name}");
            continue;
        };
        assert_eq!(chips.len(), 1, "{name}");
        let chip = chips[0];
        assert_eq!(chip.attribute("height"), Some("36"), "{name}");
        assert_eq!(chip.attribute("rx"), Some("6"), "{name}");
        assert_eq!(chip.attribute("fill"), Some(chip_fill), "{name}");
        let chip_x: f32 = chip.attribute("x").unwrap().parse().unwrap();
        let chip_y: f32 = chip.attribute("y").unwrap().parse().unwrap();
        assert!((chip_x - (icon.x - 4.0)).abs() <= 0.006, "{name}");
        assert!((chip_y - (icon.y - 4.0)).abs() <= 0.006, "{name}");
        let chip_position = elements
            .iter()
            .position(|element| *element == chip)
            .unwrap();
        assert_eq!(
            chip_position + 1,
            image_position,
            "{name}: chip right under the icon"
        );
    }
    assert_eq!(Palette::new(Theme::Dusk).icon_chip(), Some("#FFFFFF"));
    assert_eq!(Palette::new(Theme::Wire).icon_chip(), Some("#FFFFFF"));
}
