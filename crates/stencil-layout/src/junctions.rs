//! junctions-plain (section 6): the lines of a figure must not meet as four arms around one
//! center, each turning the same way at its end. Read in layout space. Under iso that is the
//! layout route, before the adjustments of section 12.3 rule 8 move legs, so a shape those
//! adjustments make or break lies outside the check.
//!
//! A center is any point where a horizontal and a vertical run meet, cross or stop within
//! `JUNCTION_GAP_PX` of each other, and the middle of every drawn outline and Tee hub, whose
//! arms may stop at its edges. The four arms from it size a window, the ink in the
//! window is rasterized, and the raster is scored against templates of the shape in both
//! turning directions and three hook lengths. A match needs each of the eight strokes
//! present on its own and the whole above `JUNCTION_SCORE_MIN`: the eight-stroke floor is
//! what keeps one reversed hook, three hooks or a plain cross apart from the shape, which a
//! whole-window score alone does not.

use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;
use stencil_model::{PagePoint, Projection};

use crate::sheet_lines::outline_edges;
use crate::{BoxRect, GEOMETRY_EPSILON_PX, NodeTag, PageGeometry, PartName};

/// The widest gap at the center that still reads as the arms meeting.
pub const JUNCTION_GAP_PX: f32 = 16.0;
/// The least combined score of a match.
pub const JUNCTION_SCORE_MIN: f32 = 0.8;
/// The least share of each arm and hook the ink must cover.
pub const JUNCTION_STROKE_MIN: f32 = 0.5;
/// The most junctions one page is examined at; a page with more is a defect.
pub const JUNCTION_CENTERS_MAX: usize = 8192;
/// Arms shorter than this are a bend or a tick, not an arm.
const ARM_MIN_PX: f32 = 12.0;
/// The longest arm is at most this many times the shortest.
const ARM_RATIO_MAX: f32 = 3.0;
/// Template cells along one arm; the window is two arms and a margin across.
const CELLS_PER_ARM: i32 = 8;
const MARGIN_CELLS: i32 = 2;
/// Hook lengths of the templates, as a share of the arm.
const HOOK_SHARES: [f32; 3] = [0.5, 0.75, 1.0];
/// Collinear runs this far apart across their axis still count as one arm line.
const ARM_OFFSET_PX: f32 = 6.0;
/// A break of up to this along an arm line, a dash gap or a small hub, still reads as one line.
const ARM_BREAK_PX: f32 = 8.0;

/// One straight run of drawn ink and the pointer of what draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct InkRun {
    pub owner: NodePointer,
    pub start: PagePoint,
    pub end: PagePoint,
}

/// A match: the center, the mean arm length, the score and the owners of the arms.
#[derive(Debug, Clone, PartialEq)]
pub struct JunctionMatch {
    pub center: PagePoint,
    pub arm: f32,
    pub score: f32,
    pub owners: Vec<NodePointer>,
}

/// What a scan examined and found.
#[derive(Debug, Clone, PartialEq)]
pub struct JunctionScan {
    pub examined: usize,
    pub found: Vec<JunctionMatch>,
    /// The best score of any center whose four arms passed the ratio test, matched or not.
    pub best_score: f32,
}

