//! The steps behind `vet`, `render` and `check`: read, parse and vet, lay out with
//! cosmic-text, render SVG, PNG and measured JSON, run the checks, write the outputs.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;
use stencil_layout::checks::{
    PrintWidth, child_inside_container, links_avoid_boxes, links_routed, pipes_land, print_fit,
    siblings_do_not_overlap, text_fits_box,
};
use stencil_layout::{LayoutError, PageGeometry, layout_page, theme_legend_labels};
use stencil_model::checks::{CheckReport, legend_consistency, remembered_constants};
use stencil_model::grammar::GrammarViolation;
use stencil_model::products::icon_matches_product;
use stencil_model::text::MeasureError;
use stencil_model::{
    DATA_FILE_BYTES_MAX, Grammar, GrammarError, ModelError, Page, Projection, Theme, ThemeError,
    apply_overrides, builtin_grammar, grammar_reference, is_grammar_reference, parse_grammar,
    parse_page, parse_theme, theme_reference, validate_page, vet_page,
};
use stencil_render::iso::{IsoScene, SolidInputs, iso_labels_clear, iso_links_clear, project_page};
use stencil_render::{
    DeviceScale, RenderError, SvgDocument, builtin_theme, measured_json, render_png, render_svg,
};
use stencil_text::{CosmicTextMeasurer, FontError};

