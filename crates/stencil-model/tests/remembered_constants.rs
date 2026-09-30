// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{g7_page, legend_entry, page_with_body, pipe_value};
use stencil_model::checks::{CheckName, REMEMBERED_CONSTANTS, remembered_constants};
use stencil_model::{
    Arrow, Callout, CalloutKind, Frame, Link, ListKind, Node, Note, NoteKind, Page, Pcard, Pipe,
    PipeDir, PipeKind, Tee, TeeArm, Text, Zone, ZoneKind, text_fields,
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
        Node::Zone(Zone {
            id: None,
            kind: ZoneKind::RegionA,
            label: pick("zone label"),
            children: vec![Node::Pcard(Pcard {
                id: None,
                icon: None,
                function_name: pick("fn"),
                product_name: Some(pick("pn")),
                fact: Some(pick("fact")),
                ask: Some(pick("ask")),
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
            dir: PipeDir::Horizontal,
            kind: PipeKind::Blue,
            label: pick("pipe label"),
            sub: Some(pick("sub")),
        }),
        Node::Tee(Tee {
            id: None,
            kind: PipeKind::Blue,
            hub: pick("hub"),
            arms: [
                TeeArm::Pipe(pipe_value(PipeDir::Horizontal, PipeKind::Blue, FILLER)),
                TeeArm::Pipe(pipe_value(PipeDir::Horizontal, PipeKind::Blue, FILLER)),
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
    page.title = pick("title");
    page.lede = pick("lede");
    page.foot = Some(pick("foot"));
    page.legend = vec![legend_entry(PipeKind::Blue, &pick("legend text"))];
    page.links = vec![Link {
        from: "text".to_string(),
        to: "frame".to_string(),
        kind: PipeKind::Blue,
        label: Some(pick("link label")),
        sub: Some(pick("link sub")),
        arrow: Arrow::End,
        from_side: None,
        to_side: None,
        via: Vec::new(),
    }];
    let pointer = match field {
        "title" => "/title",
        "lede" => "/lede",
        "foot" => "/foot",
        "legend text" => "/legend/0/text",
        "zone label" => "/body/0/label",
        "fn" => "/body/0/children/0/fn",
        "pn" => "/body/0/children/0/pn",
        "fact" => "/body/0/children/0/fact",
        "ask" => "/body/0/children/0/ask",
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

const FIELD_KINDS: [&str; 20] = [
    "title",
    "lede",
    "foot",
    "legend text",
    "zone label",
    "fn",
    "pn",
    "fact",
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
    for constant in REMEMBERED_CONSTANTS {
        for field in FIELD_KINDS {
            let (page, pointer) =
                page_with_field(field, &format!("peer ASN {} here", constant.literal));
            let report = remembered_constants(&page);
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
        let (page, _) = page_with_field("title", text);
        assert_eq!(remembered_constants(&page).defects.len(), 1, "{text}");
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
        let (page, _) = page_with_field("fn", text);
        let report = remembered_constants(&page);
        assert_eq!(report.defects, vec![], "{text}");
    }
}

#[test]
fn a_bounded_occurrence_after_an_unbounded_one_fires() {
    let (page, _) = page_with_field("fn", "AS64512 and 64512");
    assert_eq!(remembered_constants(&page).defects.len(), 1);
}

#[test]
fn check_scans_untransformed_text() {
    let mut page = page_with_body(vec![common::pcard("a")]);
    page.kicker = "asn 64512".to_string();
    let report = remembered_constants(&page);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/kicker");
}

#[test]
fn examined_equals_text_field_count() {
    for field in FIELD_KINDS {
        let (page, _) = page_with_field(field, FILLER);
        let report = remembered_constants(&page);
        assert_eq!(report.examined, text_fields(&page).len() as u64);
        assert!(report.passed());
    }
}

#[test]
fn g7_examines_36_fields_and_passes() {
    let report = remembered_constants(&g7_page());
    assert_eq!(report.check, CheckName::RememberedConstants);
    assert_eq!(report.examined, 36);
    assert!(report.passed());
}

#[test]
fn two_different_literals_give_two_defects() {
    let (page, pointer) = page_with_field("sub", "ASN 64512 and range 35.191.0.0/16");
    let report = remembered_constants(&page);
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
    assert_eq!(remembered_constants(&page).defects.len(), 1);
}
