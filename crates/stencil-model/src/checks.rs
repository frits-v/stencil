//! Check reports (section 6) and the two geometry-free checks.

use crate::document::{Node, Page, PipeKind};
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
            (CheckName::TextFitsBox, true) => "text run",
            (CheckName::TextFitsBox, false) => "text runs",
            (CheckName::RememberedConstants, true) => "text field",
            (CheckName::RememberedConstants, false) => "text fields",
            (CheckName::PipesLand, true) => "pipe end",
            (CheckName::PipesLand, false) => "pipe ends",
            (CheckName::IsoLinksClear, true) => "link leg",
            (CheckName::IsoLinksClear, false) => "link legs",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RememberedConstant {
    pub literal: &'static str,
    pub reason: &'static str,
}

const GFE_HEALTH_CHECK_REASON: &str = "GFE health-check probe range. Applies only to backend types the health-check doc names. Not serverless NEG or Cloud Run.";

pub const REMEMBERED_CONSTANTS: [RememberedConstant; 4] = [
    RememberedConstant {
        literal: "64512",
        reason: "doc example ASN, not a requirement. Dedicated Interconnect takes any private ASN (RFC 6996); Partner Interconnect is fixed at 16550.",
    },
    RememberedConstant {
        literal: "130.211.0.0/22",
        reason: GFE_HEALTH_CHECK_REASON,
    },
    RememberedConstant {
        literal: "35.191.0.0/16",
        reason: GFE_HEALTH_CHECK_REASON,
    },
    RememberedConstant {
        literal: "10.8.0.0/28",
        reason: "Serverless VPC Access connector range. Direct VPC egress does not use a connector; label it Private Google Access on the subnet instead.",
    },
];

fn is_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// True when `literal` occurs in `text` with no ASCII alphanumeric or `_` directly before
/// or after it, the `\b` boundaries of `cue/stencil.cue`.
fn contains_at_word_boundary(text: &str, literal: &str) -> bool {
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

/// One defect per listed literal a text field contains at a word boundary. Examines every
/// authored text field (`text_fields`), before any uppercase transform.
pub fn remembered_constants(page: &Page) -> CheckReport {
    let fields = text_fields(page);
    let mut defects = Vec::new();
    for field in &fields {
        for constant in &REMEMBERED_CONSTANTS {
            if contains_at_word_boundary(field.text, constant.literal) {
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

/// Each pipe-kind use (every Pipe, Tee arm and Tee spine, and each of the first
/// LINKS_MAX + 1 links) and each of the first LEGEND_ENTRIES_MAX + 1 legend entries is one
/// examined relation. Defects are listed uses first, in body order and then link order,
/// then legend entries. The bounds cap the scans on a page that was never vetted.
pub fn legend_consistency(page: &Page) -> CheckReport {
    let mut uses: Vec<(NodePointer, PipeKind, &'static str)> = Vec::new();
    for entry in body_nodes(page) {
        match entry.node {
            NodeRef::Node(Node::Pipe(pipe)) => uses.push((entry.pointer, pipe.kind, "Pipe")),
            NodeRef::TeeArm(arm) => uses.push((entry.pointer, arm.kind, "Tee arm")),
            NodeRef::Node(Node::Tee(tee)) => uses.push((entry.pointer, tee.kind, "Tee spine")),
            NodeRef::Node(
                Node::Row(_)
                | Node::Col(_)
                | Node::Zone(_)
                | Node::Pcard(_)
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
        uses.push((links_pointer.index(index), link.kind, "Link"));
    }

    let mut defects = Vec::new();
    for (pointer, kind, user) in &uses {
        if !page
            .legend
            .iter()
            .take(LEGEND_ENTRIES_MAX + 1)
            .any(|legend_entry| legend_entry.kind == *kind)
        {
            defects.push(Defect {
                pointer: pointer.clone(),
                message: format!("{user} kind {} has no legend entry", kind.as_str()),
            });
        }
    }

    let legend_pointer = NodePointer::root().child("legend");
    let legend_examined = page.legend.len().min(LEGEND_ENTRIES_MAX + 1);
    for (index, legend_entry) in page.legend.iter().enumerate().take(legend_examined) {
        let kind = legend_entry.kind;
        let entry_pointer = legend_pointer.index(index);
        if !uses.iter().any(|(_, used_kind, _)| *used_kind == kind) {
            defects.push(Defect {
                pointer: entry_pointer.clone(),
                message: format!("legend kind {} is never used", kind.as_str()),
            });
        }
        let earlier = page
            .legend
            .iter()
            .take(index)
            .position(|earlier_entry| earlier_entry.kind == kind);
        if let Some(earlier_index) = earlier {
            defects.push(Defect {
                pointer: entry_pointer,
                message: format!(
                    "legend kind {} is already listed at {}",
                    kind.as_str(),
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
