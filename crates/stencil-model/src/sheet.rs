//! The design sheet's bounded lists and its surfaces (section 14.2). One table of surfaces
//! drives the `needs-sheet` and `sheet-in-iso` vet rules, the iso writer refusal and the
//! prime vocabulary split.

use std::collections::BTreeMap;

use crate::document::{Line, Link, Node, Page, Pipe, Tee};
use crate::grammar::Grammar;
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
}

impl SheetList {
    pub const ALL: [SheetList; 8] = [
        SheetList::ProblemPoints,
        SheetList::AnswerParagraphs,
        SheetList::AttentionItems,
        SheetList::Actors,
        SheetList::Lanes,
        SheetList::Phases,
        SheetList::Steps,
        SheetList::StepLines,
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
        }
    }

    /// 1 for a list `list-empty` reports. A Phases node without Steps is `children-empty`,
    /// as an empty View is, so `steps` never reports `list-empty`.
    pub fn min(self) -> usize {
        match self {
            SheetList::ProblemPoints
            | SheetList::AttentionItems
            | SheetList::Lanes
            | SheetList::Phases => 1,
            SheetList::AnswerParagraphs
            | SheetList::Actors
            | SheetList::Steps
            | SheetList::StepLines => 0,
        }
    }

    pub fn max(self) -> usize {
        match self {
            SheetList::ProblemPoints
            | SheetList::AttentionItems
            | SheetList::Actors
            | SheetList::Lanes
            | SheetList::Phases => 8,
            SheetList::AnswerParagraphs => 4,
            SheetList::Steps => 64,
            SheetList::StepLines => 6,
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

impl SheetSurface {
    pub const ALL: [SheetSurface; 7] = [
        SheetSurface::Sheet,
        SheetSurface::Actors,
        SheetSurface::View,
        SheetSurface::Phases,
        SheetSurface::Step,
        SheetSurface::Actor,
        SheetSurface::LegendByActor,
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
        }
    }

    /// The surface's pointer on a page, for every occurrence, in document order.
    pub fn occurrences(self, page: &Page, _grammar: &Grammar) -> Vec<NodePointer> {
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
            SheetSurface::Phases | SheetSurface::Step => "phases",
        }
    }
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
