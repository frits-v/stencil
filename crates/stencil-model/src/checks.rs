//! Check reports (section 6) and the geometry-free checks; icon-matches-product lives in
//! `products`.

use crate::document::{Chrome, Line, Node, Page, line_tint};
use crate::grammar::{GRAMMAR_REMEMBERED_MAX, Grammar};
use crate::pointer::NodePointer;
use crate::walk::{NodeRef, body_nodes, text_fields};
use crate::{LEGEND_ENTRIES_MAX, LINKS_MAX};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckName {
    ChildInsideContainer,
    SiblingsDoNotOverlap,
    TextFitsBox,
    RememberedConstants,
    LegendConsistency,
    LinksRouted,
    LinksAvoidBoxes,
    PipesLand,
    IsoLabelsClear,
    IsoLinksClear,
    IsoLinkEnds,
    IsoLinksApart,
    PrintFit,
    IconMatchesProduct,
}

impl CheckName {
    /// Kebab-case name used on the CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            CheckName::ChildInsideContainer => "child-inside-container",
            CheckName::SiblingsDoNotOverlap => "siblings-do-not-overlap",
            CheckName::TextFitsBox => "text-fits-box",
            CheckName::RememberedConstants => "remembered-constants",
            CheckName::LegendConsistency => "legend-consistency",
            CheckName::LinksRouted => "links-routed",
            CheckName::LinksAvoidBoxes => "links-avoid-boxes",
            CheckName::PipesLand => "pipes-land",
            CheckName::IsoLabelsClear => "iso-labels-clear",
            CheckName::IsoLinksClear => "iso-links-clear",
            CheckName::IsoLinkEnds => "iso-link-ends",
            CheckName::IsoLinksApart => "iso-links-apart",
            CheckName::PrintFit => "print-fit",
            CheckName::IconMatchesProduct => "icon-matches-product",
        }
    }

    /// Noun for `count` examined units, singular when count is 1 (section 6).
    pub fn unit(self, count: u64) -> &'static str {
        let singular = count == 1;
        match (self, singular) {
            (CheckName::ChildInsideContainer | CheckName::LegendConsistency, true) => "relation",
            (CheckName::ChildInsideContainer | CheckName::LegendConsistency, false) => "relations",
            (
                CheckName::SiblingsDoNotOverlap
                | CheckName::LinksAvoidBoxes
                | CheckName::IsoLabelsClear,
                true,
            ) => "pair",
            (
                CheckName::SiblingsDoNotOverlap
                | CheckName::LinksAvoidBoxes
                | CheckName::IsoLabelsClear,
                false,
            ) => "pairs",
            (CheckName::LinksRouted, true) => "link",
            (CheckName::LinksRouted, false) => "links",
            (CheckName::TextFitsBox | CheckName::PrintFit, true) => "text run",
            (CheckName::TextFitsBox | CheckName::PrintFit, false) => "text runs",
            (CheckName::RememberedConstants, true) => "text field",
            (CheckName::RememberedConstants, false) => "text fields",
            (CheckName::PipesLand, true) => "pipe end",
            (CheckName::PipesLand, false) => "pipe ends",
            (CheckName::IsoLinksClear, true) => "link leg",
            (CheckName::IsoLinksClear, false) => "link legs",
            (CheckName::IsoLinkEnds, true) => "link end",
            (CheckName::IsoLinkEnds, false) => "link ends",
            (CheckName::IsoLinksApart, true) => "link pair",
            (CheckName::IsoLinksApart, false) => "link pairs",
            (CheckName::IconMatchesProduct, true) => "item",
            (CheckName::IconMatchesProduct, false) => "items",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Defect {
    pub pointer: NodePointer,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckReport {
    pub check: CheckName,
    pub examined: u64,
    pub defects: Vec<Defect>,
    /// Why the surface this check examines does not exist on the page, for example
    /// "page has no links". None for every report that looked at the page.
    pub not_applicable: Option<&'static str>,
}

/// How a report reads (section 6). A check that examined nothing fails unless the surface
/// it examines does not exist on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed,
    Failed,
    NotApplicable,
}

impl CheckReport {
    /// A report for a check whose surface does not exist on the page: examined 0, no
    /// defects, and `reason` naming the missing surface.
    pub fn not_applicable(check: CheckName, reason: &'static str) -> Self {
        CheckReport {
            check,
            examined: 0,
            defects: Vec::new(),
            not_applicable: Some(reason),
        }
    }

    /// Passed when examined > 0 with no defects. NotApplicable only when a reason is set,
    /// nothing was examined and there are no defects; a reason on a report that examined
    /// something or found defects is inconsistent and reads as Failed.
    pub fn outcome(&self) -> CheckOutcome {
        match (self.not_applicable, self.examined, self.defects.is_empty()) {
            (None, 1.., true) => CheckOutcome::Passed,
            (Some(_), 0, true) => CheckOutcome::NotApplicable,
            (None, 0, _) | (None, 1.., false) | (Some(_), _, _) => CheckOutcome::Failed,
        }
    }

    /// examined > 0, no defects and no not-applicable reason.
    pub fn passed(&self) -> bool {
        self.outcome() == CheckOutcome::Passed
    }
}

pub(crate) fn is_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// True when `literal` occurs in `text` with no ASCII alphanumeric or `_` directly before
/// or after it, the `\b` boundaries of `cue/core.cue`.
pub fn contains_at_word_boundary(text: &str, literal: &str) -> bool {
    text.char_indices().any(|(byte_index, _)| {
        let Some(rest) = text.get(byte_index..) else {
            return false;
        };
        if !rest.starts_with(literal) {
            return false;
        }
        let before = text
            .get(..byte_index)
            .and_then(|head| head.chars().next_back());
        let after = rest
            .get(literal.len()..)
            .and_then(|tail| tail.chars().next());
        !before.is_some_and(is_word_character) && !after.is_some_and(is_word_character)
    })
}

/// One defect per literal of the grammar's `remembered` list that a text field contains at a
/// word boundary. Examines every authored text field (`text_fields`), before any uppercase
/// transform. Not applicable when the grammar lists no literal.
pub fn remembered_constants(page: &Page, grammar: &Grammar) -> CheckReport {
    if grammar.remembered.is_empty() {
        return CheckReport::not_applicable(
            CheckName::RememberedConstants,
            "grammar has no remembered constants",
        );
    }
    let fields = text_fields(page);
    let mut defects = Vec::new();
    for field in &fields {
        for constant in grammar.remembered.iter().take(GRAMMAR_REMEMBERED_MAX) {
            if contains_at_word_boundary(field.text, &constant.literal) {
                defects.push(Defect {
                    pointer: field.pointer.clone(),
                    message: format!("contains {}: {}", constant.literal, constant.reason),
                });
            }
        }
    }
    CheckReport {
        check: CheckName::RememberedConstants,
        examined: count_as_u64(fields.len()),
        defects,
        not_applicable: None,
    }
}

/// What legend consistency compares: a line and its effective tint (section 13.1 rule 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegendKey {
    line: Line,
    tint: Option<u8>,
}

