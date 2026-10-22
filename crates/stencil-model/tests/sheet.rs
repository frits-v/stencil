// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! The design sheet's vet rules (section 14.3): one test per rule, the via bound on a
//! sheet, and the CUE twin of every rule.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use stencil_model::{Page, SheetList, SheetSurface, VetRule, validate_page};

/// The smallest valid sheet: one View holding two items and a link between them.
fn sheet_value() -> Value {
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "sheet": {
            "problem": { "points": [{ "text": "A point." }] },
            "answer": { "statement": "A statement." },
            "attention": { "items": [{ "id": "first", "lead": "Lead.", "text": "Text." }] }
        },
        "body": [{
            "tag": "View",
            "title": "System",
            "children": [{
                "tag": "Row",
                "children": [
                    { "tag": "Item", "id": "api", "kind": "product", "title": "API" },
                    { "tag": "Item", "id": "worker", "kind": "product", "title": "Worker" }
                ]
            }]
        }],
        "legend": [{ "line": "solid", "text": "request path" }],
        "links": [{ "from": "api", "to": "worker", "line": "solid" }]
    })
}

/// (pointer, rule name, message) for every violation of `document` under gcp.
fn violations(document: Value) -> Vec<(String, &'static str, String)> {
    let page: Page = serde_json::from_value(document).unwrap();
    validate_page(&page, &common::gcp())
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

#[test]
fn the_smallest_sheet_vets_clean() {
    assert_eq!(violations(sheet_value()), Vec::new());
}

#[test]
fn a_view_on_a_page_without_a_sheet_needs_a_sheet() {
    let mut document = sheet_value();
    document.as_object_mut().unwrap().remove("sheet");
    assert_eq!(
        violations(document),
        one(
            "/body/0",
            "needs-sheet",
            "view is drawn on a design sheet only; add sheet or remove it"
        )
    );
}

#[test]
fn a_sheet_under_iso_is_refused() {
    let mut document = sheet_value();
    document["projection"] = json!("iso");
    assert_eq!(
        violations(document),
        one(
            "/sheet",
            "sheet-in-iso",
            "a design sheet is drawn flat only"
        )
    );
}

#[test]
fn a_sheet_keeps_its_chrome() {
    let mut document = sheet_value();
    document["chrome"] = json!("none");
    assert_eq!(
        violations(document),
        one(
            "/chrome",
            "sheet-chrome-none",
            "a design sheet draws its head in the title block, so chrome cannot be none"
        )
    );
}

#[test]
fn a_sheet_body_child_that_is_not_a_view_is_refused() {
    let mut document = sheet_value();
    document["body"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "tag": "Fact", "text": "loose" }));
    assert_eq!(
        violations(document),
        one(
            "/body/1",
            "sheet-body-not-view",
            "a design sheet holds Views; wrap Fact in a View"
        )
    );
}

#[test]
fn a_view_inside_a_view_is_refused() {
    let mut document = sheet_value();
    document["body"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "tag": "View",
            "title": "Inner",
            "children": [{ "tag": "Fact", "text": "inside" }]
        }));
    assert_eq!(
        violations(document),
        one(
            "/body/0/children/1",
            "view-not-top-level",
            "a View is a body child of a sheet"
        )
    );
}

#[test]
fn a_sheet_without_lines_is_refused() {
    let mut document = sheet_value();
    document.as_object_mut().unwrap().remove("links");
    assert_eq!(
        violations(document),
        one(
            "/sheet",
            "sheet-without-lines",
            "a design sheet draws at least one Pipe, Tee or Link, which its legend strip keys"
        )
    );
}

#[test]
fn an_empty_problem_or_attention_list_is_refused() {
    let mut document = sheet_value();
    document["sheet"]["problem"]["points"] = json!([]);
    document["sheet"]["attention"]["items"] = json!([]);
    assert_eq!(
        violations(document),
        vec![
            (
                "/sheet/problem/points".to_string(),
                "list-empty",
                "problem-points is empty".to_string()
            ),
            (
                "/sheet/attention/items".to_string(),
                "list-empty",
                "attention-items is empty".to_string()
            ),
        ]
    );
}

#[test]
fn a_list_above_its_maximum_is_refused() {
    let mut document = sheet_value();
    let points: Vec<Value> = (0..9).map(|_| json!({ "text": "A point." })).collect();
    document["sheet"]["problem"]["points"] = json!(points);
    let paragraphs: Vec<Value> = (0..5).map(|_| json!("A paragraph.")).collect();
    document["sheet"]["answer"]["paragraphs"] = json!(paragraphs);
    let items: Vec<Value> = (0..9)
        .map(|index| json!({ "id": format!("item-{index}"), "lead": "Lead.", "text": "Text." }))
        .collect();
    document["sheet"]["attention"]["items"] = json!(items);
    assert_eq!(
        violations(document),
        vec![
            (
                "/sheet/problem/points".to_string(),
                "list-too-long",
                "9 problem-points, above 8".to_string()
            ),
            (
                "/sheet/answer/paragraphs".to_string(),
                "list-too-long",
                "5 answer-paragraphs, above 4".to_string()
            ),
            (
                "/sheet/attention/items".to_string(),
                "list-too-long",
                "9 attention-items, above 8".to_string()
            ),
        ]
    );
}

#[test]
fn a_wordmark_or_caption_above_its_limit_is_refused() {
    let mut document = sheet_value();
    document["sheet"]["wordmark"] = json!("w".repeat(25));
    document["sheet"]["answer"]["caption"] = json!("c".repeat(61));
    assert_eq!(
        violations(document),
        vec![
            (
                "/sheet/wordmark".to_string(),
                "sheet-text-too-long",
                "wordmark of 25 characters, above 24".to_string()
            ),
            (
                "/sheet/answer/caption".to_string(),
                "sheet-text-too-long",
                "caption of 61 characters, above 60".to_string()
            ),
        ]
    );
}

