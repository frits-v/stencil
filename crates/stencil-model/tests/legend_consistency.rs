// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{g7_page, item, legend_entry, page_with_body, pipe, pipe_value, tee};
use stencil_model::checks::{CheckName, CheckReport, legend_consistency};
use stencil_model::{Chrome, LEGEND_ENTRIES_MAX, Line};
use stencil_model::{Node, PipeDir, PipeForm};

fn defects(report: &CheckReport) -> Vec<(String, String)> {
    report
        .defects
        .iter()
        .map(|defect| (defect.pointer.as_str().to_string(), defect.message.clone()))
        .collect()
}

#[test]
fn g7_examines_8_relations_and_passes() {
    let report = legend_consistency(&g7_page());
    assert_eq!(report.check, CheckName::LegendConsistency);
    assert_eq!(report.examined, 8);
    assert!(report.passed());
}

#[test]
fn used_kind_missing_from_legend_gives_one_defect_per_use() {
    let mut page = page_with_body(vec![
        pipe(Line::Solid, Some(2), "a"),
        item("b"),
        pipe(Line::Solid, Some(2), "c"),
    ]);
    page.legend = vec![];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 2);
    assert_eq!(
        defects(&report),
        vec![
            (
                "/body/0".to_string(),
                "Pipe line solid tint 2 has no legend entry".to_string()
            ),
            (
                "/body/2".to_string(),
                "Pipe line solid tint 2 has no legend entry".to_string()
            ),
        ]
    );
}

#[test]
fn legend_kind_never_used() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(1), "a")]);
    page.legend = vec![
        legend_entry(Line::Solid, Some(1), "a"),
        legend_entry(Line::Gray, None, "b"),
    ];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 3);
    assert_eq!(
        defects(&report),
        vec![(
            "/legend/1".to_string(),
            "legend line gray is never used".to_string()
        )]
    );
}

#[test]
fn duplicate_legend_kind_gives_one_defect_at_the_later_entry() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(1), "a")]);
    page.legend = vec![
        legend_entry(Line::Solid, Some(1), "a"),
        legend_entry(Line::Solid, Some(1), "b"),
    ];
    let report = legend_consistency(&page);
    assert_eq!(
        defects(&report),
        vec![(
            "/legend/1".to_string(),
            "legend line solid tint 1 is already listed at /legend/0".to_string()
        )]
    );
}

#[test]
fn tee_spine_and_arm_kinds_are_counted() {
    let mut page = page_with_body(vec![tee(
        Line::Deny,
        None,
        pipe_value(PipeDir::Horizontal, Line::Solid, Some(1), "allowed"),
        pipe_value(PipeDir::Horizontal, Line::Solid, Some(2), "reply"),
    )]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), "request")];
    let report = legend_consistency(&page);
    // Spine, two arms and one legend entry.
    assert_eq!(report.examined, 4);
    assert_eq!(
        defects(&report),
        vec![
            (
                "/body/0".to_string(),
                "Tee spine line deny has no legend entry".to_string()
            ),
            (
                "/body/0/arms/1".to_string(),
                "Tee arm line solid tint 2 has no legend entry".to_string()
            ),
        ]
    );

    page.legend = vec![
        legend_entry(Line::Solid, Some(1), "request"),
        legend_entry(Line::Solid, Some(2), "reply"),
        legend_entry(Line::Deny, None, "blocked"),
    ];
    assert!(legend_consistency(&page).passed());
}

#[test]
fn page_with_no_pipes_and_no_legend_examines_nothing_and_fails() {
    let mut page = page_with_body(vec![item("a")]);
    page.legend = vec![];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 0);
    assert_eq!(report.defects, vec![]);
    assert!(!report.passed());
}

#[test]
fn a_legend_past_the_vet_limit_is_examined_up_to_one_past_it() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(1), "a")]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), "request"); LEGEND_ENTRIES_MAX + 4];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 1 + 17);
    let pointers: Vec<String> = defects(&report)
        .into_iter()
        .map(|(pointer, _)| pointer)
        .collect();
    let expected: Vec<String> = (1..=LEGEND_ENTRIES_MAX)
        .map(|index| format!("/legend/{index}"))
        .collect();
    assert_eq!(pointers, expected);
}

#[test]
fn solid_tint_2_used_with_only_a_tint_1_entry_gives_a_defect_at_each_side() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(2), "a")]);
    page.legend = vec![legend_entry(Line::Solid, Some(1), "request")];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 2);
    assert_eq!(
        defects(&report),
        vec![
            (
                "/body/0".to_string(),
                "Pipe line solid tint 2 has no legend entry".to_string()
            ),
            (
                "/legend/0".to_string(),
                "legend line solid tint 1 is never used".to_string()
            ),
        ]
    );
}

