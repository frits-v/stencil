//! Every color the SVG writer paints, keyed by theme (section 11.1). The constants and
//! free functions below `Palette` are the `center` tables of sections 2.4 to 2.9 and 5.2.
//! Center text colors are defined once in `stencil_layout::styles`, which resolves them
//! into each `TextRun`, and are re-exported here so the palette is complete in one place.
//! Drawing code reads colors only through a `Palette`.

use stencil_layout::styles::{badge_fill, text_color};
use stencil_model::text::TextStyleName;
use stencil_model::{Canvas, PipeKind, Theme, ZoneKind};

pub use stencil_layout::styles::{
    BADGE_FILL_CUSTOMER, BADGE_FILL_INTERNAL, BADGE_TEXT_CUSTOMER, BADGE_TEXT_INTERNAL, TEXT_AMBER,
    TEXT_BLUE, TEXT_DARK, TEXT_DENY, TEXT_MUTED, TEXT_WHITE,
};

pub const CANVAS_FILL: &str = "#FFFFFF";

pub const GCP_BORDER: &str = "#1A73E8";
pub const GCP_FRAME_FILL: &str = "#FFFFFF";
pub const GCP_BAR_FILL: &str = "#1A73E8";
pub const GCP_BODY_FILL: &str = "#FAFBFC";
pub const VPC_BORDER: &str = "#5F6368";
pub const REGION_BORDER: &str = "#BDC1C6";
pub const SUBNET_BORDER: &str = "#9AA0A6";
pub const ONPREM_BORDER: &str = "#D7CCC8";
pub const PROJECT_BORDER: &str = "#FFE082";
pub const OPTIONAL_BORDER: &str = "#4284F3";
pub const PERIMETER_BORDER: &str = "#E37400";
pub const TINT_A_FILL: &str = "#D2E3FC";
pub const TINT_B_FILL: &str = "#FCE4EC";
pub const SUBNET_FILL: &str = "#EDE7F6";
pub const PROJECT_FILL: &str = "#FFF8E1";
pub const OPTIONAL_FILL: &str = "#F8FBFF";
pub const K8S_FILL: &str = "#FCE4EC";
pub const PERIMETER_FILL: &str = "#FFFBF5";

pub const CARD_FILL: &str = "#FFFFFF";
pub const CARD_BORDER: &str = "#DADCE0";
pub const FACT_FILL: &str = "#F1F3F4";
pub const ASK_FILL: &str = "#FEF7E0";

pub const TAG_FILL: &str = "#FFFFFF";
pub const TAG_BORDER: &str = "#DADCE0";
pub const TAG_BORDER_DENY: &str = "#F4C7C3";

pub const WIRE_GRAY: &str = "#5F6368";
pub const WIRE_BLUE: &str = "#1A73E8";
pub const WIRE_PINK: &str = "#C2185B";
pub const WIRE_DENY: &str = "#C5221F";

pub const CARD_BORDER_PX: f32 = 1.5;
pub const TAG_BORDER_PX: f32 = 1.5;
pub const WIRE_WIDTH_PX: f32 = 2.0;

/// `stroke-dasharray` of every dashed border and dashed wire (section 5.2).
pub const DASH_ARRAY: &str = "6 5";
/// `stroke-dasharray` of the dotted lines of the wire theme (section 11.1).
pub const DOT_ARRAY: &str = "2 3";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

/// A Zone's section 2.4 row, the gcp frame's border and frame fill included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneStyle {
    /// None for k8s, which has no border.
    pub border: Option<Stroke>,
    /// None for vpc, which is unfilled.
    pub fill: Option<&'static str>,
    pub radius_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width_px: f32,
    pub line: LineStyle,
    pub color: &'static str,
}