/// The scan of `runs` with the drawn boxes `hubs` as extra centers; None when more than
/// `JUNCTION_CENTERS_MAX` centers are found.
pub fn scan_junctions(runs: &[InkRun], hubs: &[BoxRect]) -> Option<JunctionScan> {
    let mut centers: Vec<(PagePoint, f32)> = candidate_centers(runs)?
        .into_iter()
        .map(|point| (point, JUNCTION_GAP_PX))
        .collect();
    for hub in hubs {
        let center = PagePoint {
            x: hub.x + hub.width / 2.0,
            y: hub.y + hub.height / 2.0,
        };
        centers.push((center, hub.width.max(hub.height) / 2.0 + JUNCTION_GAP_PX));
    }
    if centers.len() > JUNCTION_CENTERS_MAX {
        return None;
    }
    let mut found: Vec<JunctionMatch> = Vec::new();
    let mut best_score: f32 = 0.0;
    for (center, gap) in &centers {
        let Some(arms) = arms_at(runs, *center, *gap) else {
            continue;
        };
        let arm = arms.iter().map(|arm| arm.length).sum::<f32>() / 4.0;
        let cell = arm / CELLS_PER_ARM as f32;
        let arm_cells = arms
            .clone()
            .map(|arm| ((arm.length / cell).round() as i32).max(1));
        let half = arm_cells.iter().copied().max().unwrap_or(CELLS_PER_ARM) + MARGIN_CELLS;
        let raster = Raster::of(runs, *center, cell, half);
        let score = best_template_score(&raster, arm_cells);
        best_score = best_score.max(score.combined);
        if score.matched {
            let mut owners: Vec<NodePointer> = Vec::new();
            for arm in &arms {
                if !owners.contains(&arm.owner) {
                    owners.push(arm.owner.clone());
                }
            }
            let near = |other: &JunctionMatch| {
                (other.center.x - center.x).abs() <= JUNCTION_GAP_PX
                    && (other.center.y - center.y).abs() <= JUNCTION_GAP_PX
            };
            if !found.iter().any(near) {
                found.push(JunctionMatch {
                    center: *center,
                    arm,
                    score: score.combined,
                    owners,
                });
            }
        }
    }
    Some(JunctionScan {
        examined: centers.len(),
        found,
        best_score,
    })
}

/// The box of every drawn outline and every Tee hub: the centers whose arms may stop at
/// their edges.
pub fn page_hubs(geometry: &PageGeometry) -> Vec<BoxRect> {
    let mut hubs: Vec<BoxRect> = Vec::new();
    for edge in outline_edges(geometry) {
        if let Some(node) = geometry.nodes.get(edge.node)
            && !hubs.contains(&node.bounds)
        {
            hubs.push(node.bounds);
        }
    }
    for node in &geometry.nodes {
        if let Some(hub) = node.part(PartName::Hub) {
            hubs.push(hub.bounds);
        }
    }
    hubs
}

/// Every run of ink the page draws, in layout space: each link's drawn line, its bends
/// flattened, each pipe wire and Tee spine along its long axis, and each drawn outline.
pub fn page_ink(geometry: &PageGeometry) -> Vec<InkRun> {
    let mut runs = Vec::new();
    let links = NodePointer::root().child("links");
    for route in &geometry.links {
        let owner = links.index(route.index);
        for chain in crate::checks::drawn_chains(geometry, route) {
            for pair in chain.windows(2) {
                if let (Some(&start), Some(&end)) = (pair.first(), pair.get(1)) {
                    runs.push(InkRun {
                        owner: owner.clone(),
                        start,
                        end,
                    });
                }
            }
        }
    }
    for node in &geometry.nodes {
        if !matches!(node.tag, NodeTag::Pipe | NodeTag::Tee) {
            continue;
        }
        for part in &node.parts {
            if !matches!(
                part.name,
                PartName::WireStart | PartName::WireEnd | PartName::Spine
            ) {
                continue;
            }
            let bounds = part.bounds;
            let (start, end) = if bounds.width >= bounds.height {
                let y = bounds.y + bounds.height / 2.0;
                (
                    PagePoint { x: bounds.x, y },
                    PagePoint {
                        x: bounds.right(),
                        y,
                    },
                )
            } else {
                let x = bounds.x + bounds.width / 2.0;
                (
                    PagePoint { x, y: bounds.y },
                    PagePoint {
                        x,
                        y: bounds.bottom(),
                    },
                )
            };
            runs.push(InkRun {
                owner: node.pointer.clone(),
                start,
                end,
            });
        }
    }
    for edge in outline_edges(geometry) {
        if let Some(node) = geometry.nodes.get(edge.node) {
            runs.push(InkRun {
                owner: node.pointer.clone(),
                start: edge.run.0,
                end: edge.run.1,
            });
        }
    }
    runs
}