#[test]
fn two_attention_items_with_one_id_are_refused() {
    let mut document = sheet_value();
    document["sheet"]["attention"]["items"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "first", "lead": "Again.", "text": "Text." }));
    assert_eq!(
        violations(document),
        one(
            "/sheet/attention/items/1/id",
            "key-duplicate",
            "\"first\" is already used by /sheet/attention/items/0"
        )
    );
}

#[test]
fn a_malformed_attention_item_id_is_refused() {
    let mut document = sheet_value();
    document["sheet"]["attention"]["items"][0]["id"] = json!("First");
    let found = violations(document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "/sheet/attention/items/0/id");
    assert_eq!(found[0].1, "id-malformed");
}

#[test]
fn sheet_text_is_held_to_the_text_rules() {
    let mut document = sheet_value();
    document["sheet"]["answer"]["statement"] = json!(" padded ");
    assert_eq!(
        violations(document),
        one(
            "/sheet/answer/statement",
            "text-untrimmed",
            "text starts or ends with whitespace"
        )
    );
}

/// On a sheet the canvas is the width plus 50 px a side; elsewhere plus 20.
#[test]
fn a_via_in_the_sheet_margin_vets() {
    let with_via = |document: &mut Value, x: f32| {
        document["links"][0]["via"] = json!([{ "x": x, "y": 10.0 }]);
    };
    let mut inside = sheet_value();
    with_via(&mut inside, 1280.0 + 95.0);
    assert_eq!(violations(inside), Vec::new());

    let mut outside = sheet_value();
    with_via(&mut outside, 1280.0 + 101.0);
    assert_eq!(
        violations(outside),
        one(
            "/links/0/via/0",
            "link-via-outside",
            "via point (1381, 10) is outside the page: x 0 to 1380, y 0 or more"
        )
    );

    let mut page = common::page_with_link();
    page.links[0].via = vec![stencil_model::PagePoint {
        x: 1280.0 + 41.0,
        y: 10.0,
    }];
    let off_sheet: Vec<&str> = validate_page(&page, &common::gcp())
        .iter()
        .map(|violation| violation.rule.as_str())
        .collect();
    assert_eq!(off_sheet, ["link-via-outside"]);
}

/// The smallest sheet with actors: two actors, each item acting for one, and a Phases node
/// of both lanes, one phase and one Step after the items.
fn actor_sheet_value() -> Value {
    let mut document = sheet_value();
    document["actors"] = json!([
        { "id": "crew", "name": "Crew", "tint": 2 },
        { "id": "ops", "name": "Ops", "text": "runs the jobs", "tint": 4 }
    ]);
    document["body"][0]["children"][0]["children"][0]["actor"] = json!("crew");
    document["body"][0]["children"][0]["children"][1]["actor"] = json!("ops");
    document["body"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "tag": "Phases",
            "lanes": ["crew", "ops"],
            "phases": [{ "id": "run", "title": "Run" }],
            "children": [{ "tag": "Step", "lane": "crew", "phase": "run", "title": "Work" }]
        }));
    document["legend"] = json!([{ "line": "solid", "by_actor": true, "text": "work moves" }]);
    document
}

const PHASES: &str = "/body/0/children/1";
const STEP: &str = "/body/0/children/1/children/0";

#[test]
fn the_smallest_actor_sheet_vets_clean() {
    assert_eq!(violations(actor_sheet_value()), Vec::new());
}

#[test]
fn two_actors_in_one_slot_are_refused() {
    let mut document = actor_sheet_value();
    document["actors"][1]["tint"] = json!(2);
    assert_eq!(
        violations(document),
        one(
            "/actors/1/tint",
            "actor-tint-duplicate",
            "tint 2 is already used by actor \"crew\""
        )
    );
}

#[test]
fn two_actors_with_one_id_are_refused() {
    let mut document = actor_sheet_value();
    document["actors"][1]["id"] = json!("crew");
    document["body"][0]["children"][0]["children"][1]["actor"] = json!("crew");
    document["body"][0]["children"][1]["lanes"] = json!(["crew"]);
    assert_eq!(
        violations(document),
        one(
            "/actors/1/id",
            "key-duplicate",
            "\"crew\" is already used by /actors/0"
        )
    );
}

#[test]
fn an_undeclared_actor_is_refused() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][0]["children"][0]["actor"] = json!("crews");
    assert_eq!(
        violations(document),
        one(
            "/body/0/children/0/children/0/actor",
            "actor-unknown",
            "actor \"crews\" is not declared; declared: crew, ops"
        )
    );
}

#[test]
fn a_tint_on_a_page_with_actors_is_refused() {
    let mut document = actor_sheet_value();
    document["links"][0]["tint"] = json!(2);
    assert_eq!(
        violations(document),
        one(
            "/links/0/tint",
            "tint-on-actor-page",
            "a page with actors colours by actor; remove tint"
        )
    );
}

#[test]
fn a_kind_that_paints_a_tint_needs_an_actor() {
    let mut document = actor_sheet_value();
    let row = document["body"][0]["children"][0].clone();
    document["body"][0]["children"][0] = json!({
        "tag": "Box", "kind": "gcp", "label": "Cloud",
        "children": [{ "tag": "Box", "kind": "region", "label": "Region", "children": [row] }]
    });
    assert_eq!(
        violations(document),
        one(
            "/body/0/children/0/children/0",
            "kind-tint-without-actor",
            "region paints tint 1 by default, which reads as an actor; name its actor"
        )
    );
}

#[test]
fn a_line_without_an_actor_on_a_page_with_actors_is_refused() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][0]["children"][0]
        .as_object_mut()
        .unwrap()
        .remove("actor");
    document["body"][0]["children"][1]["lanes"] = json!(["crew", "ops"]);
    assert_eq!(
        violations(document),
        one(
            "/links/0/line",
            "line-without-actor",
            "solid line from \"api\" has no actor; set actor or leave from an actor's node"
        )
    );
}

