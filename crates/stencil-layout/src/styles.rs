//! Text colors of section 2.9, keyed by `TextStyleName`. The style values themselves are
//! `stencil_model::text::TEXT_STYLES`.

use stencil_model::text::{TextStyle, TextStyleName};
use stencil_model::{Canvas, Line};

use crate::{ISO_FRAME_LABEL_SCALE, ISO_TYPE_SCALE, ISO_ZONE_LABEL_SCALE};

pub const TEXT_DARK: &str = "#202124";
pub const TEXT_MUTED: &str = "#5F6368";
pub const TEXT_BLUE: &str = "#1A73E8";
pub const TEXT_WHITE: &str = "#FFFFFF";
pub const TEXT_AMBER: &str = "#B06000";
pub const TEXT_DENY: &str = "#C5221F";
pub const BADGE_TEXT_CUSTOMER: &str = "#174EA6";
pub const BADGE_TEXT_INTERNAL: &str = "#7B1FA2";
pub const BADGE_FILL_CUSTOMER: &str = "#E8F0FE";
pub const BADGE_FILL_INTERNAL: &str = "#F3E5F5";

/// Text color of a style. `canvas` decides `badge`; `line` is the line of the Pipe, Tee or
/// Link a `tag_label` run belongs to, and `deny` turns it red. Other styles ignore both.
pub fn text_color(name: TextStyleName, canvas: Canvas, line: Option<Line>) -> &'static str {
    match name {
        TextStyleName::Badge => match canvas {
            Canvas::Customer => BADGE_TEXT_CUSTOMER,
            Canvas::Internal => BADGE_TEXT_INTERNAL,
        },
        TextStyleName::Kicker => TEXT_BLUE,
        TextStyleName::Title
        | TextStyleName::CardFunction
        | TextStyleName::LegendLabel
        | TextStyleName::BlockBody => TEXT_DARK,
        TextStyleName::TagLabel => match line {
            Some(Line::Deny) => TEXT_DENY,
            Some(Line::Gray | Line::Solid | Line::Dash) | None => TEXT_DARK,
        },
        TextStyleName::GcpBar => TEXT_WHITE,
        TextStyleName::PerimeterLabel | TextStyleName::Ask => TEXT_AMBER,
        TextStyleName::Lede
        | TextStyleName::ZoneLabel
        | TextStyleName::CardProduct
        | TextStyleName::Fact
        | TextStyleName::TagSub
        | TextStyleName::NoteLegend
        | TextStyleName::LegendText
        | TextStyleName::Foot => TEXT_MUTED,
    }
}

/// Background of the canvas badge on the page kicker.
pub fn badge_fill(canvas: Canvas) -> &'static str {
    match canvas {
        Canvas::Customer => BADGE_FILL_CUSTOMER,
        Canvas::Internal => BADGE_FILL_INTERNAL,
    }
}

/// The style a run is measured and drawn in: the named style flat; under iso the body
/// styles grown by `ISO_TYPE_SCALE`, a zone label by `ISO_ZONE_LABEL_SCALE`, and the frame
/// bar label by `ISO_FRAME_LABEL_SCALE`, a tier above the zone names.
/// Chrome and the legend keep their flat size.
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
        | TextStyleName::Foot => return flat,
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
    fn badge_color_follows_canvas() {
        assert_eq!(
            text_color(TextStyleName::Badge, Canvas::Customer, None),
            "#174EA6"
        );
        assert_eq!(
            text_color(TextStyleName::Badge, Canvas::Internal, None),
            "#7B1FA2"
        );
        assert_eq!(badge_fill(Canvas::Customer), "#E8F0FE");
        assert_eq!(badge_fill(Canvas::Internal), "#F3E5F5");
    }

    #[test]
    fn tag_label_is_red_only_for_deny() {
        for line in Line::ALL {
            let expected = if line == Line::Deny {
                "#C5221F"
            } else {
                "#202124"
            };
            assert_eq!(
                text_color(TextStyleName::TagLabel, Canvas::Customer, Some(line)),
                expected
            );
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
        ];
        for (named, (name, color)) in TEXT_STYLES.iter().zip(expected) {
            assert_eq!(named.name.as_str(), name);
            assert_eq!(
                text_color(named.name, Canvas::Customer, None),
                color,
                "{name}"
            );
        }
    }
}
