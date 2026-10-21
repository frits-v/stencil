//! The design sheet's bounded lists and its surfaces (section 14.2). One table of surfaces
//! drives the `needs-sheet` and `sheet-in-iso` vet rules, the iso writer refusal and the
//! prime vocabulary split.

use crate::document::{Node, Page};
use crate::grammar::Grammar;
use crate::pointer::NodePointer;
use crate::walk::{NodeRef, body_nodes};

/// Every bounded list on a sheet, with its name, minimum and maximum, read by `list-empty`,
/// `list-too-long` and the test that counts their CUE cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetList {
    ProblemPoints,
    AnswerParagraphs,
    AttentionItems,
}

impl SheetList {
    pub const ALL: [SheetList; 3] = [
        SheetList::ProblemPoints,
        SheetList::AnswerParagraphs,
        SheetList::AttentionItems,
    ];

    /// The list's name in messages and in CUE case names, for example "problem-points".
    pub fn name(self) -> &'static str {
        match self {
            SheetList::ProblemPoints => "problem-points",
            SheetList::AnswerParagraphs => "answer-paragraphs",
            SheetList::AttentionItems => "attention-items",
        }
    }

    pub fn min(self) -> usize {
        match self {
            SheetList::ProblemPoints | SheetList::AttentionItems => 1,
            SheetList::AnswerParagraphs => 0,
        }
    }

    pub fn max(self) -> usize {
        match self {
            SheetList::ProblemPoints | SheetList::AttentionItems => 8,
            SheetList::AnswerParagraphs => 4,
        }
    }

    /// The list's pointer on a page.
    pub fn pointer(self) -> NodePointer {
        let sheet = NodePointer::root().child("sheet");
        match self {
            SheetList::ProblemPoints => sheet.child("problem").child("points"),
            SheetList::AnswerParagraphs => sheet.child("answer").child("paragraphs"),
            SheetList::AttentionItems => sheet.child("attention").child("items"),
        }
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
    View,
}

const SHEET_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Field {
    object: "Page",
    field: "sheet",
}];
const VIEW_VOCABULARY: [SurfaceVocabulary; 1] = [SurfaceVocabulary::Tag("View")];

impl SheetSurface {
    pub const ALL: [SheetSurface; 2] = [SheetSurface::Sheet, SheetSurface::View];

    /// The kebab-case name in messages, in CUE's `#SheetSurface` and in check.sh case names.
    pub fn as_str(self) -> &'static str {
        match self {
            SheetSurface::Sheet => "sheet",
            SheetSurface::View => "view",
        }
    }

    /// The surface's pointer on a page, for every occurrence, in document order.
    pub fn occurrences(self, page: &Page, _grammar: &Grammar) -> Vec<NodePointer> {
        match self {
            SheetSurface::Sheet => page
                .sheet
                .iter()
                .map(|_| NodePointer::root().child("sheet"))
                .collect(),
            SheetSurface::View => body_nodes(page)
                .into_iter()
                .filter(|entry| matches!(entry.node, NodeRef::Node(Node::View(_))))
                .map(|entry| entry.pointer)
                .collect(),
        }
    }

    /// The schema names the prime vocabulary moves out of the base briefing.
    pub fn vocabulary(self) -> &'static [SurfaceVocabulary] {
        match self {
            SheetSurface::Sheet => &SHEET_VOCABULARY,
            SheetSurface::View => &VIEW_VOCABULARY,
        }
    }

    /// The prime topic that names the surface.
    pub fn topic(self) -> &'static str {
        match self {
            SheetSurface::Sheet | SheetSurface::View => "sheet",
        }
    }
}