#[test]
fn a_gray_line_takes_no_actor() {
    let mut document = actor_sheet_value();
    document["links"][0]["line"] = json!("gray");
    document["links"][0]["actor"] = json!("crew");
    assert_eq!(
        violations(document),
        one(
            "/links/0/actor",
            "actor-on-neutral-line",
            "a gray or deny line takes no actor"
        )
    );
}

#[test]
fn an_untintable_box_names_no_actor() {
    let mut document = actor_sheet_value();
    let row = document["body"][0]["children"][0].clone();
    document["body"][0]["children"][0] = json!({
        "tag": "Box", "kind": "gcp", "label": "Cloud", "actor": "crew", "children": [row]
    });
    assert_eq!(
        violations(document),
        one(
            "/body/0/children/0/actor",
            "actor-on-untintable-kind",
            "gcp takes no colour, so it cannot name an actor"
        )
    );
}

#[test]
fn a_tee_arm_acts_for_its_tee() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .insert(
            1,
            json!({
                "tag": "Tee", "line": "solid", "actor": "crew", "hub": "fan out",
                "arms": [
                    { "tag": "Pipe", "dir": "h", "line": "solid", "actor": "ops", "label": "read" },
                    { "tag": "Pipe", "dir": "h", "line": "solid", "label": "write" }
                ]
            }),
        );
    assert_eq!(
        violations(document),
        one(
            "/body/0/children/0/children/1/arms/0/actor",
            "tee-arm-actor-differs",
            "a Tee arm acts for its Tee's actor"
        )
    );
}

#[test]
fn a_by_actor_entry_names_solid_or_dashed_lines_on_a_page_with_actors() {
    let mut gray = actor_sheet_value();
    gray["legend"][0]["line"] = json!("gray");
    assert_eq!(
        violations(gray),
        one(
            "/legend/0/by_actor",
            "legend-by-actor-misused",
            "by_actor names solid or dashed lines on a page with actors"
        )
    );
    let mut without_actors = sheet_value();
    without_actors["legend"][0]["by_actor"] = json!(true);
    assert_eq!(
        violations(without_actors),
        one(
            "/legend/0/by_actor",
            "legend-by-actor-misused",
            "by_actor names solid or dashed lines on a page with actors"
        )
    );
}

#[test]
fn a_phases_node_holds_steps_only() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][1]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "tag": "Fact", "text": "loose" }));
    assert_eq!(
        violations(document),
        one(
            &format!("{PHASES}/children/1"),
            "phases-child-not-step",
            "a Phases node holds Steps only"
        )
    );
}

#[test]
fn a_step_outside_a_phases_node_is_refused() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "tag": "Step", "lane": "crew", "phase": "run", "title": "Loose" }));
    assert_eq!(
        violations(document),
        one(
            "/body/0/children/2",
            "step-outside-phases",
            "a Step sits directly in a Phases node"
        )
    );
}

#[test]
fn an_actor_with_two_lanes_is_refused() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][1]["lanes"] = json!(["crew", "ops", "crew"]);
    assert_eq!(
        violations(document),
        one(
            &format!("{PHASES}/lanes/2"),
            "phase-lane-twice",
            "actor \"crew\" already has a lane"
        )
    );
}

#[test]
fn a_step_in_a_lane_or_phase_its_node_lacks_is_refused() {
    let mut lane = actor_sheet_value();
    lane["body"][0]["children"][1]["lanes"] = json!(["crew"]);
    lane["body"][0]["children"][1]["children"][0]["lane"] = json!("ops");
    assert_eq!(
        violations(lane),
        one(
            &format!("{STEP}/lane"),
            "step-lane-unknown",
            "lane \"ops\" is not a lane of this Phases node"
        )
    );
    let mut phase = actor_sheet_value();
    phase["body"][0]["children"][1]["children"][0]["phase"] = json!("later");
    assert_eq!(
        violations(phase),
        one(
            &format!("{STEP}/phase"),
            "step-phase-unknown",
            "phase \"later\" is not a phase of this Phases node"
        )
    );
}

#[test]
fn a_step_past_column_6_is_refused() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][1]["children"][0]["col"] = json!(5);
    document["body"][0]["children"][1]["children"][0]["span"] = json!(3);
    assert_eq!(
        violations(document),
        one(
            &format!("{STEP}/span"),
            "step-span-out-of-range",
            "columns 5 to 7 run past column 6"
        )
    );
}

#[test]
fn a_text_step_with_its_own_fill_is_refused() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][1]["children"][0]["look"] = json!("text");
    document["body"][0]["children"][1]["children"][0]["fill"] = json!("tint");
    assert_eq!(
        violations(document),
        one(
            &format!("{STEP}/fill"),
            "step-text-fill",
            "a text Step draws no box, so it takes no fill"
        )
    );
}

#[test]
fn a_text_step_under_a_phases_step_fill_and_a_box_step_with_a_fill_vet() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][1]["step_fill"] = json!("none");
    document["body"][0]["children"][1]["band"] = json!("none");
    document["body"][0]["children"][1]["children"][0]["look"] = json!("text");
    assert_eq!(violations(document.clone()), Vec::new());
    document["body"][0]["children"][1]["children"][0]["look"] = json!("box");
    document["body"][0]["children"][1]["children"][0]["fill"] = json!("tint");
    assert_eq!(violations(document), Vec::new());
}

