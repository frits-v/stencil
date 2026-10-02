//! Every paint the SVG writer draws, read from the resolved `Theme` (section 13.4 rule 11).
//! Containers paint by role, tone and tint, lines by line and tint, never by a grammar's
//! kind names. No color is named here: every method returns a value borrowed from the
//! theme, or derived from one by `shade`.

use stencil_layout::ContainerLook;
use stencil_model::grammar::{BorderPattern, Role};
use stencil_model::text::TextStyleName;
use stencil_model::theme::{SolidEdges, ThemeStroke, Tint, solid_dot};
use stencil_model::{
    CalloutKind, Canvas, Line, Projection, Theme, drawn_legend_label, legend_label, line_key,
    line_tint,
};

pub use stencil_model::theme::{DotStyle, LinePattern as LineStyle};

/// Border of the section 11.3 blocks, which layout reserves.
pub const BLOCK_BORDER_PX: f32 = 1.25;
/// The Frame block's corner-to-corner diagonals.
pub const FRAME_DIAGONAL_PX: f32 = 1.0;
/// Stroke width of a hollow end dot's ring.
pub const HOLLOW_DOT_RING_PX: f32 = 1.5;
/// The ring around an icon chip under iso.
pub const ISO_ICON_CHIP_RING_PX: f32 = 1.0;

/// `stroke-dasharray` of every dashed border and dashed wire (section 5.2).
pub const DASH_ARRAY: &str = "6 5";
/// `stroke-dasharray` of every dotted border and dotted wire.
pub const DOT_ARRAY: &str = "2 3";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke<'a> {
    pub width_px: f32,
    pub line: LineStyle,
    pub color: &'a str,
}

impl<'a> Stroke<'a> {
    fn of(stroke: &'a ThemeStroke) -> Self {
        Stroke {
            width_px: stroke.width,
            line: stroke.pattern,
            color: stroke.color.as_str(),
        }
    }
}

/// A Box's paint: fill, border in the kind's pattern and the kind's radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneStyle<'a> {
    /// None for a borderless container.
    pub border: Option<Stroke<'a>>,
    /// None for an unfilled container such as a ring.
    pub fill: Option<&'a str>,
    pub radius_px: f32,
}

/// A line as drawn: its line and its effective tint (section 13.1 rule 2), slot 1 for an
/// untinted solid or dash line and None for gray and deny. Ordered as the arrow markers are
/// written: gray, solid by slot, dash by slot, deny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineUse {
    pub line: Line,
    pub tint: Option<u8>,
}

impl LineUse {
    /// The line with its authored tint resolved to the effective one.
    pub fn new(line: Line, tint: Option<u8>) -> Self {
        LineUse {
            line,
            tint: line_tint(line, tint),
        }
    }

    /// The output key of section 13.1 rule 6, for example "blue" or "dash-2".
    pub fn key(self) -> &'static str {
        line_key(self.line, self.tint)
    }
}

/// Fill and optional border of a box: a card, a tag, a badge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxPaint<'a> {
    pub fill: &'a str,
    pub border: Option<Stroke<'a>>,
}

/// A Callout box and its accent bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalloutPaint<'a> {
    pub accent: &'a str,
    pub fill: &'a str,
    pub border: Stroke<'a>,
}

/// Paint of a wire, a Tee spine and a legend swatch of one line, and of that line's end
/// dots and arrowheads, which take the wire color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WireStyle<'a> {
    pub stroke: Stroke<'a>,
    pub dot: DotStyle,
}

/// The soft shadow under an iso icon chip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipShadowPaint<'a> {
    pub color: &'a str,
    pub opacity: f32,
    pub dy: f32,
}

/// The soft shadow under every opaque iso block (section 13.11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockShadowPaint<'a> {
    pub color: &'a str,
    pub opacity: f32,
    /// Standard deviation of the Gaussian blur, in px.
    pub blur: f32,
    /// How far the shadow sits below the block's base on screen, in px.
    pub dy: f32,
}

