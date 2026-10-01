#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unreachable
)]

//! Sections 13.4 and 13.5: themes are data, and a theme decides paint only.

mod common;

use common::{LineKey, THEMES, ZoneKey};
use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::PartName;
use stencil_model::grammar::{BorderPattern, Role};
use stencil_model::theme::{DotStyle, LinePattern, TintCue};
use stencil_model::{
    BUILTIN_THEMES, DESIGNED_THEMES, IMPORTED_THEMES, Page, Projection, Theme, apply_overrides,
    validate_theme,
};
use stencil_render::iso::{SolidInputs, project_page};
use stencil_render::palette::Palette;
use stencil_render::{DeviceScale, builtin_theme_json, measured_json, render_png, render_svg};

const EXAMPLES: [(&str, &str); 6] = [
    ("g7", include_str!("../../../examples/g7.json")),
    ("hero-iso", include_str!("../../../examples/hero-iso.json")),
    (
        "hybrid-ai",
        include_str!("../../../examples/hybrid-ai.json"),
    ),
    (
        "network-hub-spoke",
        include_str!("../../../examples/network-hub-spoke.json"),
    ),
    ("onepager", include_str!("../../../examples/onepager.json")),
    (
        "stress-dense",
        include_str!("../../../examples/stress-dense.json"),
    ),
];

/// `document` with its `theme` field set.
fn with_theme(mut document: Value, theme: &str) -> Value {
    document["theme"] = json!(theme);
    document
}

fn render_themed(document: Value, theme: &str) -> common::Rendered {
    common::render_document_with_fixed_metrics(with_theme(document, theme))
}

