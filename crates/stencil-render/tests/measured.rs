#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use serde_json::Value;
use stencil_layout::NodeTag;
use stencil_render::measured_json;

fn g7_measured() -> (common::Rendered, Value) {
    let rendered = common::render_g7();
    let measured = measured_json(&common::g7_document(), &rendered.geometry, None);
    (rendered, measured)
}

fn visit_numbers(value: &Value, visit: &mut dyn FnMut(&serde_json::Number)) {
    match value {
        Value::Number(number) => visit(number),
        Value::Array(items) => items.iter().for_each(|item| visit_numbers(item, visit)),
        Value::Object(map) => map.values().for_each(|item| visit_numbers(item, visit)),
        Value::Null | Value::Bool(_) | Value::String(_) => {}
    }
}

#[test]
fn document_is_value_equal_to_the_input() {
    let (_, measured) = g7_measured();
    assert_eq!(measured["document"], common::g7_document());
}

#[test]
fn nodes_follow_geometry_order_and_resolve_in_the_document() {
    let (rendered, measured) = g7_measured();
    let nodes = measured["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), rendered.geometry.nodes.len());
    for (node, geometry_node) in nodes.iter().zip(&rendered.geometry.nodes) {
        let id = node["id"].as_str().unwrap();
        assert_eq!(id, geometry_node.pointer.as_str());
        assert!(measured["document"].pointer(id).is_some(), "{id}");
        assert_eq!(node["tag"], geometry_node.tag.as_str());
    }
}

#[test]
fn kind_appears_on_exactly_the_kinded_tags() {
    let (rendered, measured) = g7_measured();
    let nodes = measured["nodes"].as_array().unwrap();
    for (node, geometry_node) in nodes.iter().zip(&rendered.geometry.nodes) {
        let kinded = matches!(
            geometry_node.tag,
            NodeTag::Zone | NodeTag::Pipe | NodeTag::Tee | NodeTag::LegendEntry
        );
        assert_eq!(node.get("kind").is_some(), kinded, "{}", node["id"]);
        if let Some(kind) = &geometry_node.kind {
            assert_eq!(node["kind"], kind.as_str());
        }
    }
}

#[test]
fn parts_are_keyed_by_snake_case_name_with_line_count_on_text_parts() {
    let (rendered, measured) = g7_measured();
    let nodes = measured["nodes"].as_array().unwrap();
    let mut text_parts = 0;
    for (node, geometry_node) in nodes.iter().zip(&rendered.geometry.nodes) {
        let parts = node["parts"].as_object().unwrap();
        assert_eq!(parts.len(), geometry_node.parts.len(), "{}", node["id"]);
        for part in &geometry_node.parts {
            let entry = &parts[part.name.as_str()];
            for key in ["height", "width", "x", "y"] {
                assert!(entry[key].is_number(), "{} {key}", part.name.as_str());
            }
            match &part.text {
                Some(run) => {
                    text_parts += 1;
                    assert_eq!(entry["line_count"], run.metrics.line_count);
                }
                None => assert!(entry.get("line_count").is_none()),
            }
        }
    }
    assert_eq!(text_parts, 40);
    let pcard = nodes.iter().find(|node| node["tag"] == "Pcard").unwrap();
    assert!(pcard["parts"].get("function_name").is_some());
    assert!(pcard["parts"].get("product_name").is_some());
}

#[test]
fn canvas_and_root_carry_the_canvas_size() {
    let (rendered, measured) = g7_measured();
    assert_eq!(measured["canvas"]["width"], 1320);
    assert!(measured["canvas"]["width"].is_i64());
    let root = &measured["nodes"][0];
    assert_eq!(root["id"], "");
    assert_eq!(root["x"], 0);
    assert_eq!(root["y"], 0);
    assert_eq!(root["parts"], serde_json::json!({}));
    let height = measured["canvas"]["height"].as_f64().unwrap();
    assert!((height - f64::from(rendered.geometry.canvas.height)).abs() <= 0.005);
}

#[test]
fn every_number_is_rounded_and_integral_numbers_are_integers() {
    let (_, measured) = g7_measured();
    let mut examined = 0;
    visit_numbers(&measured, &mut |number| {
        examined += 1;
        if number.is_i64() || number.is_u64() {
            return;
        }
        let value = number.as_f64().unwrap();
        assert_ne!(value.fract(), 0.0, "integral {value} written as a float");
        assert!(!(value == 0.0 && value.is_sign_negative()), "negative zero");
        let text = number.to_string();
        let decimals = text
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len());
        assert!(decimals <= 2, "{text}");
    });
    assert!(examined > 500, "examined {examined} numbers");
}

#[test]
fn serialized_keys_are_in_ascending_byte_order() {
    let (_, measured) = g7_measured();
    let serialized = serde_json::to_string(&measured).unwrap();
    let canvas = serialized.find("\"canvas\"").unwrap();
    let document = serialized.find("\"document\"").unwrap();
    let nodes = serialized.find("\"nodes\"").unwrap();
    assert!(canvas < document && document < nodes);
    // Items carry `title` too, inside `body`, so the page's own title is found by its value.
    let kicker = serialized.find("\"kicker\"").unwrap();
    let title = serialized.find("\"title\":\"Four lines").unwrap();
    assert!(kicker < title, "kicker sorts before title inside document");

    fn assert_sorted(value: &Value) {
        match value {
            Value::Object(map) => {
                let keys: Vec<&String> = map.keys().collect();
                let mut sorted = keys.clone();
                sorted.sort();
                assert_eq!(keys, sorted);
                map.values().for_each(assert_sorted);
            }
            Value::Array(items) => items.iter().for_each(assert_sorted),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    let reparsed: Value = serde_json::from_str(&serialized).unwrap();
    assert_sorted(&reparsed);
}

#[test]
fn measured_json_is_deterministic() {
    let (rendered, first) = g7_measured();
    let second = measured_json(&common::g7_document(), &rendered.geometry, None);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}
