//! Link routing (section 11.2): attach points on the endpoint boxes, an orthogonal grid built
//! from the obstacle edges, A* over the grid intersections, and the tag box on the longest
//! segment. Routing reads finished geometry and never moves a node.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use stencil_model::pointer::NodePointer;
use stencil_model::text::{TextMeasurer, TextMetrics, TextStyleName};
use stencil_model::{
    LINK_SEGMENTS_MAX, LINKS_MAX, Link, Page, PagePoint, PipeKind, ROUTER_GRID_LINES_MAX, Side,
    VetRule, Violation, body_nodes,
};

use crate::styles::text_color;
use crate::{
    BoxRect, GEOMETRY_EPSILON_PX, LayoutError, LinkRoute, NodeGeometry, NodeTag, PageGeometry,
    Part, PartName, RouteStatus, TextAlign, TextRun,
};

/// Distance kept between a route and an obstacle: grid lines sit this far outside each edge.
pub const OBSTACLE_CLEARANCE_PX: f32 = 8.0;
/// Cost of one turn, in px of route length.
const TURN_COST_PX: f64 = 40.0;
/// Two grid coordinates closer than this are one line.
const GRID_MERGE_PX: f32 = 0.001;

/// Tag box insets of a pipe tag (section 2.7): border 1.5, padding 6 top and bottom, 8 left
/// and right, and 2 px between label and sub.
const TAG_BORDER_PX: f32 = 1.5;
const TAG_PADDING_X_PX: f32 = 8.0;
const TAG_PADDING_Y_PX: f32 = 6.0;
const TAG_SUB_GAP_PX: f32 = 2.0;

/// Tie order for attach sides.
const SIDE_ORDER: [Side; 4] = [Side::Right, Side::Bottom, Side::Left, Side::Top];

/// A box a link must not enter, and the geometry node it belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Obstacle {
    pub node: usize,
    pub bounds: BoxRect,
}

/// Routes every link of a vetted page against finished geometry, in link order.
pub(crate) fn route_links(
    page: &Page,
    geometry: &PageGeometry,
    measurer: &mut dyn TextMeasurer,
) -> Result<Vec<LinkRoute>, LayoutError> {
    let nodes_by_id = nodes_by_id(page, geometry);
    let links_pointer = NodePointer::root().child("links");
    let mut routes = Vec::with_capacity(page.links.len().min(LINKS_MAX));
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX) {
        let link_pointer = links_pointer.index(index);
        let from_node = resolve(&nodes_by_id, &link.from, link_pointer.child("from"))?;
        let to_node = resolve(&nodes_by_id, &link.to, link_pointer.child("to"))?;
        let (Some(from), Some(to)) = (geometry.nodes.get(from_node), geometry.nodes.get(to_node))
        else {
            return Err(LayoutError::Taffy {
                pointer: link_pointer,
                message: "link endpoint is not a geometry node".to_string(),
            });
        };
        let (from_side, to_side) = choose_sides(link, &from.bounds, &to.bounds);
        let start = side_midpoint(&from.bounds, from_side);
        let end = side_midpoint(&to.bounds, to_side);
        let obstacles = link_obstacles(geometry, from_node, to_node);
        let endpoints = Endpoints {
            from: from.bounds,
            to: to.bounds,
            start,
            end,
            from_side,
            to_side,
        };
        let (points, status) = match grid_route(&obstacles, &endpoints, &link.via) {
            Some(points) if points.len() <= LINK_SEGMENTS_MAX + 1 => (points, RouteStatus::Routed),
            Some(_) | None => (fallback_route(start, end), RouteStatus::Fallback),
        };
        let parts = tag_parts(page, link, &link_pointer, &points, measurer)?;
        let tag = parts.first().map(|part| part.bounds);
        routes.push(LinkRoute {
            index,
            kind: link.kind,
            from_node,
            to_node,
            points,
            tag,
            parts,
            status,
        });
    }
    Ok(routes)
}

