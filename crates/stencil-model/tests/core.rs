// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! The core vocabulary of section 13.1: tints, keys, canonical labels, grammar kinds in
//! vet, fact entries, chrome and Lanes.

mod common;

use common::{
    box_node, g7_page, gcp, item, legend_entry, link, page_with_body, page_with_link, pipe,
    pipe_value, plain, row, tee,
};
use stencil_model::checks::remembered_constants;
use stencil_model::{
    Chrome, FactEntry, FactSource, IconName, Item, Lanes, Line, Node, Page, PipeDir, Projection,
    box_key, box_tint, legend_label, line_key, line_tint, parse_and_vet, text_fields,
    validate_page,
};

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

fn rules(page: &Page, grammar: &stencil_model::Grammar) -> Vec<(String, &'static str, String)> {
    validate_page(page, grammar)
        .into_iter()
        .map(|violation| {
            (
                violation.pointer.as_str().to_string(),
                violation.rule.as_str(),
                violation.message,
            )
        })
        .collect()
}

fn one(pointer: &str, rule: &'static str, message: &str) -> Vec<(String, &'static str, String)> {
    vec![(pointer.to_string(), rule, message.to_string())]
}

fn plain_item(kind: &str, icon: Option<IconName>) -> Node {
    Node::Item(Item {
        id: None,
        kind: kind.to_string(),
        icon,
        title: "Thing".to_string(),
        subtitle: None,
        facts: Vec::new(),
        shape: None,
    })
}

#[test]
fn every_example_parses_round_trips_and_vets_clean_under_gcp() {
    let grammar = gcp();
    for (name, document) in EXAMPLES {
        let page =
            parse_and_vet(document, &grammar).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(page.grammar.as_deref(), Some("gcp"), "{name}");
        let written = serde_json::to_string(&page).unwrap();
        let reparsed = parse_and_vet(&written, &grammar).unwrap();
        assert_eq!(reparsed, page, "{name}: round trip");
    }
}

#[test]
fn tints_1_and_8_vet_clean_and_0_and_9_are_out_of_range_on_every_carrier() {
    let grammar = gcp();
    for (tint, expected) in [(1, 0), (8, 0), (0, 6), (9, 6)] {
        let mut page = page_with_link();
        page.body.push(box_node(
            "onprem",
            Some(tint),
            "On-prem",
            vec![item("on site")],
        ));
        page.body.push(pipe(Line::Solid, Some(tint), "a"));
        page.body.push(tee(
            Line::Solid,
            Some(tint),
            pipe_value(PipeDir::Horizontal, Line::Solid, Some(1), "b"),
            pipe_value(PipeDir::Horizontal, Line::Solid, Some(tint), "c"),
        ));
        page.legend[0].tint = Some(tint);
        page.links[0].tint = Some(tint);
        let found = rules(&page, &grammar);
        let out_of_range: Vec<&(String, &str, String)> = found
            .iter()
            .filter(|(_, rule, _)| *rule == "tint-out-of-range")
            .collect();
        assert_eq!(out_of_range.len(), expected, "tint {tint}: {found:?}");
        assert_eq!(found.len(), expected, "tint {tint}: {found:?}");
    }
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(9), "a")]);
    page.legend[0].tint = Some(1);
    assert_eq!(
        rules(&page, &grammar),
        one(
            "/body/0/tint",
            "tint-out-of-range",
            "tint 9 is outside 1 to 8"
        )
    );
}

#[test]
fn box_tint_follows_the_kind() {
    let grammar = gcp();
    let region = grammar.container("region").unwrap();
    let onprem = grammar.container("onprem").unwrap();
    let subnet = grammar.container("subnet").unwrap();
    assert_eq!(box_tint(region, None), Some(1));
    assert_eq!(box_tint(region, Some(2)), Some(2));
    assert_eq!(box_tint(onprem, None), None);
    assert_eq!(box_tint(onprem, Some(2)), Some(2));
    assert_eq!(box_tint(subnet, Some(3)), None);
}