#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error("cannot read input {}", path.display())]
    ReadInput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("input path {} has no file stem to name the outputs after", path.display())]
    InputStem { path: PathBuf },
    #[error("output {} is the input file", path.display())]
    OutputIsInput { path: PathBuf },
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Grammar(#[from] GrammarError),
    #[error("cannot read grammar {}", path.display())]
    ReadGrammar {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Theme(#[from] ThemeError),
    #[error("cannot read theme {}", path.display())]
    ReadTheme {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot measure the legend labels of theme {theme}")]
    ThemeLabels {
        theme: String,
        #[source]
        source: MeasureError,
    },
    #[error("document parses as a page but not as a JSON value")]
    DocumentValue(#[source] serde_json::Error),
    #[error(transparent)]
    Fonts(#[from] FontError),
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error("cannot serialize the {what}")]
    Serialize {
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot create output directory {}", path.display())]
    CreateOutputDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot read examples directory {}", path.display())]
    ReadExamples {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("examples directory {} holds no .json documents", path.display())]
    NoExamples { path: PathBuf },
    #[error("examples directory {} holds more than {limit} .json documents", path.display())]
    ExamplesExceeded { path: PathBuf, limit: usize },
    #[error("cannot write {}", path.display())]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// A vetted page with the grammar it names and the theme it is drawn in, together with the
/// input parsed as a plain JSON value, which the measured JSON carries unchanged as
/// `document`.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedDocument {
    pub page: Page,
    pub grammar: Grammar,
    pub theme: Theme,
    pub document: Value,
}

/// A theme reference and the directory a path reference resolves against: the current
/// directory for `--theme`, the document's directory for `Page.theme` (section 13.4 rule 1).
#[derive(Debug, Clone, Copy)]
pub struct ThemeChoice<'a> {
    pub reference: &'a str,
    pub directory: &'a Path,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedPage {
    pub geometry: PageGeometry,
    /// The projected page when the effective projection is iso (section 12.9).
    pub scene: Option<IsoScene>,
    pub svg: SvgDocument,
    pub png: Vec<u8>,
    pub measured: Value,
}

/// The three file names `render` writes, derived from the input's file stem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputNames {
    pub svg: OsString,
    pub png: OsString,
    pub measured: OsString,
}

/// Absolute paths of the written files, in the order `render` prints them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPaths {
    pub svg: PathBuf,
    pub png: PathBuf,
    pub measured: PathBuf,
}

/// Largest input `read_input` accepts, in bytes (section 7). A vetted page holds at most
/// NODES_MAX nodes of a few 400-scalar texts each, far below this.
pub const INPUT_BYTES_MAX: u64 = 64 * 1024 * 1024;

/// Reads the input file, at most INPUT_BYTES_MAX bytes. Bytes that are not UTF-8 are a
/// document defect, not a read failure, because RFC 8259 requires JSON text to be UTF-8
/// (section 7).
pub fn read_input(path: &Path) -> Result<String, Failure> {
    let read_error = |source| Failure::ReadInput {
        path: path.to_path_buf(),
        source,
    };
    let file = fs::File::open(path).map_err(read_error)?;
    let mut bytes = Vec::new();
    file.take(INPUT_BYTES_MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(read_error)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > INPUT_BYTES_MAX {
        return Err(read_error(io::Error::new(
            io::ErrorKind::FileTooLarge,
            format!("input is larger than {INPUT_BYTES_MAX} bytes"),
        )));
    }
    String::from_utf8(bytes).map_err(|error| {
        let valid_up_to = error.utf8_error().valid_up_to();
        invalid_utf8(error.as_bytes(), valid_up_to)
    })
}

/// A `ModelError::Json` at the 1-based line and byte column of the first invalid byte,
/// counted the way serde_json counts them.
fn invalid_utf8(bytes: &[u8], valid_up_to: usize) -> Failure {
    let prefix = bytes.get(..valid_up_to).unwrap_or_default();
    let line_start = prefix
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |newline| newline + 1);
    let newlines = prefix.iter().filter(|&&byte| byte == b'\n').count();
    Failure::Model(ModelError::Json {
        line: newlines + 1,
        column: valid_up_to - line_start + 1,
        message: "input is not valid UTF-8".to_string(),
    })
}

/// `load_document_from` with grammar and theme paths resolved against the current directory
/// and no `--theme`.
pub fn load_document(json_text: &str) -> Result<LoadedDocument, Failure> {
    load_document_from(json_text, Path::new("."), None)
}

/// Parses the page, resolves and loads the grammar it names (section 13.2), vets the page
/// against it, resolves and loads the theme with the page's overrides (section 13.4 rules 1
/// to 5), then parses the input again into `serde_json::Value`. A grammar or theme given by
/// path in the page resolves against `document_directory`, the directory of the input
/// document. `theme_flag`, the `--theme` value, replaces the page's theme reference.
pub fn load_document_from(
    json_text: &str,
    document_directory: &Path,
    theme_flag: Option<ThemeChoice<'_>>,
) -> Result<LoadedDocument, Failure> {
    let page = parse_page(json_text)?;
    let reference = grammar_reference(&page).to_string();
    let grammar = if is_grammar_reference(&reference) {
        resolve_grammar(&reference, document_directory)?
    } else {
        // `grammar-unknown` is reported before resolution, with every other violation the
        // page has under the default grammar.
        let fallback = resolve_grammar(stencil_model::GRAMMAR_DEFAULT, document_directory)?;
        return Err(Failure::Model(ModelError::Invalid(validate_page(
            &page, &fallback,
        ))));
    };
    let page = vet_page(page, &grammar)?;
    let choice = theme_flag.unwrap_or(ThemeChoice {
        reference: theme_reference(&page),
        directory: document_directory,
    });
    let theme = load_theme(&page, choice)?;
    let document: Value = serde_json::from_str(json_text).map_err(Failure::DocumentValue)?;
    Ok(LoadedDocument {
        page,
        grammar,
        theme,
        document,
    })
}

/// The theme `choice` names with the page's `theme_overrides` merged on, after the legend
/// label rule (section 13.4 rule 4) passed.
pub fn load_theme(page: &Page, choice: ThemeChoice<'_>) -> Result<Theme, Failure> {
    let base = resolve_theme(choice.reference, choice.directory)?;
    let theme = match &page.theme_overrides {
        Some(overrides) => apply_overrides(&base, overrides)?,
        None => base,
    };
    check_legend_labels(&theme, &theme.name)?;
    Ok(theme)
}

/// The legend label rule needs a measurer, so it runs here with the bundled fonts.
pub fn check_legend_labels(theme: &Theme, origin: &str) -> Result<(), Failure> {
    let mut measurer = CosmicTextMeasurer::new()?;
    let violations =
        theme_legend_labels(theme, &mut measurer).map_err(|source| Failure::ThemeLabels {
            theme: theme.name.clone(),
            source,
        })?;
    if violations.is_empty() {
        Ok(())
    } else {
        Err(Failure::Theme(ThemeError::Invalid {
            origin: origin.to_string(),
            violations,
        }))
    }
}

/// A built-in theme by name, or a theme file by path relative to `directory`, read up to
/// DATA_FILE_BYTES_MAX bytes and validated.
pub fn resolve_theme(reference: &str, directory: &Path) -> Result<Theme, Failure> {
    if let Some(builtin) = builtin_theme(reference) {
        return Ok(builtin?);
    }
    let path = directory.join(reference);
    let bytes = read_data_file(&path, "theme").map_err(|source| Failure::ReadTheme {
        path: path.clone(),
        source,
    })?;
    let origin = path.display().to_string();
    let json_text = String::from_utf8(bytes).map_err(|error| ThemeError::Json {
        origin: origin.clone(),
        line: 1,
        column: error.utf8_error().valid_up_to() + 1,
        message: "theme is not valid UTF-8".to_string(),
    })?;
    Ok(parse_theme(&json_text, &origin)?)
}

/// The bytes of a grammar or theme file, at most DATA_FILE_BYTES_MAX of them.
fn read_data_file(path: &Path, what: &str) -> io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    if file.metadata()?.is_dir() {
        return Err(io::Error::from(io::ErrorKind::IsADirectory));
    }
    let limit = u64::try_from(DATA_FILE_BYTES_MAX).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() > DATA_FILE_BYTES_MAX {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            format!("{what} is larger than {DATA_FILE_BYTES_MAX} bytes"),
        ));
    }
    Ok(bytes)
}

