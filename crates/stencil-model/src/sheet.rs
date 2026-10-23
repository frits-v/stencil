//! The design sheet's bounded lists and its surfaces (section 14.2). One table of surfaces
//! drives the `needs-sheet` and `sheet-in-iso` vet rules, the iso writer refusal and the
//! prime vocabulary split.

use std::collections::BTreeMap;

use crate::document::{Line, Link, Node, Outline, Page, Pipe, PipeForm, Tee};
use crate::grammar::{BorderPattern, Grammar, LabelStyle};
use crate::pointer::NodePointer;
use crate::walk::{NodeRef, body_nodes};
use crate::{LEGEND_ENTRIES_MAX, LINKS_MAX};

/// Every bounded list on a sheet, with its name, minimum and maximum, read by `list-empty`,
/// `list-too-long` and the test that counts their CUE cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetList {
    ProblemPoints,
    AnswerParagraphs,
    AttentionItems,
    Actors,
    Lanes,
    Phases,
    Steps,
    StepLines,
    Marks,
    Parts,
    GeneralNotes,
    Compartments,
    Around,
    Rungs,
    HowToItems,
    CommandRuns,
    TerminalLines,
}

impl SheetList {
    pub const ALL: [SheetList; 17] = [
        SheetList::ProblemPoints,
        SheetList::AnswerParagraphs,
        SheetList::AttentionItems,
        SheetList::Actors,
        SheetList::Lanes,
        SheetList::Phases,
        SheetList::Steps,
        SheetList::StepLines,
        SheetList::Marks,
        SheetList::Parts,
        SheetList::GeneralNotes,
        SheetList::Compartments,
        SheetList::Around,
        SheetList::Rungs,
        SheetList::HowToItems,
        SheetList::CommandRuns,
        SheetList::TerminalLines,
    ];

    /// The list's name in messages and in CUE case names, for example "problem-points".
    pub fn name(self) -> &'static str {
        match self {
            SheetList::ProblemPoints => "problem-points",
            SheetList::AnswerParagraphs => "answer-paragraphs",
            SheetList::AttentionItems => "attention-items",
            SheetList::Actors => "actors",
            SheetList::Lanes => "lanes",
            SheetList::Phases => "phases",
            SheetList::Steps => "steps",
            SheetList::StepLines => "step-lines",
            SheetList::Marks => "marks",
            SheetList::Parts => "parts",
            SheetList::GeneralNotes => "general-notes",
            SheetList::Compartments => "compartments",
            SheetList::Around => "around",
            SheetList::Rungs => "rungs",
            SheetList::HowToItems => "how-to-items",
            SheetList::CommandRuns => "command-runs",
            SheetList::TerminalLines => "terminal-lines",
        }
    }

    /// 1 for a list `list-empty` reports. A Phases node without Steps is `children-empty`,
    /// as an empty View is, so `steps` never reports `list-empty`.
    pub fn min(self) -> usize {
        match self {
            SheetList::ProblemPoints
            | SheetList::AttentionItems
            | SheetList::Lanes
            | SheetList::Phases
            | SheetList::Parts
            | SheetList::GeneralNotes
            | SheetList::Around
            | SheetList::Rungs
            | SheetList::HowToItems => 1,
            SheetList::AnswerParagraphs
            | SheetList::Actors
            | SheetList::Steps
            | SheetList::StepLines
            | SheetList::Marks
            | SheetList::Compartments
            | SheetList::CommandRuns
            | SheetList::TerminalLines => 0,
        }
    }

    pub fn max(self) -> usize {
        match self {
            SheetList::ProblemPoints
            | SheetList::AttentionItems
            | SheetList::Actors
            | SheetList::Lanes
            | SheetList::Phases => 8,
            SheetList::AnswerParagraphs | SheetList::Marks => 4,
            SheetList::Steps => 64,
            SheetList::StepLines
            | SheetList::Compartments
            | SheetList::Rungs
            | SheetList::HowToItems => 6,
            SheetList::CommandRuns => 5,
            SheetList::TerminalLines => 12,
            SheetList::Parts => 48,
            SheetList::GeneralNotes | SheetList::Around => 16,
        }
    }

    /// The list's pointer on a page, for a list of the title block or the page.
    pub fn pointer(self) -> NodePointer {
        self.pointer_in(&NodePointer::root())
    }

    /// The list's pointer: a list of the title block or the page has one pointer, and a
    /// list of a Phases node or a Step is the field of `owner`.
    pub fn pointer_in(self, owner: &NodePointer) -> NodePointer {
        let sheet = NodePointer::root().child("sheet");
        match self {
            SheetList::ProblemPoints => sheet.child("problem").child("points"),
            SheetList::AnswerParagraphs => sheet.child("answer").child("paragraphs"),
            SheetList::AttentionItems => sheet.child("attention").child("items"),
            SheetList::Actors => NodePointer::root().child("actors"),
            SheetList::Lanes => owner.child("lanes"),
            SheetList::Phases => owner.child("phases"),
            SheetList::Steps => owner.child("children"),
            SheetList::StepLines => owner.child("lines"),
            SheetList::Marks => owner.child("marks"),
            SheetList::Parts => owner.child("parts"),
            SheetList::GeneralNotes => owner.child("notes"),
            SheetList::Compartments => owner.child("compartments"),
            SheetList::Around => owner.child("around"),
            SheetList::Rungs => owner.child("rungs"),
            SheetList::HowToItems => owner.child("items"),
            SheetList::CommandRuns => owner.child("commands"),
            SheetList::TerminalLines => owner.child("terminal"),
        }
    }
}

