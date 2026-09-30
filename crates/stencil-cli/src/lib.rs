//! The `stencil` command line (SPEC sections 4.6 and 7): `vet`, `render`, `check` and
//! `schema`. `run` holds the whole program so integration tests call it directly.

pub mod pipeline;

mod exit;
mod report;

use std::error::Error;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use stencil_layout::LayoutError;
use stencil_model::checks::CheckReport;
use stencil_model::{ModelError, Theme};
use stencil_render::DeviceScale;

pub use exit::ExitCode;

use exit::{clap_exit_code, failure_exit_code, reports_exit_code};
use pipeline::{
    Failure, LoadedDocument, OutputPaths, all_checks, load_document, model_checks, output_names,
    read_input, render_page, write_outputs,
};
use report::{check_counts_text, report_lines, violation_count_text, violation_line};

/// Upper bound on the `source()` links printed for one error.
const ERROR_CHAIN_MAX: usize = 16;

#[derive(Debug, Parser)]
#[command(
    name = "stencil",
    version,
    about = "Render architecture figures from a structural JSON document"
)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse and vet a document, then run the two model checks
    Vet {
        /// Document to vet
        json: PathBuf,
    },
    /// Write <stem>.svg, <stem>.png and <stem>.measured.json
    Render {
        /// Document to render
        json: PathBuf,
        /// Directory for the three outputs, created when missing
        #[arg(long, value_name = "DIR")]
        out_dir: PathBuf,
        /// PNG device scale, 1 to 4
        #[arg(long, default_value_t = DeviceScale::DEFAULT.get())]
        scale: u8,
        /// Color theme, overriding the document's `theme`
        #[arg(long, value_enum)]
        theme: Option<ThemeArgument>,
    },
    /// Lay out and render in memory, then run all seven checks
    Check {
        /// Document to check
        json: PathBuf,
        /// Color theme, overriding the document's `theme`
        #[arg(long, value_enum)]
        theme: Option<ThemeArgument>,
    },
    /// Print the document JSON Schema
    Schema,
}

/// The `--theme` values, one per `Theme` variant (section 11.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ThemeArgument {
    Center,
    Dusk,
    Wire,
}

impl From<ThemeArgument> for Theme {
    fn from(argument: ThemeArgument) -> Self {
        match argument {
            ThemeArgument::Center => Theme::Center,
            ThemeArgument::Dusk => Theme::Dusk,
            ThemeArgument::Wire => Theme::Wire,
        }
    }
}

/// Parses and vets the input, then applies a `--theme` override to the page. The JSON value
/// that the measured JSON carries stays the input as written.
fn load_themed_document(
    path: &Path,
    theme: Option<ThemeArgument>,
) -> Result<LoadedDocument, Failure> {
    let mut loaded = load_document(&read_input(path)?)?;
    if let Some(theme) = theme {
        loaded.page.theme = Theme::from(theme);
    }
    Ok(loaded)
}

