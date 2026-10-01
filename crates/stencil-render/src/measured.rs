//! The section 5.4 measured JSON.

use serde_json::{Map, Value};
use stencil_layout::{BoxRect, LinkRoute, NodeGeometry, PageGeometry, Part, PartName, RouteStatus};
use stencil_model::pointer::NodePointer;

use crate::format_number;
use crate::iso::{IsoScene, ScreenPoint};

/// Section 5.4 shape; with Some(scene) the output also carries `projection` (section 12.8).
pub fn measured_json(document: &Value, geometry: &PageGeometry, scene: Option<&IsoScene>) -> Value {
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
    if let Some(scene) = scene {
        root.insert("projection".to_string(), projection_json(scene));
    }
    Value::Object(root)
}

/// The drawn canvas, the offset and footer shift, and every label on its plane in painter
/// order: its screen corners, top-left first then clockwise in layout terms, and its axis.
fn projection_json(scene: &IsoScene) -> Value {
    let labels: Vec<Value> = scene
        .labels
        .iter()
        .map(|label| {
            let mut object = Map::new();
            object.insert(
                "axis".to_string(),
                Value::String(label.axis.as_str().to_string()),
            );
            object.insert("corners".to_string(), corners_json(&label.corners));
            object.insert(
                "id".to_string(),
                Value::String(label.owner.as_str().to_string()),
            );
            object.insert("z".to_string(), format_number(label.z).to_json());
            Value::Object(object)
        })
        .collect();
    let mut canvas = Map::new();
    canvas.insert(
        "height".to_string(),
        format_number(scene.canvas.height).to_json(),
    );
    canvas.insert(
        "width".to_string(),
        format_number(scene.canvas.width).to_json(),
    );
    let mut offset = Map::new();
    offset.insert("x".to_string(), format_number(scene.offset.x).to_json());
    offset.insert("y".to_string(), format_number(scene.offset.y).to_json());
    let mut object = Map::new();
    object.insert("labels".to_string(), Value::Array(labels));
    object.insert("canvas".to_string(), Value::Object(canvas));
    object.insert(
        "footer_shift".to_string(),
        format_number(scene.footer_shift).to_json(),
    );
    object.insert("kind".to_string(), Value::String("iso".to_string()));
    object.insert("offset".to_string(), Value::Object(offset));
    Value::Object(object)
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
    object.insert("kind".to_string(), Value::String(route.key().to_string()));
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
/// body line, so those two are keyed `<name>/<line>` with the zero-based line index, and a
/// Lanes node keys its lifelines `lifeline/<i>` by head index (section 13.6). An
/// Item's fact parts come in box and run pairs in `facts` order: the first entry of each
/// source keeps the bare names, and every later one adds `/<i>`, its index in `facts`
/// (section 13.7).
fn parts_json(parts: &[Part]) -> Map<String, Value> {
    let mut body_lines = 0_usize;
    let mut markers = 0_usize;
    let mut lifelines = 0_usize;
    let mut fact_entries = 0_usize;
    let mut seen_sources: [bool; 3] = [false; 3];
    let mut fact_suffix: Option<usize> = None;
    let mut object = Map::new();
    for part in parts {
        let key = match part.name {
            PartName::FactBox | PartName::BuiltBox | PartName::AskBox => {
                let source = match part.name {
                    PartName::FactBox => 0,
                    PartName::BuiltBox => 1,
                    _ => 2,
                };
                let first = seen_sources
                    .get_mut(source)
                    .is_some_and(|seen| !std::mem::replace(seen, true));
                fact_suffix = if first { None } else { Some(fact_entries) };
                fact_entries += 1;
                suffixed(part.name, fact_suffix)
            }
            PartName::Fact | PartName::Built | PartName::Ask => suffixed(part.name, fact_suffix),
            PartName::BodyLine => {
                body_lines += 1;
                format!("{}/{}", part.name.as_str(), body_lines - 1)
            }
            PartName::Marker => {
                markers += 1;
                format!("{}/{}", part.name.as_str(), markers - 1)
            }
            PartName::Lifeline => {
                lifelines += 1;
                format!("{}/{}", part.name.as_str(), lifelines - 1)
            }
            _ => part.name.as_str().to_string(),
        };
        object.insert(key, part_json(part));
    }
    object
}

fn suffixed(name: PartName, index: Option<usize>) -> String {
    match index {
        Some(index) => format!("{}/{index}", name.as_str()),
        None => name.as_str().to_string(),
    }
}

fn node_json(node: &NodeGeometry) -> Value {
    let mut object = box_json(node.bounds);
    object.insert(
        "id".to_string(),
        Value::String(node.pointer.as_str().to_string()),
    );
    if let Some(kind) = &node.kind {
        object.insert("kind".to_string(), Value::String(kind.clone()));
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

fn corners_json(corners: &[ScreenPoint; 4]) -> Value {
    Value::Array(
        corners
            .iter()
            .map(|corner| {
                let mut point = Map::new();
                point.insert("x".to_string(), format_number(corner.x).to_json());
                point.insert("y".to_string(), format_number(corner.y).to_json());
                Value::Object(point)
            })
            .collect(),
    )
}
