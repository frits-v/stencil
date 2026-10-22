//! The flat writer's drawing of the step-9 sheet surfaces (section 14.5 rules 6 to 9):
//! outlined Items, rails, step badges, valves, hollow planned lines, hops, and every bend
//! drawn as a solid joint on a patterned sheet line.

use stencil_layout::{
    BoxRect, DrawnLink, LinkRoute, NodeGeometry, NodeTag, Part, PartName, PathPiece, dash_runs,
};
use stencil_model::{Link, Outline, PagePoint};

use super::{PartContext, SvgWriter, inset_rect, link_path, stroke_attributes};
use crate::format_number;
use crate::palette::{self, LineStyle, LineUse, SheetInk, Stroke};

/// The pattern of a planned line (section 14.5 rule 8): the outer stroke's dash and gap,
/// and the paper core's, offset so each outer dash holds a closed core.
const PLANNED_OUTER: (f32, f32) = (7.0, 4.0);
const PLANNED_CORE: (f32, f32) = (5.0, 6.0);
/// A planned line's outer stroke is this much wider than its line role's width.
const PLANNED_CASING_PX: f32 = 2.0;
/// A valve's outline.
const VALVE_STROKE_PX: f32 = 1.2;

/// The dash and gap of a patterned stroke; None for a solid one.
fn pattern(line: LineStyle) -> Option<(f32, f32)> {
    match line {
        LineStyle::Solid => None,
        LineStyle::Dashed => Some((6.0, 5.0)),
        LineStyle::Dotted => Some((2.0, 3.0)),
    }
}

