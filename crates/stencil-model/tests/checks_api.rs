// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use stencil_model::NodePointer;
use stencil_model::checks::{CheckName, CheckReport, Defect};

const ALL_CHECKS: [(CheckName, &str, &str, &str); 5] = [
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
    };
    assert!(!report.passed());
}

#[test]
fn examined_without_defects_passes() {
    let report = CheckReport {
        check: CheckName::TextFitsBox,
        examined: 1,
        defects: vec![],
    };
    assert!(report.passed());
}