/// The actors of a page and what each node and line acts for (section 14.2 rule 3). Built
/// once per page; every lookup is a no-op on a page without actors, so no other page's
/// colours move.
#[derive(Debug, Clone, Default)]
pub struct PageActors<'a> {
    /// Actor id to tint slot, for the first `SheetList::Actors` maximum plus one actors.
    tints: BTreeMap<&'a str, u8>,
    /// Node id to the actor it names: an Item's, Box's, Pipe's or Tee's `actor`, a Step's
    /// lane.
    nodes: BTreeMap<&'a str, &'a str>,
}

impl<'a> PageActors<'a> {
    pub fn of(page: &'a Page) -> Self {
        let tints = page
            .actors
            .iter()
            .take(SheetList::Actors.max() + 1)
            .map(|actor| (actor.id.as_str(), actor.tint))
            .collect();
        let mut nodes = BTreeMap::new();
        let entries = body_nodes(page);
        for entry in &entries {
            let NodeRef::Node(node) = entry.node else {
                continue;
            };
            let named = match node {
                Node::Item(item) => item.actor.as_deref(),
                Node::Box(box_node) => box_node.actor.as_deref(),
                Node::Tee(tee) => tee.actor.as_deref(),
                Node::Step(step) => Some(step.lane.as_str()),
                _ => None,
            };
            if let (Some(id), Some(actor)) = (node.id(), named) {
                nodes.entry(id).or_insert(actor);
            }
        }
        // A Pipe acts for its own actor, else for the actor of its `from` node, which may
        // itself be a Pipe that is resolved first.
        for entry in &entries {
            if let NodeRef::Node(Node::Pipe(pipe)) = entry.node
                && let Some(id) = pipe.id.as_deref()
                && let Some(actor) = pipe.actor.as_deref().or_else(|| {
                    pipe.from
                        .as_deref()
                        .and_then(|from| nodes.get(from).copied())
                })
            {
                nodes.entry(id).or_insert(actor);
            }
        }
        PageActors { tints, nodes }
    }

    /// True when the page declares at least one actor.
    pub fn any(&self) -> bool {
        !self.tints.is_empty()
    }

