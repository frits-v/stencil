//! What a figure description must cover (section 14.1 rule 5), computed from the page and
//! its geometry alone, what each untargeted pipe end lands on (rule 11), and on a design
//! sheet the grid reference of every unit (rule 13).

use std::collections::HashMap;
use std::fmt;

use stencil_model::pointer::NodePointer;
use stencil_model::{
    LINKS_MAX, Node, NodeRef, Page, PagePoint, PipeDir, SHEET_COLUMNS, SHEET_ROW_LETTERS,
    SHEET_ROWS, SheetList, body_nodes,
};

use crate::checks::{children_by_parent, landing_box, pipe_ends};
use crate::route::longest_segment_midpoint;
use crate::{BoxRect, NodeTag, PageGeometry, PartName};

/// The parts each keyed entry draws once per entry, in entry order, on its owner node: an
/// entry's reference is the union of its parts (section 14.1 rule 13).
pub const ENTRY_PARTS: [(SheetList, &[PartName]); 1] = [(
    SheetList::AttentionItems,
    &[PartName::AttentionLead, PartName::AttentionText],
)];

/// A grid reference: one cell such as `B3`, or the cells under a box's top-left and
/// bottom-right corners such as `B2-C5`. Rows are 0 to 5 for A to F, columns 1 to 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SheetZone {
    pub first: (u8, u8),
    pub last: (u8, u8),
}

impl fmt::Display for SheetZone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cell = |(row, column): (u8, u8)| {
            let letter = SHEET_ROW_LETTERS
                .get(usize::from(row))
                .copied()
                .unwrap_or('F');
            format!("{letter}{column}")
        };
        if self.first == self.last {
            write!(formatter, "{}", cell(self.first))
        } else {
            write!(formatter, "{}-{}", cell(self.first), cell(self.last))
        }
    }
}

/// The cell under `point` on the grid inside the sheet's inner rule, clamped to the grid.
fn cell_at(point: PagePoint, frame: BoxRect) -> (u8, u8) {
    let index = |offset: f32, extent: f32, count: u8| -> u8 {
        let cells = f32::from(count);
        let position = (offset / (extent / cells)).floor();
        // The clamp keeps the cast in range, so `as` never saturates.
        position.clamp(0.0, cells - 1.0) as u8
    };
    let row = index(point.y - frame.y, frame.height, SHEET_ROWS);
    let column = index(point.x - frame.x, frame.width, SHEET_COLUMNS) + 1;
    (row, column)
}

/// The cell under a point.
pub fn sheet_zone_at(point: PagePoint, frame: BoxRect) -> SheetZone {
    let cell = cell_at(point, frame);
    SheetZone {
        first: cell,
        last: cell,
    }
}

/// The cells under the bounds' top-left and bottom-right corners, each inset 0.5 px.
pub fn sheet_zone(bounds: BoxRect, frame: BoxRect) -> SheetZone {
    let inset = 0.5;
    let first = cell_at(
        PagePoint {
            x: bounds.x + inset,
            y: bounds.y + inset,
        },
        frame,
    );
    let last = cell_at(
        PagePoint {
            x: bounds.right() - inset,
            y: bounds.bottom() - inset,
        },
        frame,
    );
    SheetZone { first, last }
}

/// The grid the references divide: the `Grid` part of a sheet's page root; None off a sheet.
pub fn sheet_frame(geometry: &PageGeometry) -> Option<BoxRect> {
    geometry
        .nodes
        .first()
        .and_then(|page| page.part(PartName::Grid))
        .map(|grid| grid.bounds)
}

/// The units of a description, the structural pointers it may carry, and the pipe ends it
/// names.
#[derive(Debug, Clone, PartialEq)]
pub struct DescriptionUnits {
    /// Nodes in geometry order, then links in index order, then on a sheet the keyed
    /// entries in document order. Its length is the examined count of
    /// `description-covers-drawing`.
    pub units: Vec<DescribedUnit>,
    /// Pointers of geometry nodes, other than the page root, that draw no part: Body,
    /// Legend, Row and Col. Each may be written on at most one line.
    pub structural: Vec<NodePointer>,
    /// What each untargeted end of a pipe names, in body order.
    pub pipe_landings: Vec<PipeLanding>,
}

impl DescriptionUnits {
    /// The unit at `pointer`.
    pub fn unit(&self, pointer: &NodePointer) -> Option<&DescribedUnit> {
        self.units.iter().find(|unit| &unit.pointer == pointer)
    }

