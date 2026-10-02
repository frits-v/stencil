// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{
    box_node, col, fact, g7_page, item, legend_entry, nested_cols, page_with_body, pipe,
    pipe_value, row, tee,
};
use stencil_model::{
    CHILDREN_MAX, FactEntry, FactSource, Item, LEGEND_ENTRIES_MAX, Line, NODES_MAX, Node, Page,
    PipeDir, VetRule, body_nodes, validate_page,
};

/// (pointer, rule name, message) for every violation, with the rule name taken from
/// `VetRule::as_str` so each assertion also pins the section 1.3 first column.
fn violations(page: &Page) -> Vec<(String, &'static str, String)> {
    validate_page(page, &common::gcp())
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

fn first_row_mut(page: &mut Page) -> &mut stencil_model::Row {
    match page.body.first_mut() {
        Some(Node::Row(row)) => row,
        _ => panic!("first body node is a Row"),
    }
}

#[test]
fn g7_has_no_violations() {
    assert_eq!(violations(&g7_page()), vec![]);
}

#[test]
fn vet_rule_names_match_section_1_3() {
    let expected = [
        (VetRule::TextEmpty, "text-empty"),
        (VetRule::TextTooLong, "text-too-long"),
        (VetRule::TextControlCharacter, "text-control-character"),
        (VetRule::TextUntrimmed, "text-untrimmed"),
        (VetRule::PageWidthOutOfRange, "page-width-out-of-range"),
        (VetRule::BodyEmpty, "body-empty"),
        (VetRule::ChildrenEmpty, "children-empty"),
        (VetRule::ChildrenTooMany, "children-too-many"),
        (VetRule::LegendTooLong, "legend-too-long"),
        (VetRule::GapOutOfRange, "gap-out-of-range"),
        (VetRule::GrowLengthMismatch, "grow-length-mismatch"),
        (VetRule::GrowOutOfRange, "grow-out-of-range"),
        (VetRule::TeeArmNotHorizontal, "tee-arm-not-horizontal"),
        (VetRule::DepthExceeded, "depth-exceeded"),
        (VetRule::NodesExceeded, "nodes-exceeded"),
    ];
    for (rule, name) in expected {
        assert_eq!(rule.as_str(), name);
    }
}

#[test]
fn text_empty() {
    let mut page = page_with_body(vec![item("a")]);
    page.title = String::new();
    assert_eq!(
        violations(&page),
        one("/title", "text-empty", "text is empty")
    );
}

#[test]
fn text_of_400_scalars_passes_and_401_fails() {
    // Multi-byte scalars, so a byte count would give the wrong answer.
    let mut page = page_with_body(vec![item(&"é".repeat(400))]);
    assert_eq!(violations(&page), vec![]);
    page.body = vec![item(&"é".repeat(401))];
    assert_eq!(
        violations(&page),
        one(
            "/body/0/title",
            "text-too-long",
            "text has 401 scalar values, above 400"
        )
    );
}

#[test]
fn text_with_newline_is_a_control_character() {
    let mut page = page_with_body(vec![item("a")]);
    page.legend = vec![
        legend_entry(Line::Solid, Some(1), "a"),
        legend_entry(Line::Solid, Some(2), "b\nc"),
    ];
    assert_eq!(
        violations(&page),
        one(
            "/legend/1/text",
            "text-control-character",
            "text contains control character U+000A"
        )
    );
}

#[test]
fn delete_character_is_a_control_character_and_the_first_is_named() {
    let page = page_with_body(vec![item("a\u{7f}b\u{1}")]);
    assert_eq!(
        violations(&page),
        one(
            "/body/0/title",
            "text-control-character",
            "text contains control character U+007F"
        )
    );
}

#[test]
fn trimmed_text_passes_and_leading_space_fails() {
    let mut page = page_with_body(vec![fact("a")]);
    assert_eq!(violations(&page), vec![]);
    page.body = vec![fact(" a")];
    assert_eq!(
        violations(&page),
        one(
            "/body/0/text",
            "text-untrimmed",
            "text starts or ends with whitespace"
        )
    );
}

#[test]
fn interior_spaces_are_allowed() {
    let page = page_with_body(vec![fact("a  b")]);
    assert_eq!(violations(&page), vec![]);
}

#[test]
fn trailing_space_fails() {
    let page = page_with_body(vec![fact("a ")]);
    assert_eq!(
        violations(&page),
        one(
            "/body/0/text",
            "text-untrimmed",
            "text starts or ends with whitespace"
        )
    );
}

#[test]
fn width_limits() {
    let mut page = page_with_body(vec![item("a")]);
    for width in [640, 2560] {
        page.width = width;
        assert_eq!(violations(&page), vec![], "width {width}");
    }
    page.width = 639;
    assert_eq!(
        violations(&page),
        one(
            "/width",
            "page-width-out-of-range",
            "width 639 is outside 640 to 2560"
        )
    );
    page.width = 2561;
    assert_eq!(
        violations(&page),
        one(
            "/width",
            "page-width-out-of-range",
            "width 2561 is outside 640 to 2560"
        )
    );
}

#[test]
fn empty_body_is_body_empty_not_children_empty() {
    let page = page_with_body(vec![]);
    assert_eq!(
        violations(&page),
        one("/body", "body-empty", "body has no nodes")
    );
}

#[test]
fn container_without_children() {
    let page = page_with_body(vec![row(vec![
        col(vec![]),
        box_node("project", None, "VPC", vec![]),
    ])]);
    assert_eq!(
        violations(&page),
        vec![
            (
                "/body/0/children/0/children".to_string(),
                "children-empty",
                "container has no children".to_string()
            ),
            (
                "/body/0/children/1/children".to_string(),
                "children-empty",
                "container has no children".to_string()
            ),
        ]
    );
}

#[test]
fn children_limit() {
    let mut page = page_with_body(vec![row(vec![item("a"); CHILDREN_MAX])]);
    assert_eq!(violations(&page), vec![]);
    page.body = vec![row(vec![item("a"); CHILDREN_MAX + 1])];
    assert_eq!(
        violations(&page),
        one(
            "/body/0/children",
            "children-too-many",
            "257 children, above 256"
        )
    );
}

#[test]
fn zone_children_limit() {
    let page = page_with_body(vec![box_node(
        "project",
        None,
        "Project",
        vec![fact("a"); 257],
    )]);
    assert_eq!(
        violations(&page),
        one(
            "/body/0/children",
            "children-too-many",
            "257 children, above 256"
        )
    );
}

#[test]
fn body_node_limit() {
    let mut page = page_with_body(vec![item("a"); 256]);
    assert_eq!(violations(&page), vec![]);
    page.body = vec![item("a"); 257];
    assert_eq!(
        violations(&page),
        one("/body", "children-too-many", "257 children, above 256")
    );
}

#[test]
fn legend_limit() {
    let mut page = page_with_body(vec![item("a")]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), "a"); 16];
    assert_eq!(violations(&page), vec![]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), "a"); 17];
    assert_eq!(
        violations(&page),
        one(
            "/legend",
            "legend-too-long",
            "legend has 17 entries, above 16"
        )
    );
}

