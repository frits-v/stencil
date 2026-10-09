//! Readers of what opengrep and jactionlint write. A reader that cannot find a field it
//! needs returns an error naming it, so a change in a tool's output fails the check.

use serde_json::{Map, Value};

#[derive(Debug, PartialEq, Eq)]
pub struct FixtureCheck {
    pub rule: String,
    pub passed: bool,
    pub expected: usize,
    pub reported: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScanSummary {
    pub files: usize,
    pub findings: Vec<String>,
}

/// The rule ids of an opengrep rule file, sorted.
pub fn rule_ids(rules: &str) -> Vec<String> {
    let mut ids: Vec<String> = rules
        .lines()
        .filter_map(|line| line.strip_prefix("  - id: "))
        .map(|id| id.trim().to_string())
        .collect();
    ids.sort();
    ids
}

fn field<'a>(value: &'a Value, name: &str, context: &str) -> Result<&'a Value, String> {
    value
        .get(name)
        .ok_or_else(|| format!("{context}: no `{name}` field"))
}

fn array<'a>(value: &'a Value, name: &str, context: &str) -> Result<&'a Vec<Value>, String> {
    field(value, name, context)?
        .as_array()
        .ok_or_else(|| format!("{context}: `{name}` is not an array"))
}

fn object<'a>(
    value: &'a Value,
    name: &str,
    context: &str,
) -> Result<&'a Map<String, Value>, String> {
    field(value, name, context)?
        .as_object()
        .ok_or_else(|| format!("{context}: `{name}` is not an object"))
}

fn text<'a>(value: &'a Value, name: &str, context: &str) -> Result<&'a str, String> {
    field(value, name, context)?
        .as_str()
        .ok_or_else(|| format!("{context}: `{name}` is not a string"))
}

/// One entry per rule from `opengrep test --json`, sorted by rule id.
pub fn fixture_checks(test: &Value) -> Result<Vec<FixtureCheck>, String> {
    let context = "opengrep test";
    for name in ["config_with_errors", "config_missing_tests"] {
        let entries = array(test, name, context)?;
        if !entries.is_empty() {
            return Err(format!(
                "{context}: `{name}` lists {}",
                Value::Array(entries.clone())
            ));
        }
    }
    let mut checks = Vec::new();
    for config in object(test, "results", context)?.values() {
        for (rule, check) in object(config, "checks", context)? {
            let passed = field(check, "passed", context)?
                .as_bool()
                .ok_or_else(|| format!("{context}: `passed` of {rule} is not a boolean"))?;
            let mut expected = 0;
            let mut reported = 0;
            for file in object(check, "matches", context)?.values() {
                expected += array(file, "expected_lines", context)?.len();
                reported += array(file, "reported_lines", context)?.len();
            }
            checks.push(FixtureCheck {
                rule: rule.clone(),
                passed,
                expected,
                reported,
            });
        }
    }
    checks.sort_by(|left, right| left.rule.cmp(&right.rule));
    Ok(checks)
}

/// Every rule was tested, passed, and has at least one fixture line that must fire.
pub fn fixtures_cover(rule_ids: &[String], checks: &[FixtureCheck]) -> Result<(), String> {
    let tested: Vec<&str> = checks.iter().map(|check| check.rule.as_str()).collect();
    if tested != rule_ids {
        return Err(format!("rules {rule_ids:?}, tested {tested:?}"));
    }
    for check in checks {
        if !check.passed || check.expected == 0 {
            return Err(format!(
                "{} expected {} fixture findings, reported {}",
                check.rule, check.expected, check.reported
            ));
        }
    }
    Ok(())
}

/// Scanned file count and one `path:line: rule: message` line per finding from
/// `opengrep scan --json`.
pub fn scan_summary(scan: &Value) -> Result<ScanSummary, String> {
    let context = "opengrep scan";
    for name in ["errors", "skipped_rules"] {
        let entries = array(scan, name, context)?;
        if !entries.is_empty() {
            return Err(format!(
                "{context}: `{name}` lists {}",
                Value::Array(entries.clone())
            ));
        }
    }
    let files = array(field(scan, "paths", context)?, "scanned", context)?.len();
    let mut findings = Vec::new();
    for result in array(scan, "results", context)? {
        let check_id = text(result, "check_id", context)?;
        let rule = check_id.rsplit('.').next().unwrap_or(check_id);
        let path = text(result, "path", context)?;
        let line = field(field(result, "start", context)?, "line", context)?
            .as_u64()
            .ok_or_else(|| format!("{context}: `line` of a {rule} finding is not a number"))?;
        let message = text(field(result, "extra", context)?, "message", context)?;
        findings.push(format!("{path}:{line}: {rule}: {message}"));
    }
    Ok(ScanSummary { files, findings })
}

/// The number of files `jactionlint --verbose` examined, read from its stderr. jactionlint
/// drops its shellcheck rule when it cannot run shellcheck and still passes, so that is an
/// error here.
pub fn jactionlint_files(log: &str) -> Result<usize, String> {
    if log
        .lines()
        .any(|line| line.starts_with("verbose: Rule \"shellcheck\" was disabled"))
    {
        return Err("jactionlint could not run shellcheck over the run: scripts".to_string());
    }
    for line in log.lines() {
        let Some(rest) = line.strip_prefix("verbose: Found ") else {
            continue;
        };
        let words: Vec<&str> = rest.split_whitespace().collect();
        if let [_, "error" | "errors", "in", files, "file" | "files"] = words.as_slice() {
            return files
                .parse()
                .map_err(|error| format!("jactionlint: file count `{files}`: {error}"));
        }
    }
    Err("jactionlint printed no `Found N errors in M files` summary".to_string())
}