    /// The tint slot of a declared actor.
    pub fn tint(&self, actor: &str) -> Option<u8> {
        self.tints.get(actor).copied()
    }

    /// The actor a node with this id names, its lane for a Step.
    pub fn node_actor(&self, id: &str) -> Option<&'a str> {
        self.nodes.get(id).copied()
    }

    /// The effective actor of a solid or dash line: its own `actor`, else the actor of
    /// the node it leaves. A gray or deny line acts for no one.
    fn line_actor(&self, line: Line, own: Option<&'a str>, from: Option<&str>) -> Option<&'a str> {
        if !line.takes_tint() {
            return None;
        }
        own.or_else(|| from.and_then(|from| self.node_actor(from)))
    }

    pub fn link_actor(&self, link: &'a Link) -> Option<&'a str> {
        self.line_actor(link.line, link.actor.as_deref(), Some(&link.from))
    }

    /// A Pipe node's effective actor.
    pub fn pipe_actor(&self, pipe: &'a Pipe) -> Option<&'a str> {
        self.line_actor(pipe.line, pipe.actor.as_deref(), pipe.from.as_deref())
    }

    /// A Tee spine's effective actor: its own.
    pub fn tee_actor(&self, tee: &'a Tee) -> Option<&'a str> {
        self.line_actor(tee.line, tee.actor.as_deref(), None)
    }

    /// A Tee arm's effective actor: its own, else its Tee's.
    pub fn arm_actor(&self, tee: &'a Tee, arm: &'a Pipe) -> Option<&'a str> {
        self.line_actor(
            arm.line,
            arm.actor.as_deref().or(tee.actor.as_deref()),
            None,
        )
    }

    /// The tint slot an effective actor gives, when the actor is declared.
    pub fn slot(&self, actor: Option<&str>) -> Option<u8> {
        actor.and_then(|actor| self.tint(actor))
    }
}

/// A schema name the prime vocabulary moves out of the base briefing into a topic
/// (section 14.8 rule 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceVocabulary {
    /// A node tag: no base row.
    Tag(&'static str),
    /// A field of a base row, by row name: left out of that row.
    Field {
        object: &'static str,
        field: &'static str,
    },
    /// One value of a base enum line: left out of the line and its count.
    Value {
        definition: &'static str,
        value: &'static str,
    },
    /// A whole `$defs` entry: no enum line in base, and a topic row or line.
    Definition(&'static str),
}

/// Every surface a design sheet adds beyond the description. Valid on a sheet only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetSurface {
    Sheet,
    Actors,
    View,
    Phases,
    Step,
    Actor,
    LegendByActor,
    Marks,
    PartsList,
    GeneralNotes,
    ItemOutline,
    Compartments,
    Planned,
    SheetLook,
    RailForm,
    LinkStep,
    PipeStep,
    Valve,
    Ladder,
    HowTo,
}

const SHEET_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "Page",
    field: "sheet",
}];
const ACTORS_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "Page",
    field: "actors",
}];
const VIEW_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Tag("View")];
const PHASES_VOCABULARY: [SurfaceVocabulary; 2] = [
    SurfaceVocabulary::Tag("Phases"),
    SurfaceVocabulary::Definition("BandLook"),
];
const STEP_VOCABULARY: [SurfaceVocabulary; 3] = [
    SurfaceVocabulary::Tag("Step"),
    SurfaceVocabulary::Definition("StepLook"),
    SurfaceVocabulary::Definition("StepFill"),
];
const ACTOR_VOCABULARY: [SurfaceVocabulary; 5] = [
    SurfaceVocabulary::Field {
        object: "Item",
        field: "actor",
    },
    SurfaceVocabulary::Field {
        object: "Box",
        field: "actor",
    },
    SurfaceVocabulary::Field {
        object: "Pipe",
        field: "actor",
    },
    SurfaceVocabulary::Field {
        object: "Tee",
        field: "actor",
    },
    SurfaceVocabulary::Field {
        object: "Link",
        field: "actor",
    },
];
const LEGEND_BY_ACTOR_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "LegendEntry",
    field: "by_actor",
}];