pub fn zone_style(kind: ZoneKind) -> ZoneStyle {
    let solid = |width_px, color| Stroke {
        width_px,
        line: LineStyle::Solid,
        color,
    };
    let dashed = |width_px, color| Stroke {
        width_px,
        line: LineStyle::Dashed,
        color,
    };
    match kind {
        ZoneKind::Gcp => ZoneStyle {
            border: Some(solid(3.0, GCP_BORDER)),
            fill: Some(GCP_FRAME_FILL),
            radius_px: 10.0,
        },
        ZoneKind::Vpc => ZoneStyle {
            border: Some(dashed(2.0, VPC_BORDER)),
            fill: None,
            radius_px: 8.0,
        },
        ZoneKind::RegionA => ZoneStyle {
            border: Some(solid(1.5, REGION_BORDER)),
            fill: Some(TINT_A_FILL),
            radius_px: 8.0,
        },
        ZoneKind::RegionB => ZoneStyle {
            border: Some(solid(1.5, REGION_BORDER)),
            fill: Some(TINT_B_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Subnet => ZoneStyle {
            border: Some(dashed(1.5, SUBNET_BORDER)),
            fill: Some(SUBNET_FILL),
            radius_px: 8.0,
        },
        ZoneKind::OnpremA => ZoneStyle {
            border: Some(solid(1.5, ONPREM_BORDER)),
            fill: Some(TINT_A_FILL),
            radius_px: 8.0,
        },
        ZoneKind::OnpremB => ZoneStyle {
            border: Some(solid(1.5, ONPREM_BORDER)),
            fill: Some(TINT_B_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Project => ZoneStyle {
            border: Some(solid(1.5, PROJECT_BORDER)),
            fill: Some(PROJECT_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Optional => ZoneStyle {
            border: Some(dashed(2.0, OPTIONAL_BORDER)),
            fill: Some(OPTIONAL_FILL),
            radius_px: 8.0,
        },
        ZoneKind::K8s => ZoneStyle {
            border: None,
            fill: Some(K8S_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Perimeter => ZoneStyle {
            border: Some(dashed(2.5, PERIMETER_BORDER)),
            fill: Some(PERIMETER_FILL),
            radius_px: 10.0,
        },
    }
}

/// Wire, dot, spine and legend swatch color and line style (section 5.2).
pub fn wire_style(kind: PipeKind) -> (&'static str, LineStyle) {
    match kind {
        PipeKind::Gray => (WIRE_GRAY, LineStyle::Solid),
        PipeKind::Blue => (WIRE_BLUE, LineStyle::Solid),
        PipeKind::Pink => (WIRE_PINK, LineStyle::Solid),
        PipeKind::Dash => (WIRE_BLUE, LineStyle::Dashed),
        PipeKind::Deny => (WIRE_DENY, LineStyle::Dashed),
    }
}

/// Border of a Pipe tag or Tee hub (sections 2.7 and 2.8).
pub fn tag_border(kind: PipeKind) -> &'static str {
    match kind {
        PipeKind::Deny => TAG_BORDER_DENY,
        PipeKind::Gray | PipeKind::Blue | PipeKind::Pink | PipeKind::Dash => TAG_BORDER,
    }
}

/// Fill and optional border of a box: a card, a tag, a badge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxPaint {
    pub fill: &'static str,
    pub border: Option<Stroke>,
}

/// How the end dots of a wire are painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotStyle {
    /// A disc in the wire color.
    Filled,
    /// A ring in the wire color around the page background, the wire theme's pink.
    Hollow,
}

/// Paint of a wire, a Tee spine and a legend swatch of one kind, and of that kind's end dots
/// and arrowheads, which take the wire color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WireStyle {
    pub stroke: Stroke,
    pub dot: DotStyle,
}

/// Stroke width of a hollow end dot's ring.
pub const HOLLOW_DOT_RING_PX: f32 = 1.5;

/// The colors of one theme. Built once per render from `Page.theme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    theme: Theme,
}

impl Palette {
    pub fn new(theme: Theme) -> Self {
        Palette { theme }
    }

    pub fn theme(self) -> Theme {
        self.theme
    }

    /// The serialized theme name, used in marker ids.
    pub fn theme_name(self) -> &'static str {
        match self.theme {
            Theme::Center => "center",
            Theme::Dusk => "dusk",
            Theme::Wire => "wire",
        }
    }

    pub fn page_background(self) -> &'static str {
        match self.theme {
            Theme::Center => CANVAS_FILL,
            Theme::Dusk => dusk::PAGE_BACKGROUND,
            Theme::Wire => wire::WHITE,
        }
    }

    pub fn zone_style(self, kind: ZoneKind) -> ZoneStyle {
        match self.theme {
            Theme::Center => zone_style(kind),
            Theme::Dusk => dusk::zone_style(kind),
            Theme::Wire => wire::zone_style(kind),
        }
    }

    /// The gcp frame's bar fill.
    pub fn gcp_bar_fill(self) -> &'static str {
        match self.theme {
            Theme::Center => GCP_BAR_FILL,
            Theme::Dusk => dusk::GCP_BAR_FILL,
            Theme::Wire => wire::WHITE,
        }
    }

    /// A rule along the bottom edge of the gcp bar, drawn inside the bar box.
    pub fn gcp_bar_rule(self) -> Option<Stroke> {
        match self.theme {
            Theme::Center | Theme::Dusk => None,
            Theme::Wire => Some(Stroke {
                width_px: wire::GCP_BAR_RULE_PX,
                line: LineStyle::Solid,
                color: wire::INK,
            }),
        }
    }

    pub fn gcp_body_fill(self) -> &'static str {
        match self.theme {
            Theme::Center => GCP_BODY_FILL,
            Theme::Dusk => dusk::GCP_BODY_FILL,
            Theme::Wire => wire::WHITE,
        }
    }

    pub fn card(self) -> BoxPaint {
        let (fill, color, width_px) = match self.theme {
            Theme::Center => (CARD_FILL, CARD_BORDER, CARD_BORDER_PX),
            Theme::Dusk => (dusk::CARD_FILL, dusk::CARD_BORDER, CARD_BORDER_PX),
            Theme::Wire => (wire::WHITE, wire::INK, wire::BORDER_PX),
        };
        BoxPaint {
            fill,
            border: Some(Stroke {
                width_px,
                line: LineStyle::Solid,
                color,
            }),
        }
    }

    /// Fill of a Fact node and of a Pcard's fact box.
    pub fn fact_fill(self) -> &'static str {
        match self.theme {
            Theme::Center => FACT_FILL,
            Theme::Dusk => dusk::NOTE_BOX_FILL,
            Theme::Wire => wire::WHITE,
        }
    }

    pub fn ask_fill(self) -> &'static str {
        match self.theme {
            Theme::Center => ASK_FILL,
            Theme::Dusk => dusk::NOTE_BOX_FILL,
            Theme::Wire => wire::WHITE,
        }
    }

    /// A Pipe tag or Tee hub of this kind (sections 2.7 and 2.8).
    pub fn tag(self, kind: PipeKind) -> BoxPaint {
        let (fill, color, width_px) = match self.theme {
            Theme::Center => (TAG_FILL, tag_border(kind), TAG_BORDER_PX),
            Theme::Dusk => (dusk::CARD_FILL, dusk::tag_border(kind), TAG_BORDER_PX),
            Theme::Wire => (wire::WHITE, wire::INK, wire::BORDER_PX),
        };
        BoxPaint {
            fill,
            border: Some(Stroke {
                width_px,
                line: LineStyle::Solid,
                color,
            }),
        }
    }

    /// The canvas badge on the page kicker.
    pub fn badge(self, canvas: Canvas) -> BoxPaint {
        match self.theme {
            Theme::Center => BoxPaint {
                fill: badge_fill(canvas),
                border: None,
            },
            Theme::Dusk => BoxPaint {
                fill: dusk::badge_fill(canvas),
                border: None,
            },
            Theme::Wire => BoxPaint {
                fill: wire::WHITE,
                border: Some(Stroke {
                    width_px: wire::BORDER_PX,
                    line: LineStyle::Solid,
                    color: wire::INK,
                }),
            },
        }
    }

    pub fn wire_style(self, kind: PipeKind) -> WireStyle {
        match self.theme {
            Theme::Center => {
                let (color, line) = wire_style(kind);
                WireStyle {
                    stroke: Stroke {
                        width_px: WIRE_WIDTH_PX,
                        line,
                        color,
                    },
                    dot: DotStyle::Filled,
                }
            }
            Theme::Dusk => {
                let (color, line) = dusk::wire_style(kind);
                WireStyle {
                    stroke: Stroke {
                        width_px: WIRE_WIDTH_PX,
                        line,
                        color,
                    },
                    dot: DotStyle::Filled,
                }
            }
            Theme::Wire => wire::wire_style(kind),
        }
    }

    /// Fill of the rounded square under every icon (section 11.1). The center chip would be
    /// card fill on card fill, invisible, so center draws none and its output stays the
    /// pre-theme output.
    pub fn icon_chip(self) -> Option<&'static str> {
        match self.theme {
            Theme::Center => None,
            Theme::Dusk => Some(dusk::ICON_CHIP),
            Theme::Wire => Some(wire::WHITE),
        }
    }

    /// Text color of a style. `canvas` decides `badge`; `pipe_kind` is the kind of the Pipe
    /// or Tee a `tag_label` run belongs to.
    pub fn text_ink(
        self,
        name: TextStyleName,
        canvas: Canvas,
        pipe_kind: Option<PipeKind>,
    ) -> &'static str {
        match self.theme {
            Theme::Center => text_color(name, canvas, pipe_kind),
            Theme::Dusk => dusk::text_ink(name, canvas, pipe_kind),
            Theme::Wire => wire::text_ink(name),
        }
    }
}