#[test]
fn line_tint_key_and_label_follow_section_13_1() {
    assert_eq!(line_tint(Line::Gray, Some(3)), None);
    assert_eq!(line_tint(Line::Deny, None), None);
    assert_eq!(line_tint(Line::Solid, None), Some(1));
    assert_eq!(line_tint(Line::Dash, Some(4)), Some(4));

    assert_eq!(line_key(Line::Gray, Some(2)), "gray");
    assert_eq!(line_key(Line::Deny, Some(2)), "deny");
    let solid: Vec<&str> = (1..=8)
        .map(|tint| line_key(Line::Solid, Some(tint)))
        .collect();
    assert_eq!(
        solid,
        [
            "blue", "pink", "solid-3", "solid-4", "solid-5", "solid-6", "solid-7", "solid-8"
        ]
    );
    let dash: Vec<&str> = (1..=8)
        .map(|tint| line_key(Line::Dash, Some(tint)))
        .collect();
    assert_eq!(
        dash,
        [
            "dash", "dash-2", "dash-3", "dash-4", "dash-5", "dash-6", "dash-7", "dash-8"
        ]
    );
    assert_eq!(line_key(Line::Solid, None), "blue");
    assert_eq!(line_key(Line::Dash, None), "dash");

    assert_eq!(legend_label(Line::Gray, None), "Solid gray");
    assert_eq!(legend_label(Line::Deny, Some(4)), "Dashed red");
    assert_eq!(legend_label(Line::Solid, None), "Solid blue");
    assert_eq!(legend_label(Line::Solid, Some(2)), "Solid pink");
    assert_eq!(legend_label(Line::Dash, None), "Dashed blue");
    let solid_labels: Vec<&str> = (1..=8)
        .map(|tint| legend_label(Line::Solid, Some(tint)))
        .collect();
    let expected: Vec<String> = stencil_model::TINT_NAMES
        .iter()
        .map(|name| format!("Solid {name}"))
        .collect();
    assert_eq!(solid_labels, expected);
    assert_eq!(legend_label(Line::Dash, Some(8)), "Dashed cyan");
}

#[test]
fn box_keys_add_a_letter_per_slot() {
    assert_eq!(box_key("region", Some(1)), "region-a");
    assert_eq!(box_key("region", Some(2)), "region-b");
    assert_eq!(box_key("region", Some(8)), "region-h");
    assert_eq!(box_key("onprem", None), "onprem");
    assert_eq!(box_key("vpc", None), "vpc");
}

#[test]
fn kind_unknown_for_a_box_and_an_item_outside_the_grammar() {
    let grammar = gcp();
    let page = page_with_body(vec![
        box_node("zone", None, "Zone", vec![item("a")]),
        plain_item("service", None),
        pipe(Line::Solid, Some(1), "p"),
    ]);
    assert_eq!(
        rules(&page, &grammar),
        vec![
            (
                "/body/0/kind".to_string(),
                "kind-unknown",
                "kind \"zone\" is not a container kind of grammar gcp".to_string()
            ),
            (
                "/body/1/kind".to_string(),
                "kind-unknown",
                "kind \"service\" is not an item kind of grammar gcp".to_string()
            ),
        ]
    );
}

#[test]
fn kind_parent_not_allowed_reads_the_nearest_box_through_rows() {
    let grammar = gcp();
    let page = page_with_body(vec![
        box_node("subnet", None, "Subnet", vec![item("a")]),
        pipe(Line::Solid, Some(1), "p"),
    ]);
    assert_eq!(
        rules(&page, &grammar),
        one(
            "/body/0/kind",
            "kind-parent-not-allowed",
            "subnet cannot sit in page"
        )
    );

    let page = page_with_body(vec![
        box_node(
            "gcp",
            None,
            "Google Cloud",
            vec![box_node(
                "region",
                Some(1),
                "Region",
                vec![box_node("vpc", None, "VPC", vec![item("a")])],
            )],
        ),
        pipe(Line::Solid, Some(1), "p"),
    ]);
    assert_eq!(
        rules(&page, &grammar),
        one(
            "/body/0/children/0/children/0/kind",
            "kind-parent-not-allowed",
            "vpc cannot sit in region"
        )
    );

    let page = page_with_body(vec![
        box_node(
            "project",
            None,
            "Project",
            vec![row(vec![box_node(
                "subnet",
                None,
                "Subnet",
                vec![item("a")],
            )])],
        ),
        pipe(Line::Solid, Some(1), "p"),
    ]);
    assert_eq!(rules(&page, &grammar), vec![]);
}

#[test]
fn icon_outside_pack_for_a_plain_item_with_an_icon() {
    let mut page = page_with_body(vec![
        plain_item("service", Some(IconName::Gke)),
        pipe(Line::Solid, Some(1), "p"),
    ]);
    page.grammar = Some("plain".to_string());
    assert_eq!(
        rules(&page, &plain()),
        one(
            "/body/0/icon",
            "icon-outside-pack",
            "item kind service takes no icon"
        )
    );
}