#[test]
fn a_phases_node_without_lanes_or_phases_is_refused() {
    let mut lanes = actor_sheet_value();
    lanes["body"][0]["children"][1]["lanes"] = json!([]);
    assert!(violations(lanes).contains(&(
        format!("{PHASES}/lanes"),
        "list-empty",
        "lanes is empty".to_string()
    )));
    let mut phases = actor_sheet_value();
    phases["body"][0]["children"][1]["phases"] = json!([]);
    assert!(violations(phases).contains(&(
        format!("{PHASES}/phases"),
        "list-empty",
        "phases is empty".to_string()
    )));
}

#[test]
fn two_phases_with_one_id_are_refused() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][1]["phases"] =
        json!([{ "id": "run", "title": "Run" }, { "id": "run", "title": "Again" }]);
    assert_eq!(
        violations(document),
        one(
            &format!("{PHASES}/phases/1/id"),
            "key-duplicate",
            &format!("\"run\" is already used by {PHASES}/phases/0")
        )
    );
}

#[test]
fn every_actor_surface_off_a_sheet_needs_a_sheet() {
    let mut document = actor_sheet_value();
    document.as_object_mut().unwrap().remove("sheet");
    let found: Vec<(String, &'static str)> = violations(document)
        .into_iter()
        .map(|(pointer, rule, _)| (pointer, rule))
        .collect();
    let expected: Vec<(String, &'static str)> = [
        "/actors",
        "/body/0",
        "/body/0/children/0/children/0/actor",
        "/body/0/children/0/children/1/actor",
        PHASES,
        STEP,
        "/legend/0/by_actor",
    ]
    .iter()
    .map(|pointer| (pointer.to_string(), "needs-sheet"))
    .collect();
    assert_eq!(found, expected);
}

/// Every rule of section 14.3 has a test above, in table order.
#[test]
fn every_sheet_rule_is_listed_in_table_order() {
    let names: Vec<&str> = VetRule::SHEET.iter().map(|rule| rule.as_str()).collect();
    assert_eq!(
        names,
        [
            "needs-sheet",
            "sheet-in-iso",
            "sheet-chrome-none",
            "sheet-body-not-view",
            "view-not-top-level",
            "sheet-without-lines",
            "list-empty",
            "list-too-long",
            "sheet-text-too-long",
            "key-duplicate",
            "actor-tint-duplicate",
            "actor-unknown",
            "tint-on-actor-page",
            "kind-tint-without-actor",
            "line-without-actor",
            "actor-on-neutral-line",
            "actor-on-untintable-kind",
            "tee-arm-actor-differs",
            "legend-by-actor-misused",
            "phases-child-not-step",
            "step-outside-phases",
            "phase-lane-twice",
            "step-lane-unknown",
            "step-phase-unknown",
            "step-span-out-of-range",
            "step-text-fill",
            "mark-unknown",
            "mark-repeated",
            "mark-legend-unused",
            "link-marks-without-tag",
            "keyed-list-twice",
            "part-node-unknown",
            "part-node-twice",
            "part-field-from-node",
            "part-name-missing",
            "planned-not-solid",
            "line-into-planned",
            "compartments-without-store",
            "outline-person-with-icon",
            "rail-not-vertical",
            "rail-outside-row",
            "rail-with-targets",
            "step-out-of-range",
            "link-step-duplicate",
            "valve-without-arrow",
        ]
    );
}

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The names of the cases `cue/check.sh` runs: the first field of every quoted array
/// entry of the form `'<name>|<edit>|<expected>'`.
fn cue_case_names(check_sh: &str) -> BTreeSet<String> {
    check_sh
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix('\'').or_else(|| line.strip_prefix('"')))
        .filter_map(|line| line.split_once('|').map(|(name, _)| name.to_string()))
        .collect()
}

/// Section 14.7 rule 9: `#SheetSurface` names every surface, and every rule, surface and
/// list of section 14.3 has a check.sh case. Reads the files as text and runs no `cue`.
#[test]
fn every_section_14_vet_rule_has_a_cue_case() {
    let check_sh = fs::read_to_string(repository_path("cue/check.sh")).unwrap();
    let core_cue = fs::read_to_string(repository_path("cue/core.cue")).unwrap();
    let cases = cue_case_names(&check_sh);
    assert!(!cases.is_empty(), "cue/check.sh holds no case");

    let surface_line = core_cue
        .lines()
        .find(|line| line.starts_with("#SheetSurface:"))
        .expect("cue/core.cue defines #SheetSurface on one line");
    let cue_surfaces: BTreeSet<&str> = surface_line
        .trim_start_matches("#SheetSurface:")
        .split('|')
        .map(|name| name.trim().trim_matches('"'))
        .collect();
    let surfaces: BTreeSet<&str> = SheetSurface::ALL
        .iter()
        .map(|surface| surface.as_str())
        .collect();
    assert_eq!(cue_surfaces, surfaces);

    let mut expected: Vec<String> = Vec::new();
    for surface in SheetSurface::ALL {
        if surface != SheetSurface::Sheet {
            expected.push(format!("needs-sheet-{}", surface.as_str()));
        }
    }
    for list in SheetList::ALL {
        expected.push(format!("list-too-long-{}", list.name()));
        if list.min() == 1 {
            expected.push(format!("list-empty-{}", list.name()));
        }
    }
    let per_list = [
        VetRule::NeedsSheet,
        VetRule::ListEmpty,
        VetRule::ListTooLong,
    ];
    let others: Vec<VetRule> = VetRule::SHEET
        .into_iter()
        .filter(|rule| !per_list.contains(rule))
        .collect();
    for rule in &others {
        expected.push(rule.as_str().to_string());
    }
    let mut missing = Vec::new();
    let mut matched = 0;
    for name in &expected {
        let found = cases.contains(name)
            || cases
                .iter()
                .any(|case| case.starts_with(&format!("{name}-")));
        if found {
            matched += 1;
        } else {
            missing.push(name.clone());
        }
    }
    assert_eq!(missing, Vec::<String>::new());

    let sheet_rules: Vec<&str> = VetRule::SHEET.iter().map(|rule| rule.as_str()).collect();
    let stray: Vec<&String> = cases
        .iter()
        .filter(|case| {
            sheet_rules
                .iter()
                .any(|rule| case.as_str() == *rule || case.starts_with(&format!("{rule}-")))
        })
        .filter(|case| {
            !expected
                .iter()
                .any(|name| *case == name || case.starts_with(&format!("{name}-")))
        })
        .collect();
    assert_eq!(stray, Vec::<&String>::new());
    let non_empty = SheetList::ALL.iter().filter(|list| list.min() == 1).count();
    assert_eq!(
        (
            SheetSurface::ALL.len() - 1,
            SheetList::ALL.len(),
            non_empty,
            others.len()
        ),
        (17, 12, 6, 42),
        "surfaces, lists, non-empty lists and other rules examined"
    );
    assert_eq!(matched, expected.len());
}

