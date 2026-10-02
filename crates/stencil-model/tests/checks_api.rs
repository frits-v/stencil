// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use stencil_model::NodePointer;
use stencil_model::checks::{CheckName, CheckOutcome, CheckReport, Defect};

const ALL_CHECKS: [(CheckName, &str, &str, &str); 14] = [
    (
        CheckName::ChildInsideContainer,
        "child-inside-container",
        "relation",
        "relations",
    ),
    (
        CheckName::SiblingsDoNotOverlap,
        "siblings-do-not-overlap",
        "pair",
        "pairs",
    ),
    (
        CheckName::TextFitsBox,
        "text-fits-box",
        "text run",
        "text runs",
    ),
    (
        CheckName::RememberedConstants,
        "remembered-constants",
        "text field",
        "text fields",
    ),
    (
        CheckName::LegendConsistency,
        "legend-consistency",
        "relation",
        "relations",
    ),
    (CheckName::LinksRouted, "links-routed", "link", "links"),
    (
        CheckName::LinksAvoidBoxes,
        "links-avoid-boxes",
        "pair",
        "pairs",
    ),
    (CheckName::PipesLand, "pipes-land", "pipe end", "pipe ends"),
    (
        CheckName::IsoLabelsClear,
        "iso-labels-clear",
        "pair",
        "pairs",
    ),
    (
        CheckName::IsoLinksClear,
        "iso-links-clear",
        "link leg",
        "link legs",
    ),
    (
        CheckName::IsoLinkEnds,
        "iso-link-ends",
        "link end",
        "link ends",
    ),
    (
        CheckName::IsoLinksApart,
        "iso-links-apart",
        "link pair",
        "link pairs",
    ),
    (CheckName::PrintFit, "print-fit", "text run", "text runs"),
    (
        CheckName::IconMatchesProduct,
        "icon-matches-product",
        "item",
        "items",
    ),
];

#[test]
fn check_names_and_units() {
    for (check, name, singular, plural) in ALL_CHECKS {
        assert_eq!(check.as_str(), name);
        assert_eq!(check.unit(1), singular, "{name}");
        assert_eq!(check.unit(0), plural, "{name}");
        assert_eq!(check.unit(2), plural, "{name}");
    }
}

#[test]
fn nothing_examined_fails() {
    let report = CheckReport {
        check: CheckName::TextFitsBox,
        examined: 0,
        defects: vec![],
        not_applicable: None,
    };
    assert!(!report.passed());
}

#[test]
fn a_defect_fails() {
    let report = CheckReport {
        check: CheckName::TextFitsBox,
        examined: 1,
        defects: vec![Defect {
            pointer: NodePointer::root().child("title"),
            message: "too wide".to_string(),
        }],
        not_applicable: None,
    };
    assert!(!report.passed());
}

#[test]
fn examined_without_defects_passes() {
    let report = CheckReport {
        check: CheckName::TextFitsBox,
        examined: 1,
        defects: vec![],
        not_applicable: None,
    };
    assert!(report.passed());
    assert_eq!(report.outcome(), CheckOutcome::Passed);
}

#[test]
fn a_report_without_its_surface_is_not_applicable_and_not_passed() {
    let report = CheckReport::not_applicable(CheckName::LinksRouted, "page has no links");
    assert_eq!(report.examined, 0);
    assert_eq!(report.outcome(), CheckOutcome::NotApplicable);
    assert!(!report.passed());
}

#[test]
fn a_not_applicable_reason_on_an_examined_or_defective_report_fails() {
    let mut examined = CheckReport::not_applicable(CheckName::LinksRouted, "page has no links");
    examined.examined = 1;
    assert_eq!(examined.outcome(), CheckOutcome::Failed);
    let mut defective = CheckReport::not_applicable(CheckName::LinksRouted, "page has no links");
    defective.defects.push(Defect {
        pointer: NodePointer::root().child("links"),
        message: "not routed".to_string(),
    });
    assert_eq!(defective.outcome(), CheckOutcome::Failed);
}

#[test]
fn nothing_examined_without_a_reason_is_failed() {
    let report = CheckReport {
        check: CheckName::LinksAvoidBoxes,
        examined: 0,
        defects: vec![],
        not_applicable: None,
    };
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}
