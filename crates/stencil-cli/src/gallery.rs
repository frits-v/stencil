//! `stencil gallery`: every document in an examples directory rendered under every theme,
//! checked, and indexed by a static `index.html` and a `gallery.json` (section 7).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use stencil_layout::LayoutError;
use stencil_model::checks::{CheckOutcome, CheckReport};
use stencil_model::{DESIGNED_THEMES, ModelError};
use stencil_render::DeviceScale;

use crate::exit::{ExitCode, failure_exit_code, reports_exit_code};
use crate::pipeline::{
    Failure, LoadedDocument, OutputNames, ThemeChoice, all_checks, grammar_violations,
    load_document_from, load_theme, output_names, read_input, render_page, theme_violations,
    write_outputs,
};
use crate::report::{
    check_counts_text, count_text, grammar_violation_line, report_lines, theme_violation_line,
    violation_line,
};

/// Largest number of documents one gallery renders. Each one costs a render per theme.
pub const EXAMPLES_MAX: usize = 256;

pub const INDEX_FILE_NAME: &str = "index.html";
pub const MANIFEST_FILE_NAME: &str = "gallery.json";

/// `gallery.json`: the data `index.html` shows, for tooling. Paths are relative to the
/// gallery directory and use `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryManifest {
    pub themes: Vec<String>,
    pub examples: Vec<GalleryExample>,
    pub renders: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryExample {
    pub name: String,
    pub source: String,
    pub title: String,
    pub kicker: String,
    pub renders: Vec<GalleryRender>,
}

/// One document under one theme. `files` is absent when the render did not happen; `summary`
/// then carries the reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryRender {
    pub theme: String,
    pub passed: bool,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<GalleryFiles>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryFiles {
    pub png: String,
    pub svg: String,
    pub measured: String,
}

/// The six designed themes in the gallery's order, without the imported tier (section 13.9).
fn gallery_themes() -> Vec<String> {
    DESIGNED_THEMES
        .iter()
        .map(|name| name.to_string())
        .collect()
}

/// A built-in theme name as the theme reference of a gallery render.
fn builtin_choice(name: &str) -> ThemeChoice<'_> {
    ThemeChoice {
        reference: name,
        directory: Path::new("."),
    }
}

/// The `.json` files directly inside `examples`, sorted by file name so the gallery is the
/// same on every run.
pub fn example_paths(examples: &Path) -> Result<Vec<PathBuf>, Failure> {
    let read_error = |source| Failure::ReadExamples {
        path: examples.to_path_buf(),
        source,
    };
    let mut paths = Vec::new();
    for entry in fs::read_dir(examples).map_err(read_error)? {
        let entry = entry.map_err(read_error)?;
        let path = entry.path();
        let is_json = path
            .extension()
            .is_some_and(|extension| extension == "json");
        if !is_json || !entry.file_type().map_err(read_error)?.is_file() {
            continue;
        }
        if paths.len() == EXAMPLES_MAX {
            return Err(Failure::ExamplesExceeded {
                path: examples.to_path_buf(),
                limit: EXAMPLES_MAX,
            });
        }
        paths.push(path);
    }
    if paths.is_empty() {
        return Err(Failure::NoExamples {
            path: examples.to_path_buf(),
        });
    }
    paths.sort();
    Ok(paths)
}

/// What one render contributes to stdout, the page and the manifest.
struct RenderResult {
    record: GalleryRender,
    lines: Vec<String>,
}

pub fn gallery(
    out_dir: &Path,
    examples: &Path,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    match build_gallery(out_dir, examples, stdout) {
        Ok(code) => Ok(code),
        Err(GalleryStop::Failure(failure)) => {
            crate::write_error_chain("gallery", &failure, stderr)?;
            Ok(failure_exit_code(&failure))
        }
        Err(GalleryStop::Output(error)) => Err(error),
    }
}

/// A gallery stops early only when it could not run, or when stdout fails.
enum GalleryStop {
    Failure(Failure),
    Output(io::Error),
}

impl From<Failure> for GalleryStop {
    fn from(failure: Failure) -> Self {
        GalleryStop::Failure(failure)
    }
}