#[test]
fn grammar_unknown_for_a_name_that_is_neither_built_in_nor_a_path() {
    let grammar = gcp();
    let mut page = page_with_link();
    page.grammar = Some("gcpx".to_string());
    assert_eq!(
        rules(&page, &grammar),
        one(
            "/grammar",
            "grammar-unknown",
            "grammar \"gcpx\" is neither a built-in grammar nor a .json path"
        )
    );
    page.grammar = Some("grammars/mine.json".to_string());
    assert_eq!(rules(&page, &grammar), vec![]);
}

#[test]
fn facts_appear_in_text_fields_after_title_and_subtitle() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(1), "p")]);
    page.body.push(Node::Item(Item {
        id: None,
        kind: "product".to_string(),
        icon: None,
        title: "Bucket".to_string(),
        subtitle: Some("Cloud Storage".to_string()),
        facts: vec![
            FactEntry {
                text: "dual-region".to_string(),
                source: FactSource::Doc,
            },
            FactEntry {
                text: "acme-raw-64512".to_string(),
                source: FactSource::Built,
            },
            FactEntry {
                text: "Which retention?".to_string(),
                source: FactSource::Ask,
            },
        ],
        shape: None,
    }));
    let pointers: Vec<String> = text_fields(&page)
        .iter()
        .map(|field| field.pointer.as_str().to_string())
        .filter(|pointer| pointer.starts_with("/body/1"))
        .collect();
    assert_eq!(
        pointers,
        [
            "/body/1/title",
            "/body/1/subtitle",
            "/body/1/facts/0/text",
            "/body/1/facts/1/text",
            "/body/1/facts/2/text"
        ]
    );
    page.body[1] = match page.body[1].clone() {
        Node::Item(mut built) => {
            built.facts[1].text = "acme-raw 64512".to_string();
            Node::Item(built)
        }
        _ => panic!("an Item"),
    };
    let report = remembered_constants(&page, &gcp());
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/body/1/facts/1/text");
}

#[test]
fn a_fact_node_without_source_serializes_without_it() {
    let fact = common::fact("a");
    let written = serde_json::to_value(&fact).unwrap();
    assert_eq!(written, serde_json::json!({"tag": "Fact", "text": "a"}));
    let built: Node =
        serde_json::from_value(serde_json::json!({"tag": "Fact", "text": "b", "source": "built"}))
            .unwrap();
    let Node::Fact(built) = built else {
        panic!("a Fact node");
    };
    assert_eq!(built.source, FactSource::Built);
}

#[test]
fn facts_too_many_above_eight() {
    let facts = vec![
        FactEntry {
            text: "fact".to_string(),
            source: FactSource::Doc,
        };
        9
    ];
    let page = page_with_body(vec![
        Node::Item(Item {
            id: None,
            kind: "product".to_string(),
            icon: None,
            title: "Many".to_string(),
            subtitle: None,
            facts,
            shape: None,
        }),
        pipe(Line::Solid, Some(1), "p"),
    ]);
    assert_eq!(
        rules(&page, &gcp()),
        one("/body/0/facts", "facts-too-many", "9 facts, above 8")
    );
}

#[test]
fn chrome_defaults_to_full_and_is_not_serialized_as_default() {
    let page = g7_page();
    assert_eq!(page.chrome, Chrome::Full);
    let written = serde_json::to_value(&page).unwrap();
    assert!(written.get("chrome").is_none());
    let mut page = page;
    page.chrome = Chrome::None;
    let written = serde_json::to_value(&page).unwrap();
    assert_eq!(written["chrome"], "none");
}

fn lanes(count: usize) -> Node {
    Node::Lanes(Lanes {
        id: None,
        gap: None,
        children: (0..count)
            .map(|index| item(&format!("head {index}")))
            .collect(),
    })
}

#[test]
fn lanes_parse_vet_and_are_walked_as_containers() {
    let mut page = page_with_body(vec![lanes(3)]);
    page.body.push(pipe(Line::Solid, Some(1), "p"));
    assert_eq!(rules(&page, &gcp()), vec![]);
    let walked = stencil_model::body_nodes(&page);
    assert_eq!(walked.len(), 5);
    assert_eq!(walked[1].node.tag_name(), "Item");
    assert_eq!(walked[1].depth, 2);
}

#[test]
fn lanes_too_many_and_lanes_in_iso() {
    let mut page = page_with_body(vec![lanes(33), pipe(Line::Solid, Some(1), "p")]);
    assert_eq!(
        rules(&page, &gcp()),
        one("/body/0/children", "lanes-too-many", "33 lanes, above 32")
    );
    page.body[0] = lanes(2);
    page.projection = Projection::Iso;
    assert_eq!(
        rules(&page, &gcp()),
        one(
            "/projection",
            "lanes-in-iso",
            "a page with Lanes cannot be drawn in iso"
        )
    );
}