/// Section 14.6: keys-used examines every actor and reports one that no lane, node, line
/// or attention item names; a page without actors reports it not applicable.
#[test]
fn an_actor_nothing_names_is_an_unused_key() {
    let mut document = actor_sheet_value();
    document["sheet"]["problem"]["points"][0]["marks"] = json!([{ "attention": "first" }]);
    let page: Page = serde_json::from_value(document.clone()).unwrap();
    let report = stencil_model::checks::keys_used(&page);
    assert_eq!((report.examined, report.defects.len()), (3, 0));

    document["actors"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "audit", "name": "Audit", "tint": 7 }));
    let page: Page = serde_json::from_value(document).unwrap();
    let report = stencil_model::checks::keys_used(&page);
    assert_eq!(report.examined, 4);
    let defects: Vec<(&str, &str)> = report
        .defects
        .iter()
        .map(|defect| (defect.pointer.as_str(), defect.message.as_str()))
        .collect();
    assert_eq!(
        defects,
        [(
            "/actors/2",
            "actor \"audit\" is never named by a lane, a node, a line or an attention item"
        )]
    );

    let mut document = sheet_value();
    document.as_object_mut().unwrap().remove("sheet");
    document["body"] = document["body"][0]["children"].clone();
    let page: Page = serde_json::from_value(document).unwrap();
    let report = stencil_model::checks::keys_used(&page);
    assert_eq!(
        report.not_applicable,
        Some("page has no actors or keyed lists")
    );
}

/// The smallest sheet with every key list: a parts list whose first part is the `api`
/// Item and whose second is named and ballooned on `worker`, general notes with one note
/// flagged on the link, and the attention item marked on the problem point.
fn marks_sheet_value() -> Value {
    let mut document = sheet_value();
    document["sheet"]["problem"]["points"][0]["marks"] = json!([{ "attention": "first" }]);
    document["body"][0]["children"][0]["children"][1]["marks"] = json!([{ "part": "cache" }]);
    let view = document["body"][0]["children"].as_array_mut().unwrap();
    view.push(json!({
        "tag": "PartsList",
        "title": "Parts",
        "parts": [
            { "id": "api", "node": "api", "why": "Answers requests." },
            { "id": "cache", "name": "Cache", "what": "in memory", "why": "Keeps reads fast." }
        ]
    }));
    view.push(json!({
        "tag": "GeneralNotes",
        "title": "Notes",
        "notes": [{ "id": "retry", "text": "Requests retry once." }]
    }));
    document["links"][0]["label"] = json!("request");
    document["links"][0]["marks"] = json!([{ "note": "retry" }]);
    document
}

const PARTS: &str = "/body/0/children/1";
const NOTES: &str = "/body/0/children/2";
const WORKER: &str = "/body/0/children/0/children/1";

#[test]
fn the_smallest_marked_sheet_vets_clean_and_uses_every_key() {
    let document = marks_sheet_value();
    assert_eq!(violations(document.clone()), Vec::new());
    let page: Page = serde_json::from_value(document).unwrap();
    let report = stencil_model::checks::keys_used(&page);
    assert_eq!((report.examined, report.defects.len()), (4, 0));
}

#[test]
fn a_mark_whose_key_is_not_in_its_list_is_refused() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][0]["children"][1]["marks"] =
        json!([{ "part": "store" }, { "note": "nothing" }, { "attention": "later" }]);
    document["body"][0]["children"][1]["parts"][1]["name"] = json!("Cache");
    assert_eq!(
        violations(document)
            .into_iter()
            .filter(|(_, rule, _)| *rule == "mark-unknown")
            .collect::<Vec<_>>(),
        vec![
            (
                format!("{WORKER}/marks/0"),
                "mark-unknown",
                "part \"store\" is not in the parts list".to_string()
            ),
            (
                format!("{WORKER}/marks/1"),
                "mark-unknown",
                "note \"nothing\" is not in the general notes".to_string()
            ),
            (
                format!("{WORKER}/marks/2"),
                "mark-unknown",
                "attention \"later\" is not in the attention items".to_string()
            ),
        ]
    );
}

#[test]
fn a_node_that_marks_one_key_twice_is_refused() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][0]["children"][0]["marks"] = json!([{ "part": "api" }]);
    document["body"][0]["children"][0]["children"][1]["marks"] =
        json!([{ "part": "cache" }, { "part": "cache" }]);
    assert_eq!(
        violations(document),
        vec![
            (
                "/body/0/children/0/children/0/marks/0".to_string(),
                "mark-repeated",
                "part \"api\" is already marked here".to_string()
            ),
            (
                format!("{WORKER}/marks/1"),
                "mark-repeated",
                "part \"cache\" is already marked here".to_string()
            ),
        ]
    );
}

