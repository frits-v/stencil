//! The marks of a design sheet (section 14.2 rule 4): what each carrier draws, in drawn
//! order, and what each mark keys. Vet, layout, the writer and the description read the
//! marks through this one table, so they pair glyphs with marks the same way.

use std::collections::BTreeMap;

use crate::document::{
    GeneralNotes, Ladder, Link, Mark, MarkKind, Node, Page, PartEntry, PartsList,
};
use crate::pointer::NodePointer;
use crate::sheet::{PageActors, SheetList};
use crate::walk::{NodeEntry, NodeRef, body_nodes};
use crate::{LINKS_MAX, MARKS_MAX};

/// A carrier's id, its own marks, its corner mark and the marks of each of its lines or
/// compartments.
type CarrierFields<'a> = (
    Option<&'a str>,
    &'a [Mark],
    Option<&'a Mark>,
    Vec<&'a [Mark]>,
);

/// Which entries of a carrier hold their own marks: a Step's `lines` or a store's
/// `compartments`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryField {
    Lines,
    Compartments,
}

/// Where on its carrier a mark is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkPlace {
    /// Right-aligned on the carrier's first text line, or in a link's or pipe's tag.
    Trailing,
    /// Centred on the carrier's top-right corner, half outside it.
    Corner,
    /// Right-aligned on the carrier's `lines` entry of this index.
    Line(usize),
    /// Right-aligned inside the store's compartment of this index.
    Compartment(usize),
}

/// One mark a carrier draws, in drawn order: a part's balloon on its `node` first, then
/// `marks`, then `corner_mark`, then the marks of each line in line order.
#[derive(Debug, Clone, PartialEq)]
pub struct CarrierMark {
    /// The mark's pointer: `/…/parts/<i>/node` for a part's own balloon, else the mark in
    /// the carrier's `marks`, `corner_mark` or line.
    pub pointer: NodePointer,
    pub mark: Mark,
    pub place: MarkPlace,
}

/// The keyed lists of a page and the balloon each part puts on its node.
#[derive(Debug, Clone, Default)]
pub struct PageMarks<'a> {
    parts: Vec<&'a PartEntry>,
    /// The first PartsList node's pointer.
    parts_pointer: Option<NodePointer>,
    notes: Vec<&'a crate::document::GeneralNote>,
    notes_pointer: Option<NodePointer>,
    /// Node id to the index of the first part that names it.
    node_parts: BTreeMap<&'a str, usize>,
    /// Node id to the name and what a part keyed by that node takes.
    node_names: BTreeMap<&'a str, (&'a str, Option<&'a str>)>,
    /// Node id to its own `actor`, else the actor its line takes.
    node_actors: BTreeMap<&'a str, &'a str>,
    /// The ids of planned Items, Boxes and Pipes.
    planned_nodes: std::collections::BTreeSet<&'a str>,
    attention: Vec<&'a crate::document::AttentionItem>,
}