/// Geometry index of every body node that carries an id.
fn nodes_by_id<'a>(page: &'a Page, geometry: &PageGeometry) -> HashMap<&'a str, usize> {
    let index_by_pointer: HashMap<&str, usize> = geometry
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.pointer.as_str(), index))
        .collect();
    let mut by_id = HashMap::new();
    for entry in body_nodes(page) {
        let Some(id) = entry.node.id() else {
            continue;
        };
        if let Some(&index) = index_by_pointer.get(entry.pointer.as_str()) {
            by_id.insert(id, index);
        }
    }
    by_id
}

/// Vet guarantees every `from` and `to` names a node, so a miss here is reported as the
/// vet violation it would have been.
fn resolve(
    nodes_by_id: &HashMap<&str, usize>,
    id: &str,
    pointer: NodePointer,
) -> Result<usize, LayoutError> {
    nodes_by_id.get(id).copied().ok_or_else(|| {
        LayoutError::Invalid(vec![Violation {
            message: format!("id {id:?} names no node"),
            pointer,
            rule: VetRule::LinkUnknownId,
        }])
    })
}

/// The boxes a link between `from` and `to` must avoid: every leaf block (Pcard, Fact, Note,
/// Text, Callout, Frame), every Tee, the tag of every Pipe, the label of every Zone and the
/// page-level text (kicker, title, lede, legend entries, foot). The endpoints and every
/// node that contains one are left out. Order is geometry order.
pub(crate) fn link_obstacles(geometry: &PageGeometry, from: usize, to: usize) -> Vec<Obstacle> {
    let mut excluded = vec![false; geometry.nodes.len()];
    for endpoint in [from, to] {
        let mut current = Some(endpoint);
        for _ in 0..geometry.nodes.len() {
            let Some(index) = current else {
                break;
            };
            if let Some(flag) = excluded.get_mut(index) {
                *flag = true;
            }
            current = geometry.nodes.get(index).and_then(|node| node.parent);
        }
    }
    geometry
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| !excluded.get(*index).copied().unwrap_or(true))
        .filter_map(|(index, node)| {
            obstacle_box(node).map(|bounds| Obstacle {
                node: index,
                bounds,
            })
        })
        .collect()
}

fn obstacle_box(node: &NodeGeometry) -> Option<BoxRect> {
    match node.tag {
        NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Text
        | NodeTag::Callout
        | NodeTag::Frame
        | NodeTag::Tee
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::LegendEntry
        | NodeTag::Foot => Some(node.bounds),
        NodeTag::Pipe => node.part(PartName::Tag).map(|part| part.bounds),
        NodeTag::Zone => node.part(PartName::Label).map(|part| part.bounds),
        NodeTag::Page | NodeTag::Body | NodeTag::Legend | NodeTag::Row | NodeTag::Col => None,
    }
}

/// True when the axis-aligned segment from `a` to `b` reaches into the open interior of
/// `bounds` by more than the geometry epsilon. A segment along an edge does not enter.
pub(crate) fn segment_enters(a: PagePoint, b: PagePoint, bounds: &BoxRect) -> bool {
    let (x_low, x_high) = (a.x.min(b.x), a.x.max(b.x));
    let (y_low, y_high) = (a.y.min(b.y), a.y.max(b.y));
    x_high > bounds.x + GEOMETRY_EPSILON_PX
        && x_low < bounds.right() - GEOMETRY_EPSILON_PX
        && y_high > bounds.y + GEOMETRY_EPSILON_PX
        && y_low < bounds.bottom() - GEOMETRY_EPSILON_PX
}

fn side_midpoint(bounds: &BoxRect, side: Side) -> PagePoint {
    let center_x = bounds.x + bounds.width / 2.0;
    let center_y = bounds.y + bounds.height / 2.0;
    match side {
        Side::Top => PagePoint {
            x: center_x,
            y: bounds.y,
        },
        Side::Right => PagePoint {
            x: bounds.right(),
            y: center_y,
        },
        Side::Bottom => PagePoint {
            x: center_x,
            y: bounds.bottom(),
        },
        Side::Left => PagePoint {
            x: bounds.x,
            y: center_y,
        },
    }
}

