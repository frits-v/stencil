//! Section 7 stdout line formats, stable for scripts.

use stencil_model::Violation;
use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;

/// The root pointer is empty, so it prints as `""` to stay visible in a line.
pub fn pointer_text(pointer: &NodePointer) -> String {
    if pointer.as_str().is_empty() {
        "\"\"".to_string()
    } else {
        pointer.as_str().to_string()
    }
}

pub fn count_text(count: u64, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

pub fn violation_count_text(count: usize) -> String {
    count_text(count as u64, "violation", "violations")
}

pub fn violation_line(violation: &Violation) -> String {
    format!(
        "violation {} {}: {}",
        violation.rule.as_str(),
        pointer_text(&violation.pointer),
        violation.message
    )
}

pub fn check_line(report: &CheckReport) -> String {
    let name = report.check.as_str();
    let examined = report.examined;
    let unit = report.check.unit(examined);
    if examined == 0 {
        format!("check {name}: examined {examined} {unit}, FAILED: nothing examined")
    } else {
        let defects = count_text(report.defects.len() as u64, "defect", "defects");
        format!("check {name}: examined {examined} {unit}, {defects}")
    }
}

pub fn defect_line(check: CheckName, defect: &Defect) -> String {
    format!(
        "defect {} {}: {}",
        check.as_str(),
        pointer_text(&defect.pointer),
        defect.message
    )
}

/// Each check line followed directly by its defect lines, in report order.
pub fn report_lines(reports: &[CheckReport]) -> Vec<String> {
    let mut lines = Vec::new();
    for report in reports {
        lines.push(check_line(report));
        for defect in &report.defects {
            lines.push(defect_line(report.check, defect));
        }
    }
    lines
}

/// `<n> checks, <p> passed, <f> failed`.
pub fn check_counts_text(reports: &[CheckReport]) -> String {
    let passed = reports.iter().filter(|report| report.passed()).count();
    let failed = reports.len() - passed;
    format!(
        "{}, {passed} passed, {failed} failed",
        count_text(reports.len() as u64, "check", "checks")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use stencil_model::VetRule;

    fn report(check: CheckName, examined: u64, defect_count: usize) -> CheckReport {
        let defects = (0..defect_count)
            .map(|index| Defect {
                pointer: NodePointer::root().child("body").index(index),
                message: format!("defect number {index}"),
            })
            .collect();
        CheckReport {
            check,
            examined,
            defects,
        }
    }

    #[test]
    fn the_root_pointer_prints_as_two_quotes() {
        assert_eq!(pointer_text(&NodePointer::root()), "\"\"");
        assert_eq!(
            pointer_text(&NodePointer::root().child("body").index(0)),
            "/body/0"
        );
    }

    #[test]
    fn violation_lines_carry_the_kebab_case_rule() {
        let violation = Violation {
            pointer: NodePointer::root().child("body").index(0).child("gap"),
            rule: VetRule::GapOutOfRange,
            message: "gap 65 is above 64".to_string(),
        };
        assert_eq!(
            violation_line(&violation),
            "violation gap-out-of-range /body/0/gap: gap 65 is above 64"
        );
        let at_root = Violation {
            pointer: NodePointer::root(),
            rule: VetRule::TextEmpty,
            message: "text is empty".to_string(),
        };
        assert_eq!(
            violation_line(&at_root),
            "violation text-empty \"\": text is empty"
        );
    }

    #[test]
    fn violation_counts_are_singular_only_for_one() {
        assert_eq!(violation_count_text(0), "0 violations");
        assert_eq!(violation_count_text(1), "1 violation");
        assert_eq!(violation_count_text(2), "2 violations");
    }

    #[test]
    fn check_lines_follow_the_section_seven_format() {
        assert_eq!(
            check_line(&report(CheckName::ChildInsideContainer, 33, 0)),
            "check child-inside-container: examined 33 relations, 0 defects"
        );
        assert_eq!(
            check_line(&report(CheckName::TextFitsBox, 40, 1)),
            "check text-fits-box: examined 40 text runs, 1 defect"
        );
        assert_eq!(
            check_line(&report(CheckName::SiblingsDoNotOverlap, 1, 2)),
            "check siblings-do-not-overlap: examined 1 pair, 2 defects"
        );
    }

    #[test]
    fn a_check_that_examined_nothing_prints_failed() {
        assert_eq!(
            check_line(&report(CheckName::LegendConsistency, 0, 0)),
            "check legend-consistency: examined 0 relations, FAILED: nothing examined"
        );
    }

    #[test]
    fn defect_lines_follow_their_check_line() {
        let reports = [
            report(CheckName::TextFitsBox, 4, 2),
            report(CheckName::RememberedConstants, 3, 0),
        ];
        assert_eq!(
            report_lines(&reports),
            vec![
                "check text-fits-box: examined 4 text runs, 2 defects".to_string(),
                "defect text-fits-box /body/0: defect number 0".to_string(),
                "defect text-fits-box /body/1: defect number 1".to_string(),
                "check remembered-constants: examined 3 text fields, 0 defects".to_string(),
            ]
        );
    }

    #[test]
    fn check_counts_count_zero_examined_as_failed() {
        let reports = [
            report(CheckName::RememberedConstants, 36, 0),
            report(CheckName::LegendConsistency, 0, 0),
        ];
        assert_eq!(check_counts_text(&reports), "2 checks, 1 passed, 1 failed");
    }
}
