//! The related sets of section 5.2: for each element that refers to something else, the
//! units that stay drawn while the pointer rests on it or the URL fragment names it. Every
//! set is read from the model and the laid-out geometry, never from drawn text.

use std::collections::{BTreeSet, HashMap};

use stencil_layout::{
    DescriptionUnits, NodeTag, PageGeometry, PartName, PipeEndSide, description_units,
    tee_spine_landing,
};
use stencil_model::marks::{CarrierMark, PageMarks};
use stencil_model::pointer::NodePointer;
use stencil_model::{
    LINKS_MAX, LegendKey, Node, NodeRef, Page, PageActors, SheetList, body_nodes, entry_key,
    line_uses,
};

use crate::{HoverRule, HoverSource};

/// The tags of the geometry groups that fade. Each fades its own drawn elements, never its
/// child groups, so a kept unit inside a faded one stays drawn and nothing fades twice. A
/// link's group, tagged `Link`, fades too.
pub(crate) const FADE_TAGS: [NodeTag; 10] = [
    NodeTag::Pcard,
    NodeTag::Zone,
    NodeTag::Pipe,
    NodeTag::Tee,
    NodeTag::Step,
    NodeTag::Rung,
    NodeTag::ProblemPoint,
    NodeTag::ActorChip,
    NodeTag::LegendEntry,
    NodeTag::HowToItem,
];

/// The key of each part of one node or link tag, aligned with its parts.
pub(crate) type PartKeys = Vec<Option<ElementKey>>;

/// The fading tags whose groups hold other units: a Box holds its children and a Tee its
/// arms. Every other fading group, and a link, is a leaf.
pub(crate) const CONTAINER_TAGS: [NodeTag; 2] = [NodeTag::Zone, NodeTag::Tee];

/// The key a drawn element of a row or a mark carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElementKey {
    /// An element of a parts-list, general-notes or attention row: the row's pointer.
    Row(String),
    /// A balloon, flag or diamond and its numeral: the pointer of the row of its key.
    Mark(String),
}

/// What the writer adds for the highlight: the `id` and end pointers of each group, the
/// key of each part of a row or mark, the `id` of each row, and the rules.
#[derive(Debug, Clone, Default)]
pub(crate) struct Highlights {
    /// Per geometry node, the `id` its group carries; Some exactly for a fading group.
    pub anchors: Vec<Option<String>>,
    /// Per geometry node, `data-from` and `data-to`: a Pipe's end nodes, a Tee arm's hub
    /// and end node, and a Tee's spine end node.
    pub ends: Vec<(Option<String>, Option<String>)>,
    /// Per geometry node, the key of each of its parts, aligned with `parts`.
    pub part_keys: Vec<PartKeys>,
    /// Per link route index, the key of each part of its tag.
    pub link_part_keys: HashMap<usize, PartKeys>,
    pub rules: Vec<HoverRule>,
}

impl Highlights {
    /// The source of every rule: group `data-id`s and row pointers.
    pub(crate) fn rule_sources(&self) -> BTreeSet<String> {
        self.rules
            .iter()
            .map(|rule| match &rule.source {
                HoverSource::Group(pointer) | HoverSource::Row(pointer) => pointer.clone(),
            })
            .collect()
    }
}

/// A line and everything drawn with it: a Link, a Pipe, or a Tee with its arms, and the
/// nodes its ends meet.
#[derive(Debug, Clone, Default)]
struct LineFamily {
    members: Vec<String>,
    ends: Vec<usize>,
}

/// The units a rule keeps drawn: groups by `data-id`, rows by pointer.
#[derive(Debug, Clone, Default)]
struct Kept {
    groups: BTreeSet<String>,
    rows: BTreeSet<String>,
}

impl Kept {
    fn extend(&mut self, other: Kept) {
        self.groups.extend(other.groups);
        self.rows.extend(other.rows);
    }
}