fn facing_side(side: Side) -> Side {
    match side {
        Side::Top => Side::Bottom,
        Side::Right => Side::Left,
        Side::Bottom => Side::Top,
        Side::Left => Side::Right,
    }
}

fn distance_squared(a: PagePoint, b: PagePoint) -> f32 {
    (a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)
}

/// The side of `bounds` whose midpoint is closest to `target`, ties in SIDE_ORDER.
fn closest_side(bounds: &BoxRect, target: PagePoint) -> Side {
    let mut best = Side::Right;
    let mut best_distance = f32::INFINITY;
    for side in SIDE_ORDER {
        let distance = distance_squared(side_midpoint(bounds, side), target);
        if distance < best_distance {
            best = side;
            best_distance = distance;
        }
    }
    best
}

/// Section 11.2 attach sides. Both sides given: used as is. Neither given and no `via`: the
/// pair of facing sides whose midpoints are closest. Otherwise each open end takes the side
/// closest to what it faces: the first or last via point, or the other end's attach point.
fn choose_sides(link: &Link, from: &BoxRect, to: &BoxRect) -> (Side, Side) {
    match (link.from_side, link.to_side) {
        (Some(from_side), Some(to_side)) => (from_side, to_side),
        (Some(from_side), None) => {
            let target = link
                .via
                .last()
                .copied()
                .unwrap_or_else(|| side_midpoint(from, from_side));
            (from_side, closest_side(to, target))
        }
        (None, Some(to_side)) => {
            let target = link
                .via
                .first()
                .copied()
                .unwrap_or_else(|| side_midpoint(to, to_side));
            (closest_side(from, target), to_side)
        }
        (None, None) => match (link.via.first(), link.via.last()) {
            (Some(&first), Some(&last)) => (closest_side(from, first), closest_side(to, last)),
            _ => facing_pair(from, to),
        },
    }
}

fn facing_pair(from: &BoxRect, to: &BoxRect) -> (Side, Side) {
    let mut best = (Side::Right, Side::Left);
    let mut best_distance = f32::INFINITY;
    for side in SIDE_ORDER {
        let facing = facing_side(side);
        let distance = distance_squared(side_midpoint(from, side), side_midpoint(to, facing));
        if distance < best_distance {
            best = (side, facing);
            best_distance = distance;
        }
    }
    best
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Right,
    Down,
    Left,
    Up,
}

impl Direction {
    const ALL: [Direction; 4] = [
        Direction::Right,
        Direction::Down,
        Direction::Left,
        Direction::Up,
    ];

    fn index(self) -> usize {
        match self {
            Direction::Right => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Up => 3,
        }
    }

    fn opposite(self) -> Direction {
        match self {
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Up => Direction::Down,
        }
    }

    /// The direction that leaves a box through `side`.
    fn outward(side: Side) -> Direction {
        match side {
            Side::Top => Direction::Up,
            Side::Right => Direction::Right,
            Side::Bottom => Direction::Down,
            Side::Left => Direction::Left,
        }
    }
}

struct Endpoints {
    from: BoxRect,
    to: BoxRect,
    start: PagePoint,
    end: PagePoint,
    from_side: Side,
    to_side: Side,
}

/// The routing grid: sorted line coordinates and which intersections and unit moves are
/// blocked by an obstacle interior. Every accessor returns blocked for an index outside the
/// grid.
struct Grid {
    xs: Vec<f32>,
    ys: Vec<f32>,
    node_blocked: Vec<bool>,
    /// Move from column i to i + 1 on row j, at `j * (xs.len() - 1) + i`.
    horizontal_blocked: Vec<bool>,
    /// Move from row j to j + 1 on column i, at `j * xs.len() + i`.
    vertical_blocked: Vec<bool>,
}

