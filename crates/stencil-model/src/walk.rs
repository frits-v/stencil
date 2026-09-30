use crate::document::{Link, Node, Page, Pipe, TeeArm};
use crate::pointer::NodePointer;
use crate::{LEGEND_ENTRIES_MAX, LINKS_MAX, NODES_MAX, TEXT_BODY_LINES_MAX};

#[derive(Debug, Clone, PartialEq)]
pub struct NodeEntry<'a> {
    pub pointer: NodePointer,
    /// The enclosing node: `/body` for body elements, the container for its children and
    /// the Tee for its arms.
    pub parent: Option<NodePointer>,
    /// Section 1.3 depth: body elements have depth 1, and the children of a Row, Col or
    /// Zone and the arms of a Tee have their parent's depth plus 1.
    pub depth: usize,
    pub node: NodeRef<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeRef<'a> {
    Node(&'a Node),
    TeeArm(&'a Pipe),
}

impl NodeRef<'_> {
    /// The serialized `tag` value; a Tee arm is written with `"tag": "Pipe"`.
    pub fn tag_name(&self) -> &'static str {
        match self {
            NodeRef::Node(node) => node.tag_name(),
            NodeRef::TeeArm(_) => "Pipe",
        }
    }
}

impl<'a> NodeRef<'a> {
    /// The node's `id`, when it has one; a Tee arm carries the Pipe `id` field.
    pub fn id(self) -> Option<&'a str> {
        match self {
            NodeRef::Node(node) => node.id(),
            NodeRef::TeeArm(pipe) => pipe.id.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField<'a> {
    pub pointer: NodePointer,
    pub text: &'a str,
}

/// Pre-order walk of body nodes and Tee arms, each Tee's arms directly after the Tee.
/// This is the body portion of the section 4.4 geometry order. Stops after
/// NODES_MAX + 1 entries (section 1.3).
pub fn body_nodes(page: &Page) -> Vec<NodeEntry<'_>> {
    let body_pointer = NodePointer::root().child("body");
    let mut pending: Vec<NodeEntry<'_>> = page
        .body
        .iter()
        .enumerate()
        .take(NODES_MAX + 1)
        .rev()
        .map(|(index, node)| NodeEntry {
            pointer: body_pointer.index(index),
            parent: Some(body_pointer.clone()),
            depth: 1,
            node: NodeRef::Node(node),
        })
        .collect();

    let mut entries = Vec::new();
    for _ in 0..=NODES_MAX {
        let Some(entry) = pending.pop() else {
            break;
        };
        push_children_reversed(&entry, &mut pending);
        entries.push(entry);
    }
    debug_assert!(entries.len() <= NODES_MAX + 1);
    entries
}

fn push_children_reversed<'a>(entry: &NodeEntry<'a>, pending: &mut Vec<NodeEntry<'a>>) {
    let child_depth = entry.depth.saturating_add(1);
    let push_nodes = |pending: &mut Vec<NodeEntry<'a>>, children: &'a [Node]| {
        let children_pointer = entry.pointer.child("children");
        // A child past index NODES_MAX cannot be emitted before the walk stops.
        for (index, child) in children.iter().enumerate().take(NODES_MAX + 1).rev() {
            pending.push(NodeEntry {
                pointer: children_pointer.index(index),
                parent: Some(entry.pointer.clone()),
                depth: child_depth,
                node: NodeRef::Node(child),
            });
        }
    };
    match entry.node {
        NodeRef::Node(Node::Row(row)) => push_nodes(pending, &row.children),
        NodeRef::Node(Node::Col(col)) => push_nodes(pending, &col.children),
        NodeRef::Node(Node::Zone(zone)) => push_nodes(pending, &zone.children),
        NodeRef::Node(Node::Tee(tee)) => {
            let arms_pointer = entry.pointer.child("arms");
            for (index, arm) in tee.arms.iter().enumerate().rev() {
                let TeeArm::Pipe(pipe) = arm;
                pending.push(NodeEntry {
                    pointer: arms_pointer.index(index),
                    parent: Some(entry.pointer.clone()),
                    depth: child_depth,
                    node: NodeRef::TeeArm(pipe),
                });
            }
        }
        NodeRef::Node(
            Node::Pcard(_)
            | Node::Fact(_)
            | Node::Note(_)
            | Node::Pipe(_)
            | Node::Text(_)
            | Node::Callout(_)
            | Node::Frame(_),
        )
        | NodeRef::TeeArm(_) => {}
    }
}