/// The paint of one theme under one projection. Built once per render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette<'a> {
    theme: &'a Theme,
    projection: Projection,
}

impl<'a> Palette<'a> {
    pub fn new(theme: &'a Theme, projection: Projection) -> Self {
        Palette { theme, projection }
    }

    pub fn theme(self) -> &'a Theme {
        self.theme
    }

    pub fn projection(self) -> Projection {
        self.projection
    }

    /// The theme name, used in marker ids.
    pub fn theme_name(self) -> &'a str {
        &self.theme.name
    }

    pub fn page_background(self) -> &'a str {
        self.theme.page.as_str()
    }

    /// The tint a Box is painted with: only a group takes its tint (section 13.2).
    fn box_tint(self, look: ContainerLook, tint: Option<u8>) -> Option<&'a Tint> {
        if look.role == Role::Group {
            tint.map(|slot| self.theme.tint(slot))
        } else {
            None
        }
    }

    /// The paint of a Box whose container kind looks like `look`, with effective tint
    /// `tint` (section 13.4): the slot fill when tinted, else the tone's fill; the tone's
    /// border, or the slot's when the tone has none, in the kind's pattern; a frame paints
    /// from the frame role.
    pub fn zone_style(self, look: ContainerLook, tint: Option<u8>) -> ZoneStyle<'a> {
        let containers = &self.theme.containers;
        let (fill, border_color, width_px) = if look.role == Role::Frame {
            (
                Some(self.theme.frame.frame_fill.as_str()),
                Some(self.theme.frame.border.as_str()),
                containers.frame_draw_width.unwrap_or(look.border_width),
            )
        } else {
            let tinted = self.box_tint(look, tint);
            let tone = look.tone.map(|tone| self.theme.tones.get(tone));
            let fill = tinted.map(|slot| slot.fill.as_str()).or_else(|| {
                tone.and_then(|tone| tone.fill.as_ref())
                    .map(|fill| fill.as_str())
            });
            let border = tone
                .and_then(|tone| tone.border.as_ref())
                .map(|border| border.as_str())
                .or_else(|| tinted.map(|slot| slot.border.as_str()));
            (
                fill,
                border,
                containers.draw_width.unwrap_or(look.border_width),
            )
        };
        let line = match look.pattern {
            BorderPattern::Solid => Some(LineStyle::Solid),
            BorderPattern::Dashed => Some(LineStyle::Dashed),
            BorderPattern::Dotted => Some(LineStyle::Dotted),
            BorderPattern::None => None,
        };
        let border = match line {
            Some(line) => border_color.map(|color| Stroke {
                width_px,
                line,
                color,
            }),
            None => containers.borderless_outline.as_ref().map(Stroke::of),
        };
        ZoneStyle {
            border,
            fill,
            radius_px: look.radius,
        }
    }

    /// The ink of a Box label: the frame's bar ink, the slot ink when tinted, else the
    /// tone's label ink, else the zone label ink.
    pub fn container_label_ink(self, look: ContainerLook, tint: Option<u8>) -> &'a str {
        if look.role == Role::Frame {
            return self.theme.frame.bar_ink.as_str();
        }
        if let Some(slot) = self.box_tint(look, tint) {
            return slot.ink.as_str();
        }
        look.tone
            .and_then(|tone| self.theme.tones.get(tone).label_ink.as_ref())
            .unwrap_or(&self.theme.ink.zone_label)
            .as_str()
    }

    pub fn frame_bar_fill(self) -> &'a str {
        self.theme.frame.bar_fill.as_str()
    }

    /// A rule along the bottom edge of the frame bar, drawn inside the bar box.
    pub fn frame_bar_rule(self) -> Option<Stroke<'a>> {
        self.theme.frame.bar_rule.as_ref().map(Stroke::of)
    }

    pub fn frame_body_fill(self) -> &'a str {
        self.theme.frame.body_fill.as_str()
    }

    pub fn card(self) -> BoxPaint<'a> {
        BoxPaint {
            fill: self.theme.card.fill.as_str(),
            border: Some(Stroke::of(&self.theme.card.border)),
        }
    }

    /// Fill of a doc Fact node and of an Item's doc fact box.
    pub fn fact_fill(self) -> &'a str {
        self.theme.fact.fill.as_str()
    }

    pub fn ask_fill(self) -> &'a str {
        self.theme.ask.fill.as_str()
    }

    /// A Pipe tag or Tee hub on this line (sections 2.7 and 2.8).
    pub fn tag(self, line: Line) -> BoxPaint<'a> {
        let tag = &self.theme.tag;
        let color = match line {
            Line::Deny => self.theme.deny.tag_border.as_str(),
            Line::Gray | Line::Solid | Line::Dash => tag.border.color.as_str(),
        };
        BoxPaint {
            fill: tag.fill.as_str(),
            border: Some(Stroke {
                color,
                ..Stroke::of(&tag.border)
            }),
        }
    }

    /// The canvas badge on the page kicker.
    pub fn badge(self, canvas: Canvas) -> BoxPaint<'a> {
        let badge = &self.theme.badge;
        let swatch = match canvas {
            Canvas::Customer => &badge.customer,
            Canvas::Internal => &badge.internal,
        };
        BoxPaint {
            fill: swatch.fill.as_str(),
            border: badge.border.as_ref().map(Stroke::of),
        }
    }

    /// Under iso every wire takes the width of `iso_wire_width` (section 12.6).
    pub fn wire_style(self, line_use: LineUse) -> WireStyle<'a> {
        let flat = self.flat_wire_style(line_use);
        match self.projection {
            Projection::Iso => WireStyle {
                stroke: Stroke {
                    width_px: self.iso_wire_width(line_use),
                    ..flat.stroke
                },
                ..flat
            },
            Projection::Flat => flat,
        }
    }

    /// The width of a wire, link or legend swatch under iso: the primary width for solid
    /// slot 1, the gray width for gray and the secondary width for every other line.
    pub fn iso_wire_width(self, line_use: LineUse) -> f32 {
        let widths = &self.theme.iso.widths;
        match (line_use.line, line_use.tint) {
            (Line::Solid, Some(1)) => widths.primary,
            (Line::Gray, _) => widths.gray,
            (Line::Solid | Line::Dash | Line::Deny, _) => widths.secondary,
        }
    }

    fn flat_wire_style(self, line_use: LineUse) -> WireStyle<'a> {
        let theme = self.theme;
        let slot = line_use.tint.unwrap_or(1);
        let (color, width_px, line, dot) = match line_use.line {
            Line::Gray => (
                theme.gray.color.as_str(),
                theme.gray.width,
                theme.gray.pattern,
                theme.gray.dot,
            ),
            Line::Solid => (
                theme.tint(slot).wire.as_str(),
                theme.solid.width,
                LineStyle::Solid,
                solid_dot(theme, slot),
            ),
            Line::Dash => (
                theme.tint(slot).wire.as_str(),
                theme.dash.width,
                theme.dash.pattern,
                theme.dash.dot,
            ),
            Line::Deny => (
                theme.deny.color.as_str(),
                theme.deny.width,
                theme.deny.pattern,
                theme.deny.dot,
            ),
        };
        WireStyle {
            stroke: Stroke {
                width_px,
                line,
                color,
            },
            dot,
        }
    }

    /// The legend label drawn for a line when it differs from the canonical label layout
    /// measured (section 13.4 rule 9); None when they are equal.
    pub fn legend_relabel(self, line_use: LineUse) -> Option<String> {
        let drawn = drawn_legend_label(self.theme, line_use.line, line_use.tint);
        (drawn != legend_label(line_use.line, line_use.tint)).then_some(drawn)
    }

    /// A Text block: card fill and card border color at the 1.25 px block border.
    pub fn block(self) -> BoxPaint<'a> {
        let card = self.card();
        BoxPaint {
            fill: card.fill,
            border: card.border.map(|stroke| Stroke {
                width_px: BLOCK_BORDER_PX,
                ..stroke
            }),
        }
    }

    /// A Callout of this kind: the Text box border with the kind's accent and fill.
    pub fn callout(self, kind: CalloutKind) -> CalloutPaint<'a> {
        let callout = &self.theme.callout;
        let accent = match kind {
            CalloutKind::Note => &callout.note,
            CalloutKind::Risk => &callout.risk,
            CalloutKind::Decision => &callout.decision,
            CalloutKind::Open => &callout.open,
        };
        CalloutPaint {
            accent: accent.accent.as_str(),
            fill: accent.fill.as_str(),
            border: Stroke {
                width_px: BLOCK_BORDER_PX,
                line: LineStyle::Solid,
                color: self.theme.card.border.color.as_str(),
            },
        }
    }

    /// The dashed border of a Frame block.
    pub fn frame_border(self) -> Stroke<'a> {
        Stroke {
            width_px: BLOCK_BORDER_PX,
            line: LineStyle::Dashed,
            color: self.theme.placeholder.border.as_str(),
        }
    }

    /// The two corner-to-corner lines of a Frame block.
    pub fn frame_diagonal(self) -> Stroke<'a> {
        Stroke {
            width_px: FRAME_DIAGONAL_PX,
            line: LineStyle::Solid,
            color: self.theme.placeholder.diagonal.as_str(),
        }
    }

    /// A Lanes lifeline (section 13.6).
    pub fn lifeline(self) -> Stroke<'a> {
        Stroke::of(&self.theme.lanes.lifeline)
    }

    /// The chip under a Frame label, so the diagonals stop at the words.
    pub fn frame_label_chip(self) -> &'a str {
        self.page_background()
    }

    /// The dot of a bulleted Text line, in the body ink.
    pub fn list_bullet(self, canvas: Canvas) -> &'a str {
        self.text_ink(TextStyleName::BlockBody, canvas, None)
    }

    /// Fill of the flat rounded square under every icon (section 11.1); None draws none.
    pub fn icon_chip(self) -> Option<&'a str> {
        self.theme.icon_chip.as_ref().map(|chip| chip.as_str())
    }

    /// Text color of a style. `canvas` decides `badge`; `line` is the line of the Pipe, Tee
    /// or Link a `tag_label` run belongs to. A Box label takes `container_label_ink`.
    pub fn text_ink(self, name: TextStyleName, canvas: Canvas, line: Option<Line>) -> &'a str {
        let theme = self.theme;
        let color = match name {
            TextStyleName::Badge => match canvas {
                Canvas::Customer => &theme.badge.customer.ink,
                Canvas::Internal => &theme.badge.internal.ink,
            },
            TextStyleName::Kicker => &theme.kicker,
            TextStyleName::Title | TextStyleName::CardFunction | TextStyleName::BlockBody => {
                &theme.ink.primary
            }
            TextStyleName::LegendLabel => &theme.legend.label_ink,
            TextStyleName::TagLabel => match line {
                Some(Line::Deny) => &theme.deny.tag_ink,
                Some(Line::Gray | Line::Solid | Line::Dash) | None => &theme.tag.ink,
            },
            TextStyleName::GcpBar => &theme.frame.bar_ink,
            TextStyleName::PerimeterLabel => theme
                .tones
                .accent
                .label_ink
                .as_ref()
                .unwrap_or(&theme.ink.zone_label),
            TextStyleName::ZoneLabel => &theme.ink.zone_label,
            TextStyleName::Fact => &theme.fact.ink,
            TextStyleName::Ask => &theme.ask.ink,
            TextStyleName::Lede | TextStyleName::CardProduct => &theme.ink.secondary,
            TextStyleName::TagSub => &theme.tag.sub_ink,
            TextStyleName::NoteLegend | TextStyleName::LegendText => &theme.legend.text_ink,
            TextStyleName::Foot => &theme.foot,
        };
        color.as_str()
    }
}