type GridPoint = (usize, usize);

impl Grid {
    /// None when either line set exceeds ROUTER_GRID_LINES_MAX.
    fn build(obstacles: &[Obstacle], endpoints: &Endpoints, via: &[PagePoint]) -> Option<Grid> {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let clearance_boxes = obstacles
            .iter()
            .map(|obstacle| obstacle.bounds)
            .chain([endpoints.from, endpoints.to]);
        for bounds in clearance_boxes {
            xs.push(bounds.x - OBSTACLE_CLEARANCE_PX);
            xs.push(bounds.right() + OBSTACLE_CLEARANCE_PX);
            ys.push(bounds.y - OBSTACLE_CLEARANCE_PX);
            ys.push(bounds.bottom() + OBSTACLE_CLEARANCE_PX);
        }
        for point in [endpoints.start, endpoints.end].iter().chain(via) {
            xs.push(point.x);
            ys.push(point.y);
        }
        let xs = sorted_lines(xs);
        let ys = sorted_lines(ys);
        if xs.len() > ROUTER_GRID_LINES_MAX || ys.len() > ROUTER_GRID_LINES_MAX {
            return None;
        }
        let columns = xs.len();
        let rows = ys.len();
        let mut grid = Grid {
            node_blocked: vec![false; columns * rows],
            horizontal_blocked: vec![false; columns.saturating_sub(1) * rows],
            vertical_blocked: vec![false; columns * rows.saturating_sub(1)],
            xs,
            ys,
        };
        for obstacle in obstacles {
            grid.block(&obstacle.bounds);
        }
        Some(grid)
    }

    /// Marks the intersections strictly inside `bounds` and every unit move that enters it.
    fn block(&mut self, bounds: &BoxRect) {
        let columns = self.xs.len();
        let rows = self.ys.len();
        let inside_low = |lines: &[f32], low: f32| {
            lines.partition_point(|&line| line <= low + GEOMETRY_EPSILON_PX)
        };
        let inside_high = |lines: &[f32], high: f32| {
            lines.partition_point(|&line| line < high - GEOMETRY_EPSILON_PX)
        };
        let column_low = inside_low(&self.xs, bounds.x);
        let column_high = inside_high(&self.xs, bounds.right());
        let row_low = inside_low(&self.ys, bounds.y);
        let row_high = inside_high(&self.ys, bounds.bottom());
        for row in row_low..row_high {
            for column in column_low..column_high {
                set_flag(&mut self.node_blocked, row * columns + column);
            }
            for column in column_low.saturating_sub(1)..column_high.min(columns.saturating_sub(1)) {
                set_flag(
                    &mut self.horizontal_blocked,
                    row * columns.saturating_sub(1) + column,
                );
            }
        }
        for column in column_low..column_high {
            for row in row_low.saturating_sub(1)..row_high.min(rows.saturating_sub(1)) {
                set_flag(&mut self.vertical_blocked, row * columns + column);
            }
        }
    }

    fn locate(&self, point: PagePoint) -> Option<GridPoint> {
        let column = self
            .xs
            .iter()
            .position(|&x| (x - point.x).abs() <= GRID_MERGE_PX)?;
        let row = self
            .ys
            .iter()
            .position(|&y| (y - point.y).abs() <= GRID_MERGE_PX)?;
        Some((column, row))
    }

    fn point(&self, (column, row): GridPoint) -> Option<PagePoint> {
        Some(PagePoint {
            x: *self.xs.get(column)?,
            y: *self.ys.get(row)?,
        })
    }

    fn node_open(&self, (column, row): GridPoint) -> bool {
        column < self.xs.len()
            && row < self.ys.len()
            && !self
                .node_blocked
                .get(row * self.xs.len() + column)
                .copied()
                .unwrap_or(true)
    }