const MARKS_VOCABULARY: [SurfaceVocabulary; 7] = [
    SurfaceVocabulary::Field {
        object: "Item",
        field: "marks",
    },
    SurfaceVocabulary::Field {
        object: "Item",
        field: "corner_mark",
    },
    SurfaceVocabulary::Field {
        object: "Box",
        field: "marks",
    },
    SurfaceVocabulary::Field {
        object: "Box",
        field: "corner_mark",
    },
    SurfaceVocabulary::Field {
        object: "Pipe",
        field: "marks",
    },
    SurfaceVocabulary::Field {
        object: "Link",
        field: "marks",
    },
    SurfaceVocabulary::Definition("Mark"),
];
const PARTS_LIST_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Tag("PartsList")];
const GENERAL_NOTES_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Tag("GeneralNotes")];
const LADDER_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Tag("Ladder")];
const HOW_TO_VOCABULARY: [SurfaceVocabulary; 2] = [
    SurfaceVocabulary::Tag("HowTo"),
    SurfaceVocabulary::Definition("TerminalKind"),
];
const ITEM_OUTLINE_VOCABULARY: [SurfaceVocabulary; 2] = [
    SurfaceVocabulary::Field {
        object: "Item",
        field: "outline",
    },
    SurfaceVocabulary::Definition("Outline"),
];
const COMPARTMENTS_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "Item",
    field: "compartments",
}];
const PLANNED_VOCABULARY: [SurfaceVocabulary; 5] = [
    SurfaceVocabulary::Field {
        object: "Item",
        field: "planned",
    },
    SurfaceVocabulary::Field {
        object: "Box",
        field: "planned",
    },
    SurfaceVocabulary::Field {
        object: "Pipe",
        field: "planned",
    },
    SurfaceVocabulary::Field {
        object: "Link",
        field: "planned",
    },
    SurfaceVocabulary::Field {
        object: "LegendEntry",
        field: "planned",
    },
];
const RAIL_FORM_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Value {
    definition: "PipeForm",
    value: "rail",
}];
const LINK_STEP_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "Link",
    field: "step",
}];
const PIPE_STEP_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "Pipe",
    field: "step",
}];
const VALVE_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "Link",
    field: "valve",
}];

impl SheetSurface {
    pub const ALL: [SheetSurface; 20] = [
        SheetSurface::Sheet,
        SheetSurface::Actors,
        SheetSurface::View,
        SheetSurface::Phases,
        SheetSurface::Step,
        SheetSurface::Actor,
        SheetSurface::LegendByActor,
        SheetSurface::Marks,
        SheetSurface::PartsList,
        SheetSurface::GeneralNotes,
        SheetSurface::ItemOutline,
        SheetSurface::Compartments,
        SheetSurface::Planned,
        SheetSurface::SheetLook,
        SheetSurface::RailForm,
        SheetSurface::LinkStep,
        SheetSurface::PipeStep,
        SheetSurface::Valve,
        SheetSurface::Ladder,
        SheetSurface::HowTo,
    ];