/// Plain-grammar examples, each with Lanes or links, vetted under the grammar they name.
const PLAIN_EXAMPLES: [(&str, &str); 3] = [
    ("sequence", include_str!("../../../examples/sequence.json")),
    ("org", include_str!("../../../examples/org.json")),
    (
        "onprem-network",
        include_str!("../../../examples/onprem-network.json"),
    ),
];

#[test]
fn every_plain_example_parses_round_trips_and_vets_clean_under_plain() {
    let grammar = plain();
    for (name, document) in PLAIN_EXAMPLES {
        let page =
            parse_and_vet(document, &grammar).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(page.grammar.as_deref(), Some("plain"), "{name}");
        let written = serde_json::to_string(&page).unwrap();
        let reparsed = parse_and_vet(&written, &grammar).unwrap();
        assert_eq!(reparsed, page, "{name}: round trip");
    }
}

fn head(id: &str) -> Node {
    Node::Item(Item {
        id: Some(id.to_string()),
        kind: "product".to_string(),
        icon: None,
        title: id.to_string(),
        subtitle: Some("Cloud Run".to_string()),
        facts: Vec::new(),
        shape: None,
    })
}

fn lanes_of(heads: &[&str]) -> Node {
    Node::Lanes(Lanes {
        id: None,
        gap: None,
        children: heads.iter().map(|id| head(id)).collect(),
    })
}

fn ordered(from: &str, to: &str, order: u16) -> stencil_model::Link {
    let mut ordered_link = link(from, to);
    ordered_link.order = Some(order);
    ordered_link
}

/// Two Lanes nodes, `a b c` and `d e`, beside an Item `outside`, with one legend entry for
/// the solid tint 1 links the tests add.
fn two_lanes_page(links: Vec<stencil_model::Link>) -> Page {
    let mut page = page_with_body(vec![
        lanes_of(&["a", "b", "c"]),
        lanes_of(&["d", "e"]),
        head("outside"),
    ]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), "call")];
    page.links = links;
    page
}

#[test]
fn ordered_links_between_two_heads_of_one_lanes_node_vet_clean_and_round_trip() {
    let page = two_lanes_page(vec![
        ordered("a", "b", 1),
        ordered("c", "a", 2),
        ordered("d", "e", 1),
    ]);
    assert_eq!(rules(&page, &gcp()), vec![]);
    let written = serde_json::to_value(&page).unwrap();
    assert_eq!(written["links"][1]["order"], 2);
    let reparsed: Page = serde_json::from_value(written).unwrap();
    assert_eq!(reparsed, page);
    let unordered = serde_json::to_value(two_lanes_page(vec![link("a", "outside")])).unwrap();
    assert!(unordered["links"][0].get("order").is_none());
}

#[test]
fn an_ordered_link_that_leaves_its_lanes_node_is_rejected_at_its_order() {
    let message = "an ordered link joins two lanes of one Lanes node";
    for (from, to) in [("a", "outside"), ("outside", "b"), ("a", "d")] {
        let page = two_lanes_page(vec![ordered(from, to, 1)]);
        assert_eq!(
            rules(&page, &gcp()),
            one("/links/0/order", "link-order-outside-lanes", message),
            "{from} to {to}"
        );
    }
    // A node inside a head is not a head.
    let mut page = page_with_body(vec![Node::Lanes(Lanes {
        id: None,
        gap: None,
        children: vec![
            head("a"),
            box_node("project", None, "Team", vec![head("inner")]),
        ],
    })]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), "call")];
    page.links = vec![ordered("a", "inner", 1)];
    assert_eq!(
        rules(&page, &gcp()),
        one("/links/0/order", "link-order-outside-lanes", message)
    );
}

#[test]
fn two_messages_of_one_lanes_node_with_one_order_are_rejected_at_the_later_link() {
    let page = two_lanes_page(vec![
        ordered("a", "b", 3),
        ordered("d", "e", 3),
        ordered("b", "c", 3),
    ]);
    assert_eq!(
        rules(&page, &gcp()),
        one(
            "/links/2/order",
            "link-order-duplicate",
            "order 3 is already used by /links/0"
        )
    );
}

#[test]
fn a_link_carries_line_and_tint() {
    let mut page = page_with_link();
    page.links[0] = link("api", "worker");
    page.legend = vec![legend_entry(Line::Solid, None, "call")];
    assert_eq!(rules(&page, &gcp()), vec![]);
}