    /// The neighbor one line away in `direction` and the move's length, when the move stays
    /// out of every obstacle.
    fn step(&self, (column, row): GridPoint, direction: Direction) -> Option<(GridPoint, f64)> {
        let columns = self.xs.len();
        let (target, move_open) = match direction {
            Direction::Right => (
                (column + 1, row),
                self.horizontal_blocked
                    .get(row * columns.saturating_sub(1) + column),
            ),
            Direction::Left => {
                let left = column.checked_sub(1)?;
                (
                    (left, row),
                    self.horizontal_blocked
                        .get(row * columns.saturating_sub(1) + left),
                )
            }
            Direction::Down => (
                (column, row + 1),
                self.vertical_blocked.get(row * columns + column),
            ),
            Direction::Up => {
                let up = row.checked_sub(1)?;
                (
                    (column, up),
                    self.vertical_blocked.get(up * columns + column),
                )
            }
        };
        if move_open.copied().unwrap_or(true) || !self.node_open(target) {
            return None;
        }
        let from = self.point((column, row))?;
        let to = self.point(target)?;
        let length = f64::from((to.x - from.x).abs() + (to.y - from.y).abs());
        Some((target, length))
    }
}

fn set_flag(flags: &mut [bool], index: usize) {
    if let Some(flag) = flags.get_mut(index) {
        *flag = true;
    }
}

fn sorted_lines(mut lines: Vec<f32>) -> Vec<f32> {
    lines.retain(|line| line.is_finite());
    lines.sort_by(f32::total_cmp);
    lines.dedup_by(|later, earlier| (*later - *earlier).abs() <= GRID_MERGE_PX);
    lines
}

/// One A* frontier entry. The heap pops the lowest estimate, then the lower x line, then the
/// lower y line, then the direction order Right, Down, Left, Up.
#[derive(Debug, Clone, Copy)]
struct Frontier {
    estimate: f64,
    cost: f64,
    point: GridPoint,
    direction: Direction,
}

impl PartialEq for Frontier {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Frontier {}

impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Frontier {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimate
            .total_cmp(&self.estimate)
            .then_with(|| other.point.0.cmp(&self.point.0))
            .then_with(|| other.point.1.cmp(&self.point.1))
            .then_with(|| other.direction.index().cmp(&self.direction.index()))
    }
}

/// Marks a state reached directly from the leg's start.
const FROM_START: usize = usize::MAX;