/// The number of runs in a SARIF 2.1.0 log. A log without a run gives code scanning
/// nothing to record for its tool, so it is an error.
pub fn sarif_runs(log: &Value, tool: &str) -> Result<usize, String> {
    let context = format!("{tool} SARIF");
    let version = text(log, "version", &context)?;
    if version != "2.1.0" {
        return Err(format!("{context}: version {version}, expected 2.1.0"));
    }
    let runs = array(log, "runs", &context)?.len();
    if runs == 0 {
        return Err(format!("{context}: no runs"));
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn passing_test_report() -> Value {
        json!({
            "config_missing_fixtests": [],
            "config_missing_tests": [],
            "config_with_errors": [],
            "fixtest_results": {},
            "results": {
                "opengrep/rust.yml": {
                    "checks": {
                        "rust-todo-without-ticket": {
                            "passed": true,
                            "matches": {"opengrep/rust.rs": {"expected_lines": [3, 5], "reported_lines": [3, 5]}},
                            "errors": []
                        },
                        "rust-history-trace-comment": {
                            "passed": true,
                            "matches": {"opengrep/rust.rs": {"expected_lines": [16, 18, 20], "reported_lines": [16, 18, 20]}},
                            "errors": []
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn rule_ids_are_read_sorted_from_the_rule_file() {
        let rules = "rules:\n  - id: zeta\n    message: x\n  - id: alpha\n";
        assert_eq!(rule_ids(rules), vec!["alpha", "zeta"]);
    }

    #[test]
    fn a_passing_test_report_covers_both_rules() {
        let checks = fixture_checks(&passing_test_report()).unwrap();
        assert_eq!(checks.len(), 2);
        let ids = vec![
            "rust-history-trace-comment".to_string(),
            "rust-todo-without-ticket".to_string(),
        ];
        assert_eq!(fixtures_cover(&ids, &checks), Ok(()));
    }

    #[test]
    fn a_rule_without_fixture_lines_or_without_a_test_fails() {
        let mut report = passing_test_report();
        report["results"]["opengrep/rust.yml"]["checks"]["rust-todo-without-ticket"]["matches"] =
            json!({});
        let checks = fixture_checks(&report).unwrap();
        let ids = vec![
            "rust-history-trace-comment".to_string(),
            "rust-todo-without-ticket".to_string(),
        ];
        assert!(fixtures_cover(&ids, &checks).is_err());
        let more = vec!["another-rule".to_string()];
        assert!(fixtures_cover(&more, &checks).is_err());
    }

    #[test]
    fn a_test_report_missing_a_field_or_listing_a_broken_config_fails() {
        let mut report = passing_test_report();
        report
            .as_object_mut()
            .unwrap()
            .remove("config_missing_tests");
        assert!(fixture_checks(&report).is_err());
        let mut broken = passing_test_report();
        broken["config_with_errors"] = json!(["opengrep/rust.yml"]);
        assert!(fixture_checks(&broken).is_err());
    }

    #[test]
    fn a_scan_report_lists_each_finding_by_path_line_and_rule() {
        let scan = json!({
            "errors": [],
            "skipped_rules": [],
            "paths": {"scanned": ["a.rs", "b.rs"]},
            "results": [{
                "check_id": "opengrep.rust-todo-without-ticket",
                "path": "a.rs",
                "start": {"line": 7, "col": 1},
                "extra": {"message": "TODO without a ticket reference."}
            }]
        });
        let summary = scan_summary(&scan).unwrap();
        assert_eq!(summary.files, 2);
        assert_eq!(
            summary.findings,
            vec!["a.rs:7: rust-todo-without-ticket: TODO without a ticket reference."]
        );
    }

    #[test]
    fn a_scan_report_with_errors_fails() {
        let scan = json!({
            "errors": [{"message": "parse error"}],
            "skipped_rules": [],
            "paths": {"scanned": []},
            "results": []
        });
        assert!(scan_summary(&scan).is_err());
    }

    #[test]
    fn the_jactionlint_summary_gives_the_file_count() {
        let log = "verbose: Found total 0 errors in 67 ms for .github/workflows/ci.yml\n\
                   verbose: Found 0 errors in 3 files\n";
        assert_eq!(jactionlint_files(log), Ok(3));
        assert_eq!(
            jactionlint_files("verbose: Found 1 error in 1 file\n"),
            Ok(1)
        );
    }

    #[test]
    fn a_jactionlint_run_without_shellcheck_or_without_a_summary_fails() {
        let log = "verbose: Rule \"shellcheck\" was disabled: exec: \"shellcheck\": executable file not found in $PATH\n\
                   verbose: Found 0 errors in 3 files\n";
        assert!(jactionlint_files(log).is_err());
        assert!(jactionlint_files("verbose: Linting 3 files\n").is_err());
    }

    #[test]
    fn a_sarif_log_counts_its_runs_and_rejects_none_or_another_version() {
        let log = json!({"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "zizmor"}}, "results": []}]});
        assert_eq!(sarif_runs(&log, "zizmor"), Ok(1));
        assert!(sarif_runs(&json!({"version": "2.1.0", "runs": []}), "zizmor").is_err());
        assert!(sarif_runs(&json!({"version": "2.0.0", "runs": [{}]}), "zizmor").is_err());
        assert!(sarif_runs(&json!({"runs": [{}]}), "zizmor").is_err());
    }
}