impl LegendKey {
    fn new(line: Line, tint: Option<u8>) -> Self {
        LegendKey {
            line,
            tint: line_tint(line, tint),
        }
    }

    /// The key in document terms, for example "line solid tint 2" or "line gray".
    fn describe(self) -> String {
        match self.tint {
            Some(tint) => format!("line {} tint {tint}", self.line.as_str()),
            None => format!("line {}", self.line.as_str()),
        }
    }
}

/// Each line use (every Pipe, Tee arm and Tee spine, and each of the first LINKS_MAX + 1
/// links) and each of the first LEGEND_ENTRIES_MAX + 1 legend entries is one examined
/// relation, keyed on (line, effective tint). Defects are listed uses first, in body order
/// and then link order, then legend entries. The bounds cap the scans on a page that was
/// never vetted. A `chrome: none` page whose uses share one key may leave the legend empty
/// (section 13.7 rule 3).
pub fn legend_consistency(page: &Page) -> CheckReport {
    let mut uses: Vec<(NodePointer, LegendKey, &'static str)> = Vec::new();
    for entry in body_nodes(page) {
        match entry.node {
            NodeRef::Node(Node::Pipe(pipe)) => {
                uses.push((entry.pointer, LegendKey::new(pipe.line, pipe.tint), "Pipe"));
            }
            NodeRef::TeeArm(arm) => {
                uses.push((entry.pointer, LegendKey::new(arm.line, arm.tint), "Tee arm"));
            }
            NodeRef::Node(Node::Tee(tee)) => {
                uses.push((
                    entry.pointer,
                    LegendKey::new(tee.line, tee.tint),
                    "Tee spine",
                ));
            }
            NodeRef::Node(
                Node::Row(_)
                | Node::Col(_)
                | Node::Lanes(_)
                | Node::Box(_)
                | Node::Item(_)
                | Node::Fact(_)
                | Node::Note(_)
                | Node::Text(_)
                | Node::Callout(_)
                | Node::Frame(_),
            ) => {}
        }
    }
    let links_pointer = NodePointer::root().child("links");
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX + 1) {
        uses.push((
            links_pointer.index(index),
            LegendKey::new(link.line, link.tint),
            "Link",
        ));
    }

    let legend_examined = page.legend.len().min(LEGEND_ENTRIES_MAX + 1);
    let legend_keys: Vec<LegendKey> = page
        .legend
        .iter()
        .take(legend_examined)
        .map(|legend_entry| LegendKey::new(legend_entry.line, legend_entry.tint))
        .collect();

    let single_key_without_legend = page.chrome == Chrome::None
        && legend_keys.is_empty()
        && uses
            .first()
            .is_some_and(|(_, first_key, _)| uses.iter().all(|(_, key, _)| key == first_key));

    let mut defects = Vec::new();
    if !single_key_without_legend {
        for (pointer, key, user) in &uses {
            if !legend_keys.contains(key) {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!("{user} {} has no legend entry", key.describe()),
                });
            }
        }
    }

    let legend_pointer = NodePointer::root().child("legend");
    for (index, key) in legend_keys.iter().enumerate() {
        let entry_pointer = legend_pointer.index(index);
        if !uses.iter().any(|(_, used_key, _)| used_key == key) {
            defects.push(Defect {
                pointer: entry_pointer.clone(),
                message: format!("legend {} is never used", key.describe()),
            });
        }
        let earlier = legend_keys
            .iter()
            .take(index)
            .position(|earlier_key| earlier_key == key);
        if let Some(earlier_index) = earlier {
            defects.push(Defect {
                pointer: entry_pointer,
                message: format!(
                    "legend {} is already listed at {}",
                    key.describe(),
                    legend_pointer.index(earlier_index)
                ),
            });
        }
    }

    CheckReport {
        check: CheckName::LegendConsistency,
        examined: count_as_u64(uses.len().saturating_add(legend_examined)),
        defects,
        not_applicable: None,
    }
}

fn count_as_u64(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}