#[test]
fn gap_limit() {
    let mut page = page_with_body(vec![row(vec![item("a")])]);
    first_row_mut(&mut page).gap = Some(64);
    assert_eq!(violations(&page), vec![]);
    first_row_mut(&mut page).gap = Some(65);
    assert_eq!(
        violations(&page),
        one("/body/0/gap", "gap-out-of-range", "gap 65 is above 64")
    );
}

#[test]
fn col_gap_limit() {
    let mut page = page_with_body(vec![col(vec![item("a")])]);
    if let Some(Node::Col(column)) = page.body.first_mut() {
        column.gap = Some(65);
    }
    assert_eq!(
        violations(&page),
        one("/body/0/gap", "gap-out-of-range", "gap 65 is above 64")
    );
}

#[test]
fn grow_length_must_match_children() {
    let mut page = page_with_body(vec![row(vec![item("a"), item("b")])]);
    first_row_mut(&mut page).grow = Some(vec![1, 0]);
    assert_eq!(violations(&page), vec![]);
    first_row_mut(&mut page).grow = Some(vec![1, 0, 1]);
    assert_eq!(
        violations(&page),
        one(
            "/body/0/grow",
            "grow-length-mismatch",
            "grow has 3 weights for 2 children"
        )
    );
    first_row_mut(&mut page).grow = Some(vec![1]);
    assert_eq!(
        violations(&page),
        one(
            "/body/0/grow",
            "grow-length-mismatch",
            "grow has 1 weights for 2 children"
        )
    );
}

