#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! icon-matches-product and `named_product` (section 13.10): the gcp products table, the
//! longest-match rule, each defect kind beside its clean neighbor, and the examples.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::grammar::{IconClass, IconProducts};
use stencil_model::products::{icon_matches_product, named_product};
use stencil_model::{IconName, Page, parse_page};

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn gcp_table() -> Vec<IconProducts> {
    let grammar = common::gcp();
    let product = grammar.item("product").expect("gcp has the product kind");
    product.products.clone()
}

fn page_of_items(items: Value) -> Page {
    serde_json::from_value(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "canvas": "customer",
        "body": [{ "tag": "Row", "children": items }],
        "legend": []
    }))
    .unwrap()
}

fn item(subtitle: Option<&str>, icon: Option<&str>) -> Value {
    let mut value = json!({ "tag": "Item", "kind": "product", "title": "Product" });
    if let Some(subtitle) = subtitle {
        value["subtitle"] = json!(subtitle);
    }
    if let Some(icon) = icon {
        value["icon"] = json!(icon);
    }
    value
}

/// The defect messages of a page holding the one item.
fn defects_for(subtitle: Option<&str>, icon: Option<&str>) -> Vec<String> {
    let page = page_of_items(json!([item(subtitle, icon)]));
    let report = icon_matches_product(&page, &common::gcp());
    assert_eq!(report.examined, 1, "{subtitle:?} {icon:?}");
    report
        .defects
        .into_iter()
        .map(|defect| defect.message)
        .collect()
}

#[test]
fn the_gcp_table_has_every_icon_once_in_icon_order_with_its_archive_class() {
    let table = gcp_table();
    let icons: Vec<IconName> = table.iter().map(|row| row.icon).collect();
    assert_eq!(icons, IconName::ALL);

    let provenance = fs::read_to_string(repository_path("assets/icons/PROVENANCE.md")).unwrap();
    let mut examined = 0;
    for row in &table {
        let file_cell = format!("| `{}` | `", row.icon.file_name());
        let line = provenance
            .lines()
            .find(|line| line.starts_with(&file_cell))
            .unwrap_or_else(|| panic!("{} has no provenance row", row.icon.as_str()));
        let archive_path = line.strip_prefix(&file_cell).unwrap();
        let expected = if archive_path.starts_with("Unique Icons/") {
            IconClass::Product
        } else if archive_path.starts_with("Category Icons/") {
            IconClass::Category
        } else {
            panic!("{} comes from neither archive folder", row.icon.as_str())
        };
        assert_eq!(row.class, expected, "{}", row.icon.as_str());
        examined += 1;
    }
    assert_eq!(examined, IconName::ALL.len());
}

#[test]
fn named_product_picks_the_longest_name() {
    let table = gcp_table();
    assert_eq!(
        named_product(&table, "Vertex AI Agent Engine"),
        Some("Vertex AI Agent Engine")
    );
    assert_eq!(
        named_product(&table, "Cloud Run functions · events"),
        Some("Cloud Run functions")
    );
    assert_eq!(named_product(&table, "Cloud Run jobs"), Some("Cloud Run"));
}

#[test]
fn named_product_takes_the_first_of_two_names_of_equal_length() {
    let table = vec![
        IconProducts {
            icon: IconName::Gke,
            class: IconClass::Product,
            names: vec!["Alpha".to_string()],
        },
        IconProducts {
            icon: IconName::Compute,
            class: IconClass::Category,
            names: vec!["Gamma".to_string()],
        },
    ];
    assert_eq!(named_product(&table, "Gamma then Alpha"), Some("Gamma"));
    assert_eq!(named_product(&table, "Alpha then Gamma"), Some("Alpha"));
}

#[test]
fn named_product_respects_word_boundaries_and_ignores_ascii_case() {
    let table = gcp_table();
    assert_eq!(named_product(&table, "BigQueryX"), None);
    assert_eq!(named_product(&table, "XBigQuery"), None);
    assert_eq!(named_product(&table, "big_query"), None);
    assert_eq!(
        named_product(&table, "bigquery · vectors"),
        Some("BigQuery")
    );
    assert_eq!(named_product(&table, "via CLOUD SQL"), Some("Cloud SQL"));
    assert_eq!(named_product(&table, "object storage"), None);
}

#[test]
fn a_product_with_its_own_icon_matches_it_or_no_icon_and_rejects_another() {
    assert!(defects_for(Some("BigQuery"), Some("bigquery")).is_empty());
    assert!(defects_for(Some("BigQuery"), None).is_empty());
    assert_eq!(
        defects_for(Some("BigQuery"), Some("data-analytics")),
        ["subtitle names BigQuery, whose icon is bigquery; the item carries data-analytics"]
    );
    assert_eq!(
        defects_for(Some("Cloud Run · Direct VPC"), Some("serverless")),
        [
            "subtitle names Cloud Run, whose icon is cloud-run or cloud-run-flat; the item carries serverless"
        ]
    );
}