/// The model and geometry relations every related set reads.
struct Relations<'a> {
    geometry: &'a PageGeometry,
    index_by_pointer: HashMap<&'a str, usize>,
    document: HashMap<String, NodeRef<'a>>,
    /// Body node id to its pointer.
    id_pointers: HashMap<String, String>,
    families: Vec<LineFamily>,
    /// Unit key of a line member to its family.
    family_of: HashMap<String, usize>,
    /// Per carrier, the row of the key of each mark it draws, in drawn order.
    carrier_marks: HashMap<String, Vec<Option<String>>>,
    /// The carriers that mark each row's key.
    row_carriers: HashMap<String, BTreeSet<String>>,
    /// A part row's node, as a geometry index.
    part_nodes: HashMap<String, usize>,
    /// The part row each node is the node of.
    node_parts: HashMap<usize, String>,
    /// The rows of the parts list, the general notes and the attention cell, in list
    /// order.
    rows: Vec<String>,
    /// Each unit that acts for an actor: (actor id, unit key, true for a group).
    acting: Vec<(String, String, bool)>,
    /// Actor id to the pointer of its legend chip.
    actor_chips: HashMap<String, String>,
    /// An attention row's actor.
    attention_actors: HashMap<String, String>,
    /// Every line use with its legend key.
    line_keys: Vec<(String, LegendKey)>,
}

pub(crate) fn highlights(page: &Page, geometry: &PageGeometry) -> Highlights {
    let units = description_units(page, geometry);
    let relations = Relations::of(page, geometry, &units);
    let anchors: Vec<Option<String>> = geometry
        .nodes
        .iter()
        .map(|node| relations.anchor(node.tag, &node.pointer))
        .collect();
    let ends = relations.node_ends(&units);
    let (part_keys, link_part_keys) = relations.part_keys(page);
    let rules = relations.rules(page, &anchors);
    Highlights {
        anchors,
        ends,
        part_keys,
        link_part_keys,
        rules,
    }
}

/// The pointer with each `/` replaced by `_`, which no `id` can spell.
pub(crate) fn pointer_anchor(pointer: &str) -> String {
    pointer.replace('/', "_")
}

/// An own `id` the writer may use as is: one that no arrowhead marker or shadow filter id
/// can equal.
fn usable_own_id(id: &str) -> bool {
    !id.starts_with("arrow-") && id != super::iso_writer::SHADOW_FILTER_ID
}

/// The pointer of the row of the key a mark names.
fn key_row(page_marks: &PageMarks<'_>, mark: &CarrierMark) -> Option<String> {
    let position = page_marks.position(&mark.mark)?;
    page_marks
        .key_pointer(mark.mark.kind(), position)
        .map(|pointer| pointer.as_str().to_string())
}

/// The part that opens a row of a keyed list, and the parts that belong to the open row,
/// by the tag of the node that draws the list.
fn row_parts(tag: NodeTag) -> Option<(PartName, &'static [PartName])> {
    match tag {
        NodeTag::PartsList => Some((
            PartName::PartBalloon,
            &[
                PartName::PartNumeral,
                PartName::PartTitle,
                PartName::PartWhat,
                PartName::PartPlanned,
                PartName::PartWhy,
                PartName::PartWhyRule,
                PartName::PartZones,
            ],
        )),
        NodeTag::GeneralNotes => Some((
            PartName::NoteFlag,
            &[
                PartName::NoteNumeral,
                PartName::NoteText,
                PartName::NoteDetail,
                PartName::NoteZones,
            ],
        )),
        NodeTag::TitleCell => Some((
            PartName::AttentionDiamond,
            &[
                PartName::AttentionNumeral,
                PartName::AttentionLead,
                PartName::AttentionText,
                PartName::AttentionZones,
            ],
        )),
        _ => None,
    }
}

/// The key of each part: a row's parts take the row, a glyph and the numeral after it the
/// row of the glyph's key, in the order `description_units` pairs them.
fn keys_of_parts(
    parts: &[stencil_layout::Part],
    row_list: Option<(NodePointer, NodeTag)>,
    marks: Option<&Vec<Option<String>>>,
) -> PartKeys {
    let opener = row_list.as_ref().and_then(|(_, tag)| row_parts(*tag));
    let mut rows_opened = 0_usize;
    let mut glyphs = 0_usize;
    let mut glyph_key: Option<String> = None;
    parts
        .iter()
        .map(|part| {
            if let (Some((open, members)), Some((list, _))) = (opener, row_list.as_ref()) {
                let row = |index: usize| list.index(index).as_str().to_string();
                if part.name == open {
                    rows_opened += 1;
                    return Some(ElementKey::Row(row(rows_opened - 1)));
                }
                if members.contains(&part.name) && rows_opened > 0 {
                    return Some(ElementKey::Row(row(rows_opened - 1)));
                }
            }
            if part.name.is_mark_glyph() {
                glyph_key = marks.and_then(|marks| marks.get(glyphs)).cloned().flatten();
                glyphs += 1;
                return glyph_key.clone().map(ElementKey::Mark);
            }
            if part.name == PartName::MarkNumeral {
                return glyph_key.clone().map(ElementKey::Mark);
            }
            None
        })
        .collect()
}