/// junctions-plain over the page's ink. One examined unit per junction; a match is a
/// defect at its first link owner, else its first owner. Not applicable when no two runs
/// meet; more junctions than the check examines is a defect at the page root.
pub fn junctions_plain(geometry: &PageGeometry, projection: Projection) -> CheckReport {
    let runs = page_ink(geometry);
    let Some(scan) = scan_junctions(&runs, &page_hubs(geometry)) else {
        return CheckReport {
            check: CheckName::JunctionsPlain,
            examined: 0,
            defects: vec![Defect {
                pointer: NodePointer::root(),
                message: format!(
                    "more than {JUNCTION_CENTERS_MAX} junctions; the page was not examined"
                ),
            }],
            not_applicable: None,
        };
    };
    if scan.examined == 0 {
        return CheckReport::not_applicable(CheckName::JunctionsPlain, "no two lines meet");
    }
    let space = match projection {
        Projection::Flat => "",
        Projection::Iso => " in layout space",
    };
    let defects = scan
        .found
        .iter()
        .map(|found| {
            let pointer = found
                .owners
                .iter()
                .find(|owner| owner.as_str().starts_with("/links/"))
                .or(found.owners.first())
                .cloned()
                .unwrap_or_else(NodePointer::root);
            let owners: Vec<&str> = found.owners.iter().map(NodePointer::as_str).collect();
            Defect {
                pointer,
                message: format!(
                    "four arms of {:.2} px around ({:.2}, {:.2}){space} each turn the same way at their ends, score {:.2} of {JUNCTION_SCORE_MIN:.2}: {}",
                    found.arm,
                    found.center.x,
                    found.center.y,
                    found.score,
                    owners.join(", ")
                ),
            }
        })
        .collect();
    CheckReport {
        check: CheckName::JunctionsPlain,
        examined: u64::try_from(scan.examined).unwrap_or(u64::MAX),
        defects,
        not_applicable: None,
    }
}

fn horizontal(run: &InkRun) -> bool {
    (run.start.y - run.end.y).abs() <= GEOMETRY_EPSILON_PX
        && (run.start.x - run.end.x).abs() > GEOMETRY_EPSILON_PX
}

fn vertical(run: &InkRun) -> bool {
    (run.start.x - run.end.x).abs() <= GEOMETRY_EPSILON_PX
        && (run.start.y - run.end.y).abs() > GEOMETRY_EPSILON_PX
}

/// How far `value` lies outside the span from `a` to `b`; 0 inside it.
fn outside(value: f32, a: f32, b: f32) -> f32 {
    (a.min(b) - value).max(value - a.max(b)).max(0.0)
}

/// Each point where a horizontal and a vertical run meet or would meet within
/// `JUNCTION_GAP_PX` of both, the first of those within half a pixel of each other kept;
/// None past `JUNCTION_CENTERS_MAX`.
fn candidate_centers(runs: &[InkRun]) -> Option<Vec<PagePoint>> {
    let across: Vec<&InkRun> = runs.iter().filter(|run| horizontal(run)).collect();
    let down: Vec<&InkRun> = runs.iter().filter(|run| vertical(run)).collect();
    let mut centers: std::collections::BTreeMap<(i64, i64), PagePoint> =
        std::collections::BTreeMap::new();
    for h in &across {
        for v in &down {
            let point = PagePoint {
                x: v.start.x,
                y: h.start.y,
            };
            if outside(point.x, h.start.x, h.end.x) <= JUNCTION_GAP_PX
                && outside(point.y, v.start.y, v.end.y) <= JUNCTION_GAP_PX
            {
                let key = (
                    (point.x * 2.0).round() as i64,
                    (point.y * 2.0).round() as i64,
                );
                centers.entry(key).or_insert(point);
                if centers.len() > JUNCTION_CENTERS_MAX {
                    return None;
                }
            }
        }
    }
    Some(centers.into_values().collect())
}

/// One arm from a center: its length to where its run ends and the run's owner.
#[derive(Debug, Clone)]
struct Arm {
    length: f32,
    owner: NodePointer,
}

