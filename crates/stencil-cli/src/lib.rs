//! The `stencil` command line (SPEC sections 4.6, 7 and 13.12): `vet`, `render`, `check`,
//! `schema`, `prime`, `gallery` and `theme`. `run` holds the whole program so integration tests call it directly.

pub mod pipeline;
pub mod prime;

mod exit;
mod gallery;
mod report;

use std::error::Error;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use stencil_layout::LayoutError;
use stencil_layout::checks::PrintWidth;
use stencil_model::checks::CheckReport;
use stencil_model::{ModelError, Projection};
use stencil_render::DeviceScale;

pub use exit::ExitCode;

use exit::{clap_exit_code, failure_exit_code, prime_exit_code, reports_exit_code};
use pipeline::{
    Failure, LoadedDocument, OutputPaths, ThemeChoice, all_checks, grammar_violations,
    load_document_from, model_checks, output_names, print_fit_report, read_input, render_page,
    theme_violations, write_outputs,
};
use report::{
    check_counts_text, grammar_violation_line, report_lines, theme_violation_line,
    violation_count_text, violation_line,
};

mod theme_command;

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
    /// Parse and vet a document, then run the three model checks
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
        /// A built-in theme name or a theme file path ending in .json, overriding the
        /// document's `theme`
        #[arg(long, value_name = "THEME")]
        theme: Option<String>,
        /// Projection, overriding the document's `projection`
        #[arg(long, value_enum)]
        projection: Option<ProjectionArgument>,
        /// Print width in inches, 0.5 to 200; runs print-fit after the files are written
        #[arg(long, value_name = "INCHES", value_parser = parse_print_width)]
        print_width: Option<PrintWidth>,
    },
    /// Lay out and render in memory, then run all twelve checks
    Check {
        /// Document to check
        json: PathBuf,
        /// A built-in theme name or a theme file path ending in .json, overriding the
        /// document's `theme`
        #[arg(long, value_name = "THEME")]
        theme: Option<String>,
        /// Projection, overriding the document's `projection`
        #[arg(long, value_enum)]
        projection: Option<ProjectionArgument>,
        /// Print width in inches, 0.5 to 200; makes print-fit applicable
        #[arg(long, value_name = "INCHES", value_parser = parse_print_width)]
        print_width: Option<PrintWidth>,
    },
    /// Print the document JSON Schema
    Schema,
    /// Print the authoring briefing for an agent, or one deeper topic
    Prime {
        /// themes, links, blocks, layout, checks, cue, example, or grammar
        topic: Option<String>,
        /// The grammar to brief on, after `grammar`: gcp or plain
        name: Option<String>,
    },
    /// Render and check every example under every theme, with index.html and gallery.json
    Gallery {
        /// Directory for the gallery, created when missing
        out_dir: PathBuf,
        /// Directory whose .json files are rendered
        #[arg(long, value_name = "DIR", default_value = "examples")]
        examples: PathBuf,
    },
    /// Print a built-in theme, or check a theme's contrast and separation
    Theme {
        #[command(subcommand)]
        action: ThemeAction,
    },
}

#[derive(Debug, Subcommand)]
enum ThemeAction {
    /// Print a built-in theme's JSON exactly as embedded
    Show {
        /// center, paper, dusk, clear, clear-dark or wire
        name: String,
    },
    /// Load a theme and print its contrast and separation rows
    Check {
        /// A built-in theme name or a theme file path ending in .json
        theme: String,
    },
}

/// `--print-width`: a decimal number of inches that `PrintWidth` accepts (section 13.10).
/// Anything else is a clap value error, exit 2.
fn parse_print_width(text: &str) -> Result<PrintWidth, String> {
    let inches: f32 = text
        .trim()
        .parse()
        .map_err(|_| format!("{text:?} is not a number of inches"))?;
    PrintWidth::new(inches).map_err(|error| error.to_string())
}

/// The `--projection` values, one per `Projection` variant (section 12.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ProjectionArgument {
    Flat,
    Iso,
}