    /// The kebab-case name in messages, in CUE's `#SheetSurface` and in check.sh case names.
    pub fn as_str(self) -> &'static str {
        match self {
            SheetSurface::Sheet => "sheet",
            SheetSurface::Actors => "actors",
            SheetSurface::View => "view",
            SheetSurface::Phases => "phases",
            SheetSurface::Step => "step",
            SheetSurface::Actor => "actor",
            SheetSurface::LegendByActor => "legend-by-actor",
            SheetSurface::Marks => "marks",
            SheetSurface::PartsList => "parts-list",
            SheetSurface::GeneralNotes => "general-notes",
            SheetSurface::ItemOutline => "item-outline",
            SheetSurface::Compartments => "compartments",
            SheetSurface::Planned => "planned",
            SheetSurface::SheetLook => "sheet-look",
            SheetSurface::RailForm => "rail-form",
            SheetSurface::LinkStep => "link-step",
            SheetSurface::PipeStep => "pipe-step",
            SheetSurface::Valve => "valve",
            SheetSurface::Ladder => "ladder",
            SheetSurface::HowTo => "how-to",
        }
    }

    /// The surface's pointer on a page, for every occurrence, in document order.
    pub fn occurrences(self, page: &Page, grammar: &Grammar) -> Vec<NodePointer> {
        let tagged = |matches: fn(&Node) -> bool| -> Vec<NodePointer> {
            body_nodes(page)
                .into_iter()
                .filter(|entry| matches!(entry.node, NodeRef::Node(node) if matches(node)))
                .map(|entry| entry.pointer)
                .collect()
        };
        match self {
            SheetSurface::Sheet => page
                .sheet
                .iter()
                .map(|_| NodePointer::root().child("sheet"))
                .collect(),
            SheetSurface::Actors => {
                if page.actors.is_empty() {
                    Vec::new()
                } else {
                    vec![NodePointer::root().child("actors")]
                }
            }
            SheetSurface::View => tagged(|node| matches!(node, Node::View(_))),
            SheetSurface::Phases => tagged(|node| matches!(node, Node::Phases(_))),
            SheetSurface::Step => tagged(|node| matches!(node, Node::Step(_))),
            SheetSurface::Actor => actor_fields(page),
            SheetSurface::LegendByActor => {
                let legend = NodePointer::root().child("legend");
                page.legend
                    .iter()
                    .enumerate()
                    .take(LEGEND_ENTRIES_MAX + 1)
                    .filter(|(_, entry)| entry.by_actor)
                    .map(|(index, _)| legend.index(index).child("by_actor"))
                    .collect()
            }
            SheetSurface::Marks => mark_fields(page),
            SheetSurface::PartsList => tagged(|node| matches!(node, Node::PartsList(_))),
            SheetSurface::GeneralNotes => tagged(|node| matches!(node, Node::GeneralNotes(_))),
            SheetSurface::ItemOutline => body_fields(
                page,
                "outline",
                |node| matches!(node, NodeRef::Node(Node::Item(item)) if item.outline != Outline::Card),
            ),
            SheetSurface::Compartments => body_fields(
                page,
                "compartments",
                |node| matches!(node, NodeRef::Node(Node::Item(item)) if !item.compartments.is_empty()),
            ),
            SheetSurface::Planned => planned_fields(page),
            SheetSurface::SheetLook => body_fields_with(page, "kind", |node| match node {
                NodeRef::Node(Node::Box(box_node)) => {
                    grammar.container(&box_node.kind).is_some_and(|kind| {
                        kind.border.pattern == BorderPattern::DashDot
                            || kind.label == LabelStyle::Tab
                    })
                }
                _ => false,
            }),
            SheetSurface::RailForm => {
                let mut pointers = body_fields(page, "form", |node| match node {
                    NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => {
                        pipe.form == PipeForm::Rail
                    }
                    NodeRef::Node(_) => false,
                });
                let legend = NodePointer::root().child("legend");
                pointers.extend(
                    page.legend
                        .iter()
                        .enumerate()
                        .take(LEGEND_ENTRIES_MAX + 1)
                        .filter(|(_, entry)| entry.form == PipeForm::Rail)
                        .map(|(index, _)| legend.index(index).child("form")),
                );
                pointers
            }
            SheetSurface::LinkStep => link_fields(page, "step", |link| link.step.is_some()),
            SheetSurface::PipeStep => body_fields(page, "step", |node| match node {
                NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => pipe.step.is_some(),
                NodeRef::Node(_) => false,
            }),
            SheetSurface::Valve => link_fields(page, "valve", |link| link.valve),
            SheetSurface::Ladder => tagged(|node| matches!(node, Node::Ladder(_))),
            SheetSurface::HowTo => tagged(|node| matches!(node, Node::HowTo(_))),
        }
    }

    /// The schema names the prime vocabulary moves out of the base briefing.
    pub fn vocabulary(self) -> &'static [SurfaceVocabulary] {
        match self {
            SheetSurface::Sheet => &SHEET_VOCABULARY,
            SheetSurface::Actors => &ACTORS_VOCABULARY,
            SheetSurface::View => &VIEW_VOCABULARY,
            SheetSurface::Phases => &PHASES_VOCABULARY,
            SheetSurface::Step => &STEP_VOCABULARY,
            SheetSurface::Actor => &ACTOR_VOCABULARY,
            SheetSurface::LegendByActor => &LEGEND_BY_ACTOR_VOCABULARY,
            SheetSurface::Marks => &MARKS_VOCABULARY,
            SheetSurface::PartsList => &PARTS_LIST_VOCABULARY,
            SheetSurface::GeneralNotes => &GENERAL_NOTES_VOCABULARY,
            SheetSurface::ItemOutline => &ITEM_OUTLINE_VOCABULARY,
            SheetSurface::Compartments => &COMPARTMENTS_VOCABULARY,
            SheetSurface::Planned => &PLANNED_VOCABULARY,
            // The look lives in the grammar's kinds, which the page schema does not hold.
            SheetSurface::SheetLook => &[],
            SheetSurface::RailForm => &RAIL_FORM_VOCABULARY,
            SheetSurface::LinkStep => &LINK_STEP_VOCABULARY,
            SheetSurface::PipeStep => &PIPE_STEP_VOCABULARY,
            SheetSurface::Valve => &VALVE_VOCABULARY,
            SheetSurface::Ladder => &LADDER_VOCABULARY,
            SheetSurface::HowTo => &HOW_TO_VOCABULARY,
        }
    }

    /// The prime topic that names the surface.
    pub fn topic(self) -> &'static str {
        match self {
            SheetSurface::Sheet
            | SheetSurface::Actors
            | SheetSurface::View
            | SheetSurface::Actor
            | SheetSurface::LegendByActor => "sheet",
            SheetSurface::Phases | SheetSurface::Step | SheetSurface::Ladder => "phases",
            SheetSurface::Marks | SheetSurface::PartsList | SheetSurface::GeneralNotes => "marks",
            SheetSurface::ItemOutline
            | SheetSurface::Compartments
            | SheetSurface::Planned
            | SheetSurface::SheetLook
            | SheetSurface::RailForm
            | SheetSurface::LinkStep
            | SheetSurface::PipeStep
            | SheetSurface::Valve => "assembly",
            SheetSurface::HowTo => "howto",
        }
    }
}