/// A built-in grammar by name, or a grammar file by path relative to `directory`, read up
/// to DATA_FILE_BYTES_MAX bytes and validated.
pub fn resolve_grammar(reference: &str, directory: &Path) -> Result<Grammar, Failure> {
    if let Some(builtin) = builtin_grammar(reference) {
        return Ok(builtin?);
    }
    let path = directory.join(reference);
    let bytes = read_data_file(&path, "grammar").map_err(|source| Failure::ReadGrammar {
        path: path.clone(),
        source,
    })?;
    let origin = path.display().to_string();
    let json_text = String::from_utf8(bytes).map_err(|error| GrammarError::Json {
        origin: origin.clone(),
        line: 1,
        column: error.utf8_error().valid_up_to() + 1,
        message: "grammar is not valid UTF-8".to_string(),
    })?;
    Ok(parse_grammar(&json_text, &origin)?)
}

/// The violations of a theme failure, empty for a JSON error.
pub fn theme_violations(error: &ThemeError) -> &[stencil_model::ThemeViolation] {
    error.violations()
}

/// The violations of a grammar failure, empty for a JSON error.
pub fn grammar_violations(error: &GrammarError) -> &[GrammarViolation] {
    match error {
        GrammarError::Json { .. } => &[],
        GrammarError::Invalid { violations, .. } => violations,
    }
}

pub fn output_names(input: &Path) -> Result<OutputNames, Failure> {
    let Some(stem) = input.file_stem() else {
        return Err(Failure::InputStem {
            path: input.to_path_buf(),
        });
    };
    let with_suffix = |suffix: &str| {
        let mut name = stem.to_os_string();
        name.push(suffix);
        name
    };
    Ok(OutputNames {
        svg: with_suffix(".svg"),
        png: with_suffix(".png"),
        measured: with_suffix(".measured.json"),
    })
}

/// Lays out with `CosmicTextMeasurer`, projects the page once when its projection is iso,
/// and renders all three outputs in memory.
pub fn render_page(loaded: &LoadedDocument, scale: DeviceScale) -> Result<RenderedPage, Failure> {
    let mut measurer = CosmicTextMeasurer::new()?;
    let geometry = layout_page(&loaded.page, &loaded.grammar, &mut measurer)?;
    let scene = match loaded.page.projection {
        Projection::Flat => None,
        Projection::Iso => {
            let inputs = SolidInputs::new(
                &geometry,
                &loaded.page.links,
                loaded.theme.iso.slab_thickness,
            );
            Some(project_page(&geometry, &inputs)?)
        }
    };
    let svg = render_svg(&loaded.page, &loaded.theme, &geometry)?;
    let png = render_png(&svg.svg, svg.text_elements, scale)?;
    let measured = measured_json(&loaded.document, &geometry, scene.as_ref());
    Ok(RenderedPage {
        geometry,
        scene,
        svg,
        png,
        measured,
    })
}

/// The three geometry-free checks, in `CheckName` order.
pub fn model_checks(page: &Page, grammar: &Grammar) -> [CheckReport; 3] {
    [
        remembered_constants(page, grammar),
        legend_consistency(page),
        icon_matches_product(page, grammar),
    ]
}

/// print-fit against the drawn canvas: the projected canvas under iso, the layout canvas
/// otherwise.
pub fn print_fit_report(
    geometry: &PageGeometry,
    scene: Option<&IsoScene>,
    print_width: Option<PrintWidth>,
) -> CheckReport {
    let canvas_width = scene.map_or(geometry.canvas.width, |scene| scene.canvas.width);
    print_fit(geometry, canvas_width, print_width)
}

/// All twelve checks, in `CheckName` order. `iso-labels-clear` and `iso-links-clear` read
/// the scene and are not applicable without one; print-fit is not applicable without a
/// print width.
pub fn all_checks(
    page: &Page,
    grammar: &Grammar,
    geometry: &PageGeometry,
    scene: Option<&IsoScene>,
    print_width: Option<PrintWidth>,
) -> [CheckReport; 12] {
    let [remembered, legend, icons] = model_checks(page, grammar);
    [
        child_inside_container(geometry),
        siblings_do_not_overlap(geometry),
        text_fits_box(geometry),
        remembered,
        legend,
        links_routed(geometry),
        links_avoid_boxes(geometry),
        pipes_land(page, geometry),
        iso_labels_clear(scene),
        iso_links_clear(scene),
        print_fit_report(geometry, scene, print_width),
        icons,
    ]
}

