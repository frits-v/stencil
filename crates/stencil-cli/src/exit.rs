//! Section 7 exit codes. Every match below except the clap one names each variant, so a new
//! error variant does not compile until it has a code.

use clap::error::ErrorKind;
use stencil_layout::LayoutError;
use stencil_model::ModelError;
use stencil_model::checks::CheckReport;
use stencil_model::text::MeasureError;
use stencil_render::RenderError;
use stencil_text::FontError;

use crate::pipeline::Failure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Clean = 0,
    Defects = 1,
    CouldNotRun = 2,
}

pub fn failure_exit_code(failure: &Failure) -> ExitCode {
    match failure {
        Failure::Model(error) => model_exit_code(error),
        Failure::Fonts(error) => font_exit_code(error),
        Failure::Layout(error) => layout_exit_code(error),
        Failure::Render(error) => render_exit_code(error),
        Failure::ReadInput { .. }
        | Failure::InputStem { .. }
        | Failure::OutputIsInput { .. }
        | Failure::DocumentValue(_)
        | Failure::Serialize { .. }
        | Failure::CreateOutputDirectory { .. }
        | Failure::WriteOutput { .. } => ExitCode::CouldNotRun,
    }
}

pub fn model_exit_code(error: &ModelError) -> ExitCode {
    match error {
        ModelError::Json { .. } | ModelError::Invalid(_) => ExitCode::Defects,
    }
}

pub fn layout_exit_code(error: &LayoutError) -> ExitCode {
    match error {
        LayoutError::Invalid(_) => ExitCode::Defects,
        LayoutError::Measure { source, .. } => measure_exit_code(source),
        LayoutError::Taffy { .. } | LayoutError::NonFinite { .. } => ExitCode::CouldNotRun,
    }
}

/// Only a missing glyph is a defect of the document. The other variants cannot follow a
/// vetted page and a clamped width, so reaching one is an internal fault.
pub fn measure_exit_code(error: &MeasureError) -> ExitCode {
    match error {
        MeasureError::MissingGlyph { .. } => ExitCode::Defects,
        MeasureError::EmptyText
        | MeasureError::InvalidStyle { .. }
        | MeasureError::InvalidMaxWidth { .. }
        | MeasureError::Backend { .. } => ExitCode::CouldNotRun,
    }
}

pub fn font_exit_code(error: &FontError) -> ExitCode {
    match error {
        FontError::Unparseable { .. }
        | FontError::FamilyMismatch { .. }
        | FontError::WeightMismatch { .. } => ExitCode::CouldNotRun,
    }
}

pub fn render_exit_code(error: &RenderError) -> ExitCode {
    match error {
        RenderError::Fonts(font_error) => font_exit_code(font_error),
        RenderError::ScaleOutOfRange { .. }
        | RenderError::GeometryMismatch { .. }
        | RenderError::Svg { .. }
        | RenderError::TextNotRendered { .. }
        | RenderError::TextCountExceeded { .. }
        | RenderError::FontNotResolved { .. }
        | RenderError::PixmapAllocation { .. }
        | RenderError::PngEncode { .. } => ExitCode::CouldNotRun,
    }
}

/// clap's `ErrorKind` is `#[non_exhaustive]`, so this is the one match with a wildcard arm.
pub fn clap_exit_code(kind: ErrorKind) -> ExitCode {
    match kind {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => ExitCode::Clean,
        _ => ExitCode::CouldNotRun,
    }
}