impl From<io::Error> for GalleryStop {
    fn from(error: io::Error) -> Self {
        GalleryStop::Output(error)
    }
}

fn build_gallery(
    out_dir: &Path,
    examples: &Path,
    stdout: &mut dyn Write,
) -> Result<ExitCode, GalleryStop> {
    let themes = gallery_themes();
    let paths = example_paths(examples)?;
    fs::create_dir_all(out_dir).map_err(|source| Failure::CreateOutputDirectory {
        path: out_dir.to_path_buf(),
        source,
    })?;

    let mut manifest = GalleryManifest {
        themes: themes.clone(),
        examples: Vec::with_capacity(paths.len()),
        renders: 0,
        failed: 0,
    };
    for path in &paths {
        let names = output_names(path)?;
        let name = utf8_stem(path)?;
        let directory = path.parent().unwrap_or(Path::new("."));
        let loaded = match read_input(path).and_then(|json_text| {
            let first = themes.first().map_or("center", String::as_str);
            load_document_from(&json_text, directory, Some(builtin_choice(first)))
        }) {
            Ok(loaded) => loaded,
            Err(failure) => {
                stop_unless_defect(failure, |failure| {
                    for line in document_defect_lines(failure) {
                        writeln!(stdout, "{line}")?;
                    }
                    writeln!(stdout, "gallery {name}: not rendered")
                })?;
                manifest.failed += themes.len();
                manifest
                    .examples
                    .push(unloaded_example(path, &name, &themes));
                continue;
            }
        };
        let mut example = GalleryExample {
            name: name.clone(),
            source: file_name_text(path),
            title: loaded.page.title.clone(),
            kicker: loaded.page.kicker.clone(),
            renders: Vec::with_capacity(themes.len()),
        };
        for theme_name in &themes {
            let result = render_theme(out_dir, path, &names, &name, theme_name, &loaded)?;
            for line in &result.lines {
                writeln!(stdout, "{line}")?;
            }
            if result.record.files.is_some() {
                manifest.renders += 1;
            }
            if !result.record.passed {
                manifest.failed += 1;
            }
            example.renders.push(result.record);
        }
        manifest.examples.push(example);
    }

    let directory = fs::canonicalize(out_dir).map_err(|source| Failure::CreateOutputDirectory {
        path: out_dir.to_path_buf(),
        source,
    })?;
    let mut manifest_bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|source| Failure::Serialize {
            what: "gallery manifest",
            source,
        })?;
    manifest_bytes.push(b'\n');
    let index_path = directory.join(INDEX_FILE_NAME);
    let manifest_path = directory.join(MANIFEST_FILE_NAME);
    for (path, bytes) in [
        (&index_path, index_html(&manifest).into_bytes()),
        (&manifest_path, manifest_bytes),
    ] {
        fs::write(path, bytes).map_err(|source| Failure::WriteOutput {
            path: path.clone(),
            source,
        })?;
    }
    writeln!(stdout, "{}", index_path.display())?;
    writeln!(stdout, "{}", manifest_path.display())?;
    writeln!(stdout, "stencil gallery: {}", summary_text(&manifest))?;

    if manifest.failed == 0 && manifest.renders > 0 {
        Ok(ExitCode::Clean)
    } else {
        Ok(ExitCode::Defects)
    }
}

/// An example whose document does not load: listed under every theme as not rendered, with
/// its file name in place of the title the document would have given.
fn unloaded_example(path: &Path, name: &str, themes: &[String]) -> GalleryExample {
    GalleryExample {
        name: name.to_string(),
        source: file_name_text(path),
        title: file_name_text(path),
        kicker: "document does not load".to_string(),
        renders: themes
            .iter()
            .map(|theme_name| not_rendered(theme_name))
            .collect(),
    }
}

fn not_rendered(theme_name: &str) -> GalleryRender {
    GalleryRender {
        theme: theme_name.to_string(),
        passed: false,
        summary: "not rendered".to_string(),
        files: None,
    }
}

