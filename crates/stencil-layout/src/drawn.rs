//! A routed link as the writer draws it (section 11.6): the polyline shortened under its
//! arrowheads, each bend square, rounded or splined. The flat writer formats it and
//! links-avoid-boxes reads it, so the check examines the drawn line.

use stencil_model::{Arrow, Bend, Line, PagePoint};

use crate::route::{Obstacle, link_obstacles, segment_enters};
use crate::{
    ARROWHEAD_LENGTH_PX, ARROWHEAD_WIDTH_PX, BoxRect, GEOMETRY_EPSILON_PX, LinkRoute, PageGeometry,
    STOP_PLATE_THICKNESS_PX, STOP_PLATE_WIDTH_PX,
};

/// The largest |cos| between two legs that still counts as a right angle.
const PERPENDICULAR_COS_MAX: f32 = 1e-3;
/// The smallest |sin| between two legs a spline piece turns through.
const TURN_SINE_MIN: f32 = 1e-3;
/// The chords a curved piece is read as.
const CURVE_CHORDS: usize = 8;

/// One rounded bend, in flat (page or floor) coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fillet {
    /// Where the bend leaves the incoming leg, `radius` short of the corner.
    pub start: (f32, f32),
    /// Where the bend joins the outgoing leg, `radius` past the corner.
    pub end: (f32, f32),
    pub radius: f32,
    /// The turn is clockwise with y pointing down: the SVG sweep flag 1.
    pub clockwise: bool,
}

/// The bend at `corner` between the legs from `previous` and to `next`, with its radius
/// clamped to half the shorter leg, so two bends on one leg never overlap. None when the
/// clamped radius is nothing or the legs are not perpendicular; that corner stays square.
pub fn fillet(
    previous: (f32, f32),
    corner: (f32, f32),
    next: (f32, f32),
    corner_px: f32,
) -> Option<Fillet> {
    let incoming = (corner.0 - previous.0, corner.1 - previous.1);
    let outgoing = (next.0 - corner.0, next.1 - corner.1);
    let incoming_length = incoming.0.hypot(incoming.1);
    let outgoing_length = outgoing.0.hypot(outgoing.1);
    let radius = corner_px.min(incoming_length.min(outgoing_length) / 2.0);
    if !radius.is_finite() || radius < GEOMETRY_EPSILON_PX {
        return None;
    }
    let incoming = (incoming.0 / incoming_length, incoming.1 / incoming_length);
    let outgoing = (outgoing.0 / outgoing_length, outgoing.1 / outgoing_length);
    let cos = incoming.0 * outgoing.0 + incoming.1 * outgoing.1;
    if cos.abs() > PERPENDICULAR_COS_MAX {
        return None;
    }
    Some(Fillet {
        start: (
            corner.0 - incoming.0 * radius,
            corner.1 - incoming.1 * radius,
        ),
        end: (
            corner.0 + outgoing.0 * radius,
            corner.1 + outgoing.1 * radius,
        ),
        radius,
        clockwise: incoming.0 * outgoing.1 - incoming.1 * outgoing.0 > 0.0,
    })
}

/// One drawn piece, from wherever the previous piece ended to `to`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathPiece {
    Line(PagePoint),
    /// A quarter circle that rounds `corner`.
    Arc {
        radius: f32,
        clockwise: bool,
        corner: PagePoint,
        to: PagePoint,
    },
    /// A cubic with both control points on `corner`.
    Cubic {
        corner: PagePoint,
        to: PagePoint,
    },
    /// A quadratic with its control point on `corner`.
    Quad {
        corner: PagePoint,
        to: PagePoint,
    },
}

