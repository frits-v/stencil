//! The section 5.4 measured JSON.

use serde_json::{Map, Value};
use stencil_layout::{BoxRect, LinkRoute, NodeGeometry, PageGeometry, Part, PartName, RouteStatus};
use stencil_model::pointer::NodePointer;

use crate::format_number;

/// Section 5.4 shape. `document` is the input parsed as serde_json::Value.
pub fn measured_json(document: &Value, geometry: &PageGeometry) -> Value {
    debug_assert!(
        geometry
            .nodes
            .iter()
            .all(|node| document.pointer(node.pointer.as_str()).is_some()),
        "a geometry pointer does not resolve in the document (section 5.4)"
    );
    let mut canvas = Map::new();
    canvas.insert(
        "height".to_string(),
        format_number(geometry.canvas.height).to_json(),
    );
    canvas.insert(
        "width".to_string(),
        format_number(geometry.canvas.width).to_json(),
    );

    let nodes: Vec<Value> = geometry.nodes.iter().map(node_json).collect();

    let mut root = Map::new();
    root.insert("canvas".to_string(), Value::Object(canvas));
    root.insert("document".to_string(), document.clone());
    // A page without links writes no `links` key, so its output is unchanged by section 11.2.
    if !geometry.links.is_empty() {
        let links: Vec<Value> = geometry
            .links
            .iter()
            .map(|route| link_json(route, geometry))
            .collect();
        root.insert("links".to_string(), Value::Array(links));
    }
    root.insert("nodes".to_string(), Value::Array(nodes));
    Value::Object(root)
}

/// One routed link: its pointer, endpoint node pointers, kind, points, tag box, tag parts
/// and route status.
fn link_json(route: &LinkRoute, geometry: &PageGeometry) -> Value {
    let endpoint = |index: usize| {
        geometry.nodes.get(index).map_or(Value::Null, |node| {
            Value::String(node.pointer.as_str().to_string())
        })
    };
    let points: Vec<Value> = route
        .points
        .iter()
        .map(|point| {
            let mut object = Map::new();
            object.insert("x".to_string(), format_number(point.x).to_json());
            object.insert("y".to_string(), format_number(point.y).to_json());
            Value::Object(object)
        })
        .collect();
    let status = match route.status {
        RouteStatus::Routed => "routed",
        RouteStatus::Fallback => "fallback",
    };
    let mut object = Map::new();
    object.insert("from".to_string(), endpoint(route.from_node));
    object.insert(
        "id".to_string(),
        Value::String(
            NodePointer::root()
                .child("links")
                .index(route.index)
                .as_str()
                .to_string(),
        ),
    );
    object.insert(
        "kind".to_string(),
        Value::String(route.kind.as_str().to_string()),
    );
    object.insert("parts".to_string(), Value::Object(parts_json(&route.parts)));
    object.insert("points".to_string(), Value::Array(points));
    object.insert("status".to_string(), Value::String(status.to_string()));
    if let Some(tag) = route.tag {
        object.insert("tag".to_string(), Value::Object(box_json(tag)));
    }
    object.insert("to".to_string(), endpoint(route.to_node));
    Value::Object(object)
}

/// Parts keyed by snake_case name. A Text block repeats `body_line` and `marker` once per
/// body line, so those two are keyed `<name>/<line>` with the zero-based line index.
fn parts_json(parts: &[Part]) -> Map<String, Value> {
    let mut body_lines = 0_usize;
    let mut markers = 0_usize;
    let mut object = Map::new();
    for part in parts {
        let key = match part.name {
            PartName::BodyLine => {
                body_lines += 1;
                format!("{}/{}", part.name.as_str(), body_lines - 1)
            }
            PartName::Marker => {
                markers += 1;
                format!("{}/{}", part.name.as_str(), markers - 1)
            }
            _ => part.name.as_str().to_string(),
        };
        object.insert(key, part_json(part));
    }
    object
}

fn node_json(node: &NodeGeometry) -> Value {
    let mut object = box_json(node.bounds);
    object.insert(
        "id".to_string(),
        Value::String(node.pointer.as_str().to_string()),
    );
    if let Some(kind) = node.kind {
        object.insert("kind".to_string(), Value::String(kind.to_string()));
    }
    object.insert("parts".to_string(), Value::Object(parts_json(&node.parts)));
    object.insert(
        "tag".to_string(),
        Value::String(node.tag.as_str().to_string()),
    );
    Value::Object(object)
}

fn part_json(part: &Part) -> Value {
    let mut object = box_json(part.bounds);
    if let Some(run) = &part.text {
        object.insert(
            "line_count".to_string(),
            Value::from(run.metrics.line_count),
        );
    }
    Value::Object(object)
}

fn box_json(bounds: BoxRect) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("height".to_string(), format_number(bounds.height).to_json());
    object.insert("width".to_string(), format_number(bounds.width).to_json());
    object.insert("x".to_string(), format_number(bounds.x).to_json());
    object.insert("y".to_string(), format_number(bounds.y).to_json());
    object
}
