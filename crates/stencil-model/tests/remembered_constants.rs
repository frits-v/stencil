// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{g7_page, legend_entry, page_with_body, pipe_value};
use stencil_model::checks::{CheckName, remembered_constants};
use stencil_model::{
    Arrow, BoxNode, Callout, CalloutKind, FactEntry, FactSource, Frame, Item, Line, Link, ListKind,
    Node, Note, NoteKind, Page, Pipe, PipeDir, Tee, TeeArm, Text, text_fields,
};

const FILLER: &str = "x";

/// A page with one field of each kind set to `text`, all others to FILLER.
fn page_with_field(field: &str, text: &str) -> (Page, &'static str) {
    let pick = |name: &str| {
        if name == field {
            text.to_string()
        } else {
            FILLER.to_string()
        }
    };
    let mut page = page_with_body(vec![
        Node::Box(BoxNode {
            id: None,
            kind: "region".to_string(),
            tint: None,
            label: pick("zone label"),
            children: vec![Node::Item(Item {
                id: None,
                kind: "product".to_string(),
                icon: None,
                title: pick("title"),
                subtitle: Some(pick("subtitle")),
                facts: vec![
                    FactEntry {
                        text: pick("fact"),
                        source: FactSource::Doc,
                    },
                    FactEntry {
                        text: pick("built"),
                        source: FactSource::Built,
                    },
                    FactEntry {
                        text: pick("ask"),
                        source: FactSource::Ask,
                    },
                ],
                shape: None,
            })],
        }),
        Node::Note(Note {
            id: None,
            kind: NoteKind::Lede,
            text: pick("note"),
        }),
        Node::Pipe(Pipe {
            id: None,
            arrow: stencil_model::Arrow::None,
            axis: None,
            dir: PipeDir::Horizontal,
            line: Line::Solid,
            tint: Some(1),
            label: pick("pipe label"),
            sub: Some(pick("sub")),
            from: None,
            to: None,
        }),
        Node::Tee(Tee {
            id: None,
            line: Line::Solid,
            tint: Some(1),
            hub: pick("hub"),
            arms: [
                TeeArm::Pipe(pipe_value(
                    PipeDir::Horizontal,
                    Line::Solid,
                    Some(1),
                    FILLER,
                )),
                TeeArm::Pipe(pipe_value(
                    PipeDir::Horizontal,
                    Line::Solid,
                    Some(1),
                    FILLER,
                )),
            ],
        }),
        Node::Text(Text {
            id: Some("text".to_string()),
            heading: Some(pick("heading")),
            body: vec![FILLER.to_string(), pick("body line")],
            list: ListKind::Numbered,
        }),
        Node::Callout(Callout {
            id: None,
            kind: CalloutKind::Risk,
            title: Some(pick("callout title")),
            text: pick("callout text"),
        }),
        Node::Frame(Frame {
            id: Some("frame".to_string()),
            label: pick("frame label"),
            height: 200,
        }),
    ]);
    page.title = pick("page title");
    page.lede = pick("lede");
    page.foot = Some(pick("foot"));
    page.legend = vec![legend_entry(Line::Solid, Some(1), &pick("legend text"))];
    page.links = vec![Link {
        from: "text".to_string(),
        to: "frame".to_string(),
        line: Line::Solid,
        tint: Some(1),
        label: Some(pick("link label")),
        sub: Some(pick("link sub")),
        arrow: Arrow::End,
        axis: None,
        from_side: None,
        to_side: None,
        via: Vec::new(),
        order: None,
    }];
    let pointer = match field {
        "page title" => "/title",
        "lede" => "/lede",
        "foot" => "/foot",
        "legend text" => "/legend/0/text",
        "zone label" => "/body/0/label",
        "title" => "/body/0/children/0/title",
        "subtitle" => "/body/0/children/0/subtitle",
        "fact" => "/body/0/children/0/facts/0/text",
        "built" => "/body/0/children/0/facts/1/text",
        "ask" => "/body/0/children/0/facts/2/text",
        "note" => "/body/1/text",
        "pipe label" => "/body/2/label",
        "sub" => "/body/2/sub",
        "hub" => "/body/3/hub",
        "heading" => "/body/4/heading",
        "body line" => "/body/4/body/1",
        "callout title" => "/body/5/title",
        "callout text" => "/body/5/text",
        "frame label" => "/body/6/label",
        "link label" => "/links/0/label",
        "link sub" => "/links/0/sub",
        _ => panic!("unknown field {field}"),
    };
    (page, pointer)
}

