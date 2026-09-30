// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{G7_JSON, g7_value};
use stencil_model::checks::CheckName;
use stencil_model::{ModelError, Projection, page_schema, parse_page};

fn validator() -> jsonschema::Validator {
    let schema = serde_json::to_value(page_schema()).unwrap();
    jsonschema::validator_for(&schema).expect("the generated schema compiles")
}

#[test]
fn projection_parses_iso_and_flat_and_defaults_to_flat() {
    assert_eq!(parse_page(G7_JSON).unwrap().projection, Projection::Flat);
    for (name, projection) in [("flat", Projection::Flat), ("iso", Projection::Iso)] {
        let mut document = g7_value();
        document["projection"] = serde_json::json!(name);
        let page = parse_page(&document.to_string()).unwrap();
        assert_eq!(page.projection, projection, "{name}");
    }
}

#[test]
fn an_unknown_projection_is_a_json_error() {
    let mut document = g7_value();
    document["projection"] = serde_json::json!("oblique");
    assert!(matches!(
        parse_page(&document.to_string()),
        Err(ModelError::Json { .. })
    ));
}

#[test]
fn flat_is_not_serialized_and_iso_is() {
    let mut document = g7_value();
    document["projection"] = serde_json::json!("flat");
    let flat = serde_json::to_value(parse_page(&document.to_string()).unwrap()).unwrap();
    assert_eq!(flat.get("projection"), None);

    document["projection"] = serde_json::json!("iso");
    let iso = serde_json::to_value(parse_page(&document.to_string()).unwrap()).unwrap();
    assert_eq!(iso.get("projection"), Some(&serde_json::json!("iso")));
}

#[test]
fn the_schema_accepts_flat_and_iso_and_rejects_oblique() {
    let validator = validator();
    for (name, valid) in [("flat", true), ("iso", true), ("oblique", false)] {
        let mut document = g7_value();
        document["projection"] = serde_json::json!(name);
        assert_eq!(validator.is_valid(&document), valid, "{name}");
    }
}

#[test]
fn iso_labels_clear_is_the_last_check_and_counts_pairs() {
    assert_eq!(CheckName::IsoLabelsClear.as_str(), "iso-labels-clear");
    assert_eq!(CheckName::IsoLabelsClear.unit(1), "pair");
    assert_eq!(CheckName::IsoLabelsClear.unit(0), "pairs");
    assert_eq!(CheckName::IsoLabelsClear.unit(2), "pairs");
}