/// A* from `start` to `goal` with Manhattan moves, cost 1 per px plus TURN_COST_PX per
/// turn. `leave` forces the first move's direction and `arrive` the last one's. Returns the
/// grid points of the cheapest path, or None when no path exists.
fn search_leg(
    grid: &Grid,
    start: GridPoint,
    goal: GridPoint,
    leave: Option<Direction>,
    arrive: Option<Direction>,
) -> Option<Vec<GridPoint>> {
    if !grid.node_open(start) || !grid.node_open(goal) {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }
    let columns = grid.xs.len();
    let goal_point = grid.point(goal)?;
    let state_count = columns * grid.ys.len() * Direction::ALL.len();
    let state_of = |(column, row): GridPoint, direction: Direction| {
        (row * columns + column) * Direction::ALL.len() + direction.index()
    };
    let remaining = |point: GridPoint| -> f64 {
        grid.point(point).map_or(f64::INFINITY, |at| {
            f64::from((at.x - goal_point.x).abs() + (at.y - goal_point.y).abs())
        })
    };
    let mut best_cost = vec![f64::INFINITY; state_count];
    let mut parent = vec![FROM_START; state_count];
    let mut frontier = BinaryHeap::new();

    let first_moves: Vec<Direction> = match leave {
        Some(direction) => vec![direction],
        None => Direction::ALL.to_vec(),
    };
    for direction in first_moves {
        let Some((next, length)) = grid.step(start, direction) else {
            continue;
        };
        let state = state_of(next, direction);
        if let Some(slot) = best_cost.get_mut(state)
            && length < *slot
        {
            *slot = length;
            frontier.push(Frontier {
                estimate: length + remaining(next),
                cost: length,
                point: next,
                direction,
            });
        }
    }

    // Each state is expanded at most once and pushes at most three entries.
    let pops_max = state_count * 3 + Direction::ALL.len() + 1;
    for _ in 0..pops_max {
        let current = frontier.pop()?;
        let state = state_of(current.point, current.direction);
        if current.cost > best_cost.get(state).copied().unwrap_or(f64::NEG_INFINITY) {
            continue;
        }
        if current.point == goal && arrive.is_none_or(|direction| direction == current.direction) {
            return Some(reconstruct(
                start,
                current.point,
                state,
                &parent,
                columns,
                state_count,
            ));
        }
        for direction in Direction::ALL {
            if direction == current.direction.opposite() {
                continue;
            }
            let Some((next, length)) = grid.step(current.point, direction) else {
                continue;
            };
            let turn = if direction == current.direction {
                0.0
            } else {
                TURN_COST_PX
            };
            let cost = current.cost + length + turn;
            let next_state = state_of(next, direction);
            let Some(slot) = best_cost.get_mut(next_state) else {
                continue;
            };
            if cost < *slot {
                *slot = cost;
                if let Some(parent_slot) = parent.get_mut(next_state) {
                    *parent_slot = state;
                }
                frontier.push(Frontier {
                    estimate: cost + remaining(next),
                    cost,
                    point: next,
                    direction,
                });
            }
        }
    }
    None
}

fn reconstruct(
    start: GridPoint,
    goal: GridPoint,
    goal_state: usize,
    parent: &[usize],
    columns: usize,
    state_count: usize,
) -> Vec<GridPoint> {
    let point_of = |state: usize| {
        let node = state / Direction::ALL.len();
        (node % columns, node / columns)
    };
    let mut path = vec![goal];
    let mut state = goal_state;
    for _ in 0..state_count {
        let previous = parent.get(state).copied().unwrap_or(FROM_START);
        if previous == FROM_START {
            break;
        }
        path.push(point_of(previous));
        state = previous;
    }
    path.push(start);
    path.reverse();
    path
}

/// The routed polyline through every via point, simplified to its corners, or None when a
/// leg has no path or the grid is over the cap.
fn grid_route(
    obstacles: &[Obstacle],
    endpoints: &Endpoints,
    via: &[PagePoint],
) -> Option<Vec<PagePoint>> {
    let grid = Grid::build(obstacles, endpoints, via)?;
    let mut waypoints = vec![endpoints.start];
    waypoints.extend_from_slice(via);
    waypoints.push(endpoints.end);
    let leg_count = waypoints.len() - 1;
    let mut path: Vec<GridPoint> = Vec::new();
    for (leg_index, leg) in waypoints.windows(2).enumerate() {
        let (Some(&leg_start), Some(&leg_end)) = (leg.first(), leg.get(1)) else {
            return None;
        };
        let leave = (leg_index == 0).then(|| Direction::outward(endpoints.from_side));
        let arrive =
            (leg_index + 1 == leg_count).then(|| Direction::outward(endpoints.to_side).opposite());
        let leg_path = search_leg(
            &grid,
            grid.locate(leg_start)?,
            grid.locate(leg_end)?,
            leave,
            arrive,
        )?;
        let skip = usize::from(!path.is_empty());
        path.extend(leg_path.into_iter().skip(skip));
    }
    let points: Option<Vec<PagePoint>> = path.into_iter().map(|point| grid.point(point)).collect();
    Some(corner_points(points?))
}