pub fn run(arguments: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> ExitCode {
    let outcome = match Arguments::try_parse_from(arguments) {
        Ok(parsed) => run_command(parsed.command, stdout, stderr),
        Err(error) => report_usage(&error, stdout, stderr),
    };
    let flushed = outcome.and_then(|code| {
        stdout.flush()?;
        Ok(code)
    });
    match flushed {
        Ok(code) => code,
        Err(error) => {
            // An output stream failed; stderr is the last place left to say so.
            let _ = writeln!(stderr, "stencil: cannot write output: {error}");
            ExitCode::CouldNotRun
        }
    }
}

fn run_command(
    command: Command,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    match command {
        Command::Vet { json } => vet(&json, stdout, stderr),
        Command::Render {
            json,
            out_dir,
            scale,
            theme,
        } => render(&json, &out_dir, scale, theme, stdout, stderr),
        Command::Check { json, theme } => check(&json, theme, stdout, stderr),
        Command::Schema => schema(stdout, stderr),
    }
}

/// `--help` and `--version` go to stdout with exit 0; every other clap error to stderr.
fn report_usage(
    error: &clap::Error,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    let code = clap_exit_code(error.kind());
    let text = error.render().to_string();
    if code == ExitCode::Clean {
        write!(stdout, "{text}")?;
    } else {
        write!(stderr, "{text}")?;
    }
    Ok(code)
}

fn vet(path: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> io::Result<ExitCode> {
    let loaded = match read_input(path).and_then(|json_text| load_document(&json_text)) {
        Ok(loaded) => loaded,
        Err(failure) => return report_failure("vet", &failure, stdout, stderr),
    };
    let reports = model_checks(&loaded.page);
    for line in report_lines(&reports) {
        writeln!(stdout, "{line}")?;
    }
    writeln!(
        stdout,
        "stencil vet: {}, {}",
        violation_count_text(0),
        check_counts_text(&reports)
    )?;
    Ok(reports_exit_code(&reports))
}

fn render(
    path: &Path,
    out_dir: &Path,
    scale: u8,
    theme: Option<ThemeArgument>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    match render_to_disk(path, out_dir, scale, theme) {
        Ok(paths) => {
            for written in [&paths.svg, &paths.png, &paths.measured] {
                writeln!(stdout, "{}", written.display())?;
            }
            Ok(ExitCode::Clean)
        }
        Err(failure) => report_failure("render", &failure, stdout, stderr),
    }
}

/// Everything is computed before the first write, so a failing document leaves no files.
fn render_to_disk(
    path: &Path,
    out_dir: &Path,
    scale: u8,
    theme: Option<ThemeArgument>,
) -> Result<OutputPaths, Failure> {
    let scale = DeviceScale::new(scale)?;
    let names = output_names(path)?;
    let loaded = load_themed_document(path, theme)?;
    let rendered = render_page(&loaded, scale)?;
    write_outputs(out_dir, &names, &rendered, path)
}

fn check(
    path: &Path,
    theme: Option<ThemeArgument>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    let reports = match check_in_memory(path, theme) {
        Ok(reports) => reports,
        Err(failure) => return report_failure("check", &failure, stdout, stderr),
    };
    for line in report_lines(&reports) {
        writeln!(stdout, "{line}")?;
    }
    writeln!(stdout, "stencil check: {}", check_counts_text(&reports))?;
    Ok(reports_exit_code(&reports))
}

fn check_in_memory(path: &Path, theme: Option<ThemeArgument>) -> Result<[CheckReport; 7], Failure> {
    let loaded = load_themed_document(path, theme)?;
    let rendered = render_page(&loaded, DeviceScale::DEFAULT)?;
    Ok(all_checks(&loaded.page, &rendered.geometry))
}

fn schema(stdout: &mut dyn Write, stderr: &mut dyn Write) -> io::Result<ExitCode> {
    match serde_json::to_string_pretty(&stencil_model::page_schema()) {
        Ok(schema_text) => {
            writeln!(stdout, "{schema_text}")?;
            Ok(ExitCode::Clean)
        }
        Err(source) => {
            let failure = Failure::Serialize {
                what: "schema",
                source,
            };
            report_failure("schema", &failure, stdout, stderr)
        }
    }
}

/// A defect of the document (exit 1) is reported on stdout in the section 7 formats. A
/// failure to run (exit 2) goes to stderr with its error chain, and stdout stays empty.
fn report_failure(
    command: &str,
    failure: &Failure,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    let code = failure_exit_code(failure);
    if code == ExitCode::Defects {
        write_document_defect(command, failure, stdout)?;
    } else {
        write_error_chain(command, failure, stderr)?;
    }
    Ok(code)
}

fn write_document_defect(
    command: &str,
    failure: &Failure,
    stdout: &mut dyn Write,
) -> io::Result<()> {
    match failure {
        Failure::Model(ModelError::Json { .. }) => {
            writeln!(stdout, "error {failure}")?;
            writeln!(
                stdout,
                "stencil {command}: document does not parse, checks not run"
            )
        }
        Failure::Model(ModelError::Invalid(violations))
        | Failure::Layout(LayoutError::Invalid(violations)) => {
            for violation in violations {
                writeln!(stdout, "{}", violation_line(violation))?;
            }
            writeln!(
                stdout,
                "stencil {command}: {}, checks not run",
                violation_count_text(violations.len())
            )
        }
        other => {
            writeln!(stdout, "error {other}")?;
            writeln!(stdout, "stencil {command}: checks not run")
        }
    }
}

fn write_error_chain(command: &str, failure: &Failure, stderr: &mut dyn Write) -> io::Result<()> {
    writeln!(stderr, "stencil {command}: {failure}")?;
    let mut cause = failure.source();
    for _ in 0..ERROR_CHAIN_MAX {
        let Some(current) = cause else {
            break;
        };
        writeln!(stderr, "  caused by: {current}")?;
        cause = current.source();
    }
    Ok(())
}