/// A customer page with every ZoneKey, every LineKey as a Pipe, a Tee and a legend entry,
/// an Item with an icon, doc fact and ask, and a Note of each legend-ish kind.
fn palette_document() -> Value {
    let zones: Vec<_> = ZoneKey::ALL.iter().map(|kind| kind.box_json()).collect();
    let pipes: Vec<_> = LineKey::ALL
        .iter()
        .map(|kind| {
            kind.with_line(
                json!({ "tag": "Pipe", "dir": "h", "label": kind.as_str(), "sub": "sub" }),
            )
        })
        .collect();
    let tees: Vec<_> = LineKey::ALL
        .iter()
        .map(|kind| {
            kind.with_line(json!({
                "tag": "Tee",
                "hub": "hub",
                "arms": [
                    kind.with_line(json!({ "tag": "Pipe", "dir": "h", "label": "arm one" })),
                    kind.with_line(json!({ "tag": "Pipe", "dir": "h", "label": "arm two" }))
                ]
            }))
        })
        .collect();
    let legend: Vec<_> = LineKey::ALL
        .iter()
        .map(|kind| kind.with_line(json!({ "text": kind.as_str() })))
        .collect();
    let mut document = common::page_document(
        json!([
            { "tag": "Col", "children": zones },
            { "tag": "Col", "children": pipes },
            { "tag": "Col", "children": tees },
            {
                "tag": "Item", "kind": "product",
                "icon": "bigquery",
                "title": "Warehouse",
                "subtitle": "BigQuery",
                "facts": [{ "text": "a fact" }, { "text": "an ask", "source": "ask" }]
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
fn every_builtin_parses_validates_and_is_written_canonically() {
    let mut examined = 0;
    for name in BUILTIN_THEMES {
        let json_text = builtin_theme_json(name).unwrap();
        let theme = common::theme(name);
        assert_eq!(theme.name, name);
        assert_eq!(validate_theme(&theme), Vec::new(), "{name}");
        let canonical = serde_json::to_string_pretty(&theme).unwrap() + "\n";
        assert!(
            canonical == json_text,
            "themes/{name}.json is not the pretty serialization of its theme"
        );
        examined += 1;
    }
    assert_eq!(examined, 13);
    assert_eq!(DESIGNED_THEMES, THEMES);
    let designed_then_imported: Vec<&str> = DESIGNED_THEMES
        .iter()
        .chain(IMPORTED_THEMES.iter())
        .copied()
        .collect();
    assert_eq!(BUILTIN_THEMES.to_vec(), designed_then_imported);
    assert!(stencil_render::builtin_theme("nord-light").is_none());
}

#[test]
fn every_designed_theme_has_slab_thickness_six() {
    for name in THEMES {
        assert_eq!(common::theme(name).iso.slab_thickness, 6.0, "{name}");
    }
}

#[test]
fn measured_json_is_identical_and_svg_differs_across_themes() {
    for (name, document_text) in EXAMPLES {
        let document: Value = serde_json::from_str(document_text).unwrap();
        let mut measured_outputs = Vec::new();
        let mut svg_outputs = Vec::new();
        for theme_name in THEMES {
            let theme = common::theme(theme_name);
            let page: Page = serde_json::from_str(document_text).unwrap();
            let geometry = common::layout_with_cosmic_text(&page);
            let scene = (page.projection == Projection::Iso).then(|| {
                let inputs = SolidInputs::new(&geometry, &page.links, theme.iso.slab_thickness);
                project_page(&geometry, &inputs).unwrap()
            });
            let svg = render_svg(&page, &theme, &geometry).unwrap();
            let measured =
                serde_json::to_vec_pretty(&measured_json(&document, &geometry, scene.as_ref()))
                    .unwrap();
            measured_outputs.push(measured);
            svg_outputs.push(svg.svg);
        }
        for (index, theme_name) in THEMES.iter().enumerate().skip(1) {
            assert_eq!(
                measured_outputs[0], measured_outputs[index],
                "{name} center vs {theme_name}"
            );
        }
        for (first, first_name) in THEMES.iter().enumerate() {
            for (second, second_name) in THEMES.iter().enumerate().skip(first + 1) {
                assert_ne!(
                    svg_outputs[first], svg_outputs[second],
                    "{name} {first_name} vs {second_name}"
                );
            }
        }
    }
}

/// Section 13.14: the imported tier changes paint only. `canvas`, `nodes` and `links` equal
/// center's under every imported theme, and `projection` does wherever the slab thickness
/// is center's 6 (section 13.9 thickens a slab whose top barely differs from the page).
#[test]
fn the_imported_tier_keeps_center_geometry_and_changes_the_svg() {
    let center = common::theme("center");
    let mut examined = 0;
    for (name, document_text) in EXAMPLES {
        let document: Value = serde_json::from_str(document_text).unwrap();
        let page: Page = serde_json::from_str(document_text).unwrap();
        let geometry = common::layout_with_cosmic_text(&page);
        let measured_under = |theme: &Theme| {
            let scene = (page.projection == Projection::Iso).then(|| {
                let inputs = SolidInputs::new(&geometry, &page.links, theme.iso.slab_thickness);
                project_page(&geometry, &inputs).unwrap()
            });
            measured_json(&document, &geometry, scene.as_ref())
        };
        let center_measured = measured_under(&center);
        let center_svg = render_svg(&page, &center, &geometry).unwrap().svg;
        for theme_name in IMPORTED_THEMES {
            let theme = common::theme(theme_name);
            let measured = measured_under(&theme);
            for key in ["canvas", "nodes", "links"] {
                assert_eq!(
                    measured.get(key),
                    center_measured.get(key),
                    "{name} {theme_name} {key}"
                );
            }
            if theme.iso.slab_thickness == center.iso.slab_thickness {
                assert_eq!(
                    measured.get("projection"),
                    center_measured.get("projection"),
                    "{name} {theme_name} projection"
                );
            }
            let svg = render_svg(&page, &theme, &geometry).unwrap().svg;
            assert_ne!(svg, center_svg, "{name} {theme_name}");
            examined += 1;
        }
    }
    assert_eq!(examined, EXAMPLES.len() * IMPORTED_THEMES.len());
}

fn hex_color(hex: &str) -> (u8, u8, u8) {
    let channel = |range| u8::from_str_radix(&hex[range], 16).unwrap();
    (channel(1..3), channel(3..5), channel(5..7))
}

#[test]
fn png_page_background_is_the_theme_page() {
    assert_eq!(common::theme("center").page.as_str(), "#FFFFFF");
    assert_eq!(common::theme("wire").page.as_str(), "#FFFFFF");
    for name in THEMES {
        let page = common::with_theme(common::g7_page(), name);
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = common::render_svg(&page, &geometry).unwrap();
        let png = render_png(&svg.svg, svg.text_elements, DeviceScale::new(1).unwrap()).unwrap();
        let pixmap = common::decode_png(&png);
        let pixel = pixmap.pixel(2, 2).unwrap();
        let expected = hex_color(common::theme(name).page.as_str());
        assert_eq!(
            (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()),
            (expected.0, expected.1, expected.2, 255),
            "{name}"
        );
    }
}

/// (fill, stroke, stroke-width, dasharray) of a non-frame Box by the rule of section 13.4:
/// the slot fill when a tinted group, else the tone's fill; the tone's border, or the
/// slot's; in the grammar's pattern and width unless the theme draws its own width.
fn expected_zone_rect(
    theme: &Theme,
    kind: ZoneKey,
) -> (String, Option<String>, Option<String>, Option<&'static str>) {
    let (kind_name, tint) = kind.kind_and_tint();
    let (look, tint) = common::zone(kind_name, tint);
    let tone = theme.tones.get(look.tone.unwrap());
    let slot = tint
        .filter(|_| look.role == Role::Group)
        .map(|slot| theme.tint(slot));
    let fill = slot
        .map(|slot| slot.fill.as_str().to_string())
        .or_else(|| tone.fill.as_ref().map(|fill| fill.as_str().to_string()))
        .unwrap_or_else(|| "none".to_string());
    let width = theme.containers.draw_width.unwrap_or(look.border_width);
    let stroke = match look.pattern {
        BorderPattern::None => theme.containers.borderless_outline.as_ref().map(|outline| {
            (
                outline.color.as_str().to_string(),
                outline.width,
                outline.pattern,
            )
        }),
        pattern => tone
            .border
            .as_ref()
            .or(slot.map(|slot| &slot.border))
            .map(|color| {
                let line = match pattern {
                    BorderPattern::Dashed => LinePattern::Dashed,
                    BorderPattern::Dotted => LinePattern::Dotted,
                    BorderPattern::Solid | BorderPattern::None => LinePattern::Solid,
                };
                (color.as_str().to_string(), width, line)
            }),
    };
    let dash = stroke.as_ref().and_then(|(_, _, line)| match line {
        LinePattern::Solid => None,
        LinePattern::Dashed => Some("6 5"),
        LinePattern::Dotted => Some("2 3"),
    });
    (
        fill,
        stroke.as_ref().map(|(color, _, _)| color.clone()),
        stroke
            .as_ref()
            .map(|(_, width, _)| stencil_render::format_number(*width).to_string()),
        dash,
    )
}

#[test]
fn zones_paint_by_role_tone_and_tint_in_every_theme() {
    for theme_name in THEMES {
        let theme = common::theme(theme_name);
        let rendered = render_themed(palette_document(), theme_name);
        let document = common::parse_xml(&rendered.svg.svg);
        for (index, kind) in ZoneKey::ALL.iter().enumerate() {
            let group = common::group(&document, &format!("/body/0/children/{index}"));
            let label = common::children_named(group, "text")[0];
            let context = format!("{theme_name} {}", kind.as_str());
            if *kind == ZoneKey::Gcp {
                let paths = common::children_named(group, "path");
                assert_eq!(
                    paths[0].attribute("fill"),
                    Some(theme.frame.frame_fill.as_str())
                );
                assert_eq!(
                    paths[1].attribute("stroke"),
                    Some(theme.frame.border.as_str())
                );
                let rects = common::children_named(group, "rect");
                assert_eq!(
                    rects[0].attribute("fill"),
                    Some(theme.frame.bar_fill.as_str()),
                    "{context} bar"
                );
                assert_eq!(
                    rects[1].attribute("fill"),
                    Some(theme.frame.body_fill.as_str()),
                    "{context} body"
                );
                assert_eq!(
                    label.attribute("fill"),
                    Some(theme.frame.bar_ink.as_str()),
                    "{context}"
                );
                continue;
            }
            let rect = common::children_named(group, "rect")[0];
            let (fill, stroke, stroke_width, dash) = expected_zone_rect(&theme, *kind);
            assert_eq!(rect.attribute("fill"), Some(fill.as_str()), "{context}");
            assert_eq!(rect.attribute("stroke"), stroke.as_deref(), "{context}");
            assert_eq!(
                rect.attribute("stroke-width"),
                stroke_width.as_deref(),
                "{context}"
            );
            assert_eq!(rect.attribute("stroke-dasharray"), dash, "{context}");
            let (kind_name, tint) = kind.kind_and_tint();
            let (look, tint) = common::zone(kind_name, tint);
            let tone = theme.tones.get(look.tone.unwrap());
            let label_ink = match tint.filter(|_| look.role == Role::Group) {
                Some(slot) => &theme.tint(slot).ink,
                None => tone.label_ink.as_ref().unwrap_or(&theme.ink.zone_label),
            };
            assert_eq!(
                label.attribute("fill"),
                Some(label_ink.as_str()),
                "{context}"
            );
        }
    }
}

#[test]
fn wires_tags_and_deny_follow_the_theme_in_every_theme() {
    for theme_name in THEMES {
        let theme = common::theme(theme_name);
        let rendered = render_themed(palette_document(), theme_name);
        let document = common::parse_xml(&rendered.svg.svg);
        let dash_of = |pattern: LinePattern| match pattern {
            LinePattern::Solid => None,
            LinePattern::Dashed => Some("6 5"),
            LinePattern::Dotted => Some("2 3"),
        };
        for (index, kind) in LineKey::ALL.iter().enumerate() {
            let slot = kind.tint().unwrap_or(1);
            let (color, dash) = match kind {
                LineKey::Gray => (&theme.gray.color, dash_of(theme.gray.pattern)),
                LineKey::Blue | LineKey::Pink => (&theme.tint(slot).wire, None),
                LineKey::Dash => (&theme.tint(1).wire, dash_of(theme.dash.pattern)),
                LineKey::Deny => (&theme.deny.color, dash_of(theme.deny.pattern)),
            };
            let context = format!("{theme_name} {}", kind.as_str());
            let pipe = common::group(&document, &format!("/body/1/children/{index}"));
            for wire in common::children_named(pipe, "line") {
                assert_eq!(wire.attribute("stroke"), Some(color.as_str()), "{context}");
                assert_eq!(wire.attribute("stroke-dasharray"), dash, "{context}");
            }
            let (tag_border, tag_ink) = if *kind == LineKey::Deny {
                (&theme.deny.tag_border, &theme.deny.tag_ink)
            } else {
                (&theme.tag.border.color, &theme.tag.ink)
            };
            for pointer in [
                format!("/body/1/children/{index}"),
                format!("/body/2/children/{index}"),
            ] {
                let group = common::group(&document, &pointer);
                let tag = common::children_named(group, "rect")[0];
                assert_eq!(
                    tag.attribute("fill"),
                    Some(theme.tag.fill.as_str()),
                    "{context} {pointer}"
                );
                assert_eq!(
                    tag.attribute("stroke"),
                    Some(tag_border.as_str()),
                    "{context} {pointer}"
                );
                let texts = common::children_named(group, "text");
                assert_eq!(
                    texts[0].attribute("fill"),
                    Some(tag_ink.as_str()),
                    "{context} {pointer}"
                );
            }
            let pipe_texts = common::children_named(pipe, "text");
            assert_eq!(
                pipe_texts[1].attribute("fill"),
                Some(theme.tag.sub_ink.as_str()),
                "{context} tag sub"
            );
            let entry = common::group(&document, &format!("/legend/{index}"));
            let swatch = common::children_named(entry, "line")[0];
            assert_eq!(
                swatch.attribute("stroke"),
                Some(color.as_str()),
                "{context}"
            );
        }
    }
}

#[test]
fn page_card_and_text_inks_follow_the_theme_in_every_theme() {
    for theme_name in THEMES {
        let theme = common::theme(theme_name);
        let rendered = render_themed(palette_document(), theme_name);
        let document = common::parse_xml(&rendered.svg.svg);
        let first_text_fill = |pointer: &str| {
            common::children_named(common::group(&document, pointer), "text")[0]
                .attribute("fill")
                .unwrap()
        };
        let kicker_texts = common::children_named(common::group(&document, "/kicker"), "text");
        let ink = |color: &stencil_model::theme::Color| Some(color.as_str().to_string());
        let fill_of = |node: roxmltree::Node<'_, '_>| node.attribute("fill").map(str::to_string);
        assert_eq!(fill_of(kicker_texts[0]), ink(&theme.badge.customer.ink));
        assert_eq!(fill_of(kicker_texts[1]), ink(&theme.kicker));
        assert_eq!(first_text_fill("/title"), theme.ink.primary.as_str());
        assert_eq!(first_text_fill("/lede"), theme.ink.secondary.as_str());
        assert_eq!(first_text_fill("/foot"), theme.foot.as_str());
        let legend_texts = common::children_named(common::group(&document, "/legend/0"), "text");
        assert_eq!(fill_of(legend_texts[0]), ink(&theme.legend.label_ink));
        assert_eq!(fill_of(legend_texts[1]), ink(&theme.legend.text_ink));

        let card = common::group(&document, "/body/3");
        let rects = common::children_named(card, "rect");
        assert_eq!(fill_of(rects[0]), ink(&theme.card.fill), "{theme_name}");
        assert_eq!(
            rects[0].attribute("stroke"),
            Some(theme.card.border.color.as_str())
        );
        let boxes = if theme.icon_chip.is_some() { 2 } else { 1 };
        assert_eq!(fill_of(rects[boxes]), ink(&theme.fact.fill), "{theme_name}");
        assert_eq!(
            fill_of(rects[boxes + 1]),
            ink(&theme.ask.fill),
            "{theme_name}"
        );
        let card_texts: Vec<_> = common::children_named(card, "text")
            .iter()
            .map(|text| text.attribute("fill").unwrap())
            .collect();
        assert_eq!(
            card_texts,
            [
                theme.ink.primary.as_str(),
                theme.ink.secondary.as_str(),
                theme.fact.ink.as_str(),
                theme.ask.ink.as_str()
            ],
            "{theme_name}"
        );
    }
}

#[test]
fn badges_follow_the_theme_and_canvas() {
    let literal = [
        ("center", "customer", "#E8F0FE", None, "#174EA6"),
        ("center", "internal", "#F3E5F5", None, "#7B1FA2"),
        ("wire", "customer", "#FFFFFF", Some("#222222"), "#222222"),
        ("wire", "internal", "#FFFFFF", Some("#222222"), "#222222"),
    ];
    for (theme_name, canvas, fill, stroke, ink) in literal {
        let theme = common::theme(theme_name);
        let swatch = match canvas {
            "customer" => &theme.badge.customer,
            _ => &theme.badge.internal,
        };
        assert_eq!(swatch.fill.as_str(), fill);
        assert_eq!(swatch.ink.as_str(), ink);
        assert_eq!(
            theme
                .badge
                .border
                .as_ref()
                .map(|border| border.color.as_str()),
            stroke
        );
    }
    for theme_name in THEMES {
        let theme = common::theme(theme_name);
        for canvas in ["customer", "internal"] {
            let swatch = match canvas {
                "customer" => &theme.badge.customer,
                _ => &theme.badge.internal,
            };
            let mut document = palette_document();
            document["canvas"] = json!(canvas);
            let rendered = render_themed(document, theme_name);
            let svg = common::parse_xml(&rendered.svg.svg);
            let kicker = common::group(&svg, "/kicker");
            let badge = common::children_named(kicker, "rect")[0];
            let context = format!("{theme_name} {canvas}");
            assert_eq!(
                badge.attribute("fill"),
                Some(swatch.fill.as_str()),
                "{context}"
            );
            assert_eq!(
                badge.attribute("stroke"),
                theme
                    .badge
                    .border
                    .as_ref()
                    .map(|border| border.color.as_str()),
                "{context}"
            );
            let badge_text = common::children_named(kicker, "text")[0];
            assert_eq!(
                badge_text.attribute("fill"),
                Some(swatch.ink.as_str()),
                "{context}"
            );
        }
    }
}

#[test]
fn wire_zone_borders_follow_the_grammar_pattern_and_fills_stay_white() {
    let rendered = render_themed(palette_document(), "wire");
    let document = common::parse_xml(&rendered.svg.svg);
    for (index, kind) in ZoneKey::ALL.iter().enumerate() {
        let group = common::group(&document, &format!("/body/0/children/{index}"));
        if *kind == ZoneKey::Gcp {
            let paths = common::children_named(group, "path");
            assert_eq!(paths[0].attribute("fill"), Some("#FFFFFF"));
            assert_eq!(paths[1].attribute("stroke"), Some("#222222"));
            assert_eq!(paths[1].attribute("stroke-width"), Some("2"));
            assert_eq!(paths[1].attribute("stroke-dasharray"), None);
            continue;
        }
        let rect = common::children_named(group, "rect")[0];
        let expected_dash = match kind {
            ZoneKey::Vpc | ZoneKey::Optional | ZoneKey::Perimeter | ZoneKey::Subnet => Some("6 5"),
            _ => None,
        };
        let expected_fill = if *kind == ZoneKey::Vpc {
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
    let rendered = render_themed(palette_document(), "wire");
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
        .part(PartName::Bar)
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
fn wire_lines_are_told_apart_by_line_style_and_end_dots() {
    let rendered = render_themed(palette_document(), "wire");
    let document = common::parse_xml(&rendered.svg.svg);
    let expected = |kind: LineKey| match kind {
        LineKey::Gray => ("1.25", None),
        LineKey::Blue | LineKey::Pink => ("2", None),
        LineKey::Dash => ("2", Some("6 5")),
        LineKey::Deny => ("2", Some("2 3")),
    };
    for (index, kind) in LineKey::ALL.iter().enumerate() {
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
            if *kind == LineKey::Pink {
                assert_eq!(dot.attribute("fill"), Some("#FFFFFF"), "{name}");
                assert_eq!(dot.attribute("stroke"), Some("#222222"), "{name}");
            } else {
                assert_eq!(dot.attribute("fill"), Some("#222222"), "{name}");
                assert_eq!(dot.attribute("stroke"), None, "{name}");
            }
        }
        let expected_swatch_dots = if *kind == LineKey::Pink { 2 } else { 0 };
        assert_eq!(swatch_dots.len(), expected_swatch_dots, "{name}");
        let tag = common::children_named(pipe, "rect")[0];
        assert_eq!(tag.attribute("fill"), Some("#FFFFFF"), "{name}");
        assert_eq!(tag.attribute("stroke"), Some("#222222"), "{name}");
    }
}

#[test]
fn a_solid_slot_whose_dot_is_none_draws_no_end_dots() {
    let wire = common::theme("wire");
    assert_eq!(wire.solid.dots[2], DotStyle::None);
    let document = common::page_document(
        json!([{ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 3, "label": "plain" }]),
        json!([{ "line": "solid", "tint": 3, "text": "plain line" }]),
    );
    let rendered = render_themed(document, "wire");
    let svg = common::parse_xml(&rendered.svg.svg);
    let pipe = common::group(&svg, "/body/0");
    assert!(common::children_named(pipe, "circle").is_empty());
    assert_eq!(common::children_named(pipe, "line").len(), 2);
    let entry = common::group(&svg, "/legend/0");
    assert!(common::children_named(entry, "circle").is_empty());
    let label = common::children_named(entry, "text")[0];
    assert_eq!(label.text(), Some("Plain line"));
}

#[test]
fn wire_paints_only_white_fills_and_two_inks() {
    let rendered = render_themed(palette_document(), "wire");
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
fn icon_chip_sits_under_the_icon_when_the_theme_sets_one() {
    for theme_name in THEMES {
        let theme = common::theme(theme_name);
        let rendered = render_themed(palette_document(), theme_name);
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
            .part(PartName::Icon)
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
        let Some(chip_fill) = Palette::new(&theme, Projection::Flat).icon_chip() else {
            assert!(chips.is_empty(), "{theme_name}");
            continue;
        };
        assert_eq!(chips.len(), 1, "{theme_name}");
        let chip = chips[0];
        assert_eq!(chip.attribute("height"), Some("36"), "{theme_name}");
        assert_eq!(chip.attribute("rx"), Some("6"), "{theme_name}");
        assert_eq!(chip.attribute("fill"), Some(chip_fill), "{theme_name}");
        let chip_x: f32 = chip.attribute("x").unwrap().parse().unwrap();
        let chip_y: f32 = chip.attribute("y").unwrap().parse().unwrap();
        assert!((chip_x - (icon.x - 4.0)).abs() <= 0.006, "{theme_name}");
        assert!((chip_y - (icon.y - 4.0)).abs() <= 0.006, "{theme_name}");
        let chip_position = elements
            .iter()
            .position(|element| *element == chip)
            .unwrap();
        assert_eq!(
            chip_position + 1,
            image_position,
            "{theme_name}: chip right under the icon"
        );
    }
    assert_eq!(common::theme("center").icon_chip, None);
    for dark in ["dusk", "clear-dark"] {
        assert_eq!(
            common::theme(dark)
                .icon_chip
                .as_ref()
                .map(|chip| chip.as_str()),
            Some("#FFFFFF"),
            "{dark}"
        );
    }
}

/// The text of the legend label and the x of the description of each legend entry.
fn legend_runs(svg: &str, entries: usize) -> Vec<(String, f32)> {
    let document = common::parse_xml(svg);
    (0..entries)
        .map(|index| {
            let entry = common::group(&document, &format!("/legend/{index}"));
            let texts = common::children_named(entry, "text");
            let x: f32 = texts[1].attribute("x").unwrap().parse().unwrap();
            (texts[0].text().unwrap().to_string(), x)
        })
        .collect()
}

fn relabel_document() -> Value {
    let legend = json!([
        { "line": "solid", "tint": 1, "text": "primary" },
        { "line": "solid", "tint": 2, "text": "secondary" },
        { "line": "dash", "text": "failover" },
        { "line": "gray", "text": "internal" },
        { "line": "deny", "text": "blocked" }
    ]);
    let pipes = json!([
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "a" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 2, "label": "b" },
        { "tag": "Pipe", "dir": "h", "line": "dash", "label": "c" },
        { "tag": "Pipe", "dir": "h", "line": "gray", "label": "d" },
        { "tag": "Pipe", "dir": "h", "line": "deny", "label": "e" }
    ]);
    common::page_document(json!([{ "tag": "Row", "children": pipes }]), legend)
}

#[test]
fn wire_relabels_the_legend_by_line_in_flat_and_iso_moving_each_description() {
    use stencil_model::text::TextMeasurer;
    let mut measurer = stencil_text::CosmicTextMeasurer::new().unwrap();
    let style = stencil_model::text::TextStyleName::LegendLabel
        .text_style()
        .style;
    for projection in ["flat", "iso"] {
        let mut document = with_theme(relabel_document(), "wire");
        document["projection"] = json!(projection);
        let page: Page = serde_json::from_value(document.clone()).unwrap();
        let geometry = common::layout_with_cosmic_text(&page);
        let center_page: Page =
            serde_json::from_value(with_theme(document.clone(), "center")).unwrap();
        let wire_svg = common::render_svg(&page, &geometry).unwrap();
        let center_svg = common::render_svg(&center_page, &geometry).unwrap();
        let wire_runs = legend_runs(&wire_svg.svg, 5);
        let center_runs = legend_runs(&center_svg.svg, 5);
        let labels: Vec<&str> = wire_runs.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Solid line",
                "Ringed line",
                "Dashed line",
                "Thin line",
                "Dotted line"
            ],
            "{projection}"
        );
        for ((label, wire_x), (canonical, center_x)) in wire_runs.iter().zip(&center_runs) {
            let change = measurer.measure(label, &style, None).unwrap().width_px
                - measurer.measure(canonical, &style, None).unwrap().width_px;
            assert!(
                (wire_x - center_x - change).abs() <= 0.011,
                "{projection} {label}: moved {} for a change of {change}",
                wire_x - center_x
            );
        }
    }
}

#[test]
fn paper_names_slot_two_rose_and_center_moves_no_entry() {
    let rendered = render_themed(relabel_document(), "paper");
    let runs = legend_runs(&rendered.svg.svg, 5);
    assert_eq!(runs[0].0, "Solid blue");
    assert_eq!(runs[1].0, "Solid rose");
    let center = render_themed(relabel_document(), "center");
    let center_runs = legend_runs(&center.svg.svg, 5);
    let labels: Vec<&str> = center_runs
        .iter()
        .map(|(label, _)| label.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            "Solid blue",
            "Solid pink",
            "Dashed blue",
            "Solid gray",
            "Dashed red"
        ]
    );
    let entries: Vec<_> = center
        .geometry
        .nodes
        .iter()
        .filter(|node| node.tag == stencil_layout::NodeTag::LegendEntry)
        .collect();
    assert_eq!(entries.len(), 5);
    for (entry, (_, description_x)) in entries.iter().zip(&center_runs) {
        let text_part = entry.part(PartName::LegendText).unwrap();
        assert!(
            (description_x - text_part.bounds.x).abs() <= 0.006,
            "center moved {}",
            entry.pointer
        );
    }
}

#[test]
fn overrides_recolor_one_slot_wire_and_nothing_else() {
    let base = common::theme("center");
    let mut overrides = serde_json::Map::new();
    overrides.insert("tints".to_string(), json!({ "3": { "wire": "#00796B" } }));
    let overridden = apply_overrides(&base, &overrides).unwrap();
    assert_eq!(overridden.tints[2].wire.as_str(), "#00796B");
    let document = common::page_document(
        json!([{ "tag": "Col", "children": [
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 3, "label": "slot three" },
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "slot one" },
            { "tag": "Box", "kind": "region", "tint": 3, "label": "Region", "children": [
                { "tag": "Fact", "text": "fact" }
            ] }
        ] }]),
        json!([
            { "line": "solid", "tint": 3, "text": "three" },
            { "line": "solid", "tint": 1, "text": "one" }
        ]),
    );
    let page: Page = serde_json::from_value(document).unwrap();
    let geometry = common::layout_with_fixed_metrics(&page);
    let before = render_svg(&page, &base, &geometry).unwrap().svg;
    let after = render_svg(&page, &overridden, &geometry).unwrap().svg;
    assert_eq!(before.lines().count(), after.lines().count());
    let changed: Vec<(&str, &str)> = before
        .lines()
        .zip(after.lines())
        .filter(|(old, new)| old != new)
        .collect();
    assert_eq!(
        changed.len(),
        5,
        "the two wire halves and two end dots of the slot 3 pipe and its legend swatch"
    );
    for (old, new) in &changed {
        assert!(old.contains("#007D78"), "{old}");
        assert_eq!(old.replace("#007D78", "#00796B"), *new);
    }
}

#[test]
fn center_inks_equal_the_layout_text_colors() {
    use stencil_layout::styles;
    use stencil_model::text::TEXT_STYLES;
    use stencil_model::{Canvas, Line};
    let center = common::theme("center");
    let palette = Palette::new(&center, Projection::Flat);
    for named in TEXT_STYLES {
        for canvas in [Canvas::Customer, Canvas::Internal] {
            for line in [None, Some(Line::Gray), Some(Line::Deny)] {
                assert_eq!(
                    palette.text_ink(named.name, canvas, line),
                    styles::text_color(named.name, canvas, line),
                    "{}",
                    named.name.as_str()
                );
            }
        }
    }
    for canvas in [Canvas::Customer, Canvas::Internal] {
        assert_eq!(
            palette.badge(canvas).fill,
            styles::badge_fill(canvas),
            "{canvas:?}"
        );
    }
    let slot_names: Vec<&str> = center.tints.iter().map(|tint| tint.name.as_str()).collect();
    assert_eq!(slot_names, stencil_model::TINT_NAMES);
}

#[test]
fn only_the_wire_theme_tells_tints_apart_by_line() {
    let wire = common::theme("wire");
    assert_eq!(wire.tint_cue, TintCue::Line);
    for name in THEMES.iter().filter(|name| **name != "wire") {
        assert_eq!(common::theme(name).tint_cue, TintCue::Color, "{name}");
    }
}

/// Section 13.5's derived roles, which hold for every theme taken from the palette research.
#[test]
fn research_themes_follow_the_derivation_rules() {
    use stencil_model::theme::{SolidEdges, lightness};
    use stencil_render::palette::{mix, shade};
    for name in ["paper", "dusk", "clear", "clear-dark"] {
        let theme = common::theme(name);
        let dark = lightness(&theme.page).unwrap() < 50.0;
        let slot = |number: u8| theme.tint(number);
        assert_eq!(theme.frame.frame_fill, theme.page, "{name}");
        assert_eq!(theme.ink.zone_label, theme.ink.secondary, "{name}");
        assert_eq!(theme.legend.label_ink, theme.ink.primary, "{name}");
        assert_eq!(
            theme.placeholder.border,
            theme.tones.strong.border.clone().unwrap()
        );
        assert_eq!(theme.ask.fill, slot(4).fill, "{name}");
        assert_eq!(theme.ask.ink, slot(4).ink, "{name}");
        let internal = if theme.badge.customer.fill == slot(1).wire {
            (&slot(5).wire, &theme.page)
        } else {
            (&slot(5).fill, &slot(5).ink)
        };
        assert_eq!(
            (&theme.badge.internal.fill, &theme.badge.internal.ink),
            internal,
            "{name}"
        );
        assert_eq!(theme.callout.note.accent, slot(1).wire, "{name}");
        assert_eq!(theme.callout.note.fill, slot(1).fill, "{name}");
        assert_eq!(theme.callout.decision.accent, slot(6).wire, "{name}");
        assert_eq!(theme.callout.decision.fill, slot(6).fill, "{name}");
        assert_eq!(theme.callout.open.accent, slot(4).wire, "{name}");
        assert_eq!(theme.callout.open.fill, slot(4).fill, "{name}");
        assert_eq!(theme.callout.risk.accent, theme.deny.color, "{name}");
        let ratio = if dark { 0.18 } else { 0.22 };
        assert_eq!(
            Some(theme.callout.risk.fill.as_str().to_string()),
            mix(theme.page.as_str(), theme.deny.color.as_str(), ratio),
            "{name}"
        );
        let faces = &theme.iso.faces;
        let steps = if dark { (0, -4, -8) } else { (0, -8, -16) };
        assert_eq!((faces.top, faces.left, faces.right), steps, "{name}");
        let sides = theme.iso.frame_sides.as_ref().unwrap();
        assert_eq!(sides.left, theme.frame.border, "{name}");
        assert_eq!(
            Some(sides.right.as_str().to_string()),
            shade(theme.frame.border.as_str(), -12)
        );
        assert_eq!(theme.iso.solid_edges, SolidEdges::None);
        assert_eq!(theme.iso.plates, !dark, "{name}");
        assert_eq!(theme.iso.tabs.frame.fill, theme.frame.bar_fill);
        assert_eq!(theme.iso.tabs.frame.ink, theme.frame.bar_ink);
        let top_tab = if dark {
            (&theme.card.border.color, &theme.ink.primary)
        } else {
            (&theme.ink.secondary, &theme.page)
        };
        assert_eq!(
            (&theme.iso.tabs.top.fill, &theme.iso.tabs.top.ink),
            top_tab,
            "{name}"
        );
        assert_eq!(theme.iso.chip.fill.as_str(), "#FFFFFF");
        assert_eq!(theme.iso.chip.ring.is_some(), !dark, "{name}");
        assert_eq!(theme.iso.chip.shadow.is_some(), !dark, "{name}");
        if dark {
            assert_eq!(
                theme.icon_chip.as_ref().map(|chip| chip.as_str()),
                Some("#FFFFFF")
            );
        }
        assert_eq!(theme.lanes.lifeline.color, theme.ink.secondary, "{name}");
        assert_eq!(theme.lanes.lifeline.width, 1.0);
        assert_eq!(theme.lanes.lifeline.pattern, LinePattern::Dashed);
        assert_eq!(
            (
                theme.iso.widths.primary,
                theme.iso.widths.secondary,
                theme.iso.widths.gray
            ),
            (3.75, 2.5, 2.0)
        );
        assert_eq!(theme.tint_cue, TintCue::Color);
        assert!(theme.solid.dots.iter().all(|dot| *dot == DotStyle::Filled));
    }
}

#[test]
fn dusk_takes_the_widened_surface_ladder() {
    let dusk = common::theme("dusk");
    let fill = |tone: &stencil_model::theme::ToneRole| tone.fill.clone().unwrap();
    assert_eq!(dusk.page.as_str(), "#0D0F17");
    assert_eq!(dusk.frame.body_fill.as_str(), "#161926");
    let tones = &dusk.tones;
    let ladder = [
        fill(&tones.cool),
        fill(&tones.warm),
        fill(&tones.highlight),
        fill(&tones.emphasis),
        fill(&tones.soft),
        fill(&tones.accent),
        fill(&tones.neutral),
    ];
    let expected = [
        "#1E2231", "#222229", "#1E2232", "#1F222D", "#192331", "#26221C", "#1B232E",
    ];
    for (color, expected) in ladder.iter().zip(expected) {
        assert_eq!(color.as_str(), expected);
    }
    assert_eq!(dusk.card.fill.as_str(), "#272B3D");
    assert_eq!(dusk.tag.fill.as_str(), "#272B3D");
    assert_eq!(dusk.fact.fill.as_str(), "#2E3450");
}

#[test]
fn clear_keeps_each_slot_in_its_hue_family() {
    for name in ["clear", "clear-dark"] {
        let names: Vec<String> = common::theme(name)
            .tints
            .iter()
            .map(|tint| tint.name.clone())
            .collect();
        assert_eq!(
            names,
            [
                "blue", "pink", "teal", "ochre", "indigo", "green", "orange", "olive"
            ],
            "{name}"
        );
    }
    let paper: Vec<String> = common::theme("paper")
        .tints
        .iter()
        .map(|tint| tint.name.clone())
        .collect();
    assert_eq!(
        paper,
        [
            "blue", "rose", "teal", "ochre", "violet", "green", "clay", "petrol"
        ]
    );
}