#[test]
fn a_product_without_its_own_icon_takes_a_category_icon_but_no_product_icon() {
    assert!(defects_for(Some("Pub/Sub"), Some("integration")).is_empty());
    assert!(defects_for(Some("Pub/Sub"), None).is_empty());
    assert_eq!(
        defects_for(Some("Pub/Sub"), Some("cloud-run")),
        [
            "subtitle names Pub/Sub, which has no product icon; the item carries the product icon cloud-run"
        ]
    );
    assert_eq!(
        defects_for(Some("Cloud Logging · org sink"), Some("bigquery")),
        [
            "subtitle names Cloud Logging, which has no product icon; the item carries the product icon bigquery"
        ]
    );
}

#[test]
fn an_item_whose_subtitle_names_nothing_is_examined_and_clean() {
    assert!(defects_for(Some("on-prem router"), Some("bigquery")).is_empty());
    assert!(defects_for(None, Some("bigquery")).is_empty());
}

#[test]
fn an_item_with_neither_icon_nor_subtitle_is_not_examined() {
    let page = page_of_items(json!([item(None, None), item(Some("BigQuery"), None)]));
    let report = icon_matches_product(&page, &common::gcp());
    assert_eq!(report.examined, 1);
    assert_eq!(report.outcome(), CheckOutcome::Passed);
}

#[test]
fn a_gcp_page_whose_items_carry_neither_examines_nothing_and_fails() {
    let page = page_of_items(json!([item(None, None)]));
    let report = icon_matches_product(&page, &common::gcp());
    assert_eq!(report.examined, 0);
    assert_eq!(report.not_applicable, None);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}

#[test]
fn a_plain_page_is_not_applicable() {
    let page: Page = serde_json::from_value(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "canvas": "internal",
        "grammar": "plain",
        "body": [{ "tag": "Item", "kind": "service", "title": "API", "subtitle": "BigQuery" }],
        "legend": []
    }))
    .unwrap();
    let report = icon_matches_product(&page, &common::plain());
    assert_eq!(report.check, CheckName::IconMatchesProduct);
    assert_eq!(report.not_applicable, Some("grammar has no icon table"));
    assert_eq!(report.outcome(), CheckOutcome::NotApplicable);
}

#[test]
fn g7_examines_six_items_clean() {
    let report = icon_matches_product(&common::g7_page(), &common::gcp());
    assert_eq!(report.examined, 6);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
}

/// Reads an example, sets `icon` on the item whose subtitle is `subtitle`, and returns the
/// page with that item's pointer.
fn example_with_icon(stem: &str, subtitle: &str, icon: &str) -> (Page, String) {
    let text = fs::read_to_string(repository_path(&format!("examples/{stem}.json"))).unwrap();
    let mut document: Value = serde_json::from_str(&text).unwrap();
    let pointer = find_subtitle(&document, "", subtitle)
        .unwrap_or_else(|| panic!("{stem} has no item with subtitle {subtitle:?}"));
    document.pointer_mut(&pointer).unwrap()["icon"] = json!(icon);
    (parse_page(&document.to_string()).unwrap(), pointer)
}

fn find_subtitle(value: &Value, pointer: &str, subtitle: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            if map.get("subtitle").and_then(Value::as_str) == Some(subtitle) {
                return Some(pointer.to_string());
            }
            map.iter().find_map(|(key, child)| {
                find_subtitle(child, &format!("{pointer}/{key}"), subtitle)
            })
        }
        Value::Array(items) => items.iter().enumerate().find_map(|(index, child)| {
            find_subtitle(child, &format!("{pointer}/{index}"), subtitle)
        }),
        _ => None,
    }
}

#[test]
fn the_three_example_mismatches_are_found_at_their_items() {
    let cases = [
        (
            "hybrid-ai",
            "Vertex AI Registry",
            "ai-ml",
            "subtitle names Vertex AI, whose icon is vertex-ai; the item carries ai-ml",
        ),
        (
            "hybrid-ai",
            "Billing export · BigQuery",
            "data-analytics",
            "subtitle names BigQuery, whose icon is bigquery; the item carries data-analytics",
        ),
        (
            "network-hub-spoke",
            "Cloud Run · Direct VPC",
            "serverless",
            "subtitle names Cloud Run, whose icon is cloud-run or cloud-run-flat; the item carries serverless",
        ),
    ];
    for (stem, subtitle, icon, message) in cases {
        let (page, pointer) = example_with_icon(stem, subtitle, icon);
        let report = icon_matches_product(&page, &common::gcp());
        let found: Vec<(String, String)> = report
            .defects
            .iter()
            .map(|defect| (defect.pointer.to_string(), defect.message.clone()))
            .collect();
        assert_eq!(found, [(pointer, message.to_string())], "{stem}");
    }
}

#[test]
fn every_gcp_example_passes_with_items_examined() {
    let mut examined = 0;
    for stem in [
        "g7",
        "hero-iso",
        "hybrid-ai",
        "network-hub-spoke",
        "onepager",
        "stress-dense",
    ] {
        let text = fs::read_to_string(repository_path(&format!("examples/{stem}.json"))).unwrap();
        let page = parse_page(&text).unwrap();
        let report = icon_matches_product(&page, &common::gcp());
        assert!(report.passed(), "{stem}: {:?}", report.defects);
        examined += 1;
    }
    assert_eq!(examined, 6);
}