/// The dark theme of section 11.1.
mod dusk {
    use super::{LineStyle, Stroke, ZoneStyle};
    use stencil_model::text::TextStyleName;
    use stencil_model::{Canvas, PipeKind, ZoneKind};

    pub const PAGE_BACKGROUND: &str = "#0B1220";
    pub const TEXT_PRIMARY: &str = "#E6EDF7";
    pub const TEXT_SECONDARY: &str = "#9AA7BD";
    pub const KICKER: &str = "#5B9CFF";
    pub const BADGE_FILL_CUSTOMER: &str = "#16305C";
    pub const BADGE_INK_CUSTOMER: &str = "#9CC3FF";
    pub const BADGE_FILL_INTERNAL: &str = "#2E1A4A";
    pub const BADGE_INK_INTERNAL: &str = "#D6B4FF";
    pub const CARD_FILL: &str = "#111A2E";
    pub const CARD_BORDER: &str = "#2A3550";
    pub const NOTE_BOX_FILL: &str = "#182238";
    pub const NOTE_BOX_INK: &str = "#B7C2D6";
    pub const GCP_BORDER: &str = "#1A73E8";
    pub const GCP_BAR_FILL: &str = "#1A73E8";
    pub const GCP_BAR_INK: &str = "#FFFFFF";
    pub const GCP_BODY_FILL: &str = "#0F172A";
    pub const VPC_BORDER: &str = "#6B7A99";
    pub const TINT_A_FILL: &str = "#14213A";
    pub const TINT_B_FILL: &str = "#2A1626";
    pub const REGION_A_BORDER: &str = "#2F4A7A";
    pub const REGION_B_BORDER: &str = "#6A2A47";
    pub const SUBNET_FILL: &str = "#1D1836";
    pub const SUBNET_BORDER: &str = "#4A3F7A";
    pub const ONPREM_BORDER: &str = "#3A3532";
    pub const PROJECT_FILL: &str = "#1F1B10";
    pub const PROJECT_BORDER: &str = "#5A4A1A";
    pub const OPTIONAL_FILL: &str = "#10203A";
    pub const OPTIONAL_BORDER: &str = "#4284F3";
    pub const K8S_FILL: &str = "#2A1626";
    pub const PERIMETER_FILL: &str = "#17130B";
    pub const PERIMETER_BORDER: &str = "#E37400";
    pub const PERIMETER_INK: &str = "#F2A44B";
    pub const ZONE_LABEL_INK: &str = "#B7C2D6";
    pub const WIRE_GRAY: &str = "#9AA7BD";
    pub const WIRE_BLUE: &str = "#5B9CFF";
    pub const WIRE_PINK: &str = "#FF5C8A";
    pub const WIRE_DENY: &str = "#FF6B6B";
    pub const DENY_TAG_INK: &str = "#FF8A8A";
    pub const DENY_TAG_BORDER: &str = "#5A2A2A";
    pub const LEGEND_INK: &str = "#9AA7BD";
    pub const FOOT_INK: &str = "#9AA7BD";
    pub const ICON_CHIP: &str = "#FFFFFF";