/// `/…/<field>` of every body node `holds` accepts, in document order.
fn body_fields(page: &Page, field: &str, holds: fn(NodeRef<'_>) -> bool) -> Vec<NodePointer> {
    body_fields_with(page, field, holds)
}

/// `/…/<field>` of every body node `holds` accepts, in document order.
fn body_fields_with(
    page: &Page,
    field: &str,
    holds: impl Fn(NodeRef<'_>) -> bool,
) -> Vec<NodePointer> {
    body_nodes(page)
        .into_iter()
        .filter(|entry| holds(entry.node))
        .map(|entry| entry.pointer.child(field))
        .collect()
}

/// `/links/<i>/<field>` of every link `holds` accepts.
fn link_fields(page: &Page, field: &str, holds: fn(&Link) -> bool) -> Vec<NodePointer> {
    let links = NodePointer::root().child("links");
    page.links
        .iter()
        .enumerate()
        .take(LINKS_MAX + 1)
        .filter(|(_, link)| holds(link))
        .map(|(index, _)| links.index(index).child(field))
        .collect()
}

/// `/…/planned` of every planned Item, Box, Pipe and Tee arm, then of every planned link,
/// then of every planned legend entry.
fn planned_fields(page: &Page) -> Vec<NodePointer> {
    let mut pointers = body_fields(page, "planned", |node| match node {
        NodeRef::Node(Node::Item(item)) => item.planned,
        NodeRef::Node(Node::Box(box_node)) => box_node.planned,
        NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => pipe.planned,
        NodeRef::Node(_) => false,
    });
    pointers.extend(link_fields(page, "planned", |link| link.planned));
    let legend = NodePointer::root().child("legend");
    pointers.extend(
        page.legend
            .iter()
            .enumerate()
            .take(LEGEND_ENTRIES_MAX + 1)
            .filter(|(_, entry)| entry.planned)
            .map(|(index, _)| legend.index(index).child("planned")),
    );
    pointers
}

/// `/…/actor` of every Item, Box, Pipe, Tee arm, Tee and Link that names an actor, body
/// nodes first, then links.
fn actor_fields(page: &Page) -> Vec<NodePointer> {
    let mut pointers = Vec::new();
    for entry in body_nodes(page) {
        let actor = match entry.node {
            NodeRef::Node(Node::Item(item)) => item.actor.as_ref(),
            NodeRef::Node(Node::Box(box_node)) => box_node.actor.as_ref(),
            NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => pipe.actor.as_ref(),
            NodeRef::Node(Node::Tee(tee)) => tee.actor.as_ref(),
            NodeRef::Node(_) => None,
        };
        if actor.is_some() {
            pointers.push(entry.pointer.child("actor"));
        }
    }
    let links = NodePointer::root().child("links");
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX + 1) {
        if link.actor.is_some() {
            pointers.push(links.index(index).child("actor"));
        }
    }
    pointers
}

/// Every field that holds a mark, body nodes first, then links: a non-empty `marks`, a
/// `corner_mark`, and the `marks` of a marked line.
fn mark_fields(page: &Page) -> Vec<NodePointer> {
    let mut pointers = Vec::new();
    if let Some(sheet) = &page.sheet {
        let points = SheetList::ProblemPoints;
        for (index, point) in sheet
            .problem
            .points
            .iter()
            .enumerate()
            .take(points.max() + 1)
        {
            if !point.marks.is_empty() {
                pointers.push(points.pointer().index(index).child("marks"));
            }
        }
    }
    for entry in body_nodes(page) {
        let (marks, corner, lines): (&[crate::Mark], bool, &[crate::MarkedText]) = match entry.node
        {
            NodeRef::Node(Node::Item(item)) => (&item.marks, item.corner_mark.is_some(), &[]),
            NodeRef::Node(Node::Box(box_node)) => {
                (&box_node.marks, box_node.corner_mark.is_some(), &[])
            }
            NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => (&pipe.marks, false, &[]),
            NodeRef::Node(Node::Step(step)) => {
                (&step.marks, step.corner_mark.is_some(), &step.lines)
            }
            NodeRef::Node(Node::Ladder(ladder)) => {
                let rungs = SheetList::Rungs;
                for (index, rung) in ladder.rungs.iter().enumerate().take(rungs.max() + 1) {
                    if !rung.marks.is_empty() {
                        pointers.push(rungs.pointer_in(&entry.pointer).index(index).child("marks"));
                    }
                }
                continue;
            }
            NodeRef::Node(_) => continue,
        };
        if !marks.is_empty() {
            pointers.push(entry.pointer.child("marks"));
        }
        if corner {
            pointers.push(entry.pointer.child("corner_mark"));
        }
        for (index, line) in lines
            .iter()
            .enumerate()
            .take(SheetList::StepLines.max() + 1)
        {
            if !line.marks().is_empty() {
                pointers.push(entry.pointer.child("lines").index(index).child("marks"));
            }
        }
    }
    let links = NodePointer::root().child("links");
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX + 1) {
        if !link.marks.is_empty() {
            pointers.push(links.index(index).child("marks"));
        }
    }
    pointers
}
