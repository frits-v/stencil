use std::collections::{BTreeMap, BTreeSet};

use crate::document::{
    BUILTIN_GRAMMARS, BUILTIN_THEMES, FACTS_MAX, LANES_MAX, Link, Node, Page, PipeDir, Projection,
    TINT_SLOTS, Text, is_valid_id,
};
use crate::grammar::{Grammar, IconPack, PAGE_PARENT};
use crate::pointer::NodePointer;
use crate::walk::{
    NodeEntry, NodeRef, TextField, body_nodes, push_node_text_fields, push_one_link_text_fields,
    push_page_head_text_fields,
};
use crate::{
    CHILDREN_MAX, DEPTH_MAX, FRAME_HEIGHT_MAX, FRAME_HEIGHT_MIN, GAP_MAX_PX, GROW_WEIGHT_MAX,
    ID_PATTERN, LEGEND_ENTRIES_MAX, LINK_CORNER_MAX_PX, LINK_VIA_MAX, LINKS_MAX, NODES_MAX,
    PAGE_WIDTH_MAX, PAGE_WIDTH_MIN, TEXT_BODY_LINES_MAX, TEXT_SCALARS_MAX,
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
    TintOutOfRange,
    GrammarUnknown,
    ThemeUnknown,
    KindUnknown,
    KindParentNotAllowed,
    IconOutsidePack,
    PipeTargetUnknown,
    PipeTargetsEqual,
    PipeTargetOnTeeArm,
    FactsTooMany,
    LanesTooMany,
    LinkOrderOutsideLanes,
    LinkOrderDuplicate,
    LanesInIso,
    CornerOutOfRange,
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
            VetRule::TintOutOfRange => "tint-out-of-range",
            VetRule::GrammarUnknown => "grammar-unknown",
            VetRule::ThemeUnknown => "theme-unknown",
            VetRule::KindUnknown => "kind-unknown",
            VetRule::KindParentNotAllowed => "kind-parent-not-allowed",
            VetRule::IconOutsidePack => "icon-outside-pack",
            VetRule::PipeTargetUnknown => "pipe-target-unknown",
            VetRule::PipeTargetsEqual => "pipe-targets-equal",
            VetRule::PipeTargetOnTeeArm => "pipe-target-on-tee-arm",
            VetRule::FactsTooMany => "facts-too-many",
            VetRule::LanesTooMany => "lanes-too-many",
            VetRule::LinkOrderOutsideLanes => "link-order-outside-lanes",
            VetRule::LinkOrderDuplicate => "link-order-duplicate",
            VetRule::LanesInIso => "lanes-in-iso",
            VetRule::CornerOutOfRange => "corner-out-of-range",
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

/// The bound of `corner` on a page and on a grammar's line kind.
pub fn is_corner_in_range(corner: f32) -> bool {
    corner.is_finite() && (0.0..=LINK_CORNER_MAX_PX).contains(&corner)
}

/// True when `reference` is a built-in grammar name or a string ending in `.json`, the
/// condition of `grammar-unknown`.
pub fn is_grammar_reference(reference: &str) -> bool {
    BUILTIN_GRAMMARS.contains(&reference) || reference.ends_with(".json")
}

/// True when `reference` is a built-in theme name or a string ending in `.json`, the
/// condition of `theme-unknown`.
pub fn is_theme_reference(reference: &str) -> bool {
    BUILTIN_THEMES.contains(&reference) || reference.ends_with(".json")
}

/// All vet violations against `grammar`, the grammar `page.grammar` resolved to, in
/// document order (section 4.2). Empty means valid.
pub fn validate_page(page: &Page, grammar: &Grammar) -> Vec<Violation> {
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

    if let Some(reference) = &page.grammar
        && !is_grammar_reference(reference)
    {
        violations.push(
            root.child("grammar"),
            VetRule::GrammarUnknown,
            format!("grammar \"{reference}\" is neither a built-in grammar nor a .json path"),
        );
    }

    if let Some(reference) = &page.theme
        && !is_theme_reference(reference)
    {
        violations.push(
            root.child("theme"),
            VetRule::ThemeUnknown,
            format!("theme \"{reference}\" is neither a built-in theme nor a .json path"),
        );
    }

    let entries = body_nodes(page);
    let has_lanes = entries
        .iter()
        .any(|entry| matches!(entry.node, NodeRef::Node(Node::Lanes(_))));
    if has_lanes && page.projection == Projection::Iso {
        violations.push(
            root.child("projection"),
            VetRule::LanesInIso,
            "a page with Lanes cannot be drawn in iso".to_string(),
        );
    }

    if let Some(corner) = page.corner
        && !is_corner_in_range(corner)
    {
        violations.push(
            root.child("corner"),
            VetRule::CornerOutOfRange,
            format!("corner {corner} is outside 0 to {LINK_CORNER_MAX_PX}"),
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
    // The nearest Box kind above each walked container, keyed by its pointer, so a child
    // reads its parent's entry. Row, Col and Lanes pass their parent's value through.
    let mut box_context: BTreeMap<NodePointer, &str> = BTreeMap::new();
    // A pipe target may name a node later in document order, so targets are checked
    // against every well-formed id of the walk.
    let all_ids: BTreeSet<&str> = entries
        .iter()
        .filter_map(|entry| entry.node.id())
        .filter(|id| is_valid_id(id))
        .collect();
    for entry in &entries {
        let context = enclosing_box_kind(entry, &box_context);
        check_entry(entry, context, grammar, &mut ids, &mut violations);
        check_pipe_targets(entry, &all_ids, &mut violations);
        match entry.node {
            // A Box of an unknown kind is reported once and is transparent for its
            // children, so they are not reported again against a kind that does not exist.
            NodeRef::Node(Node::Box(box_node)) if grammar.container(&box_node.kind).is_some() => {
                box_context.insert(entry.pointer.clone(), box_node.kind.as_str());
            }
            NodeRef::Node(Node::Box(_)) => {
                box_context.insert(entry.pointer.clone(), context);
            }
            NodeRef::Node(Node::Row(_) | Node::Col(_) | Node::Lanes(_)) => {
                box_context.insert(entry.pointer.clone(), context);
            }
            _ => {}
        }
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
    let legend_pointer = root.child("legend");
    for (index, entry) in page.legend.iter().enumerate().take(LEGEND_ENTRIES_MAX + 1) {
        check_tint(entry.tint, &legend_pointer.index(index), &mut violations);
        check_text(
            &TextField {
                pointer: legend_pointer.index(index).child("text"),
                text: &entry.text,
            },
            &mut violations,
        );
    }

    let lane_heads = lane_heads(&entries);
    check_links(page, &ids, &lane_heads, &mut violations);

    violations.0
}

/// `pipe-target-on-tee-arm` for a Tee arm with `from` or `to`; for a Pipe node,
/// `pipe-target-unknown` for a `from` or `to` that names no node id and `pipe-targets-equal`
/// when both name the same id. `from` and `to` follow the Pipe's other fields.
fn check_pipe_targets(
    entry: &NodeEntry<'_>,
    all_ids: &BTreeSet<&str>,
    violations: &mut Violations,
) {
    match entry.node {
        NodeRef::TeeArm(arm) => {
            for (field, target) in [("from", &arm.from), ("to", &arm.to)] {
                if target.is_some() {
                    violations.push(
                        entry.pointer.child(field),
                        VetRule::PipeTargetOnTeeArm,
                        "a Tee arm cannot name a target".to_string(),
                    );
                }
            }
        }
        NodeRef::Node(Node::Pipe(pipe)) => {
            for (field, target) in [("from", &pipe.from), ("to", &pipe.to)] {
                if let Some(target) = target
                    && !all_ids.contains(target.as_str())
                {
                    violations.push(
                        entry.pointer.child(field),
                        VetRule::PipeTargetUnknown,
                        format!("pipe target \"{target}\" names no node id"),
                    );
                }
            }
            if let (Some(from), Some(to)) = (&pipe.from, &pipe.to)
                && from == to
            {
                violations.push(
                    entry.pointer.child("to"),
                    VetRule::PipeTargetsEqual,
                    format!("pipe from and to both name \"{to}\""),
                );
            }
        }
        NodeRef::Node(_) => {}
    }
}

/// The Lanes node each lane head belongs to, keyed by the head's id. A lane head is a
/// direct child of a Lanes node (section 13.6); a node inside a head is not one.
fn lane_heads<'a>(entries: &[NodeEntry<'a>]) -> BTreeMap<&'a str, NodePointer> {
    let lanes_pointers: BTreeSet<&NodePointer> = entries
        .iter()
        .filter(|entry| matches!(entry.node, NodeRef::Node(Node::Lanes(_))))
        .map(|entry| &entry.pointer)
        .collect();
    let mut heads = BTreeMap::new();
    for entry in entries {
        let (Some(parent), Some(id)) = (&entry.parent, entry.node.id()) else {
            continue;
        };
        if lanes_pointers.contains(parent) {
            heads.entry(id).or_insert_with(|| parent.clone());
        }
    }
    heads
}

/// `link-order-outside-lanes` and `link-order-duplicate` for a link with `order`. `orders`
/// holds the first link index of each (Lanes node, order) pair seen so far.
fn check_link_order<'a>(
    (link_index, link): (usize, &Link),
    link_pointer: &NodePointer,
    lane_heads: &'a BTreeMap<&str, NodePointer>,
    orders: &mut BTreeMap<(&'a NodePointer, u16), usize>,
    violations: &mut Violations,
) {
    let Some(order) = link.order else {
        return;
    };
    let order_pointer = link_pointer.child("order");
    let from_lanes = lane_heads.get(link.from.as_str());
    let to_lanes = lane_heads.get(link.to.as_str());
    let lanes = match (from_lanes, to_lanes) {
        (Some(from_lanes), Some(to_lanes)) if from_lanes == to_lanes && link.from != link.to => {
            from_lanes
        }
        _ => {
            violations.push(
                order_pointer,
                VetRule::LinkOrderOutsideLanes,
                "an ordered link joins two lanes of one Lanes node".to_string(),
            );
            return;
        }
    };
    match orders.get(&(lanes, order)) {
        Some(&first_index) => violations.push(
            order_pointer,
            VetRule::LinkOrderDuplicate,
            format!(
                "order {order} is already used by {}",
                NodePointer::root().child("links").index(first_index)
            ),
        ),
        None => {
            orders.insert((lanes, order), link_index);
        }
    }
}

/// `links-too-many` once at `/links`, then each of the first LINKS_MAX + 1 links in field
/// order: the link itself (`link-self`), `from`, `to`, the text fields, `via`, then `order`.
fn check_links(
    page: &Page,
    ids: &BTreeMap<&str, NodePointer>,
    lane_heads: &BTreeMap<&str, NodePointer>,
    violations: &mut Violations,
) {
    let links_pointer = NodePointer::root().child("links");
    if page.links.len() > LINKS_MAX {
        violations.push(
            links_pointer.clone(),
            VetRule::LinksTooMany,
            format!("links has {} entries, above {LINKS_MAX}", page.links.len()),
        );
    }
    let canvas_width = page.width as f32 + CANVAS_PADDING_TOTAL_PX;
    let mut orders = BTreeMap::new();
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
        check_tint(link.tint, &link_pointer, violations);
        let mut text_fields = Vec::new();
        push_one_link_text_fields(link, &link_pointer, &mut text_fields);
        check_text_fields(&text_fields, violations);
        check_via(link, &link_pointer, canvas_width, violations);
        check_link_order(
            (index, link),
            &link_pointer,
            lane_heads,
            &mut orders,
            violations,
        );
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
    enclosing_box: &str,
    grammar: &Grammar,
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

    check_kind_and_tint(entry, enclosing_box, grammar, violations);

    let mut fields = Vec::new();
    push_node_text_fields(entry, &mut fields);
    if let NodeRef::Node(Node::Item(item)) = entry.node {
        let head_count = 1 + usize::from(item.subtitle.is_some());
        let (head_fields, fact_fields) = fields
            .split_at_checked(head_count)
            .unwrap_or((&fields, &[]));
        check_text_fields(head_fields, violations);
        if item.facts.len() > FACTS_MAX {
            violations.push(
                entry.pointer.child("facts"),
                VetRule::FactsTooMany,
                format!("{} facts, above {FACTS_MAX}", item.facts.len()),
            );
        }
        check_text_fields(fact_fields, violations);
    } else if let NodeRef::Node(Node::Text(text)) = entry.node {
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
        NodeRef::Node(Node::Lanes(lanes)) => {
            check_gap_and_grow(entry, lanes.gap, None, lanes.children.len(), violations);
            check_children_count(entry, lanes.children.len(), violations);
            if lanes.children.len() > LANES_MAX {
                violations.push(
                    entry.pointer.child("children"),
                    VetRule::LanesTooMany,
                    format!("{} lanes, above {LANES_MAX}", lanes.children.len()),
                );
            }
        }
        NodeRef::Node(Node::Box(box_node)) => {
            check_children_count(entry, box_node.children.len(), violations);
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
            Node::Item(_)
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

/// The nearest Box kind above `entry`, or `page` when there is none.
fn enclosing_box_kind<'a>(
    entry: &NodeEntry<'_>,
    box_context: &BTreeMap<NodePointer, &'a str>,
) -> &'a str {
    entry
        .parent
        .as_ref()
        .and_then(|parent| box_context.get(parent).copied())
        .unwrap_or(PAGE_PARENT)
}

/// `kind-unknown`, `kind-parent-not-allowed` and `icon-outside-pack` for a Box or Item, and
/// `tint-out-of-range` for every tint carrier, in field order.
fn check_kind_and_tint(
    entry: &NodeEntry<'_>,
    enclosing_box: &str,
    grammar: &Grammar,
    violations: &mut Violations,
) {
    let kind_pointer = entry.pointer.child("kind");
    match entry.node {
        NodeRef::Node(Node::Box(box_node)) => {
            match grammar.container(&box_node.kind) {
                None => violations.push(
                    kind_pointer,
                    VetRule::KindUnknown,
                    format!(
                        "kind \"{}\" is not a container kind of grammar {}",
                        box_node.kind, grammar.name
                    ),
                ),
                Some(container) => {
                    check_parent(
                        &container.parents,
                        &box_node.kind,
                        enclosing_box,
                        kind_pointer,
                        violations,
                    );
                }
            }
            check_tint(box_node.tint, &entry.pointer, violations);
        }
        NodeRef::Node(Node::Item(item)) => match grammar.item(&item.kind) {
            None => violations.push(
                kind_pointer,
                VetRule::KindUnknown,
                format!(
                    "kind \"{}\" is not an item kind of grammar {}",
                    item.kind, grammar.name
                ),
            ),
            Some(item_kind) => {
                check_parent(
                    &item_kind.parents,
                    &item.kind,
                    enclosing_box,
                    kind_pointer,
                    violations,
                );
                if item.icon.is_some() && item_kind.icons == IconPack::None {
                    violations.push(
                        entry.pointer.child("icon"),
                        VetRule::IconOutsidePack,
                        format!("item kind {} takes no icon", item.kind),
                    );
                }
            }
        },
        NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => {
            check_tint(pipe.tint, &entry.pointer, violations);
        }
        NodeRef::Node(Node::Tee(tee)) => check_tint(tee.tint, &entry.pointer, violations),
        NodeRef::Node(
            Node::Row(_)
            | Node::Col(_)
            | Node::Lanes(_)
            | Node::Fact(_)
            | Node::Note(_)
            | Node::Text(_)
            | Node::Callout(_)
            | Node::Frame(_),
        ) => {}
    }
}

fn check_parent(
    parents: &[String],
    kind: &str,
    enclosing_box: &str,
    kind_pointer: NodePointer,
    violations: &mut Violations,
) {
    if !parents.iter().any(|parent| parent == enclosing_box) {
        violations.push(
            kind_pointer,
            VetRule::KindParentNotAllowed,
            format!("{kind} cannot sit in {enclosing_box}"),
        );
    }
}

/// `tint-out-of-range` at `<holder>/tint`.
fn check_tint(tint: Option<u8>, holder: &NodePointer, violations: &mut Violations) {
    if let Some(tint) = tint
        && !(1..=TINT_SLOTS).contains(&tint)
    {
        violations.push(
            holder.child("tint"),
            VetRule::TintOutOfRange,
            format!("tint {tint} is outside 1 to {TINT_SLOTS}"),
        );
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
