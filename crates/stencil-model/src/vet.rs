use std::collections::BTreeMap;

use crate::document::{Link, Node, Page, PipeDir, Text, is_valid_id};
use crate::pointer::NodePointer;
use crate::walk::{
    NodeEntry, NodeRef, TextField, body_nodes, push_legend_text_fields, push_node_text_fields,
    push_one_link_text_fields, push_page_head_text_fields,
};
use crate::{
    CHILDREN_MAX, DEPTH_MAX, FRAME_HEIGHT_MAX, FRAME_HEIGHT_MIN, GAP_MAX_PX, GROW_WEIGHT_MAX,
    ID_PATTERN, LEGEND_ENTRIES_MAX, LINK_VIA_MAX, LINKS_MAX, NODES_MAX, PAGE_WIDTH_MAX,
    PAGE_WIDTH_MIN, TEXT_BODY_LINES_MAX, TEXT_SCALARS_MAX,
};

/// Horizontal padding of the page root on both sides (section 2.2): the canvas is
/// `width + 40` wide.
const CANVAS_PADDING_TOTAL_PX: f32 = 40.0;

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
    IdMalformed,
    IdDuplicate,
    TextBodyEmpty,
    TextBodyTooLong,
    FrameHeightOutOfRange,
    LinksTooMany,
    LinkSelf,
    LinkUnknownId,
    LinkViaTooMany,
    LinkViaOutside,
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
            VetRule::IdMalformed => "id-malformed",
            VetRule::IdDuplicate => "id-duplicate",
            VetRule::TextBodyEmpty => "text-body-empty",
            VetRule::TextBodyTooLong => "text-body-too-long",
            VetRule::FrameHeightOutOfRange => "frame-height-out-of-range",
            VetRule::LinksTooMany => "links-too-many",
            VetRule::LinkSelf => "link-self",
            VetRule::LinkUnknownId => "link-unknown-id",
            VetRule::LinkViaTooMany => "link-via-too-many",
            VetRule::LinkViaOutside => "link-via-outside",
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
    // Well-formed ids of the walked nodes, each with the pointer of the first node holding
    // it. Bounded by the walk, like every other per-node rule.
    let mut ids: BTreeMap<&str, NodePointer> = BTreeMap::new();
    for entry in &entries {
        check_entry(entry, &mut ids, &mut violations);
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

    check_links(page, &ids, &mut violations);

    violations.0
}

/// `links-too-many` once at `/links`, then each of the first LINKS_MAX + 1 links in field
/// order: the link itself (`link-self`), `from`, `to`, the text fields, then `via`.
fn check_links(page: &Page, ids: &BTreeMap<&str, NodePointer>, violations: &mut Violations) {
    let links_pointer = NodePointer::root().child("links");
    if page.links.len() > LINKS_MAX {
        violations.push(
            links_pointer.clone(),
            VetRule::LinksTooMany,
            format!("links has {} entries, above {LINKS_MAX}", page.links.len()),
        );
    }
    let canvas_width = page.width as f32 + CANVAS_PADDING_TOTAL_PX;
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX + 1) {
        let link_pointer = links_pointer.index(index);
        if link.from == link.to {
            violations.push(
                link_pointer.clone(),
                VetRule::LinkSelf,
                "from and to name the same node".to_string(),
            );
        }
        for (field, value) in [("from", &link.from), ("to", &link.to)] {
            if !ids.contains_key(value.as_str()) {
                violations.push(
                    link_pointer.child(field),
                    VetRule::LinkUnknownId,
                    unknown_id_message(value),
                );
            }
        }
        let mut text_fields = Vec::new();
        push_one_link_text_fields(link, &link_pointer, &mut text_fields);
        check_text_fields(&text_fields, violations);
        check_via(link, &link_pointer, canvas_width, violations);
    }
}

fn unknown_id_message(value: &str) -> String {
    if is_valid_id(value) {
        format!("no node has id \"{value}\"")
    } else {
        format!("no node has this id; it does not match {ID_PATTERN}")
    }
}