#[test]
fn grow_weight_limit_reports_each_weight() {
    let mut page = page_with_body(vec![row(vec![item("a"), item("b"), item("c")])]);
    first_row_mut(&mut page).grow = Some(vec![100, 0, 100]);
    assert_eq!(violations(&page), vec![]);
    first_row_mut(&mut page).grow = Some(vec![101, 0, 200]);
    assert_eq!(
        violations(&page),
        vec![
            (
                "/body/0/grow/0".to_string(),
                "grow-out-of-range",
                "grow weight 101 is above 100".to_string()
            ),
            (
                "/body/0/grow/2".to_string(),
                "grow-out-of-range",
                "grow weight 200 is above 100".to_string()
            ),
        ]
    );
}

#[test]
fn tee_arm_must_be_horizontal() {
    let horizontal = pipe_value(PipeDir::Horizontal, Line::Solid, Some(1), "a");
    let vertical = pipe_value(PipeDir::Vertical, Line::Solid, Some(1), "b");
    let mut page = page_with_body(vec![tee(
        Line::Solid,
        Some(1),
        horizontal.clone(),
        horizontal.clone(),
    )]);
    assert_eq!(violations(&page), vec![]);
    page.body = vec![tee(Line::Solid, Some(1), horizontal, vertical)];
    assert_eq!(
        violations(&page),
        one(
            "/body/0/arms/1/dir",
            "tee-arm-not-horizontal",
            "Tee arm dir is \"v\", expected \"h\""
        )
    );
}

#[test]
fn depth_limit() {
    // 23 Cols around an Item: the Item is at depth 24.
    let mut page = page_with_body(vec![nested_cols(23)]);
    assert_eq!(body_nodes(&page).last().map(|entry| entry.depth), Some(24));
    assert_eq!(violations(&page), vec![]);

    page.body = vec![nested_cols(24)];
    let deepest = format!("/body/0{}", "/children/0".repeat(24));
    assert_eq!(
        violations(&page),
        one(&deepest, "depth-exceeded", "depth 25 is above 24")
    );
}

#[test]
fn depth_exceeded_is_reported_once_per_subtree() {
    let page = page_with_body(vec![nested_cols(27)]);
    let depth_25 = format!("/body/0{}", "/children/0".repeat(24));
    assert_eq!(
        violations(&page),
        one(&depth_25, "depth-exceeded", "depth 25 is above 24")
    );
}

#[test]
fn tee_arm_depth_counts() {
    let arm = pipe_value(PipeDir::Horizontal, Line::Solid, Some(1), "a");
    let mut inner = tee(Line::Solid, Some(1), arm.clone(), arm);
    for _ in 0..23 {
        inner = col(vec![inner]);
    }
    // The Tee is at depth 24 and its arms at 25.
    let page = page_with_body(vec![inner]);
    let tee_pointer = format!("/body/0{}", "/children/0".repeat(23));
    assert_eq!(
        violations(&page),
        vec![
            (
                format!("{tee_pointer}/arms/0"),
                "depth-exceeded",
                "depth 25 is above 24".to_string()
            ),
            (
                format!("{tee_pointer}/arms/1"),
                "depth-exceeded",
                "depth 25 is above 24".to_string()
            ),
        ]
    );
}

/// 16 Rows. The first holds `first_row_cards` Pcards and one Tee, the others 255 Pcards each.
fn page_with_row_grid(first_row_cards: usize) -> Page {
    let arm = pipe_value(PipeDir::Horizontal, Line::Solid, Some(1), "a");
    let mut first_row_children = vec![item("a"); first_row_cards];
    first_row_children.push(tee(Line::Solid, Some(1), arm.clone(), arm));
    let mut body = vec![row(first_row_children)];
    body.extend((0..15).map(|_| row(vec![item("a"); 255])));
    page_with_body(body)
}

#[test]
fn node_limit_counts_tee_arms() {
    // 16 Rows + 252 cards + Tee + 2 arms + 15 * 255 cards = 4096.
    let at_limit = page_with_row_grid(252);
    assert_eq!(body_nodes(&at_limit).len(), NODES_MAX);
    assert_eq!(violations(&at_limit), vec![]);

    // One more card: 4097, and the two Tee arms are what push it past the limit.
    let past_limit = page_with_row_grid(253);
    assert_eq!(body_nodes(&past_limit).len(), NODES_MAX + 1);
    assert_eq!(
        violations(&past_limit),
        one("/body", "nodes-exceeded", "more than 4096 nodes")
    );
}