/// The four arms from `center`, right, down, left and up. Each arm is the line of runs on
/// its axis, within `ARM_OFFSET_PX` across it, that starts within `gap` of the center and
/// continues across breaks of up to `ARM_BREAK_PX`, as a dashed line reads whole; its
/// length runs to where that line ends. None when an arm is missing, shorter than
/// `ARM_MIN_PX`, or the longest exceeds `ARM_RATIO_MAX` times the shortest.
fn arms_at(runs: &[InkRun], center: PagePoint, gap: f32) -> Option<[Arm; 4]> {
    let arm = |dx: f32, dy: f32| -> Option<Arm> {
        // Each collinear run as (near, far) distances from the center along the arm.
        let mut spans: Vec<(f32, f32, &NodePointer)> = runs
            .iter()
            .filter_map(|run| {
                let (on_axis, across, a, b, origin) = if dx != 0.0 {
                    (
                        horizontal(run),
                        run.start.y - center.y,
                        run.start.x,
                        run.end.x,
                        center.x,
                    )
                } else {
                    (
                        vertical(run),
                        run.start.x - center.x,
                        run.start.y,
                        run.end.y,
                        center.y,
                    )
                };
                if !on_axis || across.abs() > ARM_OFFSET_PX {
                    return None;
                }
                let sign = if dx + dy > 0.0 { 1.0 } else { -1.0 };
                let (one, two) = ((a - origin) * sign, (b - origin) * sign);
                let far = one.max(two);
                (far > 0.0).then_some((one.min(two), far, &run.owner))
            })
            .collect();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        let first = spans.iter().find(|span| span.0 <= gap)?;
        // The arm belongs to whatever draws the most of it.
        let drawn = |span: &(f32, f32, &NodePointer)| span.1 - span.0.max(0.0);
        let mut owner = first;
        let mut reach = first.1;
        for span in &spans {
            if span.0 <= reach + ARM_BREAK_PX && span.1 > 0.0 {
                reach = reach.max(span.1);
                if drawn(span) > drawn(owner) {
                    owner = span;
                }
            }
        }
        (reach >= ARM_MIN_PX).then_some(Arm {
            length: reach,
            owner: owner.2.clone(),
        })
    };
    let arms = [
        arm(1.0, 0.0)?,
        arm(0.0, 1.0)?,
        arm(-1.0, 0.0)?,
        arm(0.0, -1.0)?,
    ];
    let longest = arms.iter().map(|arm| arm.length).fold(0.0, f32::max);
    let shortest = arms
        .iter()
        .map(|arm| arm.length)
        .fold(f32::INFINITY, f32::min);
    (longest <= shortest * ARM_RATIO_MAX).then_some(arms)
}

/// The ink in a square window around a center, one flag per cell of side `cell` px; cell
/// (0, 0) is the center and cells run from -half to half on each axis.
struct Raster {
    half: i32,
    cells: Vec<bool>,
}

impl Raster {
    fn of(runs: &[InkRun], center: PagePoint, cell: f32, half: i32) -> Raster {
        let side = usize::try_from(2 * half + 1).unwrap_or(0);
        let mut raster = Raster {
            half,
            cells: vec![false; side * side],
        };
        let reach = (half as f32 + 0.5) * cell;
        for run in runs {
            let Some((start, end)) = clip(run, center, reach) else {
                continue;
            };
            let length = (end.x - start.x).hypot(end.y - start.y);
            // Two samples a cell mark every cell the run passes; the window bounds the count.
            let steps = (length / cell * 2.0).ceil().max(1.0);
            let count = steps as u32;
            for step in 0..=count {
                let t = step as f32 / count as f32;
                let x = start.x + (end.x - start.x) * t;
                let y = start.y + (end.y - start.y) * t;
                raster.mark(
                    ((x - center.x) / cell).round() as i32,
                    ((y - center.y) / cell).round() as i32,
                );
            }
        }
        raster
    }

    fn index(&self, column: i32, row: i32) -> Option<usize> {
        if column.abs() > self.half || row.abs() > self.half {
            return None;
        }
        let side = 2 * self.half + 1;
        usize::try_from((row + self.half) * side + column + self.half).ok()
    }

    fn mark(&mut self, column: i32, row: i32) {
        if let Some(index) = self.index(column, row)
            && let Some(cell) = self.cells.get_mut(index)
        {
            *cell = true;
        }
    }

    fn get(&self, column: i32, row: i32) -> bool {
        self.index(column, row)
            .and_then(|index| self.cells.get(index).copied())
            .unwrap_or(false)
    }

    /// True when a marked cell lies within one cell of (column, row).
    fn near(&self, column: i32, row: i32) -> bool {
        (-1..=1).any(|dy| (-1..=1).any(|dx| self.get(column + dx, row + dy)))
    }

