//! The steps behind `vet`, `render` and `check`: read, parse and vet, lay out with
//! cosmic-text, render SVG, PNG and measured JSON, run the checks, write the outputs.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;
use stencil_layout::checks::{child_inside_container, siblings_do_not_overlap, text_fits_box};
use stencil_layout::{LayoutError, PageGeometry, layout_page};
use stencil_model::checks::{CheckReport, legend_consistency, remembered_constants};
use stencil_model::{ModelError, Page, parse_page};
use stencil_render::{
    DeviceScale, RenderError, SvgDocument, measured_json, render_png, render_svg,
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
    #[error("cannot write {}", path.display())]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// A vetted page together with the input parsed as a plain JSON value, which the measured
/// JSON carries unchanged as `document`.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedDocument {
    pub page: Page,
    pub document: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedPage {
    pub geometry: PageGeometry,
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

/// Reads the input file. Bytes that are not UTF-8 are a document defect, not a read
/// failure, because RFC 8259 requires JSON text to be UTF-8 (section 7).
pub fn read_input(path: &Path) -> Result<String, Failure> {
    let bytes = fs::read(path).map_err(|source| Failure::ReadInput {
        path: path.to_path_buf(),
        source,
    })?;
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

/// `parse_page`, which vets, followed by a second parse into `serde_json::Value`.
pub fn load_document(json_text: &str) -> Result<LoadedDocument, Failure> {
    let page = parse_page(json_text)?;
    let document: Value = serde_json::from_str(json_text).map_err(Failure::DocumentValue)?;
    Ok(LoadedDocument { page, document })
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

/// Lays out with `CosmicTextMeasurer` and renders all three outputs in memory.
pub fn render_page(loaded: &LoadedDocument, scale: DeviceScale) -> Result<RenderedPage, Failure> {
    let mut measurer = CosmicTextMeasurer::new()?;
    let geometry = layout_page(&loaded.page, &mut measurer)?;
    let svg = render_svg(&loaded.page, &geometry)?;
    let png = render_png(&svg.svg, svg.text_elements, scale)?;
    let measured = measured_json(&loaded.document, &geometry);
    Ok(RenderedPage {
        geometry,
        svg,
        png,
        measured,
    })
}

/// The two geometry-free checks, in `CheckName` order.
pub fn model_checks(page: &Page) -> [CheckReport; 2] {
    [remembered_constants(page), legend_consistency(page)]
}

/// All five checks, in `CheckName` order.
pub fn all_checks(page: &Page, geometry: &PageGeometry) -> [CheckReport; 5] {
    let [remembered, legend] = model_checks(page);
    [
        child_inside_container(geometry),
        siblings_do_not_overlap(geometry),
        text_fits_box(geometry),
        remembered,
        legend,
    ]
}

/// Creates `out_dir` when missing and writes SVG, PNG and measured JSON, replacing existing
/// files. The measured JSON is serialized before anything touches the disk, an output that
/// resolves to `input` is refused, and the three files are staged under temporary names and
/// renamed only after every write succeeded (section 7).
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

    let mut staged: Vec<(PathBuf, &Path)> = Vec::with_capacity(outputs.len());
    for (name, path, bytes) in &outputs {
        let temporary = directory.join(temporary_name(name));
        if let Err(source) = write_new_file(&temporary, bytes) {
            remove_staged(&staged);
            let _ = fs::remove_file(&temporary);
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

/// `.<name>.<process id>.tmp`, in the output directory so the rename stays on one file
/// system.
fn temporary_name(name: &OsString) -> OsString {
    let mut temporary = OsString::from(".");
    temporary.push(name);
    temporary.push(format!(".{}.tmp", std::process::id()));
    temporary
}

/// `create_new` refuses to open through a link planted at the temporary name; a leftover
/// file from an earlier run of the same process id is removed first.
fn write_new_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut file = fs::File::options()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Best effort: the write error that led here is the one reported.
fn remove_staged(staged: &[(PathBuf, &Path)]) {
    for (temporary, _) in staged {
        let _ = fs::remove_file(temporary);
    }
}