/// A visible face of an isometric slab or block (section 12.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Top,
    Left,
    Right,
}

/// Fills and strokes of the three faces of an isometric solid (section 12.6). A None fill
/// draws the face unfilled; a None stroke draws it unstroked.
/// Lightness steps of a figure's lit top and shaded side over the secondary ink.
const FIGURE_LIT_STEP: i8 = 14;
const FIGURE_SHADE_STEP: i8 = -12;

#[derive(Debug, Clone, PartialEq)]
pub struct FacePaint<'a> {
    pub top: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub top_stroke: Option<Stroke<'a>>,
    pub side_stroke: Option<Stroke<'a>>,
}

impl<'a> Palette<'a> {
    /// HSL lightness step of a face in percentage points (`iso.faces`).
    pub fn face_lightness_step(self, face: Face) -> i8 {
        let faces = &self.theme.iso.faces;
        match face {
            Face::Top => faces.top,
            Face::Left => faces.left,
            Face::Right => faces.right,
        }
    }

    /// The fill of `face` for a node whose flat fill is `base`: base shaded by the face's
    /// step. None when `base` is not `#RRGGBB`.
    pub fn face_fill(self, base: &str, face: Face) -> Option<String> {
        shade(base, self.face_lightness_step(face))
    }

    /// The stroke of an isometric block face: the node's flat border, or
    /// `iso.block_outline` for a node whose flat drawing has none.
    pub fn face_outline(self, flat_border: Option<Stroke<'a>>) -> Option<Stroke<'a>> {
        flat_border.or_else(|| self.theme.iso.block_outline.as_ref().map(Stroke::of))
    }

    /// The outline behind upright billboard text that has no chip (section 12.4).
    pub fn text_halo(self) -> &'a str {
        self.page_background()
    }