    fn marked(&self) -> Vec<(i32, i32)> {
        let side = 2 * self.half + 1;
        (0..side * side)
            .filter(|&index| {
                usize::try_from(index)
                    .ok()
                    .and_then(|index| self.cells.get(index).copied())
                    .unwrap_or(false)
            })
            .map(|index| (index % side - self.half, index / side - self.half))
            .collect()
    }
}

/// The part of `run` inside the square of half-side `reach` around `center`.
fn clip(run: &InkRun, center: PagePoint, reach: f32) -> Option<(PagePoint, PagePoint)> {
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    let delta = (run.end.x - run.start.x, run.end.y - run.start.y);
    for (origin, step, min, max) in [
        (run.start.x, delta.0, center.x - reach, center.x + reach),
        (run.start.y, delta.1, center.y - reach, center.y + reach),
    ] {
        if step.abs() <= f32::EPSILON {
            if origin < min || origin > max {
                return None;
            }
            continue;
        }
        let (a, b) = ((min - origin) / step, (max - origin) / step);
        low = low.max(a.min(b));
        high = high.min(a.max(b));
    }
    (low <= high).then(|| {
        let at = |t: f32| PagePoint {
            x: run.start.x + delta.0 * t,
            y: run.start.y + delta.1 * t,
        };
        (at(low), at(high))
    })
}

/// A template's eight strokes as cell lists: the four arms of `arm_cells` (right, down,
/// left, up), then a hook of `hook_share` of its arm at the end of each, all turning one
/// way.
fn template(arm_cells: [i32; 4], clockwise: bool, hook_share: f32) -> [Vec<(i32, i32)>; 8] {
    let directions = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    let turn = |(x, y): (i32, i32)| if clockwise { (-y, x) } else { (y, -x) };
    let mut strokes: [Vec<(i32, i32)>; 8] = Default::default();
    for (index, ((dx, dy), length)) in directions.into_iter().zip(arm_cells).enumerate() {
        if let Some(arm) = strokes.get_mut(index) {
            *arm = (1..=length).map(|k| (dx * k, dy * k)).collect();
        }
        let hook = ((length as f32 * hook_share).round() as i32).max(1);
        let (hx, hy) = turn((dx, dy));
        let corner = (dx * length, dy * length);
        if let Some(hook_cells) = strokes.get_mut(index + 4) {
            *hook_cells = (1..=hook)
                .map(|k| (corner.0 + hx * k, corner.1 + hy * k))
                .collect();
        }
    }
    strokes
}

struct Score {
    combined: f32,
    matched: bool,
}

/// The best score over both turning directions and every hook share.
fn best_template_score(raster: &Raster, arm_cells: [i32; 4]) -> Score {
    let ink = raster.marked();
    let mut best = Score {
        combined: 0.0,
        matched: false,
    };
    for clockwise in [true, false] {
        for share in HOOK_SHARES {
            let strokes = template(arm_cells, clockwise, share);
            let mut cells: Vec<(i32, i32)> = vec![(0, 0)];
            cells.extend(strokes.iter().flatten().copied());
            let near_template = |(x, y): (i32, i32)| {
                cells
                    .iter()
                    .any(|(cx, cy)| (cx - x).abs() <= 1 && (cy - y).abs() <= 1)
            };
            let covered = |stroke: &[(i32, i32)]| {
                let hit = stroke.iter().filter(|(x, y)| raster.near(*x, *y)).count();
                hit as f32 / stroke.len().max(1) as f32
            };
            let strokes_present = strokes
                .iter()
                .all(|stroke| covered(stroke) >= JUNCTION_STROKE_MIN);
            let recall = covered(&cells);
            let on_template = ink.iter().filter(|cell| near_template(**cell)).count();
            let precision = on_template as f32 / ink.len().max(1) as f32;
            let combined = if recall + precision > 0.0 {
                2.0 * recall * precision / (recall + precision)
            } else {
                0.0
            };
            let matched = strokes_present && combined >= JUNCTION_SCORE_MIN;
            if (matched, combined) > (best.matched, best.combined) {
                best = Score { combined, matched };
            }
        }
    }
    best
}