impl From<ProjectionArgument> for Projection {
    fn from(argument: ProjectionArgument) -> Self {
        match argument {
            ProjectionArgument::Flat => Projection::Flat,
            ProjectionArgument::Iso => Projection::Iso,
        }
    }
}

/// The `--theme` and `--projection` overrides of `render` and `check`.
#[derive(Debug, Clone, Default)]
struct Overrides {
    theme: Option<String>,
    projection: Option<ProjectionArgument>,
}

/// Parses and vets the input with the `--theme` reference, resolved against the current
/// directory, in place of the page's, then applies the projection override to the page. The
/// JSON value that the measured JSON carries stays the input as written.
fn load_overridden_document(path: &Path, overrides: &Overrides) -> Result<LoadedDocument, Failure> {
    let theme_flag = overrides.theme.as_deref().map(|reference| ThemeChoice {
        reference,
        directory: Path::new("."),
    });
    let mut loaded = load_document_at(path, theme_flag)?;
    if let Some(projection) = overrides.projection {
        loaded.page.projection = Projection::from(projection);
    }
    Ok(loaded)
}

/// Reads and loads the document at `path`, resolving a grammar or theme path the page names
/// against its directory.
fn load_document_at(
    path: &Path,
    theme_flag: Option<ThemeChoice<'_>>,
) -> Result<LoadedDocument, Failure> {
    let json_text = read_input(path)?;
    let directory = path.parent().unwrap_or(Path::new("."));
    load_document_from(&json_text, directory, theme_flag)
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
            projection,
            print_width,
        } => render(
            &json,
            &out_dir,
            scale,
            print_width,
            &Overrides { theme, projection },
            stdout,
            stderr,
        ),
        Command::Check {
            json,
            theme,
            projection,
            print_width,
        } => check(
            &json,
            &Overrides { theme, projection },
            print_width,
            stdout,
            stderr,
        ),
        Command::Schema => schema(stdout, stderr),
        Command::Prime { topic, name } => prime(topic.as_deref(), name.as_deref(), stdout, stderr),
        Command::Gallery { out_dir, examples } => {
            gallery::gallery(&out_dir, &examples, stdout, stderr)
        }
        Command::Theme { action } => match action {
            ThemeAction::Show { name } => theme_command::show(&name, stdout, stderr),
            ThemeAction::Check { theme } => theme_command::check(&theme, stdout, stderr),
        },
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
    let loaded = match load_document_at(path, None) {
        Ok(loaded) => loaded,
        Err(failure) => return report_failure("vet", &failure, stdout, stderr),
    };
    let reports = model_checks(&loaded.page, &loaded.grammar);
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

/// Writes the three outputs and prints their paths. With a print width, print-fit runs
/// after the files are written: its check and defect lines follow the paths, and a failing
/// report exits 1 with the files kept (section 13.10).
fn render(
    path: &Path,
    out_dir: &Path,
    scale: u8,
    print_width: Option<PrintWidth>,
    overrides: &Overrides,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    match render_to_disk(path, out_dir, scale, overrides, print_width) {
        Ok((paths, print_report)) => {
            for written in [&paths.svg, &paths.png, &paths.measured] {
                writeln!(stdout, "{}", written.display())?;
            }
            let Some(report) = print_report else {
                return Ok(ExitCode::Clean);
            };
            let reports = [report];
            for line in report_lines(&reports) {
                writeln!(stdout, "{line}")?;
            }
            Ok(reports_exit_code(&reports))
        }
        Err(failure) => report_failure("render", &failure, stdout, stderr),
    }
}

/// Everything is computed before the first write, so a failing document leaves no files.
fn render_to_disk(
    path: &Path,
    out_dir: &Path,
    scale: u8,
    overrides: &Overrides,
    print_width: Option<PrintWidth>,
) -> Result<(OutputPaths, Option<CheckReport>), Failure> {
    let scale = DeviceScale::new(scale)?;
    let names = output_names(path)?;
    let loaded = load_overridden_document(path, overrides)?;
    let rendered = render_page(&loaded, scale)?;
    let paths = write_outputs(out_dir, &names, &rendered, path)?;
    let print_report = print_width
        .map(|width| print_fit_report(&rendered.geometry, rendered.scene.as_ref(), Some(width)));
    Ok((paths, print_report))
}