const FIELD_KINDS: [&str; 21] = [
    "page title",
    "lede",
    "foot",
    "legend text",
    "zone label",
    "title",
    "subtitle",
    "fact",
    "built",
    "ask",
    "note",
    "pipe label",
    "sub",
    "hub",
    "heading",
    "body line",
    "callout title",
    "callout text",
    "frame label",
    "link label",
    "link sub",
];

#[test]
fn each_literal_fires_in_each_kind_of_text_field() {
    let grammar = common::gcp();
    assert_eq!(grammar.remembered.len(), 4);
    for constant in &grammar.remembered {
        for field in FIELD_KINDS {
            let (page, pointer) =
                page_with_field(field, &format!("peer ASN {} here", constant.literal));
            let report = remembered_constants(&page, &common::gcp());
            assert_eq!(report.defects.len(), 1, "{} in {field}", constant.literal);
            let defect = &report.defects[0];
            assert_eq!(defect.pointer.as_str(), pointer);
            assert_eq!(
                defect.message,
                format!("contains {}: {}", constant.literal, constant.reason)
            );
        }
    }
}

#[test]
fn literal_fires_at_start_and_end_of_text_and_next_to_punctuation() {
    for text in [
        "64512",
        "ASN 64512",
        "(64512)",
        "64512.",
        "AS-64512",
        "ASN·64512",
    ] {
        let (page, _) = page_with_field("page title", text);
        assert_eq!(
            remembered_constants(&page, &common::gcp()).defects.len(),
            1,
            "{text}"
        );
    }
}

#[test]
fn literals_inside_longer_tokens_do_not_fire() {
    for text in [
        "AS64512",
        "164512",
        "645120",
        "64512_",
        "10.8.0.0/280",
        "135.191.0.0/16",
        "130.211.0.0/220",
    ] {
        let (page, _) = page_with_field("title", text);
        let report = remembered_constants(&page, &common::gcp());
        assert_eq!(report.defects, vec![], "{text}");
    }
}

#[test]
fn a_bounded_occurrence_after_an_unbounded_one_fires() {
    let (page, _) = page_with_field("title", "AS64512 and 64512");
    assert_eq!(remembered_constants(&page, &common::gcp()).defects.len(), 1);
}

#[test]
fn check_scans_untransformed_text() {
    let mut page = page_with_body(vec![common::item("a")]);
    page.kicker = "asn 64512".to_string();
    let report = remembered_constants(&page, &common::gcp());
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/kicker");
}

#[test]
fn examined_equals_text_field_count() {
    for field in FIELD_KINDS {
        let (page, _) = page_with_field(field, FILLER);
        let report = remembered_constants(&page, &common::gcp());
        assert_eq!(report.examined, text_fields(&page).len() as u64);
        assert!(report.passed());
    }
}

#[test]
fn g7_examines_36_fields_and_passes() {
    let report = remembered_constants(&g7_page(), &common::gcp());
    assert_eq!(report.check, CheckName::RememberedConstants);
    assert_eq!(report.examined, 36);
    assert!(report.passed());
}

#[test]
fn two_different_literals_give_two_defects() {
    let (page, pointer) = page_with_field("sub", "ASN 64512 and range 35.191.0.0/16");
    let report = remembered_constants(&page, &common::gcp());
    let pointers: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.pointer.as_str())
        .collect();
    assert_eq!(pointers, [pointer, pointer]);
    assert!(report.defects[0].message.starts_with("contains 64512:"));
    assert!(
        report.defects[1]
            .message
            .starts_with("contains 35.191.0.0/16:")
    );
}

#[test]
fn one_literal_twice_gives_one_defect() {
    let (page, _) = page_with_field("sub", "64512 and 64512");
    assert_eq!(remembered_constants(&page, &common::gcp()).defects.len(), 1);
}

#[test]
fn the_gcp_grammar_lists_the_four_section_6_literals_in_order() {
    let literals: Vec<String> = common::gcp()
        .remembered
        .iter()
        .map(|constant| constant.literal.clone())
        .collect();
    assert_eq!(
        literals,
        ["64512", "130.211.0.0/22", "35.191.0.0/16", "10.8.0.0/28"]
    );
}

#[test]
fn a_grammar_without_remembered_literals_is_not_applicable() {
    let (page, _) = page_with_field("title", "peer ASN 64512 here");
    let report = remembered_constants(&page, &common::plain());
    assert_eq!(report.examined, 0);
    assert!(report.defects.is_empty());
    assert_eq!(
        report.not_applicable,
        Some("grammar has no remembered constants")
    );
}