#[test]
fn page_far_past_node_limit_stops_the_walk() {
    let page = page_with_body(vec![row(vec![item("a"); 256]); 256]);
    assert_eq!(body_nodes(&page).len(), NODES_MAX + 1);
    assert_eq!(
        violations(&page),
        one("/body", "nodes-exceeded", "more than 4096 nodes")
    );
}

#[test]
fn page_past_node_limit_still_reports_per_node_violations_of_walked_entries() {
    let mut rows = vec![row(vec![item(" untrimmed")])];
    rows.extend((0..254).map(|_| row(vec![item("a"); 256])));
    rows.push(row(vec![item(" not walked")]));
    let page = page_with_body(rows);
    assert_eq!(
        violations(&page),
        vec![
            (
                "/body".to_string(),
                "nodes-exceeded",
                "more than 4096 nodes".to_string()
            ),
            (
                "/body/0/children/0/title".to_string(),
                "text-untrimmed",
                "text starts or ends with whitespace".to_string()
            ),
        ]
    );
}

#[test]
fn every_text_field_is_vetted() {
    let untrimmed = " x";
    let mut page = page_with_body(vec![
        box_node(
            "onprem",
            Some(1),
            untrimmed,
            vec![Node::Item(Item {
                id: None,
                kind: "product".to_string(),
                icon: None,
                title: untrimmed.to_string(),
                subtitle: Some(untrimmed.to_string()),
                facts: vec![
                    FactEntry {
                        text: untrimmed.to_string(),
                        source: FactSource::Doc,
                    },
                    FactEntry {
                        text: untrimmed.to_string(),
                        source: FactSource::Ask,
                    },
                ],
                shape: None,
            })],
        ),
        Node::Note(stencil_model::Note {
            id: None,
            kind: stencil_model::NoteKind::Legend,
            text: untrimmed.to_string(),
        }),
        Node::Pipe(stencil_model::Pipe {
            id: None,
            arrow: stencil_model::Arrow::None,
            axis: None,
            form: stencil_model::PipeForm::Tube,
            dir: PipeDir::Horizontal,
            line: Line::Solid,
            tint: Some(1),
            label: untrimmed.to_string(),
            sub: Some(untrimmed.to_string()),
            from: None,
            to: None,
        }),
        Node::Tee(stencil_model::Tee {
            id: None,
            line: Line::Solid,
            tint: Some(1),
            hub: untrimmed.to_string(),
            arms: [
                stencil_model::TeeArm::Pipe(pipe_value(
                    PipeDir::Horizontal,
                    Line::Solid,
                    Some(1),
                    untrimmed,
                )),
                stencil_model::TeeArm::Pipe(stencil_model::Pipe {
                    id: None,
                    arrow: stencil_model::Arrow::None,
                    axis: None,
                    form: stencil_model::PipeForm::Tube,
                    dir: PipeDir::Horizontal,
                    line: Line::Solid,
                    tint: Some(1),
                    label: "a".to_string(),
                    sub: Some(untrimmed.to_string()),
                    from: None,
                    to: None,
                }),
            ],
        }),
    ]);
    page.title = untrimmed.to_string();
    page.kicker = untrimmed.to_string();
    page.lede = untrimmed.to_string();
    page.foot = Some(untrimmed.to_string());
    page.legend = vec![legend_entry(Line::Solid, Some(1), untrimmed)];

    let pointers: Vec<String> = violations(&page)
        .into_iter()
        .map(|(pointer, rule, _)| {
            assert_eq!(rule, "text-untrimmed");
            pointer
        })
        .collect();
    assert_eq!(
        pointers,
        [
            "/title",
            "/kicker",
            "/lede",
            "/foot",
            "/body/0/label",
            "/body/0/children/0/title",
            "/body/0/children/0/subtitle",
            "/body/0/children/0/facts/0/text",
            "/body/0/children/0/facts/1/text",
            "/body/1/text",
            "/body/2/label",
            "/body/2/sub",
            "/body/3/hub",
            "/body/3/arms/0/label",
            "/body/3/arms/1/sub",
            "/legend/0/text",
        ]
    );
}