/// `via` above LINK_VIA_MAX points, then each of the first LINK_VIA_MAX + 1 points outside
/// the canvas. Layout alone knows the canvas height, so vet bounds y below only.
fn check_via(
    link: &Link,
    link_pointer: &NodePointer,
    canvas_width: f32,
    violations: &mut Violations,
) {
    let via_pointer = link_pointer.child("via");
    if link.via.len() > LINK_VIA_MAX {
        violations.push(
            via_pointer.clone(),
            VetRule::LinkViaTooMany,
            format!("via has {} points, above {LINK_VIA_MAX}", link.via.len()),
        );
    }
    for (index, point) in link.via.iter().enumerate().take(LINK_VIA_MAX + 1) {
        let x_inside = point.x.is_finite() && (0.0..=canvas_width).contains(&point.x);
        let y_inside = point.y.is_finite() && point.y >= 0.0;
        if !(x_inside && y_inside) {
            violations.push(
                via_pointer.index(index),
                VetRule::LinkViaOutside,
                format!(
                    "via point ({}, {}) is outside the page: x 0 to {canvas_width}, y 0 or more",
                    point.x, point.y
                ),
            );
        }
    }
}

/// A node's own violations in struct declaration order: depth first (it names the node
/// itself), then `id`, then a Tee arm's `dir`, then text fields, then the container rules
/// for `gap`, `grow` and `children`, which Row and Col declare before `children` and Zone
/// declares after `label`. A Text reports its `body` length between its heading and its
/// lines, and a Frame its `height` after its label.
fn check_entry<'a>(
    entry: &NodeEntry<'a>,
    ids: &mut BTreeMap<&'a str, NodePointer>,
    violations: &mut Violations,
) {
    if entry.depth == DEPTH_MAX + 1 {
        violations.push(
            entry.pointer.clone(),
            VetRule::DepthExceeded,
            format!("depth {} is above {DEPTH_MAX}", entry.depth),
        );
    }

    if let Some(id) = entry.node.id() {
        check_id(id, entry, ids, violations);
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
    if let NodeRef::Node(Node::Text(text)) = entry.node {
        let heading_count = usize::from(text.heading.is_some());
        let (heading_fields, line_fields) = fields
            .split_at_checked(heading_count)
            .unwrap_or((&[], &fields));
        check_text_fields(heading_fields, violations);
        check_text_body_length(entry, text, violations);
        check_text_fields(line_fields, violations);
    } else {
        check_text_fields(&fields, violations);
    }

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
        NodeRef::Node(Node::Frame(frame)) => {
            if !(FRAME_HEIGHT_MIN..=FRAME_HEIGHT_MAX).contains(&frame.height) {
                violations.push(
                    entry.pointer.child("height"),
                    VetRule::FrameHeightOutOfRange,
                    format!(
                        "height {} is outside {FRAME_HEIGHT_MIN} to {FRAME_HEIGHT_MAX}",
                        frame.height
                    ),
                );
            }
        }
        NodeRef::Node(
            Node::Pcard(_)
            | Node::Fact(_)
            | Node::Note(_)
            | Node::Pipe(_)
            | Node::Tee(_)
            | Node::Text(_)
            | Node::Callout(_),
        )
        | NodeRef::TeeArm(_) => {}
    }
}

fn check_id<'a>(
    id: &'a str,
    entry: &NodeEntry<'a>,
    ids: &mut BTreeMap<&'a str, NodePointer>,
    violations: &mut Violations,
) {
    let id_pointer = entry.pointer.child("id");
    if !is_valid_id(id) {
        violations.push(
            id_pointer,
            VetRule::IdMalformed,
            format!("id does not match {ID_PATTERN}"),
        );
        return;
    }
    if let Some(first_pointer) = ids.get(id) {
        violations.push(
            id_pointer,
            VetRule::IdDuplicate,
            format!("id \"{id}\" is already used at {first_pointer}"),
        );
    } else {
        ids.insert(id, entry.pointer.clone());
    }
}

fn check_text_body_length(entry: &NodeEntry<'_>, text: &Text, violations: &mut Violations) {
    let body_pointer = entry.pointer.child("body");
    if text.body.is_empty() {
        violations.push(
            body_pointer,
            VetRule::TextBodyEmpty,
            "body has no lines".to_string(),
        );
    } else if text.body.len() > TEXT_BODY_LINES_MAX {
        violations.push(
            body_pointer,
            VetRule::TextBodyTooLong,
            format!(
                "body has {} lines, above {TEXT_BODY_LINES_MAX}",
                text.body.len()
            ),
        );
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
        for (index, &weight) in weights.iter().enumerate().take(CHILDREN_MAX + 1) {
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
