//! Text colors of section 2.9, keyed by `TextStyleName`. The style values themselves are
//! `stencil_model::text::TEXT_STYLES`.

use stencil_model::Line;
use stencil_model::text::{TextStyle, TextStyleName};

use crate::{ISO_FRAME_LABEL_SCALE, ISO_TYPE_SCALE, ISO_ZONE_LABEL_SCALE};

pub const TEXT_DARK: &str = "#202124";
pub const TEXT_MUTED: &str = "#5F6368";
pub const TEXT_BLUE: &str = "#1A73E8";
pub const TEXT_WHITE: &str = "#FFFFFF";
pub const TEXT_AMBER: &str = "#B06000";
pub const TEXT_DENY: &str = "#C5221F";
pub const BADGE_TEXT: &str = "#174EA6";
/// Background of the badge before the page kicker.
pub const BADGE_FILL: &str = "#E8F0FE";

/// Text color of a style. `line` is the line of the Pipe, Tee or Link a `tag_label` run
/// belongs to, and `deny` turns it red. Other styles ignore it.
pub fn text_color(name: TextStyleName, line: Option<Line>) -> &'static str {
    match name {
        TextStyleName::Badge => BADGE_TEXT,
        TextStyleName::Kicker => TEXT_BLUE,
        TextStyleName::Title
        | TextStyleName::CardFunction
        | TextStyleName::LegendLabel
        | TextStyleName::BlockBody
        | TextStyleName::Wordmark
        | TextStyleName::Motto
        | TextStyleName::SheetTitle
        | TextStyleName::Statement
        | TextStyleName::ViewNumeral
        | TextStyleName::ViewTitle
        | TextStyleName::LeadBody
        | TextStyleName::ActorName
        | TextStyleName::LaneName
        | TextStyleName::StepTitle
        | TextStyleName::ListTitle
        | TextStyleName::MarkNumeral
        | TextStyleName::MarkNumeralSmall
        | TextStyleName::RungTitle => TEXT_DARK,
        TextStyleName::TagLabel => match line {
            Some(Line::Deny) => TEXT_DENY,
            Some(Line::Gray | Line::Solid | Line::Dash) | None => TEXT_DARK,
        },
        TextStyleName::GcpBar | TextStyleName::FlagNumeral => TEXT_WHITE,
        TextStyleName::PerimeterLabel | TextStyleName::Ask => TEXT_AMBER,
        TextStyleName::Lede
        | TextStyleName::ZoneLabel
        | TextStyleName::CardProduct
        | TextStyleName::Fact
        | TextStyleName::TagSub
        | TextStyleName::NoteLegend
        | TextStyleName::LegendText
        | TextStyleName::Foot
        | TextStyleName::SheetLabel
        | TextStyleName::SheetKicker
        | TextStyleName::ViewNote
        | TextStyleName::ViewNoteSmall
        | TextStyleName::StepText
        | TextStyleName::RungWho => TEXT_MUTED,
    }
}