#[test]
fn more_than_four_marks_on_one_carrier_are_refused() {
    let mut document = marks_sheet_value();
    document["sheet"]["problem"]["points"][0]["marks"] = json!([
        { "attention": "first" }, { "part": "api" }, { "part": "cache" }, { "note": "retry" },
        { "attention": "first" }
    ]);
    let found = violations(document);
    assert_eq!(
        found.first(),
        Some(&(
            "/sheet/problem/points/0/marks".to_string(),
            "list-too-long",
            "5 marks, above 4".to_string()
        ))
    );
}

#[test]
fn a_mark_legend_text_without_its_list_is_refused() {
    let mut document = marks_sheet_value();
    document["sheet"]["marks"] = json!({ "part": "a piece", "attention": "look here" });
    assert_eq!(violations(document.clone()), Vec::new());
    document["body"][0]["children"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    document["body"][0]["children"][0]["children"][1]
        .as_object_mut()
        .unwrap()
        .remove("marks");
    assert_eq!(
        violations(document),
        one(
            "/sheet/marks/part",
            "mark-legend-unused",
            "no part key on this sheet; remove the text"
        )
    );
}

#[test]
fn marks_on_a_link_without_a_label_are_refused() {
    let mut document = marks_sheet_value();
    document["links"][0]
        .as_object_mut()
        .unwrap()
        .remove("label");
    assert_eq!(
        violations(document),
        one(
            "/links/0/marks",
            "link-marks-without-tag",
            "a link's marks sit in its tag or under its valve; give it a label"
        )
    );
}

#[test]
fn a_second_parts_list_is_refused() {
    let mut document = marks_sheet_value();
    let second = document["body"][0]["children"][1].clone();
    document["body"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert_eq!(
        violations(document),
        one(
            "/body/0/children/3",
            "keyed-list-twice",
            &format!("the sheet has its parts list at {PARTS}")
        )
    );
}

#[test]
fn a_part_node_that_names_no_item_box_or_pipe_is_refused() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][1]["parts"][0]["node"] = json!("missing");
    assert_eq!(
        violations(document),
        one(
            &format!("{PARTS}/parts/0/node"),
            "part-node-unknown",
            "\"missing\" names no Item, Box or Pipe"
        )
    );
}

#[test]
fn two_parts_that_name_one_node_are_refused() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][1]["parts"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "front", "node": "api", "why": "Again." }));
    assert_eq!(
        violations(document),
        one(
            &format!("{PARTS}/parts/2/node"),
            "part-node-twice",
            "node \"api\" is already part \"api\""
        )
    );
}

#[test]
fn a_part_that_writes_what_its_node_gives_is_refused() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][1]["parts"][0]["what"] = json!("a server");
    assert_eq!(
        violations(document),
        one(
            &format!("{PARTS}/parts/0/what"),
            "part-field-from-node",
            "part \"api\" takes its what from node \"api\""
        )
    );
}

#[test]
fn a_part_without_a_node_or_a_name_is_refused() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][1]["parts"][1]
        .as_object_mut()
        .unwrap()
        .remove("name");
    assert_eq!(
        violations(document),
        one(
            &format!("{PARTS}/parts/1"),
            "part-name-missing",
            "part \"cache\" names a node or writes its name"
        )
    );
}

#[test]
fn empty_key_lists_and_repeated_key_ids_are_refused() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][2]["notes"] = json!([]);
    document["links"][0]
        .as_object_mut()
        .unwrap()
        .remove("marks");
    let found = violations(document);
    assert_eq!(
        found,
        one(
            &format!("{NOTES}/notes"),
            "list-empty",
            "general-notes is empty"
        )
    );
    let mut document = marks_sheet_value();
    document["body"][0]["children"][1]["parts"][1]["id"] = json!("api");
    document["body"][0]["children"][0]["children"][1]["marks"] = json!([]);
    assert_eq!(
        violations(document),
        one(
            &format!("{PARTS}/parts/1/id"),
            "key-duplicate",
            &format!("\"api\" is already used by {PARTS}/parts/0")
        )
    );
}