fn check(
    path: &Path,
    overrides: &Overrides,
    print_width: Option<PrintWidth>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    let reports = match check_in_memory(path, overrides, print_width) {
        Ok(reports) => reports,
        Err(failure) => return report_failure("check", &failure, stdout, stderr),
    };
    for line in report_lines(&reports) {
        writeln!(stdout, "{line}")?;
    }
    writeln!(stdout, "stencil check: {}", check_counts_text(&reports))?;
    Ok(reports_exit_code(&reports))
}

fn check_in_memory(
    path: &Path,
    overrides: &Overrides,
    print_width: Option<PrintWidth>,
) -> Result<[CheckReport; 12], Failure> {
    let loaded = load_overridden_document(path, overrides)?;
    let rendered = render_page(&loaded, DeviceScale::DEFAULT)?;
    Ok(all_checks(
        &loaded.page,
        &loaded.grammar,
        &rendered.geometry,
        rendered.scene.as_ref(),
        print_width,
    ))
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

/// The base briefing, or the named topic. An unknown topic is a usage error: one line on
/// stderr naming the topics, exit 2.
/// The topic name that takes a grammar name after it.
const GRAMMAR_TOPIC: &str = "grammar";

fn prime(
    topic: Option<&str>,
    name: Option<&str>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    if topic == Some(GRAMMAR_TOPIC) {
        return prime_grammar(name, stdout, stderr);
    }
    if let Some(extra) = name {
        writeln!(
            stderr,
            "stencil prime: unexpected argument {extra:?}; only `stencil prime grammar <name>` takes a second argument"
        )?;
        return Ok(ExitCode::CouldNotRun);
    }
    let Some(topic_name) = topic else {
        return match prime::base_text() {
            Ok(text) => {
                write!(stdout, "{text}")?;
                Ok(ExitCode::Clean)
            }
            Err(error) => {
                writeln!(stderr, "stencil prime: {error}")?;
                Ok(prime_exit_code(&error))
            }
        };
    };
    match prime::Topic::from_name(topic_name) {
        Some(topic) => {
            write!(stdout, "{}", topic.text())?;
            Ok(ExitCode::Clean)
        }
        None => {
            writeln!(
                stderr,
                "stencil prime: unknown topic {topic_name:?}; topics: {}",
                prime::topic_names()
            )?;
            Ok(ExitCode::CouldNotRun)
        }
    }
}

/// `stencil prime grammar <name>`: a built-in grammar's briefing. A missing or unknown name
/// writes one line naming the grammars to stderr and exits 2.
fn prime_grammar(
    name: Option<&str>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    match name.and_then(prime::grammar_text) {
        Some(Ok(text)) => {
            write!(stdout, "{text}")?;
            Ok(ExitCode::Clean)
        }
        Some(Err(error)) => {
            writeln!(stderr, "stencil prime: {error}")?;
            Ok(prime_exit_code(&error))
        }
        None => {
            let named = name.map_or_else(|| "no grammar".to_string(), |name| format!("{name:?}"));
            writeln!(
                stderr,
                "stencil prime grammar: unknown grammar {named}; grammars: {}",
                prime::grammar_names()
            )?;
            Ok(ExitCode::CouldNotRun)
        }
    }
}

/// A defect of the document (exit 1) is reported on stdout in the section 7 formats. A
/// failure to run (exit 2) goes to stderr with its error chain, and stdout stays empty.
pub(crate) fn report_failure(
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
        Failure::Grammar(error) => {
            writeln!(stdout, "error {error}")?;
            for violation in grammar_violations(error) {
                writeln!(stdout, "{}", grammar_violation_line(violation))?;
            }
            writeln!(stdout, "stencil {command}: checks not run")
        }
        Failure::Theme(error) => {
            writeln!(stdout, "error {error}")?;
            for violation in theme_violations(error) {
                writeln!(stdout, "{}", theme_violation_line(violation))?;
            }
            writeln!(stdout, "stencil {command}: checks not run")
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