/// Creates `out_dir` when missing and writes SVG, PNG and measured JSON, replacing existing
/// files. The measured JSON is serialized before anything touches the disk, an output that
/// resolves to `input` or an output name that is a directory is refused, and the three files
/// are staged under temporary names and renamed only after every write succeeded
/// (section 7).
pub fn write_outputs(
    out_dir: &Path,
    names: &OutputNames,
    rendered: &RenderedPage,
    input: &Path,
) -> Result<OutputPaths, Failure> {
    let mut measured_bytes =
        serde_json::to_vec_pretty(&rendered.measured).map_err(|source| Failure::Serialize {
            what: "measured JSON",
            source,
        })?;
    measured_bytes.push(b'\n');

    let directory_error = |source| Failure::CreateOutputDirectory {
        path: out_dir.to_path_buf(),
        source,
    };
    fs::create_dir_all(out_dir).map_err(directory_error)?;
    let directory = fs::canonicalize(out_dir).map_err(directory_error)?;

    let paths = OutputPaths {
        svg: directory.join(&names.svg),
        png: directory.join(&names.png),
        measured: directory.join(&names.measured),
    };
    let outputs = [
        (&names.svg, &paths.svg, rendered.svg.svg.as_bytes()),
        (&names.png, &paths.png, rendered.png.as_slice()),
        (&names.measured, &paths.measured, measured_bytes.as_slice()),
    ];

    let input = fs::canonicalize(input).map_err(|source| Failure::ReadInput {
        path: input.to_path_buf(),
        source,
    })?;
    for (_, path, _) in &outputs {
        // An existing output may be a link to the input, so compare resolved paths.
        let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if resolved == input {
            return Err(Failure::OutputIsInput {
                path: path.to_path_buf(),
            });
        }
    }

    // A rename onto a directory fails only after the earlier renames already replaced their
    // outputs, so every final name is checked before anything is staged.
    for (_, path, _) in &outputs {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_dir() => {
                return Err(Failure::WriteOutput {
                    path: path.to_path_buf(),
                    source: io::Error::from(io::ErrorKind::IsADirectory),
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(Failure::WriteOutput {
                    path: path.to_path_buf(),
                    source,
                });
            }
        }
    }

    let mut staged: Vec<(PathBuf, &Path)> = Vec::with_capacity(outputs.len());
    for (name, path, bytes) in &outputs {
        let temporary = directory.join(temporary_name(name));
        if let Err(source) = write_new_file(&temporary, bytes) {
            remove_staged(&staged);
            return Err(Failure::WriteOutput {
                path: path.to_path_buf(),
                source,
            });
        }
        staged.push((temporary, path.as_path()));
    }
    for (index, (temporary, path)) in staged.iter().enumerate() {
        if let Err(source) = fs::rename(temporary, path) {
            remove_staged(staged.get(index..).unwrap_or_default());
            return Err(Failure::WriteOutput {
                path: path.to_path_buf(),
                source,
            });
        }
    }
    Ok(paths)
}

/// Distinguishes the temporary names of `write_outputs` calls within one process.
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// `.<name>.<process id>.<sequence>.tmp`, in the output directory so the rename stays on
/// one file system.
fn temporary_name(name: &OsString) -> OsString {
    let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut temporary = OsString::from(".");
    temporary.push(name);
    temporary.push(format!(".{}.{sequence}.tmp", std::process::id()));
    temporary
}

/// `create_new` refuses to open through a link planted at the temporary name, and a file
/// left there by an interrupted run fails the write instead of being overwritten. Only a
/// file this call created is removed when the write fails.
fn write_new_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::options()
        .write(true)
        .create_new(true)
        .open(path)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    if written.is_err() {
        let _ = fs::remove_file(path);
    }
    written
}

/// Best effort: the write error that led here is the one reported.
fn remove_staged(staged: &[(PathBuf, &Path)]) {
    for (temporary, _) in staged {
        let _ = fs::remove_file(temporary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_names_differ_within_one_process() {
        let name = OsString::from("g7.svg");
        let first = temporary_name(&name);
        let second = temporary_name(&name);
        assert_ne!(first, second);
        let prefix = format!(".g7.svg.{}.", std::process::id());
        for temporary in [&first, &second] {
            let text = temporary.to_str().unwrap();
            assert!(text.starts_with(&prefix), "{text}");
            assert!(text.ends_with(".tmp"), "{text}");
        }
    }

    #[test]
    fn a_leftover_at_the_temporary_name_fails_the_write_and_is_kept() {
        // CARGO_TARGET_TMPDIR is set for integration tests only.
        let directory =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/stencil-cli-leftover");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(".g7.svg.leftover.tmp");
        fs::write(&path, b"leftover").unwrap();

        let error = write_new_file(&path, b"new").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&path).unwrap(), b"leftover");
        fs::remove_file(&path).unwrap();
    }
}
