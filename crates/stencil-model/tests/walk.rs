// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{g7_page, g7_value, item, page_with_body, pipe, pipe_value, row, tee};
use stencil_model::{FactSource, IconName, Line, Node, NodeRef, PipeDir, body_nodes, text_fields};

#[test]
fn g7_has_24_body_nodes_in_pre_order() {
    let page = g7_page();
    let entries = body_nodes(&page);
    assert_eq!(entries.len(), 24);
    let pointers: Vec<&str> = entries.iter().map(|entry| entry.pointer.as_str()).collect();
    assert_eq!(
        pointers[..6],
        [
            "/body/0",
            "/body/0/children/0",
            "/body/0/children/0/children/0",
            "/body/0/children/0/children/0/children/0",
            "/body/0/children/0/children/0/children/1",
            "/body/0/children/0/children/1",
        ]
    );
    assert_eq!(
        pointers.last(),
        Some(&"/body/0/children/2/children/0/children/2/children/1")
    );
}

#[test]
fn every_entry_resolves_in_the_document_with_its_tag() {
    let page = g7_page();
    let document = g7_value();
    for entry in body_nodes(&page) {
        let value = document
            .pointer(entry.pointer.as_str())
            .unwrap_or_else(|| panic!("{} resolves", entry.pointer));
        assert_eq!(value["tag"], entry.node.tag_name());
    }
}

#[test]
fn depth_and_parent() {
    let page = g7_page();
    let entries = body_nodes(&page);
    assert_eq!(entries[0].depth, 1);
    assert_eq!(
        entries[0].parent.as_ref().map(|parent| parent.as_str()),
        Some("/body")
    );
    assert_eq!(entries[3].depth, 4);
    assert_eq!(
        entries[3].parent.as_ref().map(|parent| parent.as_str()),
        Some("/body/0/children/0/children/0")
    );
}

#[test]
fn tee_arms_follow_the_tee_before_its_next_sibling() {
    let page = page_with_body(vec![row(vec![
        tee(
            Line::Solid,
            Some(1),
            pipe_value(PipeDir::Horizontal, Line::Solid, Some(1), "a"),
            pipe_value(PipeDir::Horizontal, Line::Solid, Some(2), "b"),
        ),
        item("sibling"),
    ])]);
    let entries = body_nodes(&page);
    let summary: Vec<(&str, usize, Option<&str>)> = entries
        .iter()
        .map(|entry| {
            (
                entry.pointer.as_str(),
                entry.depth,
                entry.parent.as_ref().map(|parent| parent.as_str()),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("/body/0", 1, Some("/body")),
            ("/body/0/children/0", 2, Some("/body/0")),
            ("/body/0/children/0/arms/0", 3, Some("/body/0/children/0")),
            ("/body/0/children/0/arms/1", 3, Some("/body/0/children/0")),
            ("/body/0/children/1", 2, Some("/body/0")),
        ]
    );
    assert!(matches!(entries[2].node, NodeRef::TeeArm(arm) if arm.label == "a"));
    assert_eq!(entries[2].node.tag_name(), "Pipe");
}

#[test]
fn g7_text_fields_in_document_order() {
    let page = g7_page();
    let fields = text_fields(&page);
    assert_eq!(fields.len(), 36);
    let pointers: Vec<&str> = fields.iter().map(|field| field.pointer.as_str()).collect();
    assert_eq!(
        pointers[..5],
        [
            "/title",
            "/kicker",
            "/lede",
            "/foot",
            "/body/0/children/0/children/0/label"
        ]
    );
    assert_eq!(
        pointers[33..],
        ["/legend/0/text", "/legend/1/text", "/legend/2/text"]
    );
    let document = g7_value();
    for field in &fields {
        assert_eq!(
            document
                .pointer(field.pointer.as_str())
                .and_then(|value| value.as_str()),
            Some(field.text)
        );
    }
}

#[test]
fn text_fields_skip_absent_optionals() {
    let page = page_with_body(vec![item("a"), pipe(Line::Solid, Some(1), "b")]);
    let pointers: Vec<String> = text_fields(&page)
        .iter()
        .map(|field| field.pointer.to_string())
        .collect();
    assert_eq!(
        pointers,
        [
            "/title",
            "/kicker",
            "/lede",
            "/body/0/title",
            "/body/1/label",
            "/legend/0/text"
        ]
    );
}

#[test]
fn tag_names() {
    let page = g7_page();
    let tags: Vec<&str> = body_nodes(&page)
        .iter()
        .take(4)
        .map(|entry| entry.node.tag_name())
        .collect();
    assert_eq!(tags, ["Row", "Col", "Box", "Item"]);
    assert_eq!(pipe(Line::Solid, Some(1), "a").tag_name(), "Pipe");
    assert!(matches!(page.body.first(), Some(Node::Row(_))));
}

#[test]
fn icon_file_names_match_serde_and_assets() {
    let icons_directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icons");
    for icon in IconName::ALL {
        let serialized = serde_json::to_value(icon).unwrap();
        assert_eq!(
            format!("{}.svg", serialized.as_str().unwrap()),
            icon.file_name()
        );
        let path = std::path::Path::new(icons_directory).join(icon.file_name());
        assert!(path.is_file(), "{} exists", path.display());
    }
    assert_eq!(IconName::AiMl.file_name(), "ai-ml.svg");
    let svg_count = std::fs::read_dir(icons_directory)
        .unwrap()
        .filter(|entry| {
            entry.as_ref().is_ok_and(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "svg")
            })
        })
        .count();
    assert_eq!(svg_count, IconName::ALL.len());
}

#[test]
fn kind_names_match_serde() {
    for line in Line::ALL {
        assert_eq!(serde_json::to_value(line).unwrap(), line.as_str());
    }
    for source in FactSource::ALL {
        assert_eq!(serde_json::to_value(source).unwrap(), source.as_str());
    }
    for icon in IconName::ALL {
        assert_eq!(serde_json::to_value(icon).unwrap(), icon.as_str());
    }
}