#[test]
fn several_faults_are_all_reported_in_document_order() {
    // Keys are written kicker before title and legend before body; document order is the
    // struct declaration order, so /title still comes first.
    let json_text = r#"{
        "canvas": "internal",
        "legend": [ { "line": "solid", "tint": 1, "text": "request path" } ],
        "kicker": "Kicker ",
        "body": [
            { "tag": "Row", "children": [ { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "a" } ], "gap": 65 }
        ],
        "lede": "Lede",
        "title": " Title"
    }"#;
    let page: Page = serde_json::from_str(json_text).unwrap();
    assert_eq!(
        violations(&page),
        vec![
            (
                "/title".to_string(),
                "text-untrimmed",
                "text starts or ends with whitespace".to_string()
            ),
            (
                "/kicker".to_string(),
                "text-untrimmed",
                "text starts or ends with whitespace".to_string()
            ),
            (
                "/body/0/gap".to_string(),
                "gap-out-of-range",
                "gap 65 is above 64".to_string()
            ),
        ]
    );
}

#[test]
fn node_fields_precede_children_and_container_rules_follow_zone_label() {
    let page = page_with_body(vec![
        box_node("project", None, "VPC ", vec![]),
        row(vec![pipe(Line::Solid, Some(1), " a")]),
    ]);
    let pointers: Vec<String> = violations(&page)
        .into_iter()
        .map(|(pointer, _, _)| pointer)
        .collect();
    assert_eq!(
        pointers,
        [
            "/body/0/label",
            "/body/0/children",
            "/body/1/children/0/label"
        ]
    );
}

#[test]
fn legend_text_rules_stop_one_entry_past_the_legend_limit() {
    let mut page = page_with_body(vec![item("a")]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), " untrimmed"); LEGEND_ENTRIES_MAX + 4];
    let found = violations(&page);
    assert_eq!(
        found.first(),
        Some(&(
            "/legend".to_string(),
            "legend-too-long",
            "legend has 20 entries, above 16".to_string()
        ))
    );
    let text_pointers: Vec<&str> = found
        .iter()
        .filter(|(_, rule, _)| *rule == "text-untrimmed")
        .map(|(pointer, _, _)| pointer.as_str())
        .collect();
    let expected: Vec<String> = (0..=LEGEND_ENTRIES_MAX)
        .map(|index| format!("/legend/{index}/text"))
        .collect();
    assert_eq!(text_pointers, expected);
}

#[test]
fn grow_weights_are_checked_up_to_one_past_the_children_limit() {
    let mut page = page_with_body(vec![row(vec![item("a")])]);
    first_row_mut(&mut page).grow = Some(vec![101; CHILDREN_MAX + 1]);
    let at_bound = violations(&page);
    assert_eq!(at_bound.len(), 1 + CHILDREN_MAX + 1);
    first_row_mut(&mut page).grow = Some(vec![101; 10_000]);
    let found = violations(&page);
    assert_eq!(
        found.first(),
        Some(&(
            "/body/0/grow".to_string(),
            "grow-length-mismatch",
            "grow has 10000 weights for 1 children".to_string()
        ))
    );
    let weight_violations = found
        .iter()
        .filter(|(_, rule, _)| *rule == "grow-out-of-range")
        .count();
    assert_eq!(weight_violations, CHILDREN_MAX + 1);
    assert_eq!(
        found.last().map(|(pointer, _, _)| pointer.as_str()),
        Some("/body/0/grow/256")
    );
}

#[test]
fn a_huge_children_list_is_walked_only_to_the_node_limit() {
    let page = page_with_body(vec![row(vec![item("a"); 200_000])]);
    let entries = body_nodes(&page);
    assert_eq!(entries.len(), NODES_MAX + 1);
    assert_eq!(
        entries.last().map(|entry| entry.pointer.as_str()),
        Some("/body/0/children/4095")
    );
    let found = violations(&page);
    assert_eq!(
        found,
        vec![
            (
                "/body".to_string(),
                "nodes-exceeded",
                "more than 4096 nodes".to_string()
            ),
            (
                "/body/0/children".to_string(),
                "children-too-many",
                "200000 children, above 256".to_string()
            ),
        ]
    );
}
