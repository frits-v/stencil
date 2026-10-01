//! Lanes (section 13.6): the message rows read from the page before layout, so the band
//! can be sized, and the lifelines added once the heads have their boxes.

use std::collections::{BTreeMap, BTreeSet};

use stencil_model::pointer::NodePointer;
use stencil_model::text::TextMeasurer;
use stencil_model::{LANE_ROW_MIN_PX, LINKS_MAX, Node, NodeRef, Page, body_nodes};

use crate::route::measure_tag;
use crate::{BoxRect, LayoutError, NodeTag, PageGeometry, Part, PartName};

/// Space kept above and below a message tag inside its row.
const LANE_ROW_TAG_MARGIN_PX: f32 = 8.0;

/// One message: the Lanes node it belongs to and its row in that node's band.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MessageRow {
    pub lanes: NodePointer,
    /// Distance from the band top to the row top.
    pub offset: f32,
    pub height: f32,
}

impl MessageRow {
    /// The y of the message line, the middle of its row, for a band whose top is `band_top`.
    pub fn line_y(&self, band_top: f32) -> f32 {
        band_top + self.offset + self.height / 2.0
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LanesPlan {
    band_heights: BTreeMap<NodePointer, f32>,
    messages: BTreeMap<usize, MessageRow>,
}

impl LanesPlan {
    /// The band height of the Lanes node at `pointer`: the sum of its message rows, 0 with
    /// no message.
    pub fn band_height(&self, pointer: &NodePointer) -> f32 {
        self.band_heights.get(pointer).copied().unwrap_or(0.0)
    }

    /// The row of the link at `link_index` when it is a message.
    pub fn message(&self, link_index: usize) -> Option<&MessageRow> {
        self.messages.get(&link_index)
    }
}

struct PendingMessage {
    order: u16,
    link_index: usize,
    height: f32,
}

/// Every message of every Lanes node of a vetted page, sorted by `order` then by link index,
/// each row `max(LANE_ROW_MIN_PX, tag height + 8)` tall, or LANE_ROW_MIN_PX without a tag.
pub(crate) fn plan_lanes(
    page: &Page,
    measurer: &mut dyn TextMeasurer,
) -> Result<LanesPlan, LayoutError> {
    let entries = body_nodes(page);
    let lanes_pointers: BTreeSet<&NodePointer> = entries
        .iter()
        .filter(|entry| matches!(entry.node, NodeRef::Node(Node::Lanes(_))))
        .map(|entry| &entry.pointer)
        .collect();
    let mut heads: BTreeMap<&str, &NodePointer> = BTreeMap::new();
    for entry in &entries {
        let (Some(parent), Some(id)) = (&entry.parent, entry.node.id()) else {
            continue;
        };
        if let Some(&lanes) = lanes_pointers.get(parent) {
            heads.entry(id).or_insert(lanes);
        }
    }

    let links_pointer = NodePointer::root().child("links");
    let mut pending: BTreeMap<&NodePointer, Vec<PendingMessage>> = BTreeMap::new();
    for (link_index, link) in page.links.iter().enumerate().take(LINKS_MAX) {
        let Some(order) = link.order else {
            continue;
        };
        let (Some(&from_lanes), Some(&to_lanes)) =
            (heads.get(link.from.as_str()), heads.get(link.to.as_str()))
        else {
            continue;
        };
        if from_lanes != to_lanes || link.from == link.to {
            continue;
        }
        let tag = measure_tag(page, link, &links_pointer.index(link_index), measurer)?;
        let height = tag.map_or(LANE_ROW_MIN_PX, |tag| {
            LANE_ROW_MIN_PX.max(tag.height + LANE_ROW_TAG_MARGIN_PX)
        });
        pending.entry(from_lanes).or_default().push(PendingMessage {
            order,
            link_index,
            height,
        });
    }

    let mut plan = LanesPlan::default();
    for lanes in lanes_pointers {
        let mut messages = pending.remove(lanes).unwrap_or_default();
        messages.sort_by_key(|message| (message.order, message.link_index));
        let mut offset = 0.0_f32;
        for message in &messages {
            plan.messages.insert(
                message.link_index,
                MessageRow {
                    lanes: lanes.clone(),
                    offset,
                    height: message.height,
                },
            );
            offset += message.height;
        }
        plan.band_heights.insert(lanes.clone(), offset);
    }
    Ok(plan)
}

/// Appends one Lifeline part per head to every Lanes node, in head order: a vertical line
/// of width 0 from the head's bottom center to the band's bottom.
pub(crate) fn add_lifelines(geometry: &mut PageGeometry) {
    let lanes_nodes: Vec<usize> = geometry
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.tag == NodeTag::Lanes)
        .map(|(index, _)| index)
        .collect();
    for lanes_index in lanes_nodes {
        let Some(band_bottom) = geometry
            .nodes
            .get(lanes_index)
            .and_then(|node| node.part(PartName::Band))
            .map(|band| band.bounds.bottom())
        else {
            continue;
        };
        let lifelines: Vec<Part> = geometry
            .children(lanes_index)
            .into_iter()
            .filter_map(|head_index| geometry.nodes.get(head_index))
            .map(|head| Part {
                name: PartName::Lifeline,
                bounds: BoxRect {
                    x: head.bounds.x + head.bounds.width / 2.0,
                    y: head.bounds.bottom(),
                    width: 0.0,
                    height: (band_bottom - head.bounds.bottom()).max(0.0),
                },
                text: None,
            })
            .collect();
        if let Some(lanes) = geometry.nodes.get_mut(lanes_index) {
            lanes.parts.extend(lifelines);
        }
    }
}
