//! Every color the SVG writer paints, keyed by theme (section 11.1). The constants and
//! free functions below `Palette` are the `center` tables of sections 2.4 to 2.9 and 5.2.
//! Center text colors are defined once in `stencil_layout::styles`, which resolves them
//! into each `TextRun`, and are re-exported here so the palette is complete in one place.
//! Drawing code reads colors only through a `Palette`.

use stencil_layout::styles::{badge_fill, text_color};
use stencil_model::text::TextStyleName;
use stencil_model::{CalloutKind, Canvas, PipeKind, Theme, ZoneKind};

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

pub const CALLOUT_NOTE_ACCENT: &str = "#1A73E8";
pub const CALLOUT_NOTE_FILL: &str = "#E8F0FE";
pub const CALLOUT_RISK_ACCENT: &str = "#C5221F";
pub const CALLOUT_RISK_FILL: &str = "#FCE8E6";
pub const CALLOUT_DECISION_ACCENT: &str = "#188038";
pub const CALLOUT_DECISION_FILL: &str = "#E6F4EA";
pub const CALLOUT_OPEN_ACCENT: &str = "#B06000";
pub const CALLOUT_OPEN_FILL: &str = "#FEF7E0";
pub const FRAME_BORDER: &str = "#9AA0A6";

pub const CARD_BORDER_PX: f32 = 1.5;
/// Border of the section 11.3 blocks, which layout reserves.
pub const BLOCK_BORDER_PX: f32 = 1.25;
/// The Frame's corner-to-corner diagonals.
pub const FRAME_DIAGONAL_PX: f32 = 1.0;
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

/// Accent bar color and fill of a Callout kind (section 11.3).
pub fn callout_colors(kind: CalloutKind) -> (&'static str, &'static str) {
    match kind {
        CalloutKind::Note => (CALLOUT_NOTE_ACCENT, CALLOUT_NOTE_FILL),
        CalloutKind::Risk => (CALLOUT_RISK_ACCENT, CALLOUT_RISK_FILL),
        CalloutKind::Decision => (CALLOUT_DECISION_ACCENT, CALLOUT_DECISION_FILL),
        CalloutKind::Open => (CALLOUT_OPEN_ACCENT, CALLOUT_OPEN_FILL),
    }
}

