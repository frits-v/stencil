use crate::document::{Node, Page, PipeDir};
use crate::pointer::NodePointer;
use crate::walk::{
    NodeEntry, NodeRef, TextField, body_nodes, push_legend_text_fields, push_node_text_fields,
    push_page_head_text_fields,
};
use crate::{
    CHILDREN_MAX, DEPTH_MAX, GAP_MAX_PX, GROW_WEIGHT_MAX, LEGEND_ENTRIES_MAX, NODES_MAX,
    PAGE_WIDTH_MAX, PAGE_WIDTH_MIN, TEXT_SCALARS_MAX,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub pointer: NodePointer,
    pub rule: VetRule,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VetRule {
    TextEmpty,
    TextTooLong,
    TextControlCharacter,
    TextUntrimmed,
    PageWidthOutOfRange,
    BodyEmpty,
    ChildrenEmpty,
    ChildrenTooMany,
    LegendTooLong,
    GapOutOfRange,
    GrowLengthMismatch,
    GrowOutOfRange,
    TeeArmNotHorizontal,
    DepthExceeded,
    NodesExceeded,
}

impl VetRule {
    /// Kebab-case name used on the CLI, as in section 1.3.
    pub fn as_str(self) -> &'static str {
        match self {
            VetRule::TextEmpty => "text-empty",
            VetRule::TextTooLong => "text-too-long",
            VetRule::TextControlCharacter => "text-control-character",
            VetRule::TextUntrimmed => "text-untrimmed",
            VetRule::PageWidthOutOfRange => "page-width-out-of-range",
            VetRule::BodyEmpty => "body-empty",
            VetRule::ChildrenEmpty => "children-empty",
            VetRule::ChildrenTooMany => "children-too-many",
            VetRule::LegendTooLong => "legend-too-long",
            VetRule::GapOutOfRange => "gap-out-of-range",
            VetRule::GrowLengthMismatch => "grow-length-mismatch",
            VetRule::GrowOutOfRange => "grow-out-of-range",
            VetRule::TeeArmNotHorizontal => "tee-arm-not-horizontal",
            VetRule::DepthExceeded => "depth-exceeded",
            VetRule::NodesExceeded => "nodes-exceeded",
        }
    }
}

struct Violations(Vec<Violation>);

impl Violations {
    fn push(&mut self, pointer: NodePointer, rule: VetRule, message: String) {
        self.0.push(Violation {
            pointer,
            rule,
            message,
        });
    }
}

/// All vet violations, in document order (section 4.2). Empty means valid.
pub fn validate_page(page: &Page) -> Vec<Violation> {
    let mut violations = Violations(Vec::new());
    let root = NodePointer::root();

    let mut head_fields = Vec::new();
    push_page_head_text_fields(page, &mut head_fields);
    check_text_fields(&head_fields, &mut violations);

    if !(PAGE_WIDTH_MIN..=PAGE_WIDTH_MAX).contains(&page.width) {
        violations.push(
            root.child("width"),
            VetRule::PageWidthOutOfRange,
            format!(
                "width {} is outside {PAGE_WIDTH_MIN} to {PAGE_WIDTH_MAX}",
                page.width
            ),
        );
    }

    let body_pointer = root.child("body");
    if page.body.is_empty() {
        violations.push(
            body_pointer.clone(),
            VetRule::BodyEmpty,
            "body has no nodes".to_string(),
        );
    } else if page.body.len() > CHILDREN_MAX {
        violations.push(
            body_pointer.clone(),
            VetRule::ChildrenTooMany,
            children_too_many_message(page.body.len()),
        );
    }
    let entries = body_nodes(page);
    if entries.len() > NODES_MAX {
        violations.push(
            body_pointer,
            VetRule::NodesExceeded,
            format!("more than {NODES_MAX} nodes"),
        );
    }
    for entry in &entries {
        check_entry(entry, &mut violations);
    }

    if page.legend.len() > LEGEND_ENTRIES_MAX {
        violations.push(
            root.child("legend"),
            VetRule::LegendTooLong,
            format!(
                "legend has {} entries, above {LEGEND_ENTRIES_MAX}",
                page.legend.len()
            ),
        );
    }
    let mut legend_fields = Vec::new();
    push_legend_text_fields(page, &mut legend_fields);
    check_text_fields(&legend_fields, &mut violations);

    violations.0
}