    /// Center's border widths, line styles and radii with the dusk colors.
    pub fn zone_style(kind: ZoneKind) -> ZoneStyle {
        let center = super::zone_style(kind);
        let (border_color, fill) = match kind {
            ZoneKind::Gcp => (Some(GCP_BORDER), Some(GCP_BODY_FILL)),
            ZoneKind::Vpc => (Some(VPC_BORDER), None),
            ZoneKind::RegionA => (Some(REGION_A_BORDER), Some(TINT_A_FILL)),
            ZoneKind::RegionB => (Some(REGION_B_BORDER), Some(TINT_B_FILL)),
            ZoneKind::Subnet => (Some(SUBNET_BORDER), Some(SUBNET_FILL)),
            ZoneKind::OnpremA => (Some(ONPREM_BORDER), Some(TINT_A_FILL)),
            ZoneKind::OnpremB => (Some(ONPREM_BORDER), Some(TINT_B_FILL)),
            ZoneKind::Project => (Some(PROJECT_BORDER), Some(PROJECT_FILL)),
            ZoneKind::Optional => (Some(OPTIONAL_BORDER), Some(OPTIONAL_FILL)),
            ZoneKind::K8s => (None, Some(K8S_FILL)),
            ZoneKind::Perimeter => (Some(PERIMETER_BORDER), Some(PERIMETER_FILL)),
        };
        let border = center
            .border
            .zip(border_color)
            .map(|(stroke, color)| Stroke { color, ..stroke });
        ZoneStyle {
            border,
            fill,
            radius_px: center.radius_px,
        }
    }