/// A Callout box and its accent bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalloutPaint {
    pub accent: &'static str,
    pub fill: &'static str,
    pub border: Stroke,
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
        let (fill, color, width_px) = self.card_colors();
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

    /// A Text block: card fill and card border color at the 1.25 px block border.
    pub fn block(self) -> BoxPaint {
        let card = self.card();
        BoxPaint {
            fill: card.fill,
            border: card.border.map(|stroke| Stroke {
                width_px: BLOCK_BORDER_PX,
                ..stroke
            }),
        }
    }

    /// A Callout of this kind: the Text box border with the kind's tint and accent.
    pub fn callout(self, kind: CalloutKind) -> CalloutPaint {
        let (accent, fill) = match self.theme {
            Theme::Center => callout_colors(kind),
            Theme::Dusk => dusk::callout_colors(kind),
            Theme::Wire => (wire::INK, wire::WHITE),
        };
        let (_, border_color, _) = self.card_colors();
        CalloutPaint {
            accent,
            fill,
            border: Stroke {
                width_px: BLOCK_BORDER_PX,
                line: LineStyle::Solid,
                color: border_color,
            },
        }
    }

    /// The dashed border of a Frame.
    pub fn frame_border(self) -> Stroke {
        let color = match self.theme {
            Theme::Center => FRAME_BORDER,
            Theme::Dusk => dusk::VPC_BORDER,
            Theme::Wire => wire::INK,
        };
        Stroke {
            width_px: BLOCK_BORDER_PX,
            line: LineStyle::Dashed,
            color,
        }
    }

    /// The two corner-to-corner lines of a Frame, in the secondary ink.
    pub fn frame_diagonal(self) -> Stroke {
        let color = match self.theme {
            Theme::Center => TEXT_MUTED,
            Theme::Dusk => dusk::TEXT_SECONDARY,
            Theme::Wire => wire::SECONDARY_INK,
        };
        Stroke {
            width_px: FRAME_DIAGONAL_PX,
            line: LineStyle::Solid,
            color,
        }
    }

    /// The chip under a Frame label, so the diagonals stop at the words.
    pub fn frame_label_chip(self) -> &'static str {
        self.page_background()
    }

    /// The dot of a bulleted Text line, in the body ink.
    pub fn list_bullet(self, canvas: Canvas) -> &'static str {
        self.text_ink(TextStyleName::BlockBody, canvas, None)
    }

    fn card_colors(self) -> (&'static str, &'static str, f32) {
        match self.theme {
            Theme::Center => (CARD_FILL, CARD_BORDER, CARD_BORDER_PX),
            Theme::Dusk => (dusk::CARD_FILL, dusk::CARD_BORDER, CARD_BORDER_PX),
            Theme::Wire => (wire::WHITE, wire::INK, wire::BORDER_PX),
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

/// A visible face of an isometric slab or block (section 12.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Top,
    Left,
    Right,
}

impl Palette {
    /// HSL lightness step of a face in percentage points; None in wire (no shading).
    pub fn face_lightness_step(self, face: Face) -> Option<i8> {
        match (self.theme, face) {
            (Theme::Wire, _) => None,
            (Theme::Center | Theme::Dusk, Face::Top) => Some(0),
            (Theme::Center, Face::Left) => Some(-8),
            (Theme::Center, Face::Right) => Some(-16),
            (Theme::Dusk, Face::Left) => Some(-4),
            (Theme::Dusk, Face::Right) => Some(-8),
        }
    }

    /// The fill of `face` for a node whose flat fill is `base`: base itself for Top, base
    /// shaded by the step for Left and Right, and the page background for every face in wire.
    /// None when `base` is not `#RRGGBB`.
    pub fn face_fill(self, base: &'static str, face: Face) -> Option<String> {
        parse_hex_color(base)?;
        match self.face_lightness_step(face) {
            Some(step_points) => shade(base, step_points),
            None => Some(self.page_background().to_string()),
        }
    }

    /// The stroke of an isometric face (section 12.6): the node's flat border, and in wire
    /// the ink border for a node whose flat drawing has none, so a filled block such as a
    /// Fact still reads as a line drawing instead of a white patch.
    pub fn face_outline(self, flat_border: Option<Stroke>) -> Option<Stroke> {
        match self.theme {
            Theme::Center | Theme::Dusk => flat_border,
            Theme::Wire => Some(flat_border.unwrap_or(Stroke {
                width_px: wire::BORDER_PX,
                line: LineStyle::Solid,
                color: wire::INK,
            })),
        }
    }

    /// The outline behind upright billboard text that has no chip (section 12.4).
    pub fn text_halo(self) -> &'static str {
        self.page_background()
    }

    /// The chip behind a gcp zone label drawn upright over the slab (section 12.4): the bar
    /// fill, and in wire a white chip with the tag border.
    pub fn gcp_label_chip(self) -> BoxPaint {
        match self.theme {
            Theme::Center | Theme::Dusk => BoxPaint {
                fill: self.gcp_bar_fill(),
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
}

/// `color` (`#RRGGBB`) with its HSL lightness moved by `step_points` percentage points.
/// None when `color` is not `#` followed by six hex digits. A step of 0 returns the input.
pub fn shade(color: &str, step_points: i8) -> Option<String> {
    let [red, green, blue] = parse_hex_color(color)?;
    if step_points == 0 {
        return Some(color.to_string());
    }
    let (hue, saturation, lightness) = rgb_to_hsl(
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
    );
    let shaded = (lightness + f64::from(step_points) / 100.0).clamp(0.0, 1.0);
    let channels = hsl_to_rgb(hue, saturation, shaded).map(|channel| {
        // The clamp keeps the cast in range, so `as` never saturates.
        (channel * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8
    });
    let [red, green, blue] = channels;
    Some(format!("#{red:02X}{green:02X}{blue:02X}"))
}

/// The three channels of `#RRGGBB`, or None for any other string.
fn parse_hex_color(color: &str) -> Option<[u8; 3]> {
    let digits = color.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |start: usize| {
        digits
            .get(start..start + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Hue in degrees 0 to 360, saturation and lightness 0 to 1, from channels 0 to 1.
fn rgb_to_hsl(red: f64, green: f64, blue: f64) -> (f64, f64, f64) {
    let largest = red.max(green).max(blue);
    let smallest = red.min(green).min(blue);
    let lightness = (largest + smallest) / 2.0;
    let spread = largest - smallest;
    if spread == 0.0 {
        return (0.0, 0.0, lightness);
    }
    let saturation = if lightness > 0.5 {
        spread / (2.0 - largest - smallest)
    } else {
        spread / (largest + smallest)
    };
    let sector = if largest == red {
        ((green - blue) / spread).rem_euclid(6.0)
    } else if largest == green {
        (blue - red) / spread + 2.0
    } else {
        (red - green) / spread + 4.0
    };
    (sector * 60.0, saturation, lightness)
}

/// Channels 0 to 1 from hue in degrees, saturation and lightness 0 to 1.
fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> [f64; 3] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let second = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = if sector < 1.0 {
        (chroma, second, 0.0)
    } else if sector < 2.0 {
        (second, chroma, 0.0)
    } else if sector < 3.0 {
        (0.0, chroma, second)
    } else if sector < 4.0 {
        (0.0, second, chroma)
    } else if sector < 5.0 {
        (second, 0.0, chroma)
    } else {
        (chroma, 0.0, second)
    };
    let lift = lightness - chroma / 2.0;
    [red + lift, green + lift, blue + lift]
}

/// The dark theme of section 11.1.
mod dusk {
    use super::{LineStyle, Stroke, ZoneStyle};
    use stencil_model::text::TextStyleName;
    use stencil_model::{CalloutKind, Canvas, PipeKind, ZoneKind};

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
    pub const CALLOUT_NOTE_FILL: &str = "#16305C";
    pub const CALLOUT_RISK_FILL: &str = "#3A1A1A";
    pub const CALLOUT_DECISION_FILL: &str = "#143024";
    pub const CALLOUT_OPEN_FILL: &str = "#3A2A10";

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

    /// The center accents on dark tints (section 11.3).
    pub fn callout_colors(kind: CalloutKind) -> (&'static str, &'static str) {
        let (accent, _) = super::callout_colors(kind);
        let fill = match kind {
            CalloutKind::Note => CALLOUT_NOTE_FILL,
            CalloutKind::Risk => CALLOUT_RISK_FILL,
            CalloutKind::Decision => CALLOUT_DECISION_FILL,
            CalloutKind::Open => CALLOUT_OPEN_FILL,
        };
        (accent, fill)
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
            TextStyleName::Title
            | TextStyleName::CardFunction
            | TextStyleName::LegendLabel
            | TextStyleName::BlockBody => TEXT_PRIMARY,
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
            | TextStyleName::LegendLabel
            | TextStyleName::BlockBody => INK,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shade_matches_the_section_12_6_vectors() {
        let vectors = [
            ("#D2E3FC", ["#BFD7FB", "#ACCBF9", "#85B3F7"]),
            ("#FFFFFF", ["#F5F5F5", "#EBEBEB", "#D6D6D6"]),
            ("#FAFBFC", ["#EDF1F4", "#E0E7ED", "#C7D2DD"]),
            ("#1A73E8", ["#166AD8", "#1461C5", "#104EA0"]),
            ("#14213A", ["#0F182B", "#0A101C", "#000000"]),
        ];
        for (color, expected) in vectors {
            for (step, shaded) in [-4, -8, -16].into_iter().zip(expected) {
                assert_eq!(
                    shade(color, step).as_deref(),
                    Some(shaded),
                    "{color} {step}"
                );
            }
        }
    }
}