/// A node's own violations in struct declaration order: depth first (it names the node
/// itself), then a Tee arm's `dir`, then text fields, then the container rules for
/// `gap`, `grow` and `children`, which Row and Col declare before `children` and Zone
/// declares after `label`.
fn check_entry(entry: &NodeEntry<'_>, violations: &mut Violations) {
    if entry.depth == DEPTH_MAX + 1 {
        violations.push(
            entry.pointer.clone(),
            VetRule::DepthExceeded,
            format!("depth {} is above {DEPTH_MAX}", entry.depth),
        );
    }

    if let NodeRef::TeeArm(pipe) = entry.node
        && pipe.dir == PipeDir::Vertical
    {
        violations.push(
            entry.pointer.child("dir"),
            VetRule::TeeArmNotHorizontal,
            "Tee arm dir is \"v\", expected \"h\"".to_string(),
        );
    }

    let mut fields = Vec::new();
    push_node_text_fields(entry, &mut fields);
    check_text_fields(&fields, violations);

    match entry.node {
        NodeRef::Node(Node::Row(row)) => {
            check_gap_and_grow(
                entry,
                row.gap,
                row.grow.as_deref(),
                row.children.len(),
                violations,
            );
            check_children_count(entry, row.children.len(), violations);
        }
        NodeRef::Node(Node::Col(col)) => {
            check_gap_and_grow(
                entry,
                col.gap,
                col.grow.as_deref(),
                col.children.len(),
                violations,
            );
            check_children_count(entry, col.children.len(), violations);
        }
        NodeRef::Node(Node::Zone(zone)) => {
            check_children_count(entry, zone.children.len(), violations);
        }
        NodeRef::Node(
            Node::Pcard(_) | Node::Fact(_) | Node::Note(_) | Node::Pipe(_) | Node::Tee(_),
        )
        | NodeRef::TeeArm(_) => {}
    }
}

fn check_gap_and_grow(
    entry: &NodeEntry<'_>,
    gap: Option<u16>,
    grow: Option<&[u16]>,
    child_count: usize,
    violations: &mut Violations,
) {
    if let Some(gap) = gap
        && gap > GAP_MAX_PX
    {
        violations.push(
            entry.pointer.child("gap"),
            VetRule::GapOutOfRange,
            format!("gap {gap} is above {GAP_MAX_PX}"),
        );
    }
    if let Some(weights) = grow {
        let grow_pointer = entry.pointer.child("grow");
        if weights.len() != child_count {
            violations.push(
                grow_pointer.clone(),
                VetRule::GrowLengthMismatch,
                format!(
                    "grow has {} weights for {child_count} children",
                    weights.len()
                ),
            );
        }
        for (index, &weight) in weights.iter().enumerate() {
            if weight > GROW_WEIGHT_MAX {
                violations.push(
                    grow_pointer.index(index),
                    VetRule::GrowOutOfRange,
                    format!("grow weight {weight} is above {GROW_WEIGHT_MAX}"),
                );
            }
        }
    }
}

fn check_children_count(entry: &NodeEntry<'_>, child_count: usize, violations: &mut Violations) {
    let children_pointer = entry.pointer.child("children");
    if child_count == 0 {
        violations.push(
            children_pointer,
            VetRule::ChildrenEmpty,
            "container has no children".to_string(),
        );
    } else if child_count > CHILDREN_MAX {
        violations.push(
            children_pointer,
            VetRule::ChildrenTooMany,
            children_too_many_message(child_count),
        );
    }
}

fn children_too_many_message(child_count: usize) -> String {
    format!("{child_count} children, above {CHILDREN_MAX}")
}

fn is_forbidden_control_character(character: char) -> bool {
    matches!(character, '\u{0000}'..='\u{001F}' | '\u{007F}')
}

fn check_text_fields(fields: &[TextField<'_>], violations: &mut Violations) {
    for field in fields {
        check_text(field, violations);
    }
}

fn check_text(field: &TextField<'_>, violations: &mut Violations) {
    let text = field.text;
    if text.is_empty() {
        violations.push(
            field.pointer.clone(),
            VetRule::TextEmpty,
            "text is empty".to_string(),
        );
        return;
    }
    let scalar_count = text.chars().count();
    if scalar_count > TEXT_SCALARS_MAX {
        violations.push(
            field.pointer.clone(),
            VetRule::TextTooLong,
            format!("text has {scalar_count} scalar values, above {TEXT_SCALARS_MAX}"),
        );
    }
    if let Some(character) = text
        .chars()
        .find(|&character| is_forbidden_control_character(character))
    {
        violations.push(
            field.pointer.clone(),
            VetRule::TextControlCharacter,
            format!(
                "text contains control character U+{:04X}",
                u32::from(character)
            ),
        );
    }
    if text.starts_with(char::is_whitespace) || text.ends_with(char::is_whitespace) {
        violations.push(
            field.pointer.clone(),
            VetRule::TextUntrimmed,
            "text starts or ends with whitespace".to_string(),
        );
    }
}
