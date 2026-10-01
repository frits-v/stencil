//! The legend label rule of section 13.4 rule 4: a drawn label may run past the canonical
//! label layout measured by at most LEGEND_RELABEL_SLACK_PX.

use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, TextStyleName};
use stencil_model::theme::{LEGEND_RELABEL_SLACK_PX, TintCue};
use stencil_model::{
    Line, TINT_SLOTS, Theme, ThemeRule, ThemeViolation, drawn_legend_label, legend_label,
};

/// Every (line, tint) a legend entry can name: gray, deny, and solid and dash in each slot.
fn line_uses() -> Vec<(Line, Option<u8>)> {
    let mut uses = vec![(Line::Gray, None), (Line::Deny, None)];
    for line in [Line::Solid, Line::Dash] {
        uses.extend((1..=TINT_SLOTS).map(|slot| (line, Some(slot))));
    }
    uses
}

/// Measures in the `legend_label` style every label rule 9 draws for this theme and the
/// canonical label of the same (line, tint), and reports each drawn label wider than its
/// canonical label by more than LEGEND_RELABEL_SLACK_PX: at `/tints/<i>/name` when the
/// slot name made it, at `/tint_cue` otherwise.
pub fn theme_legend_labels(
    theme: &Theme,
    measurer: &mut dyn TextMeasurer,
) -> Result<Vec<ThemeViolation>, MeasureError> {
    let style = TextStyleName::LegendLabel.text_style().style;
    let mut violations = Vec::new();
    for (line, tint) in line_uses() {
        let drawn = drawn_legend_label(theme, line, tint);
        let canonical = legend_label(line, tint);
        if drawn == canonical {
            continue;
        }
        let drawn_width = measurer.measure(&drawn, &style, None)?.width_px;
        let canonical_width = measurer.measure(canonical, &style, None)?.width_px;
        let excess = drawn_width - canonical_width;
        if excess <= LEGEND_RELABEL_SLACK_PX {
            continue;
        }
        let named_by_slot =
            theme.tint_cue == TintCue::Color && matches!(line, Line::Solid | Line::Dash);
        let pointer = match (named_by_slot, tint) {
            (true, Some(slot)) => NodePointer::root()
                .child("tints")
                .index(usize::from(slot.saturating_sub(1)))
                .child("name"),
            _ => NodePointer::root().child("tint_cue"),
        };
        violations.push(ThemeViolation {
            pointer,
            rule: ThemeRule::LegendLabelTooWide,
            message: format!(
                "legend label \"{drawn}\" runs {excess:.2} px past \"{canonical}\", above {LEGEND_RELABEL_SLACK_PX} px"
            ),
        });
    }
    Ok(violations)
}