/// Clean only when there is at least one report and every report passed; a report that
/// examined nothing fails, and so does an empty list.
pub fn reports_exit_code(reports: &[CheckReport]) -> ExitCode {
    if !reports.is_empty() && reports.iter().all(CheckReport::passed) {
        ExitCode::Clean
    } else {
        ExitCode::Defects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stencil_model::checks::{CheckName, Defect};
    use stencil_model::pointer::NodePointer;

    fn missing_glyph() -> MeasureError {
        MeasureError::MissingGlyph {
            family: "Inter",
            character: '\u{4E00}',
            codepoint: 0x4E00,
        }
    }

    #[test]
    fn exit_codes_have_the_section_seven_numbers() {
        assert_eq!(ExitCode::Clean as i32, 0);
        assert_eq!(ExitCode::Defects as i32, 1);
        assert_eq!(ExitCode::CouldNotRun as i32, 2);
    }

    #[test]
    fn model_errors_are_defects() {
        let json = ModelError::Json {
            line: 1,
            column: 1,
            message: "expected value".to_string(),
        };
        assert_eq!(model_exit_code(&json), ExitCode::Defects);
        assert_eq!(
            model_exit_code(&ModelError::Invalid(Vec::new())),
            ExitCode::Defects
        );
    }

    #[test]
    fn a_measure_error_is_decided_by_its_source() {
        let measure = |source| LayoutError::Measure {
            pointer: NodePointer::root().child("title"),
            source,
        };
        assert_eq!(
            layout_exit_code(&measure(missing_glyph())),
            ExitCode::Defects
        );
        for internal_fault in [
            MeasureError::EmptyText,
            MeasureError::InvalidStyle { reason: "size" },
            MeasureError::InvalidMaxWidth { max_width_px: -1.0 },
            MeasureError::Backend {
                message: "shaping failed".to_string(),
            },
        ] {
            assert_eq!(
                layout_exit_code(&measure(internal_fault)),
                ExitCode::CouldNotRun
            );
        }
    }

    #[test]
    fn layout_invalid_is_a_defect_and_taffy_faults_could_not_run() {
        assert_eq!(
            layout_exit_code(&LayoutError::Invalid(Vec::new())),
            ExitCode::Defects
        );
        let taffy = LayoutError::Taffy {
            pointer: NodePointer::root(),
            message: "node missing".to_string(),
        };
        assert_eq!(layout_exit_code(&taffy), ExitCode::CouldNotRun);
        let non_finite = LayoutError::NonFinite {
            pointer: NodePointer::root(),
        };
        assert_eq!(layout_exit_code(&non_finite), ExitCode::CouldNotRun);
    }

    #[test]
    fn font_and_render_errors_could_not_run() {
        let font_errors = || {
            [
                FontError::Unparseable {
                    file_name: "Inter-Bold.ttf",
                },
                FontError::FamilyMismatch {
                    file_name: "Inter-Bold.ttf",
                    found: "Inter ExtraBold".to_string(),
                },
                FontError::WeightMismatch {
                    file_name: "Inter-Bold.ttf",
                    expected: 700,
                    found: 800,
                },
            ]
        };
        for font_error in font_errors() {
            assert_eq!(font_exit_code(&font_error), ExitCode::CouldNotRun);
        }
        for font_error in font_errors() {
            assert_eq!(
                render_exit_code(&RenderError::Fonts(font_error)),
                ExitCode::CouldNotRun
            );
        }
        for render_error in [
            RenderError::ScaleOutOfRange { value: 5 },
            RenderError::GeometryMismatch {
                expected: NodePointer::root().child("title"),
                found: NodePointer::root().child("kicker"),
            },
            RenderError::Svg {
                message: "unexpected end".to_string(),
            },
            RenderError::TextNotRendered { count: 1 },
            RenderError::TextCountExceeded {
                expected: 0,
                found: 1,
            },
            RenderError::FontNotResolved { lookups: 2 },
            RenderError::PixmapAllocation {
                width: 0,
                height: 0,
            },
            RenderError::PngEncode {
                message: "encoder failed".to_string(),
            },
        ] {
            assert_eq!(render_exit_code(&render_error), ExitCode::CouldNotRun);
        }
    }

    #[test]
    fn failures_outside_the_document_could_not_run() {
        let io_error = || std::io::Error::other("denied");
        for failure in [
            Failure::ReadInput {
                path: "missing.json".into(),
                source: io_error(),
            },
            Failure::InputStem { path: "/".into() },
            Failure::OutputIsInput {
                path: "out/g7.svg".into(),
            },
            Failure::CreateOutputDirectory {
                path: "out".into(),
                source: io_error(),
            },
            Failure::WriteOutput {
                path: "out/g7.svg".into(),
                source: io_error(),
            },
        ] {
            assert_eq!(failure_exit_code(&failure), ExitCode::CouldNotRun);
        }
        let layout_defect = Failure::Layout(LayoutError::Measure {
            pointer: NodePointer::root().child("title"),
            source: missing_glyph(),
        });
        assert_eq!(failure_exit_code(&layout_defect), ExitCode::Defects);
    }

    #[test]
    fn clap_help_and_version_are_clean_and_other_kinds_could_not_run() {
        assert_eq!(clap_exit_code(ErrorKind::DisplayHelp), ExitCode::Clean);
        assert_eq!(clap_exit_code(ErrorKind::DisplayVersion), ExitCode::Clean);
        assert_eq!(
            clap_exit_code(ErrorKind::InvalidSubcommand),
            ExitCode::CouldNotRun
        );
        assert_eq!(
            clap_exit_code(ErrorKind::UnknownArgument),
            ExitCode::CouldNotRun
        );
        assert_eq!(
            clap_exit_code(ErrorKind::MissingSubcommand),
            ExitCode::CouldNotRun
        );
    }

    #[test]
    fn reports_are_clean_only_when_every_report_passed() {
        let report = |examined, defects| CheckReport {
            check: CheckName::TextFitsBox,
            examined,
            defects,
        };
        let defect = Defect {
            pointer: NodePointer::root().child("title"),
            message: "title \"x\" measured 2.00x1.00 in box 1.00x1.00".to_string(),
        };
        assert_eq!(reports_exit_code(&[report(3, Vec::new())]), ExitCode::Clean);
        assert_eq!(reports_exit_code(&[]), ExitCode::Defects);
        assert_eq!(
            reports_exit_code(&[report(3, Vec::new()), report(0, Vec::new())]),
            ExitCode::Defects
        );
        assert_eq!(
            reports_exit_code(&[report(3, vec![defect])]),
            ExitCode::Defects
        );
    }
}