#[test]
fn a_tint_on_a_deny_line_does_not_change_its_key() {
    let mut page = page_with_body(vec![pipe(Line::Deny, Some(3), "blocked")]);
    page.legend = vec![legend_entry(Line::Deny, Some(5), "blocked")];
    assert!(legend_consistency(&page).passed());
    page.legend = vec![legend_entry(Line::Deny, None, "blocked")];
    assert!(legend_consistency(&page).passed());
}

#[test]
fn a_solid_entry_without_tint_matches_a_tint_1_pipe() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(1), "a")]);
    page.legend = vec![legend_entry(Line::Solid, None, "request")];
    assert!(legend_consistency(&page).passed());
}

#[test]
fn a_dash_tint_2_needs_its_own_entry() {
    let mut page = page_with_body(vec![
        pipe(Line::Dash, None, "a"),
        pipe(Line::Dash, Some(2), "b"),
    ]);
    page.legend = vec![legend_entry(Line::Dash, None, "failover")];
    let report = legend_consistency(&page);
    assert_eq!(
        defects(&report),
        vec![(
            "/body/1".to_string(),
            "Pipe line dash tint 2 has no legend entry".to_string()
        )]
    );
    page.legend
        .push(legend_entry(Line::Dash, Some(2), "second failover"));
    assert!(legend_consistency(&page).passed());
}

#[test]
fn a_band_pipe_needs_an_entry_of_its_form() {
    let mut band = pipe_value(PipeDir::Horizontal, Line::Solid, Some(4), "a");
    band.form = PipeForm::Band;
    let mut page = page_with_body(vec![Node::Pipe(band), item("b")]);
    page.legend = vec![legend_entry(Line::Solid, Some(4), "tube entry")];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 2);
    assert_eq!(
        defects(&report),
        vec![
            (
                "/body/0".to_string(),
                "Pipe line solid tint 4 form band has no legend entry".to_string()
            ),
            (
                "/legend/0".to_string(),
                "legend line solid tint 4 is never used".to_string()
            ),
        ]
    );
    page.legend[0].form = PipeForm::Band;
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 2);
    assert!(report.passed());
}

#[test]
fn a_band_and_a_tube_of_one_line_take_two_entries() {
    let mut band = pipe_value(PipeDir::Horizontal, Line::Solid, Some(1), "a");
    band.form = PipeForm::Band;
    let mut page = page_with_body(vec![
        Node::Pipe(band),
        item("b"),
        pipe(Line::Solid, Some(1), "c"),
    ]);
    let mut band_entry = legend_entry(Line::Solid, Some(1), "band entry");
    band_entry.form = PipeForm::Band;
    page.legend = vec![legend_entry(Line::Solid, Some(1), "tube entry"), band_entry];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 4);
    assert!(report.passed());
}

#[test]
fn chrome_none_with_one_key_and_no_legend_passes_with_the_uses_examined() {
    let mut page = page_with_body(vec![
        pipe(Line::Solid, Some(1), "a"),
        pipe(Line::Solid, None, "b"),
    ]);
    page.chrome = Chrome::None;
    page.legend = vec![];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 2);
    assert!(report.passed(), "{:?}", report.defects);
}

#[test]
fn chrome_none_with_two_keys_and_no_legend_fails() {
    let mut page = page_with_body(vec![
        pipe(Line::Solid, Some(1), "a"),
        pipe(Line::Gray, None, "b"),
    ]);
    page.chrome = Chrome::None;
    page.legend = vec![];
    let report = legend_consistency(&page);
    assert_eq!(report.defects.len(), 2);
}

#[test]
fn chrome_none_with_a_legend_entry_follows_the_normal_rule() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(1), "a")]);
    page.chrome = Chrome::None;
    page.legend = vec![legend_entry(Line::Gray, None, "unused")];
    let report = legend_consistency(&page);
    assert_eq!(
        defects(&report),
        vec![
            (
                "/body/0".to_string(),
                "Pipe line solid tint 1 has no legend entry".to_string()
            ),
            (
                "/legend/0".to_string(),
                "legend line gray is never used".to_string()
            ),
        ]
    );
}

#[test]
fn chrome_full_with_one_key_and_no_legend_fails() {
    let mut page = page_with_body(vec![pipe(Line::Solid, Some(1), "a")]);
    page.legend = vec![];
    assert!(!legend_consistency(&page).passed());
}