impl<'a> PageMarks<'a> {
    pub fn of(page: &'a Page) -> Self {
        let mut marks = PageMarks::default();
        let actors = PageActors::of(page);
        for entry in body_nodes(page) {
            match entry.node {
                NodeRef::Node(Node::PartsList(list)) if marks.parts_pointer.is_none() => {
                    marks.read_parts(list, &entry.pointer);
                }
                NodeRef::Node(Node::GeneralNotes(notes)) if marks.notes_pointer.is_none() => {
                    marks.read_notes(notes, &entry.pointer);
                }
                NodeRef::Node(Node::Item(item)) => {
                    if let Some(id) = item.id.as_deref() {
                        marks
                            .node_names
                            .entry(id)
                            .or_insert((item.title.as_str(), item.subtitle.as_deref()));
                        if let Some(actor) = item.actor.as_deref() {
                            marks.node_actors.entry(id).or_insert(actor);
                        }
                        if item.planned {
                            marks.planned_nodes.insert(id);
                        }
                    }
                }
                NodeRef::Node(Node::Box(box_node)) => {
                    if let Some(id) = box_node.id.as_deref() {
                        marks
                            .node_names
                            .entry(id)
                            .or_insert((box_node.label.as_str(), None));
                        if let Some(actor) = box_node.actor.as_deref() {
                            marks.node_actors.entry(id).or_insert(actor);
                        }
                        if box_node.planned {
                            marks.planned_nodes.insert(id);
                        }
                    }
                }
                NodeRef::Node(Node::Pipe(pipe)) => {
                    if let Some(id) = pipe.id.as_deref() {
                        marks
                            .node_names
                            .entry(id)
                            .or_insert((pipe.label.as_str(), pipe.sub.as_deref()));
                        if let Some(actor) = actors.pipe_actor(pipe) {
                            marks.node_actors.entry(id).or_insert(actor);
                        }
                        if pipe.planned {
                            marks.planned_nodes.insert(id);
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(sheet) = &page.sheet {
            marks.attention = sheet
                .attention
                .items
                .iter()
                .take(SheetList::AttentionItems.max() + 1)
                .collect();
        }
        marks
    }

    fn read_parts(&mut self, list: &'a PartsList, pointer: &NodePointer) {
        self.parts_pointer = Some(pointer.clone());
        self.parts = list.parts.iter().take(SheetList::Parts.max() + 1).collect();
        for (index, part) in self.parts.iter().enumerate() {
            if let Some(node) = part.node.as_deref() {
                self.node_parts.entry(node).or_insert(index);
            }
        }
    }

    fn read_notes(&mut self, notes: &'a GeneralNotes, pointer: &NodePointer) {
        self.notes_pointer = Some(pointer.clone());
        self.notes = notes
            .notes
            .iter()
            .take(SheetList::GeneralNotes.max() + 1)
            .collect();
    }

    /// The pointer of the PartsList node, when the page has one.
    pub fn parts_pointer(&self) -> Option<&NodePointer> {
        self.parts_pointer.as_ref()
    }

    /// The pointer of the GeneralNotes node, when the page has one.
    pub fn notes_pointer(&self) -> Option<&NodePointer> {
        self.notes_pointer.as_ref()
    }

    /// The position of the key a mark names in its list, from 0.
    pub fn position(&self, mark: &Mark) -> Option<usize> {
        match mark {
            Mark::Part(id) => self.parts.iter().position(|part| part.id == *id),
            Mark::Note(id) => self.notes.iter().position(|note| note.id == *id),
            Mark::Attention(id) => self.attention.iter().position(|item| item.id == *id),
        }
    }

    /// The numeral a mark draws: its key's position plus one.
    pub fn number(&self, mark: &Mark) -> Option<usize> {
        self.position(mark).map(|position| position + 1)
    }

    /// How many keys the list of `kind` holds.
    pub fn key_count(&self, kind: MarkKind) -> usize {
        match kind {
            MarkKind::Part => self.parts.len(),
            MarkKind::Note => self.notes.len(),
            MarkKind::Attention => self.attention.len(),
        }
    }

    /// The pointer of the key at `position` in the list of `kind`.
    pub fn key_pointer(&self, kind: MarkKind, position: usize) -> Option<NodePointer> {
        match kind {
            MarkKind::Part => self
                .parts_pointer
                .as_ref()
                .map(|pointer| SheetList::Parts.pointer_in(pointer).index(position)),
            MarkKind::Note => self
                .notes_pointer
                .as_ref()
                .map(|pointer| SheetList::GeneralNotes.pointer_in(pointer).index(position)),
            MarkKind::Attention => Some(SheetList::AttentionItems.pointer().index(position)),
        }
    }

    /// A part's name: its node's name when it is keyed by `node`, else its own.
    pub fn part_name(&self, part: &PartEntry) -> String {
        match part.node.as_deref() {
            Some(node) => self
                .node_names
                .get(node)
                .map(|(name, _)| (*name).to_string())
                .unwrap_or_default(),
            None => part.name.clone().unwrap_or_default(),
        }
    }

    /// A part's what: its node's subtitle or sub when it is keyed by `node`, else its own.
    pub fn part_what(&self, part: &PartEntry) -> Option<String> {
        match part.node.as_deref() {
            Some(node) => self
                .node_names
                .get(node)
                .and_then(|(_, what)| what.map(str::to_string)),
            None => part.what.clone(),
        }
    }

    /// A part's actor: its node's when it is keyed by `node`, else its own.
    pub fn part_actor(&self, part: &PartEntry) -> Option<&'a str> {
        match part.node.as_deref() {
            Some(node) => self.node_actors.get(node).copied(),
            None => self
                .parts
                .iter()
                .find(|candidate| candidate.id == part.id)
                .and_then(|candidate| candidate.actor.as_deref()),
        }
    }

    /// True for a planned part: its node's `planned` when it is keyed by `node`, else its
    /// own.
    pub fn part_planned(&self, part: &PartEntry) -> bool {
        match part.node.as_deref() {
            Some(node) => self.planned_nodes.contains(node),
            None => part.planned,
        }
    }

    /// The id of the general note at `position`.
    pub fn note_id(&self, position: usize) -> Option<String> {
        self.notes.get(position).map(|note| note.id.clone())
    }

    /// The parts, in list order.
    pub fn parts(&self) -> &[&'a PartEntry] {
        &self.parts
    }

    /// The text a mark's description names its key by: a part's name, a note's text, an
    /// attention item's lead.
    pub fn key_name(&self, mark: &Mark) -> Option<String> {
        let position = self.position(mark)?;
        match mark {
            Mark::Part(_) => self.parts.get(position).map(|part| self.part_name(part)),
            Mark::Note(_) => self.notes.get(position).map(|note| note.text.clone()),
            Mark::Attention(_) => self.attention.get(position).map(|item| item.lead.clone()),
        }
    }

    /// The actor whose colours a mark's glyph takes: its part's or its attention item's. A
    /// flag takes none.
    pub fn key_actor(&self, mark: &Mark) -> Option<&'a str> {
        let position = self.position(mark)?;
        match mark {
            Mark::Part(_) => self
                .parts
                .get(position)
                .and_then(|part| self.part_actor(part)),
            Mark::Note(_) => None,
            Mark::Attention(_) => self
                .attention
                .get(position)
                .and_then(|item| item.actor.as_deref()),
        }
    }

    /// The balloon a part puts on the node with this id, when a part names it.
    fn node_balloon(&self, id: Option<&str>) -> Option<CarrierMark> {
        let index = *self.node_parts.get(id?)?;
        let part = self.parts.get(index)?;
        let pointer = SheetList::Parts
            .pointer_in(self.parts_pointer.as_ref()?)
            .index(index)
            .child("node");
        Some(CarrierMark {
            pointer,
            mark: Mark::Part(part.id.clone()),
            place: MarkPlace::Trailing,
        })
    }

    /// The marks a body node draws, in drawn order: an Item, Box, Pipe, Tee arm or Step.
    pub fn node_marks(&self, entry: &NodeEntry<'_>) -> Vec<CarrierMark> {
        self.marks_at(&entry.pointer, entry.node)
    }

    /// The marks of the body node `node` at `pointer`, in drawn order.
    pub fn marks_at(&self, pointer: &NodePointer, node: NodeRef<'_>) -> Vec<CarrierMark> {
        let mut field = EntryField::Lines;
        let (id, marks, corner, lines): CarrierFields<'_> = match node {
            NodeRef::Node(Node::Item(item)) => {
                field = EntryField::Compartments;
                (
                    item.id.as_deref(),
                    &item.marks,
                    item.corner_mark.as_ref(),
                    item.compartments
                        .iter()
                        .take(SheetList::Compartments.max() + 1)
                        .map(crate::document::MarkedText::marks)
                        .collect(),
                )
            }
            NodeRef::Node(Node::Box(box_node)) => (
                box_node.id.as_deref(),
                &box_node.marks,
                box_node.corner_mark.as_ref(),
                Vec::new(),
            ),
            NodeRef::Node(Node::Pipe(pipe)) => (pipe.id.as_deref(), &pipe.marks, None, Vec::new()),
            NodeRef::TeeArm(pipe) => (None, &pipe.marks, None, Vec::new()),
            NodeRef::Node(Node::Step(step)) => (
                None,
                &step.marks,
                step.corner_mark.as_ref(),
                step.lines
                    .iter()
                    .take(SheetList::StepLines.max() + 1)
                    .map(crate::document::MarkedText::marks)
                    .collect(),
            ),
            NodeRef::Node(_) => return Vec::new(),
        };
        let mut drawn: Vec<CarrierMark> = self.node_balloon(id).into_iter().collect();
        let marks_pointer = pointer.child("marks");
        drawn.extend(
            marks
                .iter()
                .take(MARKS_MAX + 1)
                .enumerate()
                .map(|(index, mark)| CarrierMark {
                    pointer: marks_pointer.index(index),
                    mark: mark.clone(),
                    place: MarkPlace::Trailing,
                }),
        );
        if let Some(mark) = corner {
            drawn.push(CarrierMark {
                pointer: pointer.child("corner_mark"),
                mark: mark.clone(),
                place: MarkPlace::Corner,
            });
        }
        let lines_pointer = pointer.child(match field {
            EntryField::Lines => "lines",
            EntryField::Compartments => "compartments",
        });
        for (line, line_marks) in lines.into_iter().enumerate() {
            let line_pointer = lines_pointer.index(line).child("marks");
            let place = match field {
                EntryField::Lines => MarkPlace::Line(line),
                EntryField::Compartments => MarkPlace::Compartment(line),
            };
            drawn.extend(
                line_marks
                    .iter()
                    .take(MARKS_MAX + 1)
                    .enumerate()
                    .map(|(index, mark)| CarrierMark {
                        pointer: line_pointer.index(index),
                        mark: mark.clone(),
                        place,
                    }),
            );
        }
        drawn
    }

    /// The marks of problem point `index`, at the right of its row.
    pub fn point_marks(&self, page: &Page, index: usize) -> Vec<CarrierMark> {
        let Some(point) = page
            .sheet
            .as_ref()
            .and_then(|sheet| sheet.problem.points.get(index))
        else {
            return Vec::new();
        };
        let pointer = SheetList::ProblemPoints
            .pointer()
            .index(index)
            .child("marks");
        point
            .marks
            .iter()
            .take(MARKS_MAX + 1)
            .enumerate()
            .map(|(position, mark)| CarrierMark {
                pointer: pointer.index(position),
                mark: mark.clone(),
                place: MarkPlace::Trailing,
            })
            .collect()
    }

    /// The marks of rung `index` of the Ladder at `ladder_pointer`, on its title line.
    pub fn rung_marks(
        ladder: &Ladder,
        ladder_pointer: &NodePointer,
        index: usize,
    ) -> Vec<CarrierMark> {
        let Some(rung) = ladder.rungs.get(index) else {
            return Vec::new();
        };
        let pointer = SheetList::Rungs
            .pointer_in(ladder_pointer)
            .index(index)
            .child("marks");
        rung.marks
            .iter()
            .take(MARKS_MAX + 1)
            .enumerate()
            .map(|(position, mark)| CarrierMark {
                pointer: pointer.index(position),
                mark: mark.clone(),
                place: MarkPlace::Trailing,
            })
            .collect()
    }

    /// The marks of link `index`, in its tag.
    pub fn link_marks(link: &Link, index: usize) -> Vec<CarrierMark> {
        let pointer = NodePointer::root()
            .child("links")
            .index(index)
            .child("marks");
        link.marks
            .iter()
            .take(MARKS_MAX + 1)
            .enumerate()
            .map(|(position, mark)| CarrierMark {
                pointer: pointer.index(position),
                mark: mark.clone(),
                place: MarkPlace::Trailing,
            })
            .collect()
    }

    /// Every mark of the page in document order: problem points, the body, then links.
    pub fn all(&self, page: &Page) -> Vec<CarrierMark> {
        let mut all = Vec::new();
        if let Some(sheet) = &page.sheet {
            for index in 0..sheet
                .problem
                .points
                .len()
                .min(SheetList::ProblemPoints.max() + 1)
            {
                all.extend(self.point_marks(page, index));
            }
        }
        for entry in body_nodes(page) {
            all.extend(self.node_marks(&entry));
            if let NodeRef::Node(Node::Ladder(ladder)) = entry.node {
                for index in 0..ladder.rungs.len().min(SheetList::Rungs.max() + 1) {
                    all.extend(Self::rung_marks(ladder, &entry.pointer, index));
                }
            }
        }
        for (index, link) in page.links.iter().enumerate().take(LINKS_MAX + 1) {
            all.extend(Self::link_marks(link, index));
        }
        all
    }
}