/// The style a run is measured and drawn in: the named style flat; under iso the body
/// styles grown by `ISO_TYPE_SCALE`, a zone label by `ISO_ZONE_LABEL_SCALE`, and the frame
/// bar label by `ISO_FRAME_LABEL_SCALE`, a tier above the zone names.
/// Chrome, the legend and the design sheet styles keep their flat size; a sheet is never
/// laid out under iso.
pub(crate) fn text_style_for(name: TextStyleName, iso: bool) -> TextStyle {
    let flat = name.text_style().style;
    if !iso {
        return flat;
    }
    let scale = match name {
        TextStyleName::Badge
        | TextStyleName::Kicker
        | TextStyleName::Title
        | TextStyleName::Lede
        | TextStyleName::NoteLegend
        | TextStyleName::LegendLabel
        | TextStyleName::LegendText
        | TextStyleName::Foot
        | TextStyleName::SheetLabel
        | TextStyleName::SheetKicker
        | TextStyleName::Wordmark
        | TextStyleName::Motto
        | TextStyleName::SheetTitle
        | TextStyleName::Statement
        | TextStyleName::ViewNumeral
        | TextStyleName::ViewTitle
        | TextStyleName::ViewNote
        | TextStyleName::ViewNoteSmall
        | TextStyleName::LeadBody
        | TextStyleName::ActorName
        | TextStyleName::LaneName
        | TextStyleName::StepTitle
        | TextStyleName::StepText
        | TextStyleName::ListTitle
        | TextStyleName::MarkNumeral
        | TextStyleName::MarkNumeralSmall
        | TextStyleName::FlagNumeral
        | TextStyleName::RungTitle
        | TextStyleName::RungWho => return flat,
        TextStyleName::ZoneLabel | TextStyleName::PerimeterLabel => ISO_ZONE_LABEL_SCALE,
        TextStyleName::GcpBar => ISO_FRAME_LABEL_SCALE,
        TextStyleName::CardFunction
        | TextStyleName::CardProduct
        | TextStyleName::Fact
        | TextStyleName::Ask
        | TextStyleName::TagLabel
        | TextStyleName::TagSub
        | TextStyleName::BlockBody => ISO_TYPE_SCALE,
    };
    TextStyle {
        size_px: flat.size_px * scale,
        line_height_px: flat.line_height_px * scale,
        ..flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stencil_model::text::TEXT_STYLES;

    #[test]
    fn tag_label_is_red_only_for_deny() {
        for line in Line::ALL {
            let expected = if line == Line::Deny {
                "#C5221F"
            } else {
                "#202124"
            };
            assert_eq!(text_color(TextStyleName::TagLabel, Some(line)), expected);
        }
    }

    #[test]
    fn every_style_matches_the_section_2_9_table() {
        let expected = [
            ("badge", "#174EA6"),
            ("kicker", "#1A73E8"),
            ("title", "#202124"),
            ("lede", "#5F6368"),
            ("zone_label", "#5F6368"),
            ("gcp_bar", "#FFFFFF"),
            ("perimeter_label", "#B06000"),
            ("card_function", "#202124"),
            ("card_product", "#5F6368"),
            ("fact", "#5F6368"),
            ("ask", "#B06000"),
            ("tag_label", "#202124"),
            ("tag_sub", "#5F6368"),
            ("note_legend", "#5F6368"),
            ("legend_label", "#202124"),
            ("legend_text", "#5F6368"),
            ("foot", "#5F6368"),
            ("block_body", "#202124"),
            ("sheet_label", "#5F6368"),
            ("sheet_kicker", "#5F6368"),
            ("wordmark", "#202124"),
            ("motto", "#202124"),
            ("sheet_title", "#202124"),
            ("statement", "#202124"),
            ("view_numeral", "#202124"),
            ("view_title", "#202124"),
            ("view_note", "#5F6368"),
            ("view_note_small", "#5F6368"),
            ("lead_body", "#202124"),
            ("actor_name", "#202124"),
            ("lane_name", "#202124"),
            ("step_title", "#202124"),
            ("step_text", "#5F6368"),
            ("list_title", "#202124"),
            ("mark_numeral", "#202124"),
            ("mark_numeral_small", "#202124"),
            ("flag_numeral", "#FFFFFF"),
            ("rung_title", "#202124"),
            ("rung_who", "#5F6368"),
        ];
        assert_eq!(TEXT_STYLES.len(), expected.len());
        for (named, (name, color)) in TEXT_STYLES.iter().zip(expected) {
            assert_eq!(named.name.as_str(), name);
            assert_eq!(text_color(named.name, None), color, "{name}");
        }
    }

    /// A lead run sets its body on the lead's last line, so each pair shares size and
    /// line height (section 14.4 rule 9).
    #[test]
    fn every_lead_run_pairs_styles_of_one_line_height() {
        let pairs = [
            (TextStyleName::LeadBody, TextStyleName::BlockBody),
            (TextStyleName::RungTitle, TextStyleName::RungWho),
        ];
        for (lead, body) in pairs {
            let lead_style = lead.text_style().style;
            let body_style = body.text_style().style;
            assert_eq!(lead_style.size_px, body_style.size_px, "{}", lead.as_str());
            assert_eq!(
                lead_style.line_height_px,
                body_style.line_height_px,
                "{}",
                lead.as_str()
            );
            assert_ne!(lead_style.weight, body_style.weight, "{}", lead.as_str());
        }
        assert_eq!(pairs.len(), 2, "examined {} lead-run pairs", pairs.len());
    }
}
