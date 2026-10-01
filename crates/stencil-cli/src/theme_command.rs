//! `stencil theme show` and `stencil theme check` (section 13.12).

use std::io::{self, Write};
use std::path::Path;

use stencil_model::theme_quality;
use stencil_render::builtin_theme_json;

use crate::ExitCode;
use crate::pipeline::{check_legend_labels, resolve_theme};
use crate::report::{quality_row_line, quality_summary_text};
use crate::report_failure;

/// Prints a built-in theme's JSON byte for byte; an unknown name writes one line naming
/// the built-ins to stderr and exits 2.
pub fn show(name: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> io::Result<ExitCode> {
    match builtin_theme_json(name) {
        Some(json_text) => {
            write!(stdout, "{json_text}")?;
            Ok(ExitCode::Clean)
        }
        None => {
            writeln!(
                stderr,
                "stencil theme show: unknown theme {name:?}; themes: {}",
                stencil_model::BUILTIN_THEMES.join(", ")
            )?;
            Ok(ExitCode::CouldNotRun)
        }
    }
}

/// Loads the theme (a structural failure exits 1, an unreadable file 2), then prints one
/// line per quality row and a summary. Exits 0 when every applicable row passed.
pub fn check(
    reference: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    let loaded = resolve_theme(reference, Path::new(".")).and_then(|theme| {
        check_legend_labels(&theme, reference)?;
        Ok(theme)
    });
    let theme = match loaded {
        Ok(theme) => theme,
        Err(failure) => return report_failure("theme check", &failure, stdout, stderr),
    };
    let report = theme_quality(&theme);
    for row in &report.rows {
        writeln!(stdout, "{}", quality_row_line(row))?;
    }
    if report.rows.is_empty() {
        writeln!(
            stdout,
            "stencil theme check {reference}: FAILED: nothing examined"
        )?;
        return Ok(ExitCode::Defects);
    }
    writeln!(
        stdout,
        "stencil theme check {reference}: {}",
        quality_summary_text(&report)
    )?;
    let (_, failed, _) = report.counts();
    Ok(if failed == 0 {
        ExitCode::Clean
    } else {
        ExitCode::Defects
    })
}
