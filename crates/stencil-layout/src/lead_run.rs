//! A lead run: a lead in a bold style and a body in a regular style of the same size and
//! line height that continues on the lead's last line (section 3).

use stencil_model::text::{MeasureError, TextMeasurer, TextMetrics, TextStyle};

/// U+00A0 measures as the advance of one inter-word space, letter spacing included; a lone
/// U+0020 measures 0 because a line's width excludes its trailing spaces.
const SPACE_PROBE: &str = "\u{a0}";

#[derive(Debug, Clone, PartialEq)]
pub struct LeadRun {
    /// The lead measured at the run's width, its lines starting at the run's left edge.
    pub lead: TextMetrics,
    /// The body measured at the run's width with `body_indent_px` on its first line. Its
    /// `width_px` includes the indent when its line 0 holds text.
    pub body: TextMetrics,
    /// The width of the lead's last line plus one space in the body style, from the run's
    /// left edge.
    pub body_indent_px: f32,
    /// The top of the body below the top of the run: the top of the lead's last line.
    pub body_top_px: f32,
    pub width_px: f32,
    pub height_px: f32,
}

impl LeadRun {
    /// The x of a body line from the run's left edge: the indent for line 0, 0 after it.
    pub fn body_line_x_px(&self, line_index: usize) -> f32 {
        if line_index == 0 {
            self.body_indent_px
        } else {
            0.0
        }
    }

    /// True when the body's first word sits on the lead's last line, false when it starts
    /// on the next line and the body's line 0 is empty.
    pub fn body_continues_lead_line(&self) -> bool {
        self.body
            .lines
            .first()
            .is_some_and(|line| line.byte_start < line.byte_end)
    }
}

/// Measures `lead` at `max_width_px`, then `body` at `max_width_px` with a first-line indent
/// of the lead's last line plus one space in `body_style`. The two styles must share size
/// and line height, or the body's first line could not share the lead's last line.
pub fn measure_lead_run(
    measurer: &mut dyn TextMeasurer,
    lead: &str,
    lead_style: &TextStyle,
    body: &str,
    body_style: &TextStyle,
    max_width_px: Option<f32>,
) -> Result<LeadRun, MeasureError> {
    if lead_style.size_px != body_style.size_px
        || lead_style.line_height_px != body_style.line_height_px
    {
        return Err(MeasureError::InvalidStyle {
            reason: "a lead and its body differ in size or line height",
        });
    }
    let lead_metrics = measurer.measure(lead, lead_style, max_width_px)?;
    let lead_last_line = lead_metrics
        .lines
        .last()
        .ok_or_else(|| MeasureError::Backend {
            message: "the lead measured no lines".to_string(),
        })?;
    let space_px = measurer.measure(SPACE_PROBE, body_style, None)?.width_px;
    let body_indent_px = lead_last_line.width_px + space_px;
    let body_metrics = measurer.measure_indented(body, body_style, max_width_px, body_indent_px)?;
    let body_top_px = lead_metrics.height_px - lead_style.line_height_px;
    Ok(LeadRun {
        width_px: lead_metrics.width_px.max(body_metrics.width_px),
        height_px: body_top_px + body_metrics.height_px,
        lead: lead_metrics,
        body: body_metrics,
        body_indent_px,
        body_top_px,
    })
}
