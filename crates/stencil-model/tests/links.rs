// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{
    legend_entry, link, page_with_body, page_with_link, pcard, pcard_with_id, pipe, pipe_value,
};
use stencil_model::checks::{legend_consistency, remembered_constants};
use stencil_model::{
    LINK_VIA_MAX, LINKS_MAX, Node, Page, PagePoint, PipeDir, PipeKind, Tee, TeeArm, VetRule,
    text_fields, validate_page,
};

fn violations(page: &Page) -> Vec<(String, &'static str, String)> {
    validate_page(page)
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

fn set_first_id(page: &mut Page, id: &str) {
    match page.body.first_mut() {
        Some(Node::Pcard(pcard)) => pcard.id = Some(id.to_string()),
        _ => panic!("first body node is a Pcard"),
    }
}

#[test]
fn vet_rule_names_for_ids_and_links() {
    let expected = [
        (VetRule::IdMalformed, "id-malformed"),
        (VetRule::IdDuplicate, "id-duplicate"),
        (VetRule::LinksTooMany, "links-too-many"),
        (VetRule::LinkSelf, "link-self"),
        (VetRule::LinkUnknownId, "link-unknown-id"),
        (VetRule::LinkViaTooMany, "link-via-too-many"),
        (VetRule::LinkViaOutside, "link-via-outside"),
    ];
    for (rule, name) in expected {
        assert_eq!(rule.as_str(), name);
    }
}

#[test]
fn a_link_between_two_ids_passes() {
    assert_eq!(violations(&page_with_link()), vec![]);
}

#[test]
fn duplicate_id_is_reported_at_the_later_node() {
    let mut page = page_with_link();
    page.body.push(pcard_with_id("api", "Second API"));
    assert_eq!(
        violations(&page),
        one(
            "/body/2/id",
            "id-duplicate",
            "id \"api\" is already used at /body/0"
        )
    );
}

#[test]
fn tee_arm_ids_take_part_in_uniqueness_and_resolution() {
    let mut arm = pipe_value(PipeDir::Horizontal, PipeKind::Blue, "arm");
    arm.id = Some("api".to_string());
    let mut page = page_with_link();
    page.body.push(Node::Tee(Tee {
        id: None,
        kind: PipeKind::Blue,
        hub: "hub".to_string(),
        arms: [
            TeeArm::Pipe(arm),
            TeeArm::Pipe(pipe_value(PipeDir::Horizontal, PipeKind::Blue, "other")),
        ],
    }));
    assert_eq!(
        violations(&page),
        one(
            "/body/2/arms/0/id",
            "id-duplicate",
            "id \"api\" is already used at /body/0"
        )
    );

    let mut arm = pipe_value(PipeDir::Horizontal, PipeKind::Blue, "arm");
    arm.id = Some("arm".to_string());
    if let Some(Node::Tee(tee)) = page.body.last_mut() {
        tee.arms[0] = TeeArm::Pipe(arm);
    }
    page.links.push(link("worker", "arm"));
    assert_eq!(violations(&page), vec![]);
}

#[test]
fn id_pattern_accepts_64_characters_and_rejects_65() {
    let mut page = page_with_link();
    let longest = format!("a{}", "-".repeat(63));
    set_first_id(&mut page, &longest);
    page.links = vec![link(&longest, "worker")];
    assert_eq!(violations(&page), vec![]);

    let too_long = format!("a{}", "-".repeat(64));
    set_first_id(&mut page, &too_long);
    page.links = vec![];
    assert_eq!(
        violations(&page),
        one(
            "/body/0/id",
            "id-malformed",
            "id does not match ^[a-z0-9][a-z0-9-]{0,63}$"
        )
    );
}

#[test]
fn id_with_uppercase_leading_hyphen_or_empty_is_malformed() {
    for id in ["Gateway", "-gateway", "", "api gateway", "api_gateway"] {
        let mut page = page_with_link();
        set_first_id(&mut page, id);
        page.links = vec![];
        assert_eq!(
            violations(&page),
            one(
                "/body/0/id",
                "id-malformed",
                "id does not match ^[a-z0-9][a-z0-9-]{0,63}$"
            ),
            "{id:?}"
        );
    }
    let mut page = page_with_link();
    set_first_id(&mut page, "0");
    page.links = vec![link("0", "worker")];
    assert_eq!(violations(&page), vec![]);
}

#[test]
fn unknown_endpoint_is_reported_at_its_field() {
    let mut page = page_with_link();
    page.links = vec![link("api", "queue")];
    assert_eq!(
        violations(&page),
        one("/links/0/to", "link-unknown-id", "no node has id \"queue\"")
    );
    page.links = vec![link("Queue", "worker")];
    assert_eq!(
        violations(&page),
        one(
            "/links/0/from",
            "link-unknown-id",
            "no node has this id; it does not match ^[a-z0-9][a-z0-9-]{0,63}$"
        )
    );
}

#[test]
fn self_link_is_reported_at_the_link() {
    let mut page = page_with_link();
    page.links = vec![link("api", "api")];
    assert_eq!(
        violations(&page),
        one("/links/0", "link-self", "from and to name the same node")
    );
}

#[test]
fn via_inside_the_canvas_passes_and_outside_fails() {
    let mut page = page_with_link();
    // Width 1280 gives a 1320 px canvas (section 2.2). The canvas height is known only
    // after layout, so any y at or above 0 is inside.
    page.links[0].via = vec![
        PagePoint { x: 0.0, y: 0.0 },
        PagePoint {
            x: 1320.0,
            y: 100_000.0,
        },
    ];
    assert_eq!(violations(&page), vec![]);

    page.links[0].via = vec![
        PagePoint { x: -1.0, y: 10.0 },
        PagePoint { x: 1320.5, y: 10.0 },
        PagePoint { x: 10.0, y: -0.5 },
        PagePoint {
            x: 10.0,
            y: f32::INFINITY,
        },
    ];
    assert_eq!(
        violations(&page),
        vec![
            (
                "/links/0/via/0".to_string(),
                "link-via-outside",
                "via point (-1, 10) is outside the page: x 0 to 1320, y 0 or more".to_string()
            ),
            (
                "/links/0/via/1".to_string(),
                "link-via-outside",
                "via point (1320.5, 10) is outside the page: x 0 to 1320, y 0 or more".to_string()
            ),
            (
                "/links/0/via/2".to_string(),
                "link-via-outside",
                "via point (10, -0.5) is outside the page: x 0 to 1320, y 0 or more".to_string()
            ),
            (
                "/links/0/via/3".to_string(),
                "link-via-outside",
                "via point (10, inf) is outside the page: x 0 to 1320, y 0 or more".to_string()
            ),
        ]
    );
}

#[test]
fn via_outside_follows_the_page_width() {
    let mut page = page_with_link();
    page.width = 640;
    page.links[0].via = vec![PagePoint { x: 681.0, y: 0.0 }];
    assert_eq!(
        violations(&page),
        one(
            "/links/0/via/0",
            "link-via-outside",
            "via point (681, 0) is outside the page: x 0 to 680, y 0 or more"
        )
    );
}

#[test]
fn an_overflowing_json_coordinate_is_outside() {
    let document = serde_json::json!({
        "title": "t", "kicker": "k", "lede": "l", "canvas": "internal",
        "body": [ { "tag": "Pcard", "id": "a", "fn": "A" }, { "tag": "Pcard", "id": "b", "fn": "B" } ],
        "legend": [ { "kind": "blue", "text": "b" } ],
        "links": [ { "from": "a", "to": "b", "kind": "blue", "via": [ { "x": 1e39, "y": 0 } ] } ]
    });
    let page: Page = serde_json::from_value(document).unwrap();
    let rules: Vec<&str> = violations(&page).iter().map(|found| found.1).collect();
    assert_eq!(rules, ["link-via-outside"]);
}

#[test]
fn eight_via_points_pass_and_nine_fail() {
    let mut page = page_with_link();
    page.links[0].via = vec![PagePoint { x: 10.0, y: 10.0 }; LINK_VIA_MAX];
    assert_eq!(violations(&page), vec![]);
    page.links[0].via.push(PagePoint { x: 10.0, y: 10.0 });
    assert_eq!(
        violations(&page),
        one(
            "/links/0/via",
            "link-via-too-many",
            "via has 9 points, above 8"
        )
    );
}

#[test]
fn via_points_are_checked_up_to_one_past_the_limit() {
    let mut page = page_with_link();
    page.links[0].via = vec![PagePoint { x: -1.0, y: 0.0 }; 20];
    let found = violations(&page);
    assert_eq!(found.len(), 1 + LINK_VIA_MAX + 1);
    assert_eq!(found[0].1, "link-via-too-many");
    assert_eq!(
        found.last().map(|last| last.0.as_str()),
        Some("/links/0/via/8")
    );
}

#[test]
fn links_limit() {
    let mut page = page_with_link();
    page.links = vec![link("api", "worker"); LINKS_MAX];
    assert_eq!(violations(&page), vec![]);
    page.links.push(link("api", "worker"));
    assert_eq!(
        violations(&page),
        one(
            "/links",
            "links-too-many",
            "links has 257 entries, above 256"
        )
    );
}

#[test]
fn link_rules_stop_one_link_past_the_limit() {
    let mut page = page_with_link();
    let mut untrimmed = link("api", "worker");
    untrimmed.label = Some(" x".to_string());
    page.links = vec![untrimmed; 300];
    let found = violations(&page);
    assert_eq!(found.len(), 1 + LINKS_MAX + 1);
    assert_eq!(
        found[0],
        (
            "/links".to_string(),
            "links-too-many",
            "links has 300 entries, above 256".to_string()
        )
    );
    assert_eq!(
        found.last().map(|last| (last.0.as_str(), last.1)),
        Some(("/links/256/label", "text-untrimmed"))
    );
    assert_eq!(text_fields(&page).len(), 3 + 2 + 1 + LINKS_MAX + 1);
}

#[test]
fn link_violations_follow_legend_violations_in_field_order() {
    let mut page = page_with_link();
    page.legend[0].text = " request path".to_string();
    let mut bad = link("api", "api");
    bad.label = Some(String::new());
    bad.sub = Some("sub ".to_string());
    bad.via = vec![PagePoint { x: -5.0, y: 0.0 }];
    page.links.push(bad);
    page.links.push(link("gone", "worker"));
    let pointers_and_rules: Vec<(String, &str)> = violations(&page)
        .into_iter()
        .map(|(pointer, rule, _)| (pointer, rule))
        .collect();
    assert_eq!(
        pointers_and_rules,
        vec![
            ("/legend/0/text".to_string(), "text-untrimmed"),
            ("/links/1".to_string(), "link-self"),
            ("/links/1/label".to_string(), "text-empty"),
            ("/links/1/sub".to_string(), "text-untrimmed"),
            ("/links/1/via/0".to_string(), "link-via-outside"),
            ("/links/2/from".to_string(), "link-unknown-id"),
        ]
    );
}

#[test]
fn link_label_and_sub_are_text_fields_after_the_legend() {
    let mut page = page_with_link();
    page.links[0].label = Some("1 submit".to_string());
    page.links[0].sub = Some("HTTPS".to_string());
    let pointers: Vec<String> = text_fields(&page)
        .iter()
        .map(|field| field.pointer.as_str().to_string())
        .collect();
    assert_eq!(
        pointers,
        [
            "/title",
            "/kicker",
            "/lede",
            "/body/0/fn",
            "/body/1/fn",
            "/legend/0/text",
            "/links/0/label",
            "/links/0/sub"
        ]
    );
    page.links[0].sub = Some("ASN 64512".to_string());
    let report = remembered_constants(&page);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/links/0/sub");
}

#[test]
fn link_kind_counts_as_a_legend_use() {
    let page = page_with_link();
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 2);
    assert!(report.passed(), "{:?}", report.defects);
}

