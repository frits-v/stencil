//! The section 5.4 measured JSON.

use serde_json::{Map, Value};
use stencil_layout::{BoxRect, NodeGeometry, PageGeometry, Part};

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
    root.insert("nodes".to_string(), Value::Array(nodes));
    Value::Object(root)
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
    let parts: Map<String, Value> = node
        .parts
        .iter()
        .map(|part| (part.name.as_str().to_string(), part_json(part)))
        .collect();
    object.insert("parts".to_string(), Value::Object(parts));
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
