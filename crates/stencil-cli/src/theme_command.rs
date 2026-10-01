//! `stencil theme show`, `check` and `preview` (section 13.12); `stencil theme import` is in
//! `theme_import`.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use stencil_model::theme::ThemeReport;
use stencil_model::theme_quality;
use stencil_render::{DeviceScale, builtin_theme_json};

use crate::ExitCode;
use crate::pipeline::{
    Failure, ThemeChoice, check_legend_labels, load_document_from, render_page, resolve_theme,
};
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
    for line in quality_report_lines(&report, reference) {
        writeln!(stdout, "{line}")?;
    }
    Ok(quality_exit_code(&report))
}

/// The lines `stencil theme check <reference>` prints: one per quality row, then the
/// summary, which reads `FAILED: nothing examined` when there are no rows.
pub(crate) fn quality_report_lines(report: &ThemeReport, reference: &str) -> Vec<String> {
    let mut lines: Vec<String> = report.rows.iter().map(quality_row_line).collect();
    if report.rows.is_empty() {
        lines.push(format!(
            "stencil theme check {reference}: FAILED: nothing examined"
        ));
    } else {
        lines.push(format!(
            "stencil theme check {reference}: {}",
            quality_summary_text(report)
        ));
    }
    lines
}

/// Clean when at least one row was examined and no row failed.
pub(crate) fn quality_exit_code(report: &ThemeReport) -> ExitCode {
    let (_, failed, _) = report.counts();
    if report.rows.is_empty() || failed > 0 {
        ExitCode::Defects
    } else {
        ExitCode::Clean
    }
}

/// The preview figure every theme is drawn on: one gcp page that uses every role.
pub const PREVIEW_JSON: &str = include_str!("../preview/theme-preview.json");
/// The preview's kicker before the theme name.
pub const PREVIEW_KICKER_PREFIX: &str = "theme preview · ";

/// The preview document with its kicker naming `theme_name`. Only the kicker changes, so
/// the body lays out the same under every theme.
pub fn preview_document(theme_name: &str) -> Result<String, Failure> {
    let serialize_error = |source| Failure::Serialize {
        what: "theme preview",
        source,
    };
    let mut document: serde_json::Value =
        serde_json::from_str(PREVIEW_JSON).map_err(serialize_error)?;
    if let Some(page) = document.as_object_mut() {
        page.insert(
            "kicker".to_string(),
            serde_json::Value::String(format!("{PREVIEW_KICKER_PREFIX}{theme_name}")),
        );
    }
    serde_json::to_string_pretty(&document).map_err(serialize_error)
}

/// Renders the preview figure under a theme to a PNG at `png_path`, then prints its
/// absolute path. A theme that does not load fails as in `theme check`.
pub fn preview(
    reference: &str,
    png_path: &Path,
    scale: u8,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    match render_preview(reference, png_path, scale) {
        Ok(written) => {
            writeln!(stdout, "{}", written.display())?;
            Ok(ExitCode::Clean)
        }
        Err(failure) => report_failure("theme preview", &failure, stdout, stderr),
    }
}

fn render_preview(reference: &str, png_path: &Path, scale: u8) -> Result<PathBuf, Failure> {
    let scale = DeviceScale::new(scale)?;
    let current = Path::new(".");
    let theme = resolve_theme(reference, current)?;
    check_legend_labels(&theme, reference)?;
    let document = preview_document(&theme.name)?;
    let theme_flag = ThemeChoice {
        reference,
        directory: current,
    };
    let loaded = load_document_from(&document, current, Some(theme_flag))?;
    let rendered = render_page(&loaded, scale)?;
    fs::write(png_path, &rendered.png).map_err(|source| Failure::WriteOutput {
        path: png_path.to_path_buf(),
        source,
    })?;
    Ok(std::path::absolute(png_path).unwrap_or_else(|_| png_path.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_kicker_names_the_theme_and_nothing_else_changes() {
        let original: serde_json::Value = serde_json::from_str(PREVIEW_JSON).unwrap();
        let named: serde_json::Value =
            serde_json::from_str(&preview_document("nord").unwrap()).unwrap();
        assert_eq!(named["kicker"], "theme preview · nord");
        let mut restored = named.clone();
        restored["kicker"] = original["kicker"].clone();
        assert_eq!(restored, original);
    }
}