/// `stroke-dasharray` and `stroke-dashoffset` for a run that starts on a dash (the first
/// run of a line) or in a gap (every run after a joint), or nothing for a joint.
fn dash_attributes((dash, gap): (f32, f32), shift: f32, starts_on: bool, joint: bool) -> String {
    if joint {
        return String::new();
    }
    let offset = if starts_on { shift } else { dash + shift };
    let offset_part = if offset == 0.0 {
        String::new()
    } else {
        format!(r#" stroke-dashoffset="{}""#, format_number(offset))
    };
    format!(
        r#" stroke-dasharray="{} {}"{offset_part}"#,
        format_number(dash),
        format_number(gap)
    )
}

/// The path of one run: from its start through its pieces.
fn run_path(start: PagePoint, pieces: &[PathPiece]) -> String {
    link_path(&DrawnLink {
        start,
        legs: vec![pieces.to_vec()],
        start_arrow: None,
        end_arrow: None,
    })
}

impl SvgWriter<'_> {
    /// The line of a sheet link. A solid built line is one path, its hops in it. A
    /// patterned or planned line is cut at every bend and hop into runs, and each joint
    /// around a bend or hop is drawn solid, so every turn shows whatever the dash phase;
    /// the run after a joint starts in a gap (section 14.5 rule 1 of the bends). A planned
    /// line draws each run twice, a wider stroke in the wire and a paper core, so each
    /// dash is a closed outline.
    pub(super) fn write_sheet_link_line(
        &mut self,
        depth: usize,
        drawn: &DrawnLink,
        line_use: LineUse,
        planned: bool,
    ) {
        let stroke = self.palette.wire_style(line_use).stroke;
        let dashes = pattern(stroke.line);
        if !planned && dashes.is_none() {
            self.line(
                depth,
                &format!(
                    r#"<path d="{}" fill="none"{} stroke-linejoin="round"/>"#,
                    link_path(drawn),
                    stroke_attributes(stroke)
                ),
            );
            return;
        }
        let paper = self.palette.sheet_ink(SheetInk::Paper).to_string();
        for (index, run) in dash_runs(drawn).iter().enumerate() {
            let path = run_path(run.start, &run.pieces);
            let starts_on = index == 0;
            if planned {
                self.line(
                    depth,
                    &format!(
                        r#"<path d="{path}" fill="none" stroke="{}" stroke-width="{}"{}/>"#,
                        stroke.color,
                        format_number(stroke.width_px + PLANNED_CASING_PX),
                        dash_attributes(PLANNED_OUTER, 0.0, starts_on, run.joint)
                    ),
                );
                self.line(
                    depth,
                    &format!(
                        r#"<path d="{path}" fill="none" stroke="{paper}" stroke-width="{}"{}/>"#,
                        format_number(stroke.width_px),
                        dash_attributes(PLANNED_CORE, -1.0, starts_on, run.joint)
                    ),
                );
            } else if let Some(dashes) = dashes {
                let solid = Stroke {
                    line: LineStyle::Solid,
                    ..stroke
                };
                self.line(
                    depth,
                    &format!(
                        r#"<path d="{path}" fill="none"{}{}/>"#,
                        stroke_attributes(solid),
                        dash_attributes(dashes, 0.0, starts_on, run.joint)
                    ),
                );
            }
        }
    }

    /// A step badge, a valve or a rail dot on a sheet link, in the line's wire; a hop draws
    /// nothing of its own, since the line's path holds it. False for every other part.
    pub(super) fn write_link_mark(
        &mut self,
        depth: usize,
        route: &LinkRoute,
        part: &Part,
        line_use: LineUse,
    ) -> bool {
        let wire = self.palette.wire_style(line_use).stroke.color.to_string();
        match part.name {
            PartName::StepBadge => {
                self.write_step_badge(depth, part.bounds, &wire);
                true
            }
            PartName::ValveNear | PartName::ValveFar => {
                let valve = stencil_layout::sheet_lines::valve_box(&route.parts);
                if let Some(valve) = valve {
                    self.write_valve_half(depth, part.bounds, valve, &wire);
                }
                true
            }
            PartName::RailDot => {
                let bounds = part.bounds;
                self.line(
                    depth,
                    &format!(
                        r#"<circle cx="{}" cy="{}" r="{}" fill="{wire}"/>"#,
                        format_number(bounds.x + bounds.width / 2.0),
                        format_number(bounds.y + bounds.height / 2.0),
                        format_number(bounds.width.min(bounds.height) / 2.0),
                    ),
                );
                true
            }
            PartName::Hop => true,
            _ => false,
        }
    }

    /// A step badge: an 18 px square with rounded corners on the tag's fill, outlined in
    /// the line's wire (section 14.5 rule 4).
    fn write_step_badge(&mut self, depth: usize, bounds: BoxRect, stroke_color: &str) {
        let fill = self.palette.sheet_ink(SheetInk::TagFill).to_string();
        let stroke = Stroke {
            width_px: palette::STEP_BADGE_STROKE_PX,
            line: LineStyle::Solid,
            color: stroke_color,
        };
        self.write_rect(
            depth,
            bounds,
            palette::STEP_BADGE_RADIUS_PX,
            Some(&fill),
            Some(stroke),
        );
    }

    /// One triangle of a valve: its base on the half's outer edge across the leg, its tip
    /// at the valve's centre, outlined in the wire on the paper.
    fn write_valve_half(&mut self, depth: usize, half: BoxRect, valve: BoxRect, wire: &str) {
        let center = (valve.x + valve.width / 2.0, valve.y + valve.height / 2.0);
        let horizontal = valve.width >= valve.height;
        let inset = VALVE_STROKE_PX / 2.0;
        let points = if horizontal {
            let outer = if half.x < center.0 - 0.01 {
                half.x + inset
            } else {
                half.right() - inset
            };
            [
                (outer, half.y + inset),
                (outer, half.bottom() - inset),
                (center.0, center.1),
            ]
        } else {
            let outer = if half.y < center.1 - 0.01 {
                half.y + inset
            } else {
                half.bottom() - inset
            };
            [
                (half.x + inset, outer),
                (half.right() - inset, outer),
                (center.0, center.1),
            ]
        };
        let paper = self.palette.sheet_ink(SheetInk::Paper).to_string();
        let path = format!(
            "M{} {} L{} {} L{} {} Z",
            format_number(points[0].0),
            format_number(points[0].1),
            format_number(points[1].0),
            format_number(points[1].1),
            format_number(points[2].0),
            format_number(points[2].1),
        );
        self.line(
            depth,
            &format!(
                r#"<path d="{path}" fill="{paper}" stroke="{wire}" stroke-width="{}" stroke-linejoin="round"/>"#,
                format_number(VALVE_STROKE_PX)
            ),
        );
    }

    /// The parts a sheet draws differently from their paint row: an outlined Item's front
    /// and back cards, a rail's bar, a pipe's or the strip's step badge, and a rail's or a
    /// planned entry's sample. True when the part was drawn here.
    pub(super) fn write_assembly_part(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        part: &Part,
        (context, slot): (PartContext<'_>, Option<u8>),
    ) -> bool {
        let bounds = part.bounds;
        match (node.tag, part.name) {
            (NodeTag::Pcard, PartName::Front | PartName::StackBack | PartName::Head) => {
                self.write_outline_card(depth, bounds, (part.name, context), slot);
                true
            }
            (NodeTag::Pipe, PartName::Rail) => {
                let Some(line_use) = context.line else {
                    return false;
                };
                let color = self.palette.wire_style(line_use).stroke.color.to_string();
                let bar = stencil_layout::RAIL_BAR_PX;
                let center_x = bounds.x + bounds.width / 2.0;
                self.line(
                    depth,
                    &format!(
                        r#"<line x1="{x}" y1="{}" x2="{x}" y2="{}" stroke="{color}" stroke-width="{}" stroke-linecap="round"/>"#,
                        format_number(bounds.y + bar / 2.0),
                        format_number(bounds.bottom() - bar / 2.0),
                        format_number(bar),
                        x = format_number(center_x),
                    ),
                );
                true
            }
            (_, PartName::StepBadge) => {
                // A pipe's badge takes its line's wire; the strip's sample is unowned.
                let color = match context.line {
                    Some(line_use) => self.palette.wire_style(line_use).stroke.color.to_string(),
                    None => self.palette.sheet_ink(SheetInk::Frame).to_string(),
                };
                self.write_step_badge(depth, bounds, &color);
                true
            }
            (NodeTag::LegendEntry, PartName::Swatch) if context.planned || context.rail => {
                let Some(line_use) = context.line else {
                    return false;
                };
                let color = if context.by_actor {
                    self.palette.sheet_ink(SheetInk::Frame).to_string()
                } else {
                    self.palette.wire_style(line_use).stroke.color.to_string()
                };
                let width = self.palette.wire_style(line_use).stroke.width_px;
                if context.rail {
                    let center_x = bounds.x + bounds.width / 2.0;
                    self.line(
                        depth,
                        &format!(
                            r#"<line x1="{x}" y1="{}" x2="{x}" y2="{}" stroke="{color}" stroke-width="{}" stroke-linecap="round"/>"#,
                            format_number(bounds.y + bounds.width / 2.0),
                            format_number(bounds.bottom() - bounds.width / 2.0),
                            format_number(bounds.width),
                            x = format_number(center_x),
                        ),
                    );
                } else {
                    let center_y = bounds.y + bounds.height / 2.0;
                    let path = format!(
                        "M {} {} H {}",
                        format_number(bounds.x),
                        format_number(center_y),
                        format_number(bounds.right())
                    );
                    let paper = self.palette.sheet_ink(SheetInk::Paper).to_string();
                    self.line(
                        depth,
                        &format!(
                            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{}"{}/>"#,
                            format_number(width + PLANNED_CASING_PX),
                            dash_attributes(PLANNED_OUTER, 0.0, true, false)
                        ),
                    );
                    self.line(
                        depth,
                        &format!(
                            r#"<path d="{path}" fill="none" stroke="{paper}" stroke-width="{}"{}/>"#,
                            format_number(width),
                            dash_attributes(PLANNED_CORE, -1.0, true, false)
                        ),
                    );
                }
                true
            }
            _ => false,
        }
    }

    /// An outlined Item's card (section 14.5 rule 6): a store's cylinder, a stack's front
    /// and back cards, a person's head, each in the actor's colours; planned draws the same
    /// outline dashed with no fill.
    fn write_outline_card(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        (name, context): (PartName, PartContext<'_>),
        slot: Option<u8>,
    ) {
        let wire = self.palette.sheet_color(SheetInk::TintWire, slot);
        let fill = if context.planned {
            "none".to_string()
        } else {
            match name {
                PartName::Head => self.palette.sheet_color(SheetInk::TintFill, slot),
                _ => self.palette.sheet_color(SheetInk::Cell, slot),
            }
        };
        let width = palette::STEP_BORDER_PX;
        let dash = if context.planned {
            format!(r#" stroke-dasharray="{}""#, palette::PLANNED_DASH)
        } else {
            String::new()
        };
        let stroke = format!(
            r#" stroke="{wire}" stroke-width="{}"{dash}"#,
            format_number(width)
        );
        match (name, context.outline) {
            (PartName::Front, Some(Outline::Store)) => {
                let body = inset_rect(bounds, width / 2.0);
                let ry = stencil_layout::STORE_LID_RY_PX - width / 2.0;
                let rx = body.width / 2.0;
                let lid = if context.planned {
                    "none".to_string()
                } else {
                    self.palette.sheet_color(SheetInk::LaneBand, slot)
                };
                let path = format!(
                    "M {} {} V {} A {} {} 0 0 0 {} {} V {}",
                    format_number(body.x),
                    format_number(body.y + ry),
                    format_number(body.bottom() - ry),
                    format_number(rx),
                    format_number(ry),
                    format_number(body.right()),
                    format_number(body.bottom() - ry),
                    format_number(body.y + ry),
                );
                self.line(
                    depth,
                    &format!(r#"<path d="{path}" fill="{fill}"{stroke}/>"#),
                );
                self.line(
                    depth,
                    &format!(
                        r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{lid}"{stroke}/>"#,
                        format_number(body.x + rx),
                        format_number(body.y + ry),
                        format_number(rx),
                        format_number(ry),
                    ),
                );
            }
            (PartName::Head, _) => {
                let radius = bounds.width.min(bounds.height) / 2.0 - width / 2.0;
                self.line(
                    depth,
                    &format!(
                        r#"<circle cx="{}" cy="{}" r="{}" fill="{fill}"{stroke}/>"#,
                        format_number(bounds.x + bounds.width / 2.0),
                        format_number(bounds.y + bounds.height / 2.0),
                        format_number(radius.max(0.0)),
                    ),
                );
            }
            _ => {
                let rect = inset_rect(bounds, width / 2.0);
                self.line(
                    depth,
                    &format!(
                        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}"{stroke}/>"#,
                        format_number(rect.x),
                        format_number(rect.y),
                        format_number(rect.width),
                        format_number(rect.height),
                    ),
                );
            }
        }
    }

    /// A planned Item card or Box on a sheet: its border box outlined dashed in its
    /// colour, with no fill.
    pub(super) fn write_planned_box(&mut self, depth: usize, bounds: BoxRect, color: &str) {
        let width = palette::STEP_BORDER_PX;
        let rect = inset_rect(bounds, width / 2.0);
        self.line(
            depth,
            &format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="{color}" stroke-width="{}" stroke-dasharray="{}"/>"#,
                format_number(rect.x),
                format_number(rect.y),
                format_number(rect.width),
                format_number(rect.height),
                format_number(width),
                palette::PLANNED_DASH,
            ),
        );
    }
}

/// True for a link a sheet draws hollow.
pub(super) fn is_planned(link: Option<&Link>) -> bool {
    link.is_some_and(|link| link.planned)
}
