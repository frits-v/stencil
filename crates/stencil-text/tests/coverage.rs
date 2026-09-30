//! Every string g7 draws has a glyph in the face its style uses (spec section 10).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use stencil_model::text::{TextMeasurer, TextStyleName};
use stencil_model::{
    Canvas, Node, NodeRef, NoteKind, Page, PipeKind, ZoneKind, body_nodes, parse_page, text_fields,
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

fn legend_label(kind: PipeKind) -> &'static str {
    match kind {
        PipeKind::Gray => "Solid gray",
        PipeKind::Blue => "Solid blue",
        PipeKind::Pink => "Solid pink",
        PipeKind::Dash => "Dashed blue",
        PipeKind::Deny => "Dashed red",
    }
}

/// The section 2.2 to 2.8 style of every authored text value, plus the fixed strings layout
/// adds (the canvas badge, the `Ask: ` prefix and the legend labels). Returns the drawn
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
            NodeRef::Node(Node::Row(_) | Node::Col(_)) => {}
            NodeRef::Node(Node::Zone(zone)) => {
                let style = match zone.kind {
                    ZoneKind::Gcp => TextStyleName::GcpBar,
                    ZoneKind::Perimeter => TextStyleName::PerimeterLabel,
                    _ => TextStyleName::ZoneLabel,
                };
                authored.push(drawn(&zone.label, style));
            }
            NodeRef::Node(Node::Pcard(pcard)) => {
                authored.push(drawn(&pcard.function_name, TextStyleName::CardFunction));
                if let Some(product_name) = &pcard.product_name {
                    authored.push(drawn(product_name, TextStyleName::CardProduct));
                }
                if let Some(fact) = &pcard.fact {
                    authored.push(drawn(fact, TextStyleName::Fact));
                }
                if let Some(ask) = &pcard.ask {
                    authored.push(drawn(&format!("Ask: {ask}"), TextStyleName::Ask));
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
        authored.push(drawn(legend_label(entry.kind), TextStyleName::LegendLabel));
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