impl PathPiece {
    pub fn to(self) -> PagePoint {
        match self {
            PathPiece::Line(to)
            | PathPiece::Arc { to, .. }
            | PathPiece::Cubic { to, .. }
            | PathPiece::Quad { to, .. } => to,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawnLink {
    /// The first drawn point: the route's first point, or the arrowhead base at an arrowed
    /// start.
    pub start: PagePoint,
    /// The pieces along each route leg in order, the bend at the leg's far corner included.
    pub legs: Vec<Vec<PathPiece>>,
    /// (base, tip) of the mark at each arrowed end: the arrowhead, or a deny line's stop
    /// plate, whose base is its inner face.
    pub start_arrow: Option<(PagePoint, PagePoint)>,
    pub end_arrow: Option<(PagePoint, PagePoint)>,
    pub cap: EndCap,
}

/// What a line draws at an arrowed end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndCap {
    #[default]
    Arrowhead,
    /// A deny line ends in a bar across it on the endpoint's outline (section 11.2).
    StopPlate,
}

impl EndCap {
    pub fn of(line: Line) -> Self {
        match line {
            Line::Deny => EndCap::StopPlate,
            Line::Gray | Line::Solid | Line::Dash => EndCap::Arrowhead,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            EndCap::Arrowhead => "arrowhead",
            EndCap::StopPlate => "stop plate",
        }
    }

    /// How far the mark reaches back from the tip along the line.
    pub fn length_px(self) -> f32 {
        match self {
            EndCap::Arrowhead => ARROWHEAD_LENGTH_PX,
            EndCap::StopPlate => STOP_PLATE_THICKNESS_PX,
        }
    }

    /// How wide the mark is across the line.
    pub fn width_px(self) -> f32 {
        match self {
            EndCap::Arrowhead => ARROWHEAD_WIDTH_PX,
            EndCap::StopPlate => STOP_PLATE_WIDTH_PX,
        }
    }

    /// The box the mark covers, from its (base, tip) on a horizontal or vertical run.
    pub fn bounds(self, (base, tip): (PagePoint, PagePoint)) -> BoxRect {
        let half = self.width_px() / 2.0;
        let (x_low, x_high) = (base.x.min(tip.x), base.x.max(tip.x));
        let (y_low, y_high) = (base.y.min(tip.y), base.y.max(tip.y));
        if (base.x - tip.x).abs() > (base.y - tip.y).abs() {
            BoxRect {
                x: x_low,
                y: tip.y - half,
                width: x_high - x_low,
                height: 2.0 * half,
            }
        } else {
            BoxRect {
                x: tip.x - half,
                y: y_low,
                width: 2.0 * half,
                height: y_high - y_low,
            }
        }
    }
}

/// The drawn line of `route`. A spline piece that would enter a box the route avoided
/// falls back to a curve of the route's corner radius at that bend.
pub fn drawn_link(geometry: &PageGeometry, route: &LinkRoute) -> DrawnLink {
    let mut points = route.points.clone();
    let cap = EndCap::of(route.line);
    let start_arrow = if matches!(route.arrow, Arrow::Start | Arrow::Both) {
        trim_start(&mut points, cap.length_px())
    } else {
        None
    };
    let end_arrow = if matches!(route.arrow, Arrow::End | Arrow::Both) {
        trim_end(&mut points, cap.length_px())
    } else {
        None
    };
    let obstacles = if route.bend == Bend::Spline {
        link_obstacles(geometry, route.from_node, route.to_node)
    } else {
        Vec::new()
    };
    let start = points
        .first()
        .copied()
        .unwrap_or(PagePoint { x: 0.0, y: 0.0 });
    let mut legs = Vec::with_capacity(points.len().saturating_sub(1));
    let mut at = start;
    for index in 1..points.len() {
        let (Some(&previous), Some(&point)) = (points.get(index - 1), points.get(index)) else {
            continue;
        };
        let turn = points
            .get(index + 1)
            .and_then(|next| bend_piece(at, previous, point, *next, route, &obstacles));
        let leg = match turn {
            Some((entry, piece)) => vec![PathPiece::Line(entry), piece],
            None => vec![PathPiece::Line(point)],
        };
        at = leg.last().map_or(at, |piece| piece.to());
        legs.push(leg);
    }
    let hops: Vec<PagePoint> = route
        .parts
        .iter()
        .filter(|part| part.name == crate::PartName::Hop)
        .map(crate::sheet_lines::hop_point)
        .collect();
    if !hops.is_empty() {
        insert_hops(start, &mut legs, &hops);
    }
    DrawnLink {
        start,
        legs,
        start_arrow,
        end_arrow,
        cap,
    }
}

/// Replaces the stretch of a straight piece around each hop point with a semicircle of
/// `HOP_RADIUS_PX` drawn as two quarter arcs, bulging toward -y on a horizontal piece and
/// toward -x on a vertical one (section 14.5 rule 9).
fn insert_hops(start: PagePoint, legs: &mut [Vec<PathPiece>], hops: &[PagePoint]) {
    let radius = crate::sheet_lines::HOP_RADIUS_PX;
    let mut at = start;
    for leg in legs.iter_mut() {
        let mut pieces = Vec::with_capacity(leg.len());
        for piece in leg.iter() {
            let PathPiece::Line(to) = *piece else {
                at = piece.to();
                pieces.push(*piece);
                continue;
            };
            let horizontal = (to.y - at.y).abs() < GEOMETRY_EPSILON_PX;
            let vertical = (to.x - at.x).abs() < GEOMETRY_EPSILON_PX;
            let length = (to.x - at.x).abs() + (to.y - at.y).abs();
            let mut on_piece: Vec<(f32, PagePoint)> = hops
                .iter()
                .filter(|hop| {
                    (horizontal
                        && (hop.y - at.y).abs() < GEOMETRY_EPSILON_PX
                        && hop.x > at.x.min(to.x)
                        && hop.x < at.x.max(to.x))
                        || (vertical
                            && (hop.x - at.x).abs() < GEOMETRY_EPSILON_PX
                            && hop.y > at.y.min(to.y)
                            && hop.y < at.y.max(to.y))
                })
                .map(|hop| ((hop.x - at.x).abs() + (hop.y - at.y).abs(), *hop))
                .collect();
            on_piece.sort_by(|first, second| first.0.total_cmp(&second.0));
            if length <= GEOMETRY_EPSILON_PX || on_piece.is_empty() {
                pieces.push(*piece);
                at = to;
                continue;
            }
            let unit = ((to.x - at.x) / length, (to.y - at.y) / length);
            // The bulge: -y across a horizontal piece, -x across a vertical one.
            let bulge = if horizontal { (0.0, -1.0) } else { (-1.0, 0.0) };
            for (_, hop) in on_piece {
                let before = PagePoint {
                    x: hop.x - unit.0 * radius,
                    y: hop.y - unit.1 * radius,
                };
                let after = PagePoint {
                    x: hop.x + unit.0 * radius,
                    y: hop.y + unit.1 * radius,
                };
                let apex = PagePoint {
                    x: hop.x + bulge.0 * radius,
                    y: hop.y + bulge.1 * radius,
                };
                let first_corner = PagePoint {
                    x: before.x + bulge.0 * radius,
                    y: before.y + bulge.1 * radius,
                };
                let second_corner = PagePoint {
                    x: after.x + bulge.0 * radius,
                    y: after.y + bulge.1 * radius,
                };
                let turn = |from: PagePoint, corner: PagePoint, to: PagePoint| {
                    let incoming = (corner.x - from.x, corner.y - from.y);
                    let outgoing = (to.x - corner.x, to.y - corner.y);
                    incoming.0 * outgoing.1 - incoming.1 * outgoing.0 > 0.0
                };
                pieces.push(PathPiece::Line(before));
                pieces.push(PathPiece::Arc {
                    radius,
                    clockwise: turn(before, first_corner, apex),
                    corner: first_corner,
                    to: apex,
                });
                pieces.push(PathPiece::Arc {
                    radius,
                    clockwise: turn(apex, second_corner, after),
                    corner: second_corner,
                    to: after,
                });
            }
            pieces.push(PathPiece::Line(to));
            at = to;
        }
        *leg = pieces;
    }
}

/// One stretch of a drawn line on a design sheet (section 14.5 rule 1 of the bends): a
/// joint is drawn solid around a bend or a hop, so the turn always shows; a run between
/// joints carries the line's pattern, starting in a gap after a joint.
#[derive(Debug, Clone, PartialEq)]
pub struct DashRun {
    pub start: PagePoint,
    pub pieces: Vec<PathPiece>,
    pub joint: bool,
    /// Where the run starts along the whole line, in px.
    pub from_px: f32,
    pub length_px: f32,
}

/// How far a joint reaches along the straight on each side of a bend or hop: half the
/// longest dash, so the joint reads as one dash turning the corner.
pub const JOINT_REACH_PX: f32 = 3.5;

/// The length of a piece that starts at `from`.
pub fn piece_length(from: PagePoint, piece: PathPiece) -> f32 {
    let mut chain = vec![from];
    push_flattened(&mut chain, from, piece);
    chain
        .windows(2)
        .filter_map(|pair| Some((*pair.first()?, *pair.get(1)?)))
        .map(|(a, b)| (b.x - a.x).hypot(b.y - a.y))
        .sum()
}

/// The pieces of a drawn link in order, each with its start point.
fn flat_pieces(drawn: &DrawnLink) -> Vec<(PagePoint, PathPiece)> {
    let mut pieces = Vec::new();
    let mut at = drawn.start;
    for piece in drawn.legs.iter().flatten() {
        pieces.push((at, *piece));
        at = piece.to();
    }
    pieces
}

/// The point `distance` along the straight from `from` to `to`.
fn along(from: PagePoint, to: PagePoint, distance: f32) -> PagePoint {
    let length = (to.x - from.x).hypot(to.y - from.y);
    if length <= GEOMETRY_EPSILON_PX {
        return from;
    }
    PagePoint {
        x: from.x + (to.x - from.x) * distance / length,
        y: from.y + (to.y - from.y) * distance / length,
    }
}

/// The intervals, in px along the line, that a sheet draws solid: each curved piece (a
/// rounded bend or a hop arc) and each square corner, reaching `JOINT_REACH_PX` along the
/// straight on both sides, merged where they meet and clamped to the line.
pub fn joint_intervals(drawn: &DrawnLink) -> Vec<(f32, f32)> {
    let pieces = flat_pieces(drawn);
    let mut intervals: Vec<(f32, f32)> = Vec::new();
    let mut travelled = 0.0;
    let mut previous_direction: Option<(f32, f32)> = None;
    for (from, piece) in &pieces {
        let length = piece_length(*from, *piece);
        match piece {
            PathPiece::Line(to) => {
                if length > GEOMETRY_EPSILON_PX {
                    let direction = ((to.x - from.x) / length, (to.y - from.y) / length);
                    // Two straights meeting at an angle: a square corner.
                    if let Some(before) = previous_direction {
                        let cos = before.0 * direction.0 + before.1 * direction.1;
                        if cos < 1.0 - 1e-3 {
                            intervals
                                .push((travelled - JOINT_REACH_PX, travelled + JOINT_REACH_PX));
                        }
                    }
                    previous_direction = Some(direction);
                }
            }
            PathPiece::Arc { .. } | PathPiece::Cubic { .. } | PathPiece::Quad { .. } => {
                intervals.push((
                    travelled - JOINT_REACH_PX,
                    travelled + length + JOINT_REACH_PX,
                ));
                previous_direction = None;
            }
        }
        travelled += length;
    }
    let total = travelled;
    let mut merged: Vec<(f32, f32)> = Vec::new();
    for (low, high) in intervals {
        let (low, high) = (low.max(0.0), high.min(total));
        match merged.last_mut() {
            Some(last) if low <= last.1 => last.1 = last.1.max(high),
            _ => merged.push((low, high)),
        }
    }
    merged
}

/// The drawn link cut into runs at the joint intervals (section 14.5 rule 1 of the bends).
/// A boundary always falls on a straight piece, since every curve lies inside a joint.
pub fn dash_runs(drawn: &DrawnLink) -> Vec<DashRun> {
    let joints = joint_intervals(drawn);
    let pieces = flat_pieces(drawn);
    let mut cuts: Vec<(f32, bool)> = Vec::new();
    for (low, high) in &joints {
        cuts.push((*low, true));
        cuts.push((*high, false));
    }
    let mut runs: Vec<DashRun> = Vec::new();
    let mut current = DashRun {
        start: drawn.start,
        pieces: Vec::new(),
        joint: joints
            .first()
            .is_some_and(|(low, _)| *low <= GEOMETRY_EPSILON_PX),
        from_px: 0.0,
        length_px: 0.0,
    };
    let mut cut_index = usize::from(current.joint);
    let mut travelled = 0.0;
    for (from, piece) in pieces {
        let length = piece_length(from, piece);
        let mut done = 0.0;
        if let PathPiece::Line(to) = piece {
            while let Some((cut, opens)) = cuts.get(cut_index).copied() {
                if cut >= travelled + length - GEOMETRY_EPSILON_PX {
                    break;
                }
                let point = along(from, to, cut - travelled);
                if cut - travelled > done + GEOMETRY_EPSILON_PX {
                    current.pieces.push(PathPiece::Line(point));
                    current.length_px += cut - travelled - done;
                }
                done = cut - travelled;
                let next = DashRun {
                    start: point,
                    pieces: Vec::new(),
                    joint: opens,
                    from_px: cut,
                    length_px: 0.0,
                };
                if !current.pieces.is_empty() {
                    runs.push(std::mem::replace(&mut current, next));
                } else {
                    current = next;
                }
                cut_index += 1;
            }
            if length - done > GEOMETRY_EPSILON_PX {
                current.pieces.push(PathPiece::Line(to));
                current.length_px += length - done;
            }
        } else {
            current.pieces.push(piece);
            current.length_px += length;
        }
        travelled += length;
    }
    if !current.pieces.is_empty() {
        runs.push(current);
    }
    runs
}

/// Where each bend and hop of a drawn link lies along it, in px: the middle of each curved
/// piece and each square corner.
pub fn turn_positions(drawn: &DrawnLink) -> Vec<f32> {
    let mut positions = Vec::new();
    let mut travelled = 0.0;
    let mut previous_direction: Option<(f32, f32)> = None;
    for (from, piece) in flat_pieces(drawn) {
        let length = piece_length(from, piece);
        match piece {
            PathPiece::Line(to) => {
                if length > GEOMETRY_EPSILON_PX {
                    let direction = ((to.x - from.x) / length, (to.y - from.y) / length);
                    if let Some(before) = previous_direction
                        && before.0 * direction.0 + before.1 * direction.1 < 1.0 - 1e-3
                    {
                        positions.push(travelled);
                    }
                    previous_direction = Some(direction);
                }
            }
            PathPiece::Arc { .. } | PathPiece::Cubic { .. } | PathPiece::Quad { .. } => {
                positions.push(travelled + length / 2.0);
                previous_direction = None;
            }
        }
        travelled += length;
    }
    positions
}

/// True when the stroke a pattern of `dash` on and `gap` off draws over `runs` is on at
/// `position`: a joint is on throughout; a patterned run starts on at the line's start and
/// in a gap after a joint (section 14.5 rule 1 of the bends).
pub fn stroke_on_at(runs: &[DashRun], (dash, gap): (f32, f32), position: f32) -> bool {
    let period = dash + gap;
    for (index, run) in runs.iter().enumerate() {
        let end = run.from_px + run.length_px;
        if position < run.from_px - GEOMETRY_EPSILON_PX || position > end + GEOMETRY_EPSILON_PX {
            continue;
        }
        if run.joint || period <= GEOMETRY_EPSILON_PX {
            return true;
        }
        let phase = if index == 0 { 0.0 } else { dash };
        let into = (position - run.from_px + phase).rem_euclid(period);
        return into < dash;
    }
    false
}

/// The bend at `corner`: where it leaves the incoming leg and the piece that turns it.
/// `at` is where the drawn line stands before the bend.
fn bend_piece(
    at: PagePoint,
    previous: PagePoint,
    corner: PagePoint,
    next: PagePoint,
    route: &LinkRoute,
    obstacles: &[Obstacle],
) -> Option<(PagePoint, PathPiece)> {
    if route.bend == Bend::Spline
        && let Some((entry, piece)) = spline_piece(previous, corner, next)
    {
        let mut chords = vec![at, entry];
        push_flattened(&mut chords, entry, piece);
        let clear = obstacles
            .iter()
            .all(|obstacle| !chain_enters(&chords, &obstacle.bounds));
        if clear {
            return Some((entry, piece));
        }
    }
    let rounded = fillet(
        (previous.x, previous.y),
        (corner.x, corner.y),
        (next.x, next.y),
        route.corner,
    )?;
    let start = PagePoint {
        x: rounded.start.0,
        y: rounded.start.1,
    };
    let to = PagePoint {
        x: rounded.end.0,
        y: rounded.end.1,
    };
    let piece = match route.bend {
        Bend::Arc => PathPiece::Arc {
            radius: rounded.radius,
            clockwise: rounded.clockwise,
            corner,
            to,
        },
        Bend::Curve | Bend::Spline => PathPiece::Cubic { corner, to },
    };
    Some((start, piece))
}

/// The quadratic from the middle of the incoming leg to the middle of the outgoing one,
/// controlled by the corner. None when a leg has no length or the legs do not turn.
fn spline_piece(
    previous: PagePoint,
    corner: PagePoint,
    next: PagePoint,
) -> Option<(PagePoint, PathPiece)> {
    let incoming = (corner.x - previous.x, corner.y - previous.y);
    let outgoing = (next.x - corner.x, next.y - corner.y);
    let incoming_length = incoming.0.hypot(incoming.1);
    let outgoing_length = outgoing.0.hypot(outgoing.1);
    if incoming_length < GEOMETRY_EPSILON_PX || outgoing_length < GEOMETRY_EPSILON_PX {
        return None;
    }
    let sine =
        (incoming.0 * outgoing.1 - incoming.1 * outgoing.0) / (incoming_length * outgoing_length);
    if sine.abs() <= TURN_SINE_MIN {
        return None;
    }
    let middle = |a: PagePoint, b: PagePoint| PagePoint {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
    };
    Some((
        middle(previous, corner),
        PathPiece::Quad {
            corner,
            to: middle(corner, next),
        },
    ))
}

/// The points of `piece` after `from`, a curve as CURVE_CHORDS chords.
pub fn push_flattened(points: &mut Vec<PagePoint>, from: PagePoint, piece: PathPiece) {
    let along = |t: f32| -> PagePoint {
        let u = 1.0 - t;
        match piece {
            PathPiece::Line(to) => to,
            PathPiece::Arc { corner, to, .. } => {
                let center = (from.x + to.x - corner.x, from.y + to.y - corner.y);
                let angle = t * std::f32::consts::FRAC_PI_2;
                let (sine, cosine) = angle.sin_cos();
                PagePoint {
                    x: center.0 + (from.x - center.0) * cosine + (to.x - center.0) * sine,
                    y: center.1 + (from.y - center.1) * cosine + (to.y - center.1) * sine,
                }
            }
            PathPiece::Cubic { corner, to } => {
                let on_corner = 3.0 * u * u * t + 3.0 * u * t * t;
                PagePoint {
                    x: u * u * u * from.x + on_corner * corner.x + t * t * t * to.x,
                    y: u * u * u * from.y + on_corner * corner.y + t * t * t * to.y,
                }
            }
            PathPiece::Quad { corner, to } => PagePoint {
                x: u * u * from.x + 2.0 * u * t * corner.x + t * t * to.x,
                y: u * u * from.y + 2.0 * u * t * corner.y + t * t * to.y,
            },
        }
    };
    match piece {
        PathPiece::Line(to) => points.push(to),
        _ => {
            for step in 1..=CURVE_CHORDS {
                points.push(along(step as f32 / CURVE_CHORDS as f32));
            }
        }
    }
}

/// True when any step of `chain` reaches into the open interior of `bounds`. An axis-aligned
/// step reads as a route segment does; a diagonal chord of a curve is clipped against the
/// interior.
pub(crate) fn chain_enters(chain: &[PagePoint], bounds: &BoxRect) -> bool {
    chain.windows(2).any(|pair| {
        let (Some(&a), Some(&b)) = (pair.first(), pair.get(1)) else {
            return false;
        };
        if a.x == b.x || a.y == b.y {
            segment_enters(a, b, bounds)
        } else {
            chord_enters(a, b, bounds)
        }
    })
}

/// Liang-Barsky: true when the segment from `a` to `b` has a stretch of positive length
/// inside `bounds` shrunk by the geometry epsilon.
fn chord_enters(a: PagePoint, b: PagePoint, bounds: &BoxRect) -> bool {
    let (left, right) = (
        bounds.x + GEOMETRY_EPSILON_PX,
        bounds.right() - GEOMETRY_EPSILON_PX,
    );
    let (top, bottom) = (
        bounds.y + GEOMETRY_EPSILON_PX,
        bounds.bottom() - GEOMETRY_EPSILON_PX,
    );
    if left >= right || top >= bottom {
        return false;
    }
    let delta = (b.x - a.x, b.y - a.y);
    let mut low = 0.0_f32;
    let mut high = 1.0_f32;
    for (step, room) in [
        (-delta.0, a.x - left),
        (delta.0, right - a.x),
        (-delta.1, a.y - top),
        (delta.1, bottom - a.y),
    ] {
        if step == 0.0 {
            if room <= 0.0 {
                return false;
            }
            continue;
        }
        let edge = room / step;
        if step < 0.0 {
            low = low.max(edge);
        } else {
            high = high.min(edge);
        }
    }
    low < high
}

/// Pulls the last point back along the last segment by the end mark's `length`, or to the
/// previous corner when the segment is shorter, so the stroke ends under the mark's base.
/// Returns the (base, tip) of the mark, which points along the last segment.
fn trim_end(points: &mut [PagePoint], length: f32) -> Option<(PagePoint, PagePoint)> {
    let count = points.len();
    let tip = *points.last()?;
    let previous = *points.get(count.checked_sub(2)?)?;
    let base = arrow_base(previous, tip, length);
    if let Some(last) = points.last_mut() {
        *last = base.stroke_end;
    }
    Some((base.marker_base, tip))
}

/// `trim_end` for the first point, with the arrowhead pointing back along the first segment.
fn trim_start(points: &mut [PagePoint], length: f32) -> Option<(PagePoint, PagePoint)> {
    let tip = *points.first()?;
    let next = *points.get(1)?;
    let base = arrow_base(next, tip, length);
    if let Some(first) = points.first_mut() {
        *first = base.stroke_end;
    }
    Some((base.marker_base, tip))
}

#[derive(Debug, Clone, Copy)]
struct ArrowBase {
    /// Where the stroke stops: the arrowhead base, or the segment's far end when the
    /// segment is shorter than the arrowhead.
    stroke_end: PagePoint,
    /// A point one arrowhead length back from the tip along the segment's direction, which
    /// orients the marker.
    marker_base: PagePoint,
}

/// The base of an end mark `mark_length` long on the segment from `from` to the tip `tip`.
fn arrow_base(from: PagePoint, tip: PagePoint, mark_length: f32) -> ArrowBase {
    let delta_x = tip.x - from.x;
    let delta_y = tip.y - from.y;
    let length = (delta_x * delta_x + delta_y * delta_y).sqrt();
    if length <= f32::EPSILON {
        return ArrowBase {
            stroke_end: tip,
            marker_base: from,
        };
    }
    let unit_x = delta_x / length;
    let unit_y = delta_y / length;
    let marker_base = PagePoint {
        x: tip.x - unit_x * mark_length,
        y: tip.y - unit_y * mark_length,
    };
    let trimmed = mark_length.min(length);
    ArrowBase {
        stroke_end: PagePoint {
            x: tip.x - unit_x * trimmed,
            y: tip.y - unit_y * trimmed,
        },
        marker_base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> PagePoint {
        PagePoint { x, y }
    }

    #[test]
    fn a_right_turn_is_clockwise_and_takes_the_full_radius_on_long_legs() {
        let bend = fillet((0.0, 0.0), (100.0, 0.0), (100.0, 100.0), 6.0).unwrap();
        assert_eq!(bend.start, (94.0, 0.0));
        assert_eq!(bend.end, (100.0, 6.0));
        assert_eq!(bend.radius, 6.0);
        assert!(bend.clockwise);
        let left = fillet((0.0, 0.0), (100.0, 0.0), (100.0, -100.0), 6.0).unwrap();
        assert!(!left.clockwise);
    }

    #[test]
    fn the_radius_clamps_to_half_the_shorter_leg() {
        let bend = fillet((0.0, 0.0), (100.0, 0.0), (100.0, 8.0), 6.0).unwrap();
        assert_eq!(bend.radius, 4.0);
        assert_eq!(bend.start, (96.0, 0.0));
        assert_eq!(bend.end, (100.0, 4.0));
    }

    #[test]
    fn a_zero_leg_a_zero_corner_or_a_straight_run_stays_square() {
        assert_eq!(fillet((0.0, 0.0), (100.0, 0.0), (100.0, 0.0), 6.0), None);
        assert_eq!(fillet((0.0, 0.0), (100.0, 0.0), (100.0, 50.0), 0.0), None);
        assert_eq!(fillet((0.0, 0.0), (50.0, 0.0), (100.0, 0.0), 6.0), None);
        assert_eq!(fillet((0.0, 0.0), (50.0, 0.0), (0.0, 0.0), 6.0), None);
    }

    #[test]
    fn a_spline_piece_runs_from_leg_middle_to_leg_middle_under_its_corner() {
        let (entry, piece) =
            spline_piece(point(0.0, 0.0), point(100.0, 0.0), point(100.0, 40.0)).unwrap();
        assert_eq!(entry, point(50.0, 0.0));
        assert_eq!(
            piece,
            PathPiece::Quad {
                corner: point(100.0, 0.0),
                to: point(100.0, 20.0)
            }
        );
        assert_eq!(
            spline_piece(point(0.0, 0.0), point(50.0, 0.0), point(100.0, 0.0)),
            None
        );
    }

    #[test]
    fn a_flattened_arc_stays_on_its_circle_and_ends_on_its_end() {
        let mut points = Vec::new();
        let from = point(94.0, 0.0);
        let to = point(100.0, 6.0);
        push_flattened(
            &mut points,
            from,
            PathPiece::Arc {
                radius: 6.0,
                clockwise: true,
                corner: point(100.0, 0.0),
                to,
            },
        );
        assert_eq!(points.len(), CURVE_CHORDS);
        for at in &points {
            let distance = (at.x - 94.0).hypot(at.y - 6.0);
            assert!((distance - 6.0).abs() < 1e-3, "{at:?}");
        }
        let last = points[CURVE_CHORDS - 1];
        assert!((last.x - to.x).abs() < 1e-4 && (last.y - to.y).abs() < 1e-4);
    }

    #[test]
    fn a_diagonal_chord_enters_only_the_boxes_it_crosses() {
        let corner_box = BoxRect {
            x: 90.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        assert!(chord_enters(
            point(85.0, 0.0),
            point(100.0, 15.0),
            &corner_box
        ));
        assert!(!chord_enters(
            point(80.0, 0.0),
            point(100.0, 20.0),
            &corner_box
        ));
        let beside = BoxRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        assert!(!chord_enters(point(9.0, 20.0), point(20.0, 9.0), &beside));
        // The bounding-box test of route segments would report this chord.
        assert!(segment_enters(point(9.0, 20.0), point(20.0, 9.0), &beside));
        assert!(!chord_enters(point(10.0, 0.0), point(20.0, 10.0), &beside));
    }

    #[test]
    fn a_short_end_segment_trims_to_its_corner_and_keeps_a_full_length_arrowhead() {
        let mut points = vec![point(0.0, 0.0), point(100.0, 0.0), point(100.0, 6.0)];
        let (base, tip) = trim_end(&mut points, ARROWHEAD_LENGTH_PX).unwrap();
        assert_eq!(tip, point(100.0, 6.0));
        assert_eq!(base, point(100.0, -4.0));
        assert_eq!(points[2], point(100.0, 0.0));

        let mut long = vec![point(0.0, 0.0), point(50.0, 0.0)];
        let (base, tip) = trim_start(&mut long, ARROWHEAD_LENGTH_PX).unwrap();
        assert_eq!(tip, point(0.0, 0.0));
        assert_eq!(base, point(10.0, 0.0));
        assert_eq!(long[0], base);
    }

    #[test]
    fn a_stop_plate_end_trims_the_stroke_to_the_plate_and_covers_a_bar_across_the_line() {
        let mut points = vec![point(0.0, 0.0), point(0.0, 50.0)];
        let cap = EndCap::of(Line::Deny);
        assert_eq!(cap, EndCap::StopPlate);
        let (base, tip) = trim_end(&mut points, cap.length_px()).unwrap();
        assert_eq!(tip, point(0.0, 50.0));
        assert_eq!(base, point(0.0, 47.0));
        assert_eq!(points[1], base);
        assert_eq!(
            cap.bounds((base, tip)),
            BoxRect {
                x: -8.0,
                y: 47.0,
                width: 16.0,
                height: 3.0
            }
        );
    }
}