impl<'a> Relations<'a> {
    fn of(page: &'a Page, geometry: &'a PageGeometry, units: &DescriptionUnits) -> Self {
        let entries = body_nodes(page);
        let mut relations = Relations {
            geometry,
            index_by_pointer: geometry
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node.pointer.as_str(), index))
                .collect(),
            document: entries
                .iter()
                .map(|entry| (entry.pointer.as_str().to_string(), entry.node))
                .collect(),
            id_pointers: entries
                .iter()
                .filter_map(|entry| match entry.node {
                    NodeRef::Node(node) => node
                        .id()
                        .map(|id| (id.to_string(), entry.pointer.as_str().to_string())),
                    NodeRef::TeeArm(_) => None,
                })
                .collect(),
            families: Vec::new(),
            family_of: HashMap::new(),
            carrier_marks: HashMap::new(),
            row_carriers: HashMap::new(),
            part_nodes: HashMap::new(),
            node_parts: HashMap::new(),
            rows: Vec::new(),
            acting: Vec::new(),
            actor_chips: HashMap::new(),
            attention_actors: HashMap::new(),
            line_keys: line_uses(page)
                .into_iter()
                .map(|legend_use| (legend_use.pointer.as_str().to_string(), legend_use.key))
                .collect(),
        };
        relations.read_lines(units);
        relations.read_marks(page);
        relations.read_actors(page);
        relations
    }

    fn index_of(&self, pointer: &str) -> Option<usize> {
        self.index_by_pointer.get(pointer).copied()
    }

    fn pointer_of(&self, index: usize) -> Option<String> {
        self.geometry
            .nodes
            .get(index)
            .map(|node| node.pointer.as_str().to_string())
    }

    /// The `id` of a fading group: a body node's own `id`, else its pointer form.
    fn anchor(&self, tag: NodeTag, pointer: &NodePointer) -> Option<String> {
        if !FADE_TAGS.contains(&tag) {
            return None;
        }
        let own = match self.document.get(pointer.as_str()) {
            Some(NodeRef::Node(node)) => node.id(),
            _ => None,
        };
        Some(match own {
            Some(id) if usable_own_id(id) => id.to_string(),
            _ => pointer_anchor(pointer.as_str()),
        })
    }

    /// The node an end of the pipe at `pointer` meets: its target, else what it lands on.
    fn pipe_end(
        &self,
        units: &DescriptionUnits,
        (pointer, target): (&NodePointer, Option<&str>),
        side: PipeEndSide,
    ) -> Option<usize> {
        match target {
            Some(id) => self
                .id_pointers
                .get(id)
                .and_then(|target| self.index_of(target)),
            None => units
                .landing(pointer, side)
                .and_then(|landing| landing.node.as_ref())
                .and_then(|node| self.index_of(node.as_str())),
        }
    }

    fn read_lines(&mut self, units: &DescriptionUnits) {
        let links_pointer = NodePointer::root().child("links");
        for route in self.geometry.links.iter().take(LINKS_MAX) {
            self.families.push(LineFamily {
                members: vec![links_pointer.index(route.index).as_str().to_string()],
                ends: vec![route.from_node, route.to_node],
            });
        }
        for (index, node) in self.geometry.nodes.iter().enumerate() {
            match (node.tag, self.document.get(node.pointer.as_str()).copied()) {
                (NodeTag::Pipe, Some(NodeRef::Node(Node::Pipe(pipe)))) => {
                    let ends = [
                        (pipe.from.as_deref(), PipeEndSide::Start),
                        (pipe.to.as_deref(), PipeEndSide::End),
                    ]
                    .into_iter()
                    .filter_map(|(target, side)| {
                        self.pipe_end(units, (&node.pointer, target), side)
                    })
                    .collect();
                    self.families.push(LineFamily {
                        members: vec![node.pointer.as_str().to_string()],
                        ends,
                    });
                }
                (NodeTag::Tee, Some(NodeRef::Node(Node::Tee(_)))) => {
                    let mut family = LineFamily {
                        members: vec![node.pointer.as_str().to_string()],
                        ends: tee_spine_landing(self.geometry, index)
                            .and_then(|landed| self.index_of(landed.as_str()))
                            .into_iter()
                            .collect(),
                    };
                    for arm in self.geometry.children(index) {
                        let Some(arm_node) = self.geometry.nodes.get(arm) else {
                            continue;
                        };
                        family.members.push(arm_node.pointer.as_str().to_string());
                        family.ends.extend(self.pipe_end(
                            units,
                            (&arm_node.pointer, None),
                            PipeEndSide::End,
                        ));
                    }
                    self.families.push(family);
                }
                _ => {}
            }
        }
        for (family_index, family) in self.families.iter().enumerate() {
            for member in &family.members {
                self.family_of.insert(member.clone(), family_index);
            }
        }
    }

    fn read_marks(&mut self, page: &Page) {
        let page_marks = PageMarks::of(page);
        let mut carriers: Vec<(String, Vec<CarrierMark>)> = Vec::new();
        if let Some(sheet) = &page.sheet {
            let points = SheetList::ProblemPoints;
            for index in 0..sheet.problem.points.len().min(points.max() + 1) {
                carriers.push((
                    points.pointer().index(index).as_str().to_string(),
                    page_marks.point_marks(page, index),
                ));
            }
        }
        for entry in body_nodes(page) {
            carriers.push((
                entry.pointer.as_str().to_string(),
                page_marks.node_marks(&entry),
            ));
            if let NodeRef::Node(Node::Ladder(ladder)) = entry.node {
                let rungs = SheetList::Rungs;
                for index in 0..ladder.rungs.len().min(rungs.max() + 1) {
                    carriers.push((
                        rungs
                            .pointer_in(&entry.pointer)
                            .index(index)
                            .as_str()
                            .to_string(),
                        PageMarks::rung_marks(ladder, &entry.pointer, index),
                    ));
                }
            }
        }
        let links_pointer = NodePointer::root().child("links");
        for (index, link) in page.links.iter().enumerate().take(LINKS_MAX) {
            carriers.push((
                links_pointer.index(index).as_str().to_string(),
                PageMarks::link_marks(link, index),
            ));
        }
        for (carrier, marks) in carriers {
            if marks.is_empty() {
                continue;
            }
            let rows: Vec<Option<String>> = marks
                .iter()
                .map(|mark| key_row(&page_marks, mark))
                .collect();
            for row in rows.iter().flatten() {
                self.row_carriers
                    .entry(row.clone())
                    .or_default()
                    .insert(carrier.clone());
            }
            self.carrier_marks.insert(carrier, rows);
        }
        if let Some(list) = page_marks.parts_pointer() {
            for (position, part) in page_marks.parts().iter().enumerate() {
                let row = SheetList::Parts
                    .pointer_in(list)
                    .index(position)
                    .as_str()
                    .to_string();
                if let Some(node) = part
                    .node
                    .as_deref()
                    .and_then(|id| self.id_pointers.get(id))
                    .and_then(|pointer| self.index_of(pointer))
                {
                    self.part_nodes.insert(row.clone(), node);
                    self.node_parts.entry(node).or_insert_with(|| row.clone());
                }
                if let Some(actor) = page_marks.part_actor(part) {
                    self.acting.push((actor.to_string(), row.clone(), false));
                }
                self.rows.push(row);
            }
        }
        if let Some(list) = page_marks.notes_pointer() {
            for position in 0..page_marks.key_count(stencil_model::MarkKind::Note) {
                self.rows.push(
                    SheetList::GeneralNotes
                        .pointer_in(list)
                        .index(position)
                        .as_str()
                        .to_string(),
                );
            }
        }
    }

    fn read_actors(&mut self, page: &Page) {
        let actors = PageActors::of(page);
        for (index, actor) in page
            .actors
            .iter()
            .enumerate()
            .take(SheetList::Actors.max() + 1)
        {
            self.actor_chips.insert(
                actor.id.clone(),
                SheetList::Actors
                    .pointer()
                    .index(index)
                    .as_str()
                    .to_string(),
            );
        }
        if let Some(sheet) = &page.sheet {
            let list = SheetList::AttentionItems;
            for (index, item) in sheet
                .attention
                .items
                .iter()
                .enumerate()
                .take(list.max() + 1)
            {
                let row = list.pointer().index(index).as_str().to_string();
                if let Some(actor) = &item.actor {
                    self.attention_actors.insert(row.clone(), actor.clone());
                    self.acting.push((actor.clone(), row.clone(), false));
                }
                self.rows.push(row);
            }
        }
        let mut tee: Option<&stencil_model::Tee> = None;
        for entry in body_nodes(page) {
            let actor = match entry.node {
                NodeRef::Node(Node::Item(item)) => item.actor.as_deref(),
                NodeRef::Node(Node::Box(box_node)) => box_node.actor.as_deref(),
                NodeRef::Node(Node::Step(step)) => Some(step.lane.as_str()),
                NodeRef::Node(Node::Pipe(pipe)) => actors.pipe_actor(pipe),
                NodeRef::Node(Node::Tee(tee_node)) => {
                    tee = Some(tee_node);
                    actors.tee_actor(tee_node)
                }
                NodeRef::TeeArm(arm) => tee.and_then(|tee| actors.arm_actor(tee, arm)),
                NodeRef::Node(Node::Ladder(ladder)) => {
                    let rungs = SheetList::Rungs;
                    for (index, rung) in ladder.rungs.iter().enumerate().take(rungs.max() + 1) {
                        if let Some(actor) = &rung.actor {
                            let pointer = rungs.pointer_in(&entry.pointer).index(index);
                            self.acting
                                .push((actor.clone(), pointer.as_str().to_string(), true));
                        }
                    }
                    None
                }
                NodeRef::Node(_) => None,
            };
            if let Some(actor) = actor {
                self.acting
                    .push((actor.to_string(), entry.pointer.as_str().to_string(), true));
            }
        }
        let links_pointer = NodePointer::root().child("links");
        for (index, link) in page.links.iter().enumerate().take(LINKS_MAX) {
            if let Some(actor) = actors.link_actor(link) {
                self.acting.push((
                    actor.to_string(),
                    links_pointer.index(index).as_str().to_string(),
                    true,
                ));
            }
        }
    }

    /// `data-from` and `data-to` of each geometry node.
    fn node_ends(&self, units: &DescriptionUnits) -> Vec<(Option<String>, Option<String>)> {
        self.geometry
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                let end = |target: Option<&str>, side: PipeEndSide| {
                    self.pipe_end(units, (&node.pointer, target), side)
                        .and_then(|end| self.pointer_of(end))
                };
                match (node.tag, self.document.get(node.pointer.as_str()).copied()) {
                    (NodeTag::Pipe, Some(NodeRef::Node(Node::Pipe(pipe)))) => (
                        end(pipe.from.as_deref(), PipeEndSide::Start),
                        end(pipe.to.as_deref(), PipeEndSide::End),
                    ),
                    (NodeTag::Pipe, Some(NodeRef::TeeArm(_))) => (
                        node.parent.and_then(|parent| self.pointer_of(parent)),
                        end(None, PipeEndSide::End),
                    ),
                    (NodeTag::Tee, _) => (
                        tee_spine_landing(self.geometry, index)
                            .map(|landed| landed.as_str().to_string()),
                        None,
                    ),
                    _ => (None, None),
                }
            })
            .collect()
    }

    /// The key of every part of every node and link tag.
    fn part_keys(&self, page: &Page) -> (Vec<PartKeys>, HashMap<usize, PartKeys>) {
        let page_marks = PageMarks::of(page);
        let attention = NodePointer::root().child("sheet").child("attention");
        let node_keys = self
            .geometry
            .nodes
            .iter()
            .map(|node| {
                let row_list = match node.tag {
                    NodeTag::PartsList if page_marks.parts_pointer() == Some(&node.pointer) => {
                        Some((SheetList::Parts.pointer_in(&node.pointer), node.tag))
                    }
                    NodeTag::GeneralNotes if page_marks.notes_pointer() == Some(&node.pointer) => {
                        Some((SheetList::GeneralNotes.pointer_in(&node.pointer), node.tag))
                    }
                    NodeTag::TitleCell if node.pointer == attention => {
                        Some((SheetList::AttentionItems.pointer(), node.tag))
                    }
                    _ => None,
                };
                keys_of_parts(
                    &node.parts,
                    row_list,
                    self.carrier_marks.get(node.pointer.as_str()),
                )
            })
            .collect();
        let links_pointer = NodePointer::root().child("links");
        let link_keys = self
            .geometry
            .links
            .iter()
            .take(LINKS_MAX)
            .map(|route| {
                let pointer = links_pointer.index(route.index);
                (
                    route.index,
                    keys_of_parts(&route.parts, None, self.carrier_marks.get(pointer.as_str())),
                )
            })
            .collect();
        (node_keys, link_keys)
    }

    /// The Boxes that frame the node at `index`, innermost first.
    fn framing_zones(&self, index: usize) -> Vec<String> {
        let mut zones = Vec::new();
        let mut current = self.geometry.nodes.get(index).and_then(|node| node.parent);
        for _ in 0..self.geometry.nodes.len() {
            let Some(parent) = current.and_then(|parent| self.geometry.nodes.get(parent)) else {
                break;
            };
            if parent.tag == NodeTag::Zone {
                zones.push(parent.pointer.as_str().to_string());
            }
            current = parent.parent;
        }
        zones
    }

    /// The rows of the keys `carrier` marks, and the node of each part among them.
    fn references(&self, carrier: &str, kept: &mut Kept) {
        for row in self
            .carrier_marks
            .get(carrier)
            .into_iter()
            .flatten()
            .flatten()
        {
            kept.rows.insert(row.clone());
            if let Some(node) = self
                .part_nodes
                .get(row)
                .and_then(|node| self.pointer_of(*node))
            {
                kept.groups.insert(node);
            }
        }
    }

    /// The part row the node at `index` is the node of, and every carrier of its balloon.
    fn own_part(&self, index: usize, kept: &mut Kept) {
        if let Some(row) = self.node_parts.get(&index) {
            kept.rows.insert(row.clone());
            kept.groups
                .extend(self.row_carriers.get(row).into_iter().flatten().cloned());
        }
    }

    /// A line family: its members, the nodes its ends meet, and what its members refer to.
    fn family_kept(&self, family: &LineFamily) -> Kept {
        let mut kept = Kept::default();
        for member in &family.members {
            kept.groups.insert(member.clone());
            self.references(member, &mut kept);
            if let Some(index) = self.index_of(member) {
                self.own_part(index, &mut kept);
            }
        }
        kept.groups
            .extend(family.ends.iter().filter_map(|end| self.pointer_of(*end)));
        kept
    }

    /// The families with an end on one of `nodes`.
    fn families_touching(&self, nodes: &BTreeSet<usize>) -> Vec<&LineFamily> {
        self.families
            .iter()
            .filter(|family| family.ends.iter().any(|end| nodes.contains(end)))
            .collect()
    }

    /// The node at `index` and every node inside it.
    fn subtree(&self, index: usize) -> BTreeSet<usize> {
        let mut nodes = BTreeSet::from([index]);
        for (candidate, _) in self.geometry.nodes.iter().enumerate().skip(index + 1) {
            let mut current = self
                .geometry
                .nodes
                .get(candidate)
                .and_then(|node| node.parent);
            for _ in 0..self.geometry.nodes.len() {
                match current {
                    Some(parent) if parent == index => {
                        nodes.insert(candidate);
                        break;
                    }
                    Some(parent) => {
                        current = self.geometry.nodes.get(parent).and_then(|node| node.parent);
                    }
                    None => break,
                }
            }
        }
        nodes
    }

    /// Everything that acts for `actor`.
    fn acting_for(&self, actor: &str) -> Kept {
        let mut kept = Kept::default();
        for (acting_actor, unit, is_group) in &self.acting {
            if acting_actor != actor {
                continue;
            }
            if *is_group {
                kept.groups.insert(unit.clone());
            } else {
                kept.rows.insert(unit.clone());
            }
        }
        kept
    }

    /// What stays drawn while the group of the node at `index` is highlighted.
    fn node_kept(&self, page: &Page, index: usize) -> Option<Kept> {
        let node = self.geometry.nodes.get(index)?;
        let pointer = node.pointer.as_str().to_string();
        let mut kept = Kept::default();
        kept.groups.insert(pointer.clone());
        kept.groups.extend(self.framing_zones(index));
        match node.tag {
            NodeTag::Pipe | NodeTag::Tee => {
                let family = self.families.get(*self.family_of.get(&pointer)?)?;
                kept.extend(self.family_kept(family));
            }
            NodeTag::Zone => {
                let inside = self.subtree(index);
                for member in &inside {
                    if let Some(inner) = self.geometry.nodes.get(*member)
                        && FADE_TAGS.contains(&inner.tag)
                    {
                        kept.groups.insert(inner.pointer.as_str().to_string());
                    }
                }
                for family in self.families_touching(&inside) {
                    kept.groups.extend(family.members.iter().cloned());
                }
                self.references(&pointer, &mut kept);
                self.own_part(index, &mut kept);
            }
            NodeTag::ActorChip => {
                let actor = page
                    .actors
                    .iter()
                    .find(|actor| self.actor_chips.get(&actor.id) == Some(&pointer))?;
                kept.extend(self.acting_for(&actor.id));
            }
            NodeTag::LegendEntry => {
                let position = node
                    .pointer
                    .as_str()
                    .rsplit('/')
                    .next()?
                    .parse::<usize>()
                    .ok()?;
                let key = entry_key(page.legend.get(position)?);
                kept.groups.extend(
                    self.line_keys
                        .iter()
                        .filter(|(_, use_key)| *use_key == key)
                        .map(|(user, _)| user.clone()),
                );
            }
            _ => {
                for family in self.families_touching(&BTreeSet::from([index])) {
                    kept.extend(self.family_kept(family));
                }
                self.references(&pointer, &mut kept);
                self.own_part(index, &mut kept);
            }
        }
        Some(kept)
    }

    /// What stays drawn while a link is highlighted.
    fn link_kept(&self, route_index: usize) -> Option<Kept> {
        let pointer = NodePointer::root().child("links").index(route_index);
        let family = self.families.get(*self.family_of.get(pointer.as_str())?)?;
        Some(self.family_kept(family))
    }

    /// What stays drawn while a row, or a mark of its key, is highlighted.
    fn row_kept(&self, page: &Page, row: &str) -> Kept {
        let mut kept = Kept::default();
        if let Some(node) = self.part_nodes.get(row)
            && let Some(node_kept) = self.node_kept(page, *node)
        {
            kept.extend(node_kept);
        }
        kept.rows.insert(row.to_string());
        kept.groups
            .extend(self.row_carriers.get(row).into_iter().flatten().cloned());
        if let Some(chip) = self
            .attention_actors
            .get(row)
            .and_then(|actor| self.actor_chips.get(actor))
        {
            kept.groups.insert(chip.clone());
        }
        kept
    }

    /// True for a `data-id` that names a fading group.
    fn is_fade_group(&self, key: &str) -> bool {
        if key.starts_with("/links/") {
            return true;
        }
        self.index_of(key)
            .and_then(|index| self.geometry.nodes.get(index))
            .is_some_and(|node| FADE_TAGS.contains(&node.tag))
    }

    /// One rule per hover source whose set holds more than itself and its framing Boxes:
    /// fading groups in geometry order, then links, then rows in list order.
    fn rules(&self, page: &Page, anchors: &[Option<String>]) -> Vec<HoverRule> {
        let mut rules = Vec::new();
        let mut push = |source: HoverSource, anchor: String, framing: Vec<String>, kept: Kept| {
            let mut kept = kept;
            kept.groups.retain(|key| self.is_fade_group(key));
            kept.rows.retain(|row| self.rows.contains(row));
            let own = match &source {
                HoverSource::Group(pointer) | HoverSource::Row(pointer) => pointer.clone(),
            };
            let beyond = kept
                .groups
                .iter()
                .chain(&kept.rows)
                .any(|key| *key != own && !framing.contains(key));
            if beyond {
                rules.push(HoverRule {
                    source,
                    anchor,
                    kept_groups: kept.groups.into_iter().collect(),
                    kept_rows: kept.rows.into_iter().collect(),
                });
            }
        };
        for (index, node) in self.geometry.nodes.iter().enumerate() {
            let Some(anchor) = anchors.get(index).cloned().flatten() else {
                continue;
            };
            if let Some(kept) = self.node_kept(page, index) {
                push(
                    HoverSource::Group(node.pointer.as_str().to_string()),
                    anchor,
                    self.framing_zones(index),
                    kept,
                );
            }
        }
        let links_pointer = NodePointer::root().child("links");
        for route in self.geometry.links.iter().take(LINKS_MAX) {
            let pointer = links_pointer.index(route.index).as_str().to_string();
            if let Some(kept) = self.link_kept(route.index) {
                push(
                    HoverSource::Group(pointer.clone()),
                    pointer_anchor(&pointer),
                    Vec::new(),
                    kept,
                );
            }
        }
        for row in &self.rows {
            push(
                HoverSource::Row(row.clone()),
                pointer_anchor(row),
                Vec::new(),
                self.row_kept(page, row),
            );
        }
        rules
    }
}