/// `<r> renders of <e> examples in <t> themes, <f> failed`.
pub fn summary_text(manifest: &GalleryManifest) -> String {
    let example_count = manifest.examples.len() as u64;
    format!(
        "{} of {} in {}, {} failed",
        count_text(manifest.renders as u64, "render", "renders"),
        count_text(example_count, "example", "examples"),
        count_text(manifest.themes.len() as u64, "theme", "themes"),
        manifest.failed
    )
}

/// A defect of the document is reported by `report` and the gallery goes on; any other
/// failure stops it.
fn stop_unless_defect(
    failure: Failure,
    report: impl FnOnce(&Failure) -> io::Result<()>,
) -> Result<(), GalleryStop> {
    if failure_exit_code(&failure) == ExitCode::Defects {
        report(&failure)?;
        Ok(())
    } else {
        Err(GalleryStop::Failure(failure))
    }
}

fn document_defect_lines(failure: &Failure) -> Vec<String> {
    match failure {
        Failure::Model(ModelError::Invalid(violations))
        | Failure::Layout(LayoutError::Invalid(violations)) => {
            violations.iter().map(violation_line).collect()
        }
        Failure::Grammar(error) => std::iter::once(format!("error {error}"))
            .chain(grammar_violations(error).iter().map(grammar_violation_line))
            .collect(),
        Failure::Theme(error) => std::iter::once(format!("error {error}"))
            .chain(theme_violations(error).iter().map(theme_violation_line))
            .collect(),
        other => vec![format!("error {other}")],
    }
}

/// Renders at the default PNG scale into `<out_dir>/<name>/<theme>/` and runs the checks.
fn render_theme(
    out_dir: &Path,
    path: &Path,
    names: &OutputNames,
    name: &str,
    theme_name: &str,
    loaded: &LoadedDocument,
) -> Result<RenderResult, GalleryStop> {
    let mut themed = loaded.clone();
    let rendered = load_theme(&loaded.page, builtin_choice(theme_name)).and_then(|theme| {
        themed.theme = theme;
        render_page(&themed, DeviceScale::DEFAULT)
    });
    let rendered = match rendered {
        Ok(rendered) => rendered,
        Err(failure) => {
            let mut lines = Vec::new();
            stop_unless_defect(failure, |failure| {
                lines.extend(document_defect_lines(failure));
                Ok(())
            })?;
            lines.push(format!("gallery {name} {theme_name}: not rendered"));
            return Ok(RenderResult {
                record: not_rendered(theme_name),
                lines,
            });
        }
    };
    let reports = all_checks(
        &themed.page,
        &themed.grammar,
        &rendered.geometry,
        rendered.scene.as_ref(),
        None,
    );
    write_outputs(&out_dir.join(name).join(theme_name), names, &rendered, path)?;

    let passed = reports_exit_code(&reports) == ExitCode::Clean;
    let summary = check_counts_text(&reports);
    let mut lines = Vec::new();
    if !passed {
        lines.extend(failing_report_lines(&reports));
    }
    lines.push(format!("gallery {name} {theme_name}: {summary}"));
    let relative = |file_name: &std::ffi::OsString| {
        format!("{name}/{theme_name}/{}", file_name.to_string_lossy())
    };
    Ok(RenderResult {
        record: GalleryRender {
            theme: theme_name.to_string(),
            passed,
            summary,
            files: Some(GalleryFiles {
                png: relative(&names.png),
                svg: relative(&names.svg),
                measured: relative(&names.measured),
            }),
        },
        lines,
    })
}

/// The check and defect lines of the reports that failed, in `CheckName` order.
fn failing_report_lines(reports: &[CheckReport]) -> Vec<String> {
    let failing: Vec<CheckReport> = reports
        .iter()
        .filter(|report| report.outcome() == CheckOutcome::Failed)
        .cloned()
        .collect();
    report_lines(&failing)
}

/// The file stem as UTF-8, since it names a directory and appears in links.
fn utf8_stem(path: &Path) -> Result<String, Failure> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
        .ok_or_else(|| Failure::InputStem {
            path: path.to_path_buf(),
        })
}