    /// What the `end` of the pipe at `pipe` lands on, when that end names no target.
    pub fn landing(&self, pipe: &NodePointer, end: PipeEndSide) -> Option<&PipeLanding> {
        self.pipe_landings
            .iter()
            .find(|landing| &landing.pipe == pipe && landing.end == end)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DescribedUnit {
    pub kind: UnitKind,
    /// A pointer that resolves in the input document.
    pub pointer: NodePointer,
    /// The text that names the unit; the line holding the pointer contains it, escaped.
    pub name: String,
    /// The reference the line must carry on a sheet; None off a sheet.
    pub zone: Option<SheetZone>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Node,
    Link,
    /// A keyed entry that is not a geometry node, such as an attention item.
    Entry,
}

/// One untargeted end of a Pipe or Tee arm.
#[derive(Debug, Clone, PartialEq)]
pub struct PipeLanding {
    pub pipe: NodePointer,
    pub end: PipeEndSide,
    /// The named node, or None for an open end.
    pub node: Option<NodePointer>,
    /// `left neighbour`, `right neighbour`, `neighbour above`, `neighbour below`, `sibling
    /// above`, `sibling below`, or `hub` for the start of a Tee arm, which leaves the spine;
    /// empty for an open end.
    pub relation: &'static str,
}

/// The left or top end of a pipe is `Start`, the right or bottom end `End`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeEndSide {
    Start,
    End,
}

/// The units, structural pointers and pipe landings of a laid-out page.
pub fn description_units(page: &Page, geometry: &PageGeometry) -> DescriptionUnits {
    let entries = body_nodes(page);
    let references: HashMap<&str, NodeRef<'_>> = entries
        .iter()
        .map(|entry| (entry.pointer.as_str(), entry.node))
        .collect();

    let frame = sheet_frame(geometry);
    let zone_of = |bounds: BoxRect| frame.map(|frame| sheet_zone(bounds, frame));
    let mut units = Vec::new();
    let mut structural = Vec::new();
    for node in geometry.nodes.iter().filter(|node| node.parent.is_some()) {
        if node.parts.is_empty() {
            structural.push(node.pointer.clone());
        } else {
            units.push(DescribedUnit {
                kind: UnitKind::Node,
                pointer: node.pointer.clone(),
                name: node_name(
                    page,
                    node.tag,
                    &node.pointer,
                    references.get(node.pointer.as_str()).copied(),
                ),
                zone: zone_of(node.bounds),
            });
        }
    }
    let links_pointer = NodePointer::root().child("links");
    for route in geometry.links.iter().take(LINKS_MAX + 1) {
        let label = page
            .links
            .get(route.index)
            .and_then(|link| link.label.clone());
        let name = label.unwrap_or_else(|| {
            geometry
                .nodes
                .get(route.from_node)
                .map(|from| {
                    node_name(
                        page,
                        from.tag,
                        &from.pointer,
                        references.get(from.pointer.as_str()).copied(),
                    )
                })
                .unwrap_or_default()
        });
        let anchor = match route.tag {
            Some(tag) => PagePoint {
                x: tag.x + tag.width / 2.0,
                y: tag.y + tag.height / 2.0,
            },
            None => longest_segment_midpoint(&route.points).0,
        };
        units.push(DescribedUnit {
            kind: UnitKind::Link,
            pointer: links_pointer.index(route.index),
            name,
            zone: frame.map(|frame| sheet_zone_at(anchor, frame)),
        });
    }
    if let (Some(sheet), Some(frame)) = (&page.sheet, frame) {
        let cell_pointer = NodePointer::root().child("sheet").child("attention");
        let cell = geometry.node(&cell_pointer);
        let list = SheetList::AttentionItems;
        for (index, item) in sheet.attention.items.iter().enumerate().take(list.max() + 1) {
            let bounds = cell.and_then(|cell| entry_bounds(cell, list, index));
            units.push(DescribedUnit {
                kind: UnitKind::Entry,
                pointer: list.pointer().index(index),
                name: item.lead.clone(),
                zone: bounds.map(|bounds| sheet_zone(bounds, frame)),
            });
        }
    }

    DescriptionUnits {
        units,
        structural,
        pipe_landings: pipe_landings(geometry, &references),
    }
}

/// The union of the parts `ENTRY_PARTS` names for `list`, each the `index`-th of its name
/// on `owner`; None when one of them is not drawn.
fn entry_bounds(owner: &crate::NodeGeometry, list: SheetList, index: usize) -> Option<BoxRect> {
    let names = ENTRY_PARTS
        .iter()
        .find(|(entry_list, _)| *entry_list == list)
        .map(|(_, names)| *names)?;
    let mut union: Option<BoxRect> = None;
    for name in names {
        let part = owner
            .parts
            .iter()
            .filter(|part| part.name == *name)
            .nth(index)?;
        union = Some(match union {
            None => part.bounds,
            Some(bounds) => {
                let x = bounds.x.min(part.bounds.x);
                let y = bounds.y.min(part.bounds.y);
                BoxRect {
                    x,
                    y,
                    width: bounds.right().max(part.bounds.right()) - x,
                    height: bounds.bottom().max(part.bounds.bottom()) - y,
                }
            }
        });
    }
    union
}

/// The name a description gives the node at `pointer` drawn as `tag`.
fn node_name(
    page: &Page,
    tag: NodeTag,
    pointer: &NodePointer,
    node: Option<NodeRef<'_>>,
) -> String {
    match tag {
        NodeTag::Page => page.title.clone(),
        NodeTag::Kicker => page.kicker.clone(),
        NodeTag::Title => page.title.clone(),
        NodeTag::Lede => page.lede.clone(),
        NodeTag::Foot => page.foot.clone().unwrap_or_default(),
        NodeTag::LegendEntry => page
            .legend
            .iter()
            .enumerate()
            .find(|(index, _)| NodePointer::root().child("legend").index(*index) == *pointer)
            .map(|(_, entry)| entry.text.clone())
            .unwrap_or_default(),
        NodeTag::Body => "Body".to_string(),
        NodeTag::Legend => "Legend".to_string(),
        NodeTag::Row => "Row".to_string(),
        NodeTag::Col => "Col".to_string(),
        NodeTag::Lanes => "Lanes".to_string(),
        NodeTag::Zone
        | NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Pipe
        | NodeTag::Tee
        | NodeTag::Text
        | NodeTag::Callout
        | NodeTag::Frame
        | NodeTag::View => node.map(body_node_name).unwrap_or_default(),
        NodeTag::TitleBlock => page
            .sheet
            .as_ref()
            .and_then(|sheet| sheet.wordmark.clone())
            .unwrap_or_else(|| page.title.clone()),
        NodeTag::TitleCell => title_cell_caption(page, pointer).unwrap_or_default(),
        NodeTag::ProblemPoint => page
            .sheet
            .iter()
            .flat_map(|sheet| sheet.problem.points.iter().enumerate())
            .find(|(index, _)| SheetList::ProblemPoints.pointer().index(*index) == *pointer)
            .map(|(_, point)| point.text.clone())
            .unwrap_or_default(),
    }
}

/// The caption a title cell opens with: the authored one, or the cell's fixed word.
pub fn title_cell_caption(page: &Page, pointer: &NodePointer) -> Option<String> {
    let sheet = page.sheet.as_ref()?;
    let cell = NodePointer::root().child("sheet");
    let (caption, fixed) = if *pointer == cell.child("problem") {
        (&sheet.problem.caption, "Problem")
    } else if *pointer == cell.child("answer") {
        (&sheet.answer.caption, "Answer")
    } else if *pointer == cell.child("attention") {
        (&sheet.attention.caption, "Attention")
    } else {
        return None;
    };
    Some(caption.clone().unwrap_or_else(|| fixed.to_string()))
}

fn body_node_name(node: NodeRef<'_>) -> String {
    match node {
        NodeRef::Node(Node::Box(box_node)) => box_node.label.clone(),
        NodeRef::Node(Node::Item(item)) => item.title.clone(),
        NodeRef::Node(Node::Fact(fact)) => fact.text.clone(),
        NodeRef::Node(Node::Note(note)) => note.text.clone(),
        NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => pipe.label.clone(),
        NodeRef::Node(Node::Tee(tee)) => tee.hub.clone(),
        NodeRef::Node(Node::Text(text)) => text
            .heading
            .clone()
            .or_else(|| text.body.first().cloned())
            .unwrap_or_default(),
        NodeRef::Node(Node::Callout(callout)) => callout
            .title
            .clone()
            .unwrap_or_else(|| callout.text.clone()),
        NodeRef::Node(Node::Frame(frame)) => frame.label.clone(),
        NodeRef::Node(Node::View(view)) => view.title.clone(),
        NodeRef::Node(Node::Row(_)) => "Row".to_string(),
        NodeRef::Node(Node::Col(_)) => "Col".to_string(),
        NodeRef::Node(Node::Lanes(_)) => "Lanes".to_string(),
    }
}

/// Rule 11: each untargeted end of every Pipe and Tee arm, in geometry order.
fn pipe_landings(
    geometry: &PageGeometry,
    references: &HashMap<&str, NodeRef<'_>>,
) -> Vec<PipeLanding> {
    let children = children_by_parent(geometry);
    let mut landings = Vec::new();
    for (index, node) in geometry.nodes.iter().enumerate() {
        if node.tag != NodeTag::Pipe {
            continue;
        }
        let (pipe, is_arm) = match references.get(node.pointer.as_str()).copied() {
            Some(NodeRef::Node(Node::Pipe(pipe))) => (pipe, false),
            Some(NodeRef::TeeArm(pipe)) => (pipe, true),
            Some(NodeRef::Node(
                Node::Row(_)
                | Node::Col(_)
                | Node::Lanes(_)
                | Node::Box(_)
                | Node::Item(_)
                | Node::Fact(_)
                | Node::Note(_)
                | Node::Tee(_)
                | Node::Text(_)
                | Node::Callout(_)
                | Node::Frame(_)
                | Node::View(_),
            ))
            | None => continue,
        };
        let neighbours = pipe_ends(geometry, &children, index, pipe.dir);
        let center = match pipe.dir {
            PipeDir::Horizontal => node.bounds.y + node.bounds.height / 2.0,
            PipeDir::Vertical => node.bounds.x + node.bounds.width / 2.0,
        };
        for (end, targeted) in [
            (PipeEndSide::Start, pipe.from.is_some()),
            (PipeEndSide::End, pipe.to.is_some()),
        ] {
            if targeted {
                continue;
            }
            let parent = node.parent.and_then(|parent| geometry.nodes.get(parent));
            let (landed, relation) = if is_arm && end == PipeEndSide::Start {
                (parent.map(|tee| tee.pointer.clone()), "hub")
            } else {
                neighbour_landing(
                    geometry,
                    &children,
                    (index, pipe.dir, center),
                    end,
                    &neighbours,
                )
            };
            landings.push(PipeLanding {
                pipe: node.pointer.clone(),
                end,
                node: landed,
                relation,
            });
        }
    }
    landings
}

/// The node one end of the pipe at `index` lands on. A Pipe v inside a Box lands on the
/// sibling before or after it, since a Box stacks its children; any other pipe on the
/// neighbour on that side of the Row or Col that `pipe_ends` finds. Either way the end
/// names what `landing_box` finds in that sibling or neighbour, else the sibling or
/// neighbour itself, and is open when there is none.
fn neighbour_landing(
    geometry: &PageGeometry,
    children: &[Vec<usize>],
    (index, dir, center): (usize, PipeDir, f32),
    end: PipeEndSide,
    neighbours: &[crate::checks::PipeEnd],
) -> (Option<NodePointer>, &'static str) {
    let pipe_before = end == PipeEndSide::End;
    let landed = |root: usize| {
        let landed = landing_box(geometry, children, root, (dir, center, pipe_before))
            .map_or(root, |(landed, _)| landed);
        geometry.nodes.get(landed).map(|node| node.pointer.clone())
    };
    let parent = geometry.nodes.get(index).and_then(|node| node.parent);
    let in_box = dir == PipeDir::Vertical
        && parent
            .and_then(|parent| geometry.nodes.get(parent))
            .is_some_and(|parent| parent.tag == NodeTag::Zone);
    if let (true, Some(parent)) = (in_box, parent) {
        let siblings = children.get(parent).map(Vec::as_slice).unwrap_or_default();
        let position = siblings.iter().position(|&sibling| sibling == index);
        let (sibling, relation) = match end {
            PipeEndSide::Start => (
                position
                    .and_then(|position| position.checked_sub(1))
                    .and_then(|before| siblings.get(before)),
                "sibling above",
            ),
            PipeEndSide::End => (
                position.and_then(|position| siblings.get(position + 1)),
                "sibling below",
            ),
        };
        return match sibling {
            Some(&sibling) => (landed(sibling), relation),
            None => (None, ""),
        };
    }
    let (side, relation) = match (dir, end) {
        (PipeDir::Horizontal, PipeEndSide::Start) => ("left", "left neighbour"),
        (PipeDir::Horizontal, PipeEndSide::End) => ("right", "right neighbour"),
        (PipeDir::Vertical, PipeEndSide::Start) => ("above", "neighbour above"),
        (PipeDir::Vertical, PipeEndSide::End) => ("below", "neighbour below"),
    };
    match neighbours.iter().find(|neighbour| neighbour.side == side) {
        Some(neighbour) => (landed(neighbour.neighbor), relation),
        None => (None, ""),
    }
}