/// Drops repeated points and every point that lies straight between its neighbors, so only
/// the corners remain. A point where the route reverses along one line is a corner.
fn corner_points(points: Vec<PagePoint>) -> Vec<PagePoint> {
    let mut corners: Vec<PagePoint> = Vec::with_capacity(points.len());
    for point in points {
        if corners.last() == Some(&point) {
            continue;
        }
        let length = corners.len();
        if let (Some(&before), Some(&middle)) = (
            corners.get(length.wrapping_sub(2)),
            corners.get(length.wrapping_sub(1)),
        ) {
            let vertical_run = before.x == middle.x
                && middle.x == point.x
                && (middle.y - before.y) * (point.y - middle.y) >= 0.0;
            let horizontal_run = before.y == middle.y
                && middle.y == point.y
                && (middle.x - before.x) * (point.x - middle.x) >= 0.0;
            if vertical_run || horizontal_run {
                corners.pop();
            }
        }
        corners.push(point);
    }
    if let [only] = corners.as_slice() {
        let only = *only;
        corners.push(only);
    }
    corners
}

/// Section 11.2 fallback: an L from the from attach point to the to attach point,
/// horizontal leg first.
fn fallback_route(start: PagePoint, end: PagePoint) -> Vec<PagePoint> {
    corner_points(vec![
        start,
        PagePoint {
            x: end.x,
            y: start.y,
        },
        end,
    ])
}

/// The tag of a labeled link: sized like a pipe tag from the label and sub measured at
/// max-content, centered on the midpoint of the longest segment (the first one on a tie).
fn tag_parts(
    page: &Page,
    link: &Link,
    link_pointer: &NodePointer,
    points: &[PagePoint],
    measurer: &mut dyn TextMeasurer,
) -> Result<Vec<Part>, LayoutError> {
    let Some(label) = &link.label else {
        return Ok(Vec::new());
    };
    let label_run = tag_run(
        page,
        link.kind,
        label,
        TextStyleName::TagLabel,
        link_pointer.child("label"),
        measurer,
    )?;
    let sub_run = match &link.sub {
        Some(sub) => Some(tag_run(
            page,
            link.kind,
            sub,
            TextStyleName::TagSub,
            link_pointer.child("sub"),
            measurer,
        )?),
        None => None,
    };
    let label_size = &label_run.metrics;
    let content_width = sub_run.as_ref().map_or(label_size.width_px, |sub| {
        label_size.width_px.max(sub.metrics.width_px)
    });
    let content_height = label_size.height_px
        + sub_run
            .as_ref()
            .map_or(0.0, |sub| TAG_SUB_GAP_PX + sub.metrics.height_px);
    let tag_width = content_width + 2.0 * (TAG_PADDING_X_PX + TAG_BORDER_PX);
    let tag_height = content_height + 2.0 * (TAG_PADDING_Y_PX + TAG_BORDER_PX);
    let center = longest_segment_midpoint(points);
    let tag = BoxRect {
        x: center.x - tag_width / 2.0,
        y: center.y - tag_height / 2.0,
        width: tag_width,
        height: tag_height,
    };
    let content_x = tag.x + TAG_BORDER_PX + TAG_PADDING_X_PX;
    let content_y = tag.y + TAG_BORDER_PX + TAG_PADDING_Y_PX;
    let centered = |metrics: &TextMetrics, y: f32| BoxRect {
        x: content_x + (content_width - metrics.width_px) / 2.0,
        y,
        width: metrics.width_px,
        height: metrics.height_px,
    };
    let mut parts = vec![
        Part {
            name: PartName::Tag,
            bounds: tag,
            text: None,
        },
        Part {
            name: PartName::TagLabel,
            bounds: centered(&label_run.metrics, content_y),
            text: None,
        },
    ];
    let sub_y = content_y + label_run.metrics.height_px + TAG_SUB_GAP_PX;
    if let Some(label_part) = parts.get_mut(1) {
        label_part.text = Some(label_run);
    }
    if let Some(sub_run) = sub_run {
        parts.push(Part {
            name: PartName::TagSub,
            bounds: centered(&sub_run.metrics, sub_y),
            text: Some(sub_run),
        });
    }
    Ok(parts)
}

