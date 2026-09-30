// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{g7_page, legend_entry, page_with_body, pcard, pipe, pipe_value, tee};
use stencil_model::PipeDir;
use stencil_model::checks::{CheckName, CheckReport, legend_consistency};
use stencil_model::{LEGEND_ENTRIES_MAX, PipeKind};

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
        pipe(PipeKind::Pink, "a"),
        pcard("b"),
        pipe(PipeKind::Pink, "c"),
    ]);
    page.legend = vec![];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 2);
    assert_eq!(
        defects(&report),
        vec![
            (
                "/body/0".to_string(),
                "Pipe kind pink has no legend entry".to_string()
            ),
            (
                "/body/2".to_string(),
                "Pipe kind pink has no legend entry".to_string()
            ),
        ]
    );
}

#[test]
fn legend_kind_never_used() {
    let mut page = page_with_body(vec![pipe(PipeKind::Blue, "a")]);
    page.legend = vec![
        legend_entry(PipeKind::Blue, "a"),
        legend_entry(PipeKind::Gray, "b"),
    ];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 3);
    assert_eq!(
        defects(&report),
        vec![(
            "/legend/1".to_string(),
            "legend kind gray is never used".to_string()
        )]
    );
}

#[test]
fn duplicate_legend_kind_gives_one_defect_at_the_later_entry() {
    let mut page = page_with_body(vec![pipe(PipeKind::Blue, "a")]);
    page.legend = vec![
        legend_entry(PipeKind::Blue, "a"),
        legend_entry(PipeKind::Blue, "b"),
    ];
    let report = legend_consistency(&page);
    assert_eq!(
        defects(&report),
        vec![(
            "/legend/1".to_string(),
            "legend kind blue is already listed at /legend/0".to_string()
        )]
    );
}

#[test]
fn tee_spine_and_arm_kinds_are_counted() {
    let mut page = page_with_body(vec![tee(
        PipeKind::Deny,
        pipe_value(PipeDir::Horizontal, PipeKind::Blue, "allowed"),
        pipe_value(PipeDir::Horizontal, PipeKind::Pink, "reply"),
    )]);
    page.legend = vec![legend_entry(PipeKind::Blue, "request")];
    let report = legend_consistency(&page);
    // Spine, two arms and one legend entry.
    assert_eq!(report.examined, 4);
    assert_eq!(
        defects(&report),
        vec![
            (
                "/body/0".to_string(),
                "Tee spine kind deny has no legend entry".to_string()
            ),
            (
                "/body/0/arms/1".to_string(),
                "Tee arm kind pink has no legend entry".to_string()
            ),
        ]
    );

    page.legend = vec![
        legend_entry(PipeKind::Blue, "request"),
        legend_entry(PipeKind::Pink, "reply"),
        legend_entry(PipeKind::Deny, "blocked"),
    ];
    assert!(legend_consistency(&page).passed());
}

#[test]
fn page_with_no_pipes_and_no_legend_examines_nothing_and_fails() {
    let mut page = page_with_body(vec![pcard("a")]);
    page.legend = vec![];
    let report = legend_consistency(&page);
    assert_eq!(report.examined, 0);
    assert_eq!(report.defects, vec![]);
    assert!(!report.passed());
}

#[test]
fn a_legend_past_the_vet_limit_is_examined_up_to_one_past_it() {
    let mut page = page_with_body(vec![pipe(PipeKind::Blue, "a")]);
    page.legend = vec![legend_entry(PipeKind::Blue, "request"); LEGEND_ENTRIES_MAX + 4];
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