/// Every authored text value with its pointer, in document order.
pub fn text_fields(page: &Page) -> Vec<TextField<'_>> {
    let mut fields = Vec::new();
    push_page_head_text_fields(page, &mut fields);
    for entry in body_nodes(page) {
        push_node_text_fields(&entry, &mut fields);
    }
    push_legend_text_fields(page, &mut fields);
    push_link_text_fields(page, &mut fields);
    fields
}

/// `title`, `kicker`, `lede` and `foot`, the page text fields declared before `body`.
pub(crate) fn push_page_head_text_fields<'a>(page: &'a Page, fields: &mut Vec<TextField<'a>>) {
    let root = NodePointer::root();
    fields.push(TextField {
        pointer: root.child("title"),
        text: &page.title,
    });
    fields.push(TextField {
        pointer: root.child("kicker"),
        text: &page.kicker,
    });
    fields.push(TextField {
        pointer: root.child("lede"),
        text: &page.lede,
    });
    if let Some(foot) = &page.foot {
        fields.push(TextField {
            pointer: root.child("foot"),
            text: foot,
        });
    }
}

/// The first LEGEND_ENTRIES_MAX + 1 legend texts: one past the limit is enough for vet to
/// report `legend-too-long` (section 1.3).
pub(crate) fn push_legend_text_fields<'a>(page: &'a Page, fields: &mut Vec<TextField<'a>>) {
    let legend_pointer = NodePointer::root().child("legend");
    for (index, entry) in page.legend.iter().enumerate().take(LEGEND_ENTRIES_MAX + 1) {
        fields.push(TextField {
            pointer: legend_pointer.index(index).child("text"),
            text: &entry.text,
        });
    }
}

/// `label` and `sub` of the first LINKS_MAX + 1 links, the bound vet reports
/// `links-too-many` at. `from` and `to` are identifiers, not text.
pub(crate) fn push_link_text_fields<'a>(page: &'a Page, fields: &mut Vec<TextField<'a>>) {
    let links_pointer = NodePointer::root().child("links");
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX + 1) {
        push_one_link_text_fields(link, &links_pointer.index(index), fields);
    }
}

pub(crate) fn push_one_link_text_fields<'a>(
    link: &'a Link,
    link_pointer: &NodePointer,
    fields: &mut Vec<TextField<'a>>,
) {
    if let Some(label) = &link.label {
        fields.push(TextField {
            pointer: link_pointer.child("label"),
            text: label,
        });
    }
    if let Some(sub) = &link.sub {
        fields.push(TextField {
            pointer: link_pointer.child("sub"),
            text: sub,
        });
    }
}

/// The text fields a node itself holds, in struct declaration order. Children are separate
/// entries of the walk.
pub(crate) fn push_node_text_fields<'a>(entry: &NodeEntry<'a>, fields: &mut Vec<TextField<'a>>) {
    let mut push = |token: &str, text: &'a str| {
        fields.push(TextField {
            pointer: entry.pointer.child(token),
            text,
        });
    };
    match entry.node {
        NodeRef::Node(Node::Row(_) | Node::Col(_)) => {}
        NodeRef::Node(Node::Zone(zone)) => push("label", &zone.label),
        NodeRef::Node(Node::Pcard(pcard)) => {
            push("fn", &pcard.function_name);
            if let Some(product_name) = &pcard.product_name {
                push("pn", product_name);
            }
            if let Some(fact) = &pcard.fact {
                push("fact", fact);
            }
            if let Some(ask) = &pcard.ask {
                push("ask", ask);
            }
        }
        NodeRef::Node(Node::Fact(fact)) => push("text", &fact.text),
        NodeRef::Node(Node::Note(note)) => push("text", &note.text),
        NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => {
            push("label", &pipe.label);
            if let Some(sub) = &pipe.sub {
                push("sub", sub);
            }
        }
        NodeRef::Node(Node::Tee(tee)) => push("hub", &tee.hub),
        NodeRef::Node(Node::Text(text)) => {
            if let Some(heading) = &text.heading {
                push("heading", heading);
            }
            let body_pointer = entry.pointer.child("body");
            // One line past the limit is enough for vet to report `text-body-too-long`.
            for (index, line) in text.body.iter().enumerate().take(TEXT_BODY_LINES_MAX + 1) {
                fields.push(TextField {
                    pointer: body_pointer.index(index),
                    text: line,
                });
            }
        }
        NodeRef::Node(Node::Callout(callout)) => {
            if let Some(title) = &callout.title {
                push("title", title);
            }
            push("text", &callout.text);
        }
        NodeRef::Node(Node::Frame(frame)) => push("label", &frame.label),
    }
}