#[test]
fn every_mark_surface_off_a_sheet_needs_a_sheet() {
    let mut document = marks_sheet_value();
    document.as_object_mut().unwrap().remove("sheet");
    let rules: Vec<(String, &'static str)> = violations(document)
        .into_iter()
        .map(|(pointer, rule, _)| (pointer, rule))
        .collect();
    assert_eq!(
        rules,
        [
            ("/body/0", "needs-sheet"),
            ("/body/0/children/0/children/1/marks", "needs-sheet"),
            (PARTS, "needs-sheet"),
            (NOTES, "needs-sheet"),
            ("/links/0/marks", "needs-sheet"),
        ]
        .map(|(pointer, rule)| (pointer.to_string(), rule))
    );
}

#[test]
fn an_unused_part_note_or_attention_item_is_an_unused_key() {
    let mut document = marks_sheet_value();
    document["body"][0]["children"][0]["children"][1]
        .as_object_mut()
        .unwrap()
        .remove("marks");
    document["links"][0]
        .as_object_mut()
        .unwrap()
        .remove("marks");
    document["sheet"]["problem"]["points"][0]
        .as_object_mut()
        .unwrap()
        .remove("marks");
    let page: Page = serde_json::from_value(document).unwrap();
    let report = stencil_model::checks::keys_used(&page);
    let defects: Vec<(&str, &str)> = report
        .defects
        .iter()
        .map(|defect| (defect.pointer.as_str(), defect.message.as_str()))
        .collect();
    assert_eq!(report.examined, 4);
    assert_eq!(
        defects,
        [
            (
                "/sheet/attention/items/0",
                "attention item \"first\" is never marked"
            ),
            (
                "/body/0/children/1/parts/1",
                "part \"cache\" is never ballooned"
            ),
            (
                "/body/0/children/2/notes/0",
                "general note \"retry\" is never flagged"
            ),
        ]
    );
}

#[test]
fn a_step_line_is_a_string_or_a_marked_line_and_round_trips() {
    let mut document = actor_sheet_value();
    document["body"][0]["children"][1]["children"][0]["lines"] = json!(["plain", { "text": "marked", "marks": [{ "attention": "first" }] }, { "text": "bare" }]);
    let page: Page = serde_json::from_value(document.clone()).unwrap();
    assert_eq!(validate_page(&page, &common::gcp()), Vec::new());
    assert_eq!(
        serde_json::to_value(&page).unwrap()["body"][0]["children"][1]["children"][0]["lines"],
        document["body"][0]["children"][1]["children"][0]["lines"]
    );
    let mut wrong = document.clone();
    wrong["body"][0]["children"][1]["children"][0]["lines"][1]["mark"] = json!([]);
    assert!(serde_json::from_value::<Page>(wrong).is_err());
}

/// The smallest sheet with the assembly surfaces: a store with a compartment, a rail in a
/// Row, an assembly Box holding a stack, a planned Item, and links with a step badge, a
/// valve and a planned line.
fn assembly_sheet_value() -> Value {
    let mut document = actor_sheet_value();
    document["sheet"]["marks"] = json!({ "step": "a request step" });
    document["body"][0]["children"][0]["children"] = json!([
        { "tag": "Item", "id": "api", "kind": "product", "title": "API", "actor": "crew",
          "outline": "store", "compartments": ["Jobs", { "text": "Plans" }] },
        { "tag": "Pipe", "id": "queue", "dir": "v", "form": "rail", "line": "solid",
          "actor": "crew", "label": "Queue" },
        { "tag": "Box", "id": "path", "kind": "assembly", "label": "Path", "actor": "ops",
          "children": [{ "tag": "Item", "id": "worker", "kind": "product", "title": "Worker",
                         "outline": "stack", "actor": "ops" }] },
        { "tag": "Item", "id": "later", "kind": "product", "title": "Later", "actor": "crew",
          "planned": true }
    ]);
    document["legend"] = json!([
        { "line": "solid", "by_actor": true, "text": "work moves" },
        { "line": "solid", "by_actor": true, "form": "rail", "text": "the queue" },
        { "line": "solid", "by_actor": true, "planned": true, "text": "not built" }
    ]);
    document["links"] = json!([
        { "from": "api", "to": "queue", "line": "solid", "step": 1, "valve": true },
        { "from": "queue", "to": "worker", "line": "solid", "step": 2, "label": "deliver" },
        { "from": "api", "to": "later", "line": "solid", "planned": true }
    ]);
    document
}

const ROW: &str = "/body/0/children/0/children";

#[test]
fn the_smallest_assembly_sheet_vets_clean() {
    assert_eq!(violations(assembly_sheet_value()), Vec::new());
}

#[test]
fn a_planned_line_that_is_not_solid_is_refused() {
    let mut document = assembly_sheet_value();
    document["links"][2]["line"] = json!("dash");
    document["legend"][2]["line"] = json!("dash");
    let found = violations(document);
    assert!(found.contains(&(
        "/links/2/planned".to_string(),
        "planned-not-solid",
        "a planned line is a solid line drawn hollow; set line solid".to_string()
    )));
    assert!(found.contains(&(
        "/legend/2/planned".to_string(),
        "planned-not-solid",
        "a planned line is a solid line drawn hollow; set line solid".to_string()
    )));
}

#[test]
fn a_built_line_into_planned_work_is_refused() {
    let mut document = assembly_sheet_value();
    document["links"][2]
        .as_object_mut()
        .unwrap()
        .remove("planned");
    document["legend"][2]
        .as_object_mut()
        .unwrap()
        .remove("planned");
    document["legend"].as_array_mut().unwrap().pop();
    assert_eq!(
        violations(document.clone()),
        one(
            "/links/2/to",
            "line-into-planned",
            "\"later\" is planned, so a line to it is planned too; set planned"
        )
    );
    document["links"][2]["line"] = json!("dash");
    document["legend"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "line": "dash", "by_actor": true, "text": "reads" }));
    assert_eq!(
        violations(document),
        one(
            "/links/2/to",
            "line-into-planned",
            "\"later\" is planned, and a dash line cannot be planned; draw it solid and planned, or remove it"
        )
    );
}

#[test]
fn compartments_outside_a_store_or_above_six_are_refused() {
    let mut document = assembly_sheet_value();
    document["body"][0]["children"][0]["children"][0]["outline"] = json!("card");
    assert_eq!(
        violations(document),
        one(
            &format!("{ROW}/0/compartments"),
            "compartments-without-store",
            "compartments are drawn in a store"
        )
    );
    let mut long = assembly_sheet_value();
    long["body"][0]["children"][0]["children"][0]["compartments"] =
        json!(["a", "b", "c", "d", "e", "f", "g"]);
    assert_eq!(
        violations(long),
        one(
            &format!("{ROW}/0/compartments"),
            "list-too-long",
            "7 compartments, above 6"
        )
    );
}

#[test]
fn a_person_outline_with_an_icon_is_refused() {
    let mut document = assembly_sheet_value();
    document["body"][0]["children"][0]["children"][3]["outline"] = json!("person");
    document["body"][0]["children"][0]["children"][3]["icon"] = json!("devops");
    assert_eq!(
        violations(document),
        one(
            &format!("{ROW}/3/icon"),
            "outline-person-with-icon",
            "a person outline draws no icon"
        )
    );
}