#[test]
fn link_kind_missing_from_legend_is_a_defect_at_the_link() {
    let mut page = page_with_link();
    page.links.push(link("worker", "api"));
    page.links[1].kind = PipeKind::Deny;
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 3);
    let defects: Vec<(&str, &str)> = report
        .defects
        .iter()
        .map(|defect| (defect.pointer.as_str(), defect.message.as_str()))
        .collect();
    assert_eq!(
        defects,
        [("/links/1", "Link kind deny has no legend entry")]
    );
}

#[test]
fn legend_kind_used_by_a_pipe_and_a_link_is_one_entry() {
    let mut page = page_with_body(vec![
        pcard_with_id("api", "API"),
        pipe(PipeKind::Gray, "internal"),
        pcard_with_id("worker", "Worker"),
    ]);
    page.legend = vec![legend_entry(PipeKind::Gray, "internal call")];
    page.links = vec![link("api", "worker")];
    page.links[0].kind = PipeKind::Gray;
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 3);
    assert!(report.passed(), "{:?}", report.defects);
}

#[test]
fn legend_counts_links_up_to_one_past_the_limit() {
    let mut page = page_with_body(vec![pcard("a")]);
    page.links = vec![link("a", "b"); 300];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, (LINKS_MAX + 1 + 1) as u64);
}
