//! The review's verdict, a rule over the verified findings alone. Check results never enter
//! it: they are reported beside the verdict, and the branch rules enforce the required ones.

use super::findings::{Finding, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Approve,
    RequestChanges,
    Comment,
}

impl Event {
    /// The `event` the pull request reviews API takes.
    pub const fn api_name(self) -> &'static str {
        match self {
            Self::Approve => "APPROVE",
            Self::RequestChanges => "REQUEST_CHANGES",
            Self::Comment => "COMMENT",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::RequestChanges => "changes requested",
            Self::Comment => "comment only",
        }
    }
}

/// What the verdict is decided from.
#[derive(Debug, Default)]
pub struct Evidence<'a> {
    /// Every finding a verify pass confirmed, posted before, withheld or new.
    pub confirmed: &'a [Finding],
    /// High or medium candidates no verify pass judged: over the cap, or a pass that failed.
    pub unjudged_blocking: usize,
}

pub const fn blocks(finding: &Finding) -> bool {
    matches!(finding.severity, Severity::High | Severity::Medium)
}

/// A confirmed high or medium finding requests changes. Without one, a blocking candidate
/// nobody judged gets a comment: the App approves only what it checked.
pub fn decide(evidence: &Evidence<'_>) -> (Event, String) {
    let blocking = evidence
        .confirmed
        .iter()
        .filter(|finding| blocks(finding))
        .count();
    if blocking > 0 {
        return (
            Event::RequestChanges,
            format!("{blocking} confirmed high or medium findings"),
        );
    }
    if evidence.unjudged_blocking > 0 {
        return (
            Event::Comment,
            format!(
                "{} high or medium candidates were not verified",
                evidence.unjudged_blocking
            ),
        );
    }
    (
        Event::Approve,
        "no confirmed high or medium findings".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::findings::Category;

    fn finding(severity: Severity) -> Finding {
        Finding {
            path: "a.rs".to_string(),
            line: 1,
            severity,
            category: Category::Correctness,
            summary: "s".to_string(),
            failure_scenario: "f".to_string(),
        }
    }

    fn decided(confirmed: &[Finding], unjudged_blocking: usize) -> Event {
        decide(&Evidence {
            confirmed,
            unjudged_blocking,
        })
        .0
    }

    #[test]
    fn a_confirmed_high_or_medium_finding_requests_changes_whatever_else_holds() {
        for severity in [Severity::High, Severity::Medium] {
            let confirmed = [finding(Severity::Low), finding(severity)];
            assert_eq!(decided(&confirmed, 0), Event::RequestChanges);
            assert_eq!(decided(&confirmed, 3), Event::RequestChanges);
        }
    }

    #[test]
    fn nothing_blocking_approves_and_low_findings_do_not_block() {
        assert_eq!(decided(&[], 0), Event::Approve);
        assert_eq!(decided(&[finding(Severity::Low)], 0), Event::Approve);
    }

    #[test]
    fn an_unjudged_blocking_candidate_never_approves() {
        assert_eq!(decided(&[], 1), Event::Comment);
        assert_eq!(decided(&[finding(Severity::Low)], 1), Event::Comment);
        let (_, reason) = decide(&Evidence {
            confirmed: &[],
            unjudged_blocking: 2,
        });
        assert!(reason.contains("2 high or medium candidates"), "{reason}");
    }

    #[test]
    fn each_event_has_its_api_name() {
        assert_eq!(Event::Approve.api_name(), "APPROVE");
        assert_eq!(Event::RequestChanges.api_name(), "REQUEST_CHANGES");
        assert_eq!(Event::Comment.api_name(), "COMMENT");
    }
}