#[test]
fn a_rail_stands_vertical_in_a_row_with_no_targets() {
    let mut flat = assembly_sheet_value();
    flat["body"][0]["children"][0]["children"][1]["dir"] = json!("h");
    assert_eq!(
        violations(flat),
        one(
            &format!("{ROW}/1/dir"),
            "rail-not-vertical",
            "a rail stands vertical"
        )
    );
    let mut targeted = assembly_sheet_value();
    targeted["body"][0]["children"][0]["children"][1]["to"] = json!("worker");
    assert_eq!(
        violations(targeted),
        one(
            &format!("{ROW}/1/to"),
            "rail-with-targets",
            "a rail is attached by links, not targets"
        )
    );
    let mut in_col = assembly_sheet_value();
    let rail = in_col["body"][0]["children"][0]["children"][1].clone();
    in_col["body"][0]["children"][0]["children"][1] = json!({ "tag": "Col", "children": [rail] });
    assert_eq!(
        violations(in_col),
        one(
            &format!("{ROW}/1/children/0"),
            "rail-outside-row",
            "a rail is a column of a Row"
        )
    );
}

#[test]
fn a_step_badge_out_of_range_or_used_twice_is_refused() {
    let mut out_of_range = assembly_sheet_value();
    out_of_range["links"][0]["step"] = json!(65);
    assert_eq!(
        violations(out_of_range),
        one(
            "/links/0/step",
            "step-out-of-range",
            "step 65 is outside 1 to 64"
        )
    );
    let mut twice = assembly_sheet_value();
    twice["links"][1]["step"] = json!(1);
    assert_eq!(
        violations(twice),
        one(
            "/links/1/step",
            "link-step-duplicate",
            "step 1 is already used by /links/0"
        )
    );
    let mut pipe_first = assembly_sheet_value();
    pipe_first["body"][0]["children"][0]["children"][1]["step"] = json!(2);
    assert_eq!(
        violations(pipe_first),
        one(
            "/links/1/step",
            "link-step-duplicate",
            &format!("step 2 is already used by {ROW}/1")
        )
    );
}

#[test]
fn a_valve_without_an_arrowhead_is_refused() {
    let mut document = assembly_sheet_value();
    document["links"][0]["arrow"] = json!("none");
    assert_eq!(
        violations(document),
        one(
            "/links/0/valve",
            "valve-without-arrow",
            "a valve sits before an arrowhead"
        )
    );
}

#[test]
fn marks_under_a_valve_or_with_a_step_need_no_label() {
    let mut document = assembly_sheet_value();
    document["sheet"]["attention"]["items"][0]["actor"] = json!("crew");
    document["links"][0]["marks"] = json!([{ "attention": "first" }]);
    assert_eq!(violations(document.clone()), Vec::new());
    document["links"][0]
        .as_object_mut()
        .unwrap()
        .remove("valve");
    assert_eq!(violations(document.clone()), Vec::new());
    document["links"][0].as_object_mut().unwrap().remove("step");
    assert_eq!(
        violations(document),
        one(
            "/links/0/marks",
            "link-marks-without-tag",
            "a link's marks sit in its tag or under its valve; give it a label"
        )
    );
}

#[test]
fn a_step_legend_text_without_a_step_badge_is_refused() {
    let mut document = assembly_sheet_value();
    for link in document["links"].as_array_mut().unwrap() {
        link.as_object_mut().unwrap().remove("step");
    }
    document["links"][0]["label"] = json!("send");
    assert_eq!(
        violations(document),
        one(
            "/sheet/marks/step",
            "mark-legend-unused",
            "no step key on this sheet; remove the text"
        )
    );
}

#[test]
fn every_assembly_surface_off_a_sheet_needs_a_sheet() {
    let mut document = assembly_sheet_value();
    document.as_object_mut().unwrap().remove("sheet");
    let found: Vec<(String, &'static str)> = violations(document)
        .into_iter()
        .filter(|(_, rule, _)| *rule == "needs-sheet")
        .map(|(pointer, rule, _)| (pointer, rule))
        .collect();
    for pointer in [
        format!("{ROW}/0/outline"),
        format!("{ROW}/0/compartments"),
        format!("{ROW}/1/form"),
        format!("{ROW}/2/kind"),
        format!("{ROW}/3/planned"),
        "/links/0/step".to_string(),
        "/links/0/valve".to_string(),
        "/links/2/planned".to_string(),
        "/legend/1/form".to_string(),
        "/legend/2/planned".to_string(),
    ] {
        assert!(
            found.contains(&(pointer.clone(), "needs-sheet")),
            "{pointer} in {found:?}"
        );
    }
}

/// The sheet look is read from the grammar's kind, whatever its name: a kind of another
/// name with a dash-dot border needs a sheet as an assembly does.
#[test]
fn a_dash_dot_kind_of_any_name_needs_a_sheet() {
    let mut grammar: Value =
        serde_json::from_str(stencil_model::grammar::builtin_grammar_json("plain").unwrap())
            .unwrap();
    let containers = grammar["containers"].as_array_mut().unwrap();
    let mut crew = containers
        .iter()
        .find(|kind| kind["name"] == "assembly")
        .unwrap()
        .clone();
    crew["name"] = json!("crew-cell");
    crew["label"] = json!("plain");
    containers.push(crew);
    for item in grammar["items"].as_array_mut().unwrap() {
        item["parents"]
            .as_array_mut()
            .unwrap()
            .push(json!("crew-cell"));
    }
    let grammar =
        stencil_model::grammar::parse_grammar(&grammar.to_string(), "custom.json").unwrap();
    let page: Page = serde_json::from_value(json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "body": [{ "tag": "Box", "kind": "crew-cell", "label": "Cell",
                   "children": [{ "tag": "Item", "kind": "service", "title": "Worker" }] }],
        "legend": []
    }))
    .unwrap();
    let found: Vec<(String, &'static str)> = validate_page(&page, &grammar)
        .into_iter()
        .map(|violation| {
            (
                violation.pointer.as_str().to_string(),
                violation.rule.as_str(),
            )
        })
        .collect();
    assert_eq!(found, [("/body/0/kind".to_string(), "needs-sheet")]);
}