fn tag_run(
    page: &Page,
    kind: PipeKind,
    text: &str,
    style_name: TextStyleName,
    source: NodePointer,
    measurer: &mut dyn TextMeasurer,
) -> Result<TextRun, LayoutError> {
    let style = style_name.text_style().style;
    let metrics = measurer
        .measure(text, &style, None)
        .map_err(|source_error| LayoutError::Measure {
            pointer: source,
            source: source_error,
        })?;
    Ok(TextRun {
        text: text.to_string(),
        style,
        color: text_color(style_name, page.canvas, Some(kind)),
        align: TextAlign::Center,
        metrics,
    })
}

fn longest_segment_midpoint(points: &[PagePoint]) -> PagePoint {
    let mut best: Option<(f32, PagePoint)> = None;
    for segment in points.windows(2) {
        let (Some(a), Some(b)) = (segment.first(), segment.get(1)) else {
            continue;
        };
        let length = (b.x - a.x).abs() + (b.y - a.y).abs();
        let midpoint = PagePoint {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0,
        };
        if best.is_none_or(|(best_length, _)| length > best_length) {
            best = Some((length, midpoint));
        }
    }
    best.map_or_else(
        || {
            points
                .first()
                .copied()
                .unwrap_or(PagePoint { x: 0.0, y: 0.0 })
        },
        |(_, midpoint)| midpoint,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> PagePoint {
        PagePoint { x, y }
    }

    #[test]
    fn corner_points_keep_only_turns() {
        let simplified = corner_points(vec![
            point(0.0, 0.0),
            point(5.0, 0.0),
            point(10.0, 0.0),
            point(10.0, 0.0),
            point(10.0, 5.0),
            point(10.0, 10.0),
        ]);
        assert_eq!(
            simplified,
            vec![point(0.0, 0.0), point(10.0, 0.0), point(10.0, 10.0)]
        );
    }

    #[test]
    fn corner_points_keep_a_reversal() {
        let simplified = corner_points(vec![point(0.0, 0.0), point(10.0, 0.0), point(4.0, 0.0)]);
        assert_eq!(simplified.len(), 3);
    }

    #[test]
    fn fallback_is_an_l_with_the_horizontal_leg_first() {
        assert_eq!(
            fallback_route(point(0.0, 0.0), point(30.0, 20.0)),
            vec![point(0.0, 0.0), point(30.0, 0.0), point(30.0, 20.0)]
        );
        assert_eq!(
            fallback_route(point(0.0, 5.0), point(30.0, 5.0)),
            vec![point(0.0, 5.0), point(30.0, 5.0)]
        );
    }

    #[test]
    fn a_segment_along_an_edge_does_not_enter() {
        let bounds = BoxRect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        assert!(!segment_enters(
            point(0.0, 10.0),
            point(40.0, 10.0),
            &bounds
        ));
        assert!(segment_enters(
            point(0.0, 10.02),
            point(40.0, 10.02),
            &bounds
        ));
        assert!(segment_enters(point(0.0, 20.0), point(40.0, 20.0), &bounds));
        assert!(!segment_enters(
            point(0.0, 20.0),
            point(10.0, 20.0),
            &bounds
        ));
    }

    #[test]
    fn facing_sides_prefer_right_on_a_tie() {
        let square = |x: f32, y: f32| BoxRect {
            x,
            y,
            width: 10.0,
            height: 10.0,
        };
        assert_eq!(
            facing_pair(&square(0.0, 0.0), &square(40.0, 0.0)),
            (Side::Right, Side::Left)
        );
        assert_eq!(
            facing_pair(&square(0.0, 0.0), &square(0.0, 40.0)),
            (Side::Bottom, Side::Top)
        );
        assert_eq!(
            facing_pair(&square(40.0, 40.0), &square(0.0, 0.0)),
            (Side::Left, Side::Right)
        );
    }
}