    /// The ink of a Box name lying on its slab (section 12.6 rule 8): the frame's name in
    /// `iso.labels.frame`; a tinted Box in its tint's ink; any other Box in
    /// `iso.labels.zone`.
    /// The paint of a figure sprite: a dark silhouette in the secondary ink, its top lit
    /// and its right side shaded like a block's faces.
    pub fn figure_paint(self) -> FacePaint<'a> {
        let ink = self.theme.ink.secondary.as_str();
        FacePaint {
            top: shade(ink, FIGURE_LIT_STEP).or_else(|| Some(ink.to_string())),
            left: Some(ink.to_string()),
            right: shade(ink, FIGURE_SHADE_STEP).or_else(|| Some(ink.to_string())),
            top_stroke: None,
            side_stroke: None,
        }
    }

    pub fn iso_label_ink(self, look: ContainerLook, tint: Option<u8>) -> &'a str {
        if look.role == Role::Frame {
            return self.theme.iso.labels.frame.as_str();
        }
        match self.box_tint(look, tint) {
            Some(slot) => slot.ink.as_str(),
            None => self.theme.iso.labels.zone.as_str(),
        }
    }

    /// The faces of a Box slab (section 12.6 over the theme's roles): the top in the flat
    /// fill, the sides shaded by the face steps or, for a frame, `iso.frame_sides`. A solid
    /// border is dropped under `solid_edges: none`, where the shaded sides carry the edge,
    /// and drawn at `edge_width` on the top and the sides under `outline`. A dashed or
    /// dotted border is drawn on the top face only; a ring takes `iso.ring` when set. A
    /// frame's top outline is `iso.frame_outline` wide. None when a color does not parse.
    pub fn slab_faces(self, look: ContainerLook, tint: Option<u8>) -> Option<FacePaint<'a>> {
        let iso = &self.theme.iso;
        let style = self.zone_style(look, tint);
        let is_frame = look.role == Role::Frame;
        let base = if is_frame {
            Some(self.frame_body_fill())
        } else {
            style.fill
        };
        let (left, right) = match (is_frame, &iso.frame_sides, base) {
            (true, Some(sides), _) => (
                Some(sides.left.as_str().to_string()),
                Some(sides.right.as_str().to_string()),
            ),
            (_, _, Some(color)) => (
                Some(self.face_fill(color, Face::Left)?),
                Some(self.face_fill(color, Face::Right)?),
            ),
            (_, _, None) => (None, None),
        };
        let top = match base {
            Some(color) => Some(self.face_fill(color, Face::Top)?),
            None => None,
        };
        let border = style.border;
        let solid_border = border.filter(|stroke| stroke.line == LineStyle::Solid);
        let edge = |stroke: Stroke<'a>| Stroke {
            width_px: iso.edge_width,
            ..stroke
        };
        let top_stroke = if is_frame {
            border.map(|stroke| Stroke {
                width_px: iso.frame_outline,
                ..stroke
            })
        } else if look.is_ring(tint)
            && let Some(ring) = &iso.ring
        {
            Some(Stroke::of(ring))
        } else {
            match (solid_border, iso.solid_edges) {
                (Some(_), SolidEdges::None) => None,
                (Some(stroke), SolidEdges::Outline) => Some(edge(stroke)),
                (None, _) => border,
            }
        };
        let side_stroke = match iso.solid_edges {
            SolidEdges::None => None,
            SolidEdges::Outline => solid_border.map(edge),
        };
        Some(FacePaint {
            top,
            left,
            right,
            top_stroke,
            side_stroke,
        })
    }

    /// The faces of a leaf block whose flat fill is `fill` and flat border `border`: every
    /// face stroked with `face_outline`, each face the flat fill shaded by its step. None
    /// when a color does not parse.
    pub fn block_faces(
        self,
        fill: Option<&'a str>,
        border: Option<Stroke<'a>>,
    ) -> Option<FacePaint<'a>> {
        let stroke = self.face_outline(border);
        let (top, left, right) = match fill {
            None => (None, None, None),
            Some(color) => (
                Some(self.face_fill(color, Face::Top)?),
                Some(self.face_fill(color, Face::Left)?),
                Some(self.face_fill(color, Face::Right)?),
            ),
        };
        Some(FacePaint {
            top,
            left,
            right,
            top_stroke: stroke,
            side_stroke: stroke,
        })
    }

    /// The rounded square under every icon under iso, ringed when `iso.chip.ring` is set.
    pub fn iso_icon_chip(self) -> BoxPaint<'a> {
        let chip = &self.theme.iso.chip;
        BoxPaint {
            fill: chip.fill.as_str(),
            border: chip.ring.as_ref().map(|ring| Stroke {
                width_px: ISO_ICON_CHIP_RING_PX,
                line: LineStyle::Solid,
                color: ring.as_str(),
            }),
        }
    }

    /// The soft shadow under an iso icon chip, when the theme sets one.
    pub fn iso_icon_chip_shadow(self) -> Option<ChipShadowPaint<'a>> {
        self.theme
            .iso
            .chip
            .shadow
            .as_ref()
            .map(|shadow| ChipShadowPaint {
                color: shadow.color.as_str(),
                opacity: shadow.opacity,
                dy: shadow.dy,
            })
    }

    /// The shadow under every opaque iso block, when the theme sets one.
    pub fn iso_block_shadow(self) -> Option<BlockShadowPaint<'a>> {
        self.theme
            .iso
            .shadow
            .as_ref()
            .map(|shadow| BlockShadowPaint {
                color: shadow.color.as_str(),
                opacity: shadow.opacity,
                blur: shadow.blur,
                dy: shadow.dy,
            })
    }
}

/// `color` moved `fraction` of the way to `toward` in each sRGB channel, as `#RRGGBB`.
/// None when either is not `#` followed by six hex digits.
pub fn mix(color: &str, toward: &str, fraction: f32) -> Option<String> {
    let from = parse_hex_color(color)?;
    let to = parse_hex_color(toward)?;
    let fraction = fraction.clamp(0.0, 1.0);
    let channel = |index: usize| -> Option<u8> {
        let start = f32::from(*from.get(index)?);
        let end = f32::from(*to.get(index)?);
        // The clamp keeps the cast in range, so `as` never saturates.
        Some(
            (start + (end - start) * fraction + 0.5)
                .floor()
                .clamp(0.0, 255.0) as u8,
        )
    };
    Some(format!(
        "#{:02X}{:02X}{:02X}",
        channel(0)?,
        channel(1)?,
        channel(2)?
    ))
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