fn file_name_text(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The static page: one section per theme, reached from anchor tabs, each listing every
/// example with its thumbnail, links and check summary.
pub fn index_html(manifest: &GalleryManifest) -> String {
    let mut html = String::from(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>stencil gallery</title>\n<style>\n\
         body { font-family: system-ui, sans-serif; margin: 24px; color: #202124; }\n\
         nav a { margin-right: 16px; }\n\
         section { margin-top: 32px; }\n\
         article { margin: 16px 0; padding-bottom: 16px; border-bottom: 1px solid #dadce0; }\n\
         .kicker { color: #5f6368; margin: 0; }\n\
         img { width: 100%; max-width: 480px; border: 1px solid #dadce0; }\n\
         .failed { color: #c5221f; }\n\
         </style>\n</head>\n<body>\n<h1>stencil gallery</h1>\n",
    );
    html.push_str(&format!(
        "<p>{}</p>\n<nav>",
        escape(&summary_text(manifest))
    ));
    for theme in &manifest.themes {
        html.push_str(&format!("<a href=\"#{0}\">{0}</a>", escape(theme)));
    }
    html.push_str("</nav>\n");
    for theme in &manifest.themes {
        html.push_str(&format!(
            "<section id=\"{0}\">\n<h2>{0}</h2>\n",
            escape(theme)
        ));
        for example in &manifest.examples {
            let Some(render) = example.renders.iter().find(|render| &render.theme == theme) else {
                continue;
            };
            html.push_str(&example_html(example, render));
        }
        html.push_str("</section>\n");
    }
    html.push_str("</body>\n</html>\n");
    html
}

fn example_html(example: &GalleryExample, render: &GalleryRender) -> String {
    let mut html = format!(
        "<article id=\"{}-{}\">\n<p class=\"kicker\">{}</p>\n<h3>{}</h3>\n",
        escape(&render.theme),
        escape(&example.name),
        escape(&example.kicker),
        escape(&example.title)
    );
    if let Some(files) = &render.files {
        let png = escape(&href(&files.png));
        html.push_str(&format!(
            "<a href=\"{png}\"><img src=\"{png}\" alt=\"{} in the {} theme\" loading=\"lazy\"></a>\n",
            escape(&example.name),
            escape(&render.theme)
        ));
        html.push_str(&format!(
            "<p><a href=\"{png}\">PNG</a> | <a href=\"{}\">SVG</a> | <a href=\"{}\">measured JSON</a></p>\n",
            escape(&href(&files.svg)),
            escape(&href(&files.measured))
        ));
    }
    let class = if render.passed {
        "checks"
    } else {
        "checks failed"
    };
    html.push_str(&format!(
        "<p class=\"{class}\">{}</p>\n</article>\n",
        escape(&render.summary)
    ));
    html
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Percent-encodes every byte of a relative path except unreserved characters and `/`, so a
/// file name with a space or `#` still links to itself.
fn href(relative: &str) -> String {
    let mut encoded = String::with_capacity(relative.len());
    for byte in relative.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_replaces_the_markup_characters() {
        assert_eq!(
            escape("a < b & \"c\" > 'd'"),
            "a &lt; b &amp; &quot;c&quot; &gt; &#39;d&#39;"
        );
        assert_eq!(escape("two metros · 99.99%"), "two metros · 99.99%");
    }

    #[test]
    fn href_encodes_everything_but_unreserved_characters_and_slashes() {
        assert_eq!(href("g7/center/g7.png"), "g7/center/g7.png");
        assert_eq!(href("a b#c/x.png"), "a%20b%23c/x.png");
        assert_eq!(href("é"), "%C3%A9");
    }

    #[test]
    fn the_gallery_covers_the_six_designed_themes_in_order() {
        assert_eq!(
            gallery_themes(),
            ["center", "paper", "dusk", "clear", "clear-dark", "wire"]
        );
    }

    #[test]
    fn the_summary_counts_renders_examples_and_themes() {
        let manifest = GalleryManifest {
            themes: vec!["center".to_string()],
            examples: Vec::new(),
            renders: 1,
            failed: 0,
        };
        assert_eq!(
            summary_text(&manifest),
            "1 render of 0 examples in 1 theme, 0 failed"
        );
    }
}
