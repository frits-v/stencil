//! Every string g7 draws has a glyph in the face its style uses (spec section 10).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use stencil_model::text::{TextMeasurer, TextStyleName};
use stencil_model::{
    Canvas, FactSource, Node, NodeRef, NoteKind, Page, body_nodes, legend_label, parse_page,
    text_fields,
};
use stencil_text::CosmicTextMeasurer;

const G7_JSON: &str = include_str!("../../../examples/g7.json");

/// A string as layout draws it, with the style that draws it.
struct DrawnText {
    text: String,
    style: TextStyleName,
}

fn drawn(text: &str, style: TextStyleName) -> DrawnText {
    let text = if style.text_style().uppercase {
        text.to_uppercase()
    } else {
        text.to_string()
    };
    DrawnText { text, style }
}

/// The section 2.2 to 2.8 style of every authored text value, plus the fixed strings layout
/// adds (the canvas badge, the `• ` and `Ask: ` prefixes and the legend labels). Returns the drawn
/// strings and how many of them are authored text values.
fn drawn_texts(page: &Page) -> (Vec<DrawnText>, usize) {
    let mut authored = vec![
        drawn(&page.kicker, TextStyleName::Kicker),
        drawn(&page.title, TextStyleName::Title),
        drawn(&page.lede, TextStyleName::Lede),
    ];
    if let Some(foot) = &page.foot {
        authored.push(drawn(foot, TextStyleName::Foot));
    }
    for entry in body_nodes(page) {
        match entry.node {
            NodeRef::Node(Node::Row(_) | Node::Col(_) | Node::Lanes(_)) => {}
            NodeRef::Node(Node::Box(box_node)) => {
                let style = match box_node.kind.as_str() {
                    "gcp" => TextStyleName::GcpBar,
                    "perimeter" => TextStyleName::PerimeterLabel,
                    _ => TextStyleName::ZoneLabel,
                };
                authored.push(drawn(&box_node.label, style));
            }
            NodeRef::Node(Node::Item(item)) => {
                authored.push(drawn(&item.title, TextStyleName::CardFunction));
                if let Some(subtitle) = &item.subtitle {
                    authored.push(drawn(subtitle, TextStyleName::CardProduct));
                }
                for fact in &item.facts {
                    authored.push(match fact.source {
                        FactSource::Doc => drawn(&fact.text, TextStyleName::Fact),
                        FactSource::Built => {
                            drawn(&format!("\u{2022} {}", fact.text), TextStyleName::Fact)
                        }
                        FactSource::Ask => {
                            drawn(&format!("Ask: {}", fact.text), TextStyleName::Ask)
                        }
                    });
                }
            }
            NodeRef::Node(Node::Fact(fact)) => {
                authored.push(drawn(&fact.text, TextStyleName::Fact))
            }
            NodeRef::Node(Node::Note(note)) => {
                let style = match note.kind {
                    NoteKind::Kicker => TextStyleName::Kicker,
                    NoteKind::H1 => TextStyleName::Title,
                    NoteKind::Lede => TextStyleName::Lede,
                    NoteKind::Legend => TextStyleName::NoteLegend,
                    NoteKind::Foot => TextStyleName::Foot,
                };
                authored.push(drawn(&note.text, style));
            }
            NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => {
                authored.push(drawn(&pipe.label, TextStyleName::TagLabel));
                if let Some(sub) = &pipe.sub {
                    authored.push(drawn(sub, TextStyleName::TagSub));
                }
            }
            NodeRef::Node(Node::Tee(tee)) => {
                authored.push(drawn(&tee.hub, TextStyleName::TagLabel))
            }
            // Section 11.3 blocks have no TextStyleName yet; the authored-count assertion
            // below fails if g7 ever holds one.
            NodeRef::Node(Node::Text(_) | Node::Callout(_) | Node::Frame(_)) => {}
        }
    }
    for entry in &page.legend {
        authored.push(drawn(&entry.text, TextStyleName::LegendText));
    }
    let authored_count = authored.len();

    let badge = match page.canvas {
        Canvas::Customer => "Customer",
        Canvas::Internal => "Internal",
    };
    authored.push(drawn(badge, TextStyleName::Badge));
    for entry in &page.legend {
        authored.push(drawn(
            legend_label(entry.line, entry.tint),
            TextStyleName::LegendLabel,
        ));
    }
    (authored, authored_count)
}

#[test]
fn every_g7_character_has_a_glyph_in_its_style_weight() {
    let page = parse_page(G7_JSON).unwrap();
    let (texts, authored_count) = drawn_texts(&page);
    assert_eq!(
        authored_count,
        text_fields(&page).len(),
        "the style map must cover every authored text value"
    );

    let mut measurer = CosmicTextMeasurer::new().unwrap();
    let mut examined_characters = BTreeSet::new();
    for drawn_text in &texts {
        let style = drawn_text.style.text_style().style;
        measurer
            .measure(&drawn_text.text, &style, None)
            .unwrap_or_else(|error| panic!("{:?}: {error}", drawn_text.text));
        for character in drawn_text
            .text
            .chars()
            .filter(|character| *character != ' ')
        {
            measurer
                .measure(&character.to_string(), &style, None)
                .unwrap_or_else(|error| {
                    panic!("{character:?} in {:?}: {error}", drawn_text.style.as_str())
                });
            examined_characters.insert(character);
        }
    }
    assert!(examined_characters.contains(&'·'));
    assert!(examined_characters.contains(&'↔'));
    assert!(examined_characters.len() > 40, "{examined_characters:?}");
}

#[test]
fn the_built_fact_bullet_has_a_glyph_in_the_fact_face() {
    let mut measurer = CosmicTextMeasurer::new().unwrap();
    let style = TextStyleName::Fact.text_style().style;
    let metrics = measurer.measure("\u{2022} acme-raw", &style, None).unwrap();
    assert!(metrics.width_px > 0.0);
    measurer.measure("\u{2022}", &style, None).unwrap();
}

/// Section 13.11: the measurer breaks after a dot by shaping U+200B ZERO WIDTH SPACE after
/// it, which is sound only when every bundled face maps U+200B to a real glyph with no
/// advance.
#[test]
fn every_bundled_face_maps_zero_width_space_to_a_glyph_without_advance() {
    use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight, fontdb};
    use stencil_text::BUNDLED_FONTS;

    let mut database = fontdb::Database::new();
    for file in &BUNDLED_FONTS {
        database.load_font_data(file.bytes.to_vec());
    }
    let mut font_system = FontSystem::new_with_locale_and_db("en-US".into(), database);
    let mut examined_faces = 0;
    for file in &BUNDLED_FONTS {
        let attributes = Attrs::new()
            .family(Family::Name("Inter"))
            .weight(Weight(file.weight.css_value()));
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(13.0, 15.6));
        buffer.set_text("a\u{200B}b", &attributes, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut font_system, false);
        let runs: Vec<_> = buffer.layout_runs().collect();
        assert_eq!(runs.len(), 1, "{}", file.file_name);
        let zero_width_space = runs[0]
            .glyphs
            .iter()
            .find(|glyph| glyph.start == 1)
            .unwrap_or_else(|| panic!("{}: no glyph for U+200B", file.file_name));
        assert_ne!(zero_width_space.glyph_id, 0, "{}", file.file_name);
        assert_eq!(zero_width_space.w, 0.0, "{}", file.file_name);
        examined_faces += 1;
    }
    assert_eq!(examined_faces, 4);
}