    pub fn wire_style(kind: PipeKind) -> (&'static str, LineStyle) {
        match kind {
            PipeKind::Gray => (WIRE_GRAY, LineStyle::Solid),
            PipeKind::Blue => (WIRE_BLUE, LineStyle::Solid),
            PipeKind::Pink => (WIRE_PINK, LineStyle::Solid),
            PipeKind::Dash => (WIRE_BLUE, LineStyle::Dashed),
            PipeKind::Deny => (WIRE_DENY, LineStyle::Dashed),
        }
    }

    pub fn tag_border(kind: PipeKind) -> &'static str {
        match kind {
            PipeKind::Deny => DENY_TAG_BORDER,
            PipeKind::Gray | PipeKind::Blue | PipeKind::Pink | PipeKind::Dash => CARD_BORDER,
        }
    }

    pub fn badge_fill(canvas: Canvas) -> &'static str {
        match canvas {
            Canvas::Customer => BADGE_FILL_CUSTOMER,
            Canvas::Internal => BADGE_FILL_INTERNAL,
        }
    }

    pub fn text_ink(
        name: TextStyleName,
        canvas: Canvas,
        pipe_kind: Option<PipeKind>,
    ) -> &'static str {
        match name {
            TextStyleName::Badge => match canvas {
                Canvas::Customer => BADGE_INK_CUSTOMER,
                Canvas::Internal => BADGE_INK_INTERNAL,
            },
            TextStyleName::Kicker => KICKER,
            TextStyleName::Title | TextStyleName::CardFunction | TextStyleName::LegendLabel => {
                TEXT_PRIMARY
            }
            TextStyleName::TagLabel => match pipe_kind {
                Some(PipeKind::Deny) => DENY_TAG_INK,
                Some(PipeKind::Gray | PipeKind::Blue | PipeKind::Pink | PipeKind::Dash) | None => {
                    TEXT_PRIMARY
                }
            },
            TextStyleName::GcpBar => GCP_BAR_INK,
            TextStyleName::PerimeterLabel => PERIMETER_INK,
            TextStyleName::ZoneLabel => ZONE_LABEL_INK,
            TextStyleName::Fact | TextStyleName::Ask => NOTE_BOX_INK,
            TextStyleName::Lede | TextStyleName::CardProduct | TextStyleName::TagSub => {
                TEXT_SECONDARY
            }
            TextStyleName::NoteLegend | TextStyleName::LegendText => LEGEND_INK,
            TextStyleName::Foot => FOOT_INK,
        }
    }
}

/// The monochrome wireframe theme of section 11.1.
mod wire {
    use super::{DotStyle, LineStyle, Stroke, WIRE_WIDTH_PX, WireStyle, ZoneStyle};
    use stencil_model::text::TextStyleName;
    use stencil_model::{PipeKind, ZoneKind};

    pub const WHITE: &str = "#FFFFFF";
    pub const INK: &str = "#222222";
    pub const SECONDARY_INK: &str = "#555555";
    pub const BORDER_PX: f32 = 1.25;
    pub const GCP_BORDER_PX: f32 = 2.0;
    pub const GCP_BAR_RULE_PX: f32 = 2.0;
    pub const THIN_WIRE_PX: f32 = 1.25;

    /// Center's radii; one ink, white fills where center fills, and line style by kind.
    pub fn zone_style(kind: ZoneKind) -> ZoneStyle {
        let center = super::zone_style(kind);
        let (width_px, line) = match kind {
            ZoneKind::Gcp => (GCP_BORDER_PX, LineStyle::Solid),
            ZoneKind::Vpc | ZoneKind::Optional | ZoneKind::Perimeter => {
                (BORDER_PX, LineStyle::Dashed)
            }
            ZoneKind::Subnet => (BORDER_PX, LineStyle::Dotted),
            ZoneKind::RegionA
            | ZoneKind::RegionB
            | ZoneKind::OnpremA
            | ZoneKind::OnpremB
            | ZoneKind::Project
            | ZoneKind::K8s => (BORDER_PX, LineStyle::Solid),
        };
        ZoneStyle {
            border: Some(Stroke {
                width_px,
                line,
                color: INK,
            }),
            fill: center.fill.map(|_| WHITE),
            radius_px: center.radius_px,
        }
    }

    /// Kinds are told apart by line style alone.
    pub fn wire_style(kind: PipeKind) -> WireStyle {
        let (width_px, line, dot) = match kind {
            PipeKind::Gray => (THIN_WIRE_PX, LineStyle::Solid, DotStyle::Filled),
            PipeKind::Blue => (WIRE_WIDTH_PX, LineStyle::Solid, DotStyle::Filled),
            PipeKind::Pink => (WIRE_WIDTH_PX, LineStyle::Solid, DotStyle::Hollow),
            PipeKind::Dash => (WIRE_WIDTH_PX, LineStyle::Dashed, DotStyle::Filled),
            PipeKind::Deny => (WIRE_WIDTH_PX, LineStyle::Dotted, DotStyle::Filled),
        };
        WireStyle {
            stroke: Stroke {
                width_px,
                line,
                color: INK,
            },
            dot,
        }
    }

    pub fn text_ink(name: TextStyleName) -> &'static str {
        match name {
            TextStyleName::Lede
            | TextStyleName::ZoneLabel
            | TextStyleName::CardProduct
            | TextStyleName::Fact
            | TextStyleName::TagSub
            | TextStyleName::NoteLegend
            | TextStyleName::LegendText
            | TextStyleName::Foot => SECONDARY_INK,
            TextStyleName::Badge
            | TextStyleName::Kicker
            | TextStyleName::Title
            | TextStyleName::GcpBar
            | TextStyleName::PerimeterLabel
            | TextStyleName::CardFunction
            | TextStyleName::Ask
            | TextStyleName::TagLabel
            | TextStyleName::LegendLabel => INK,
        }
    }
}
