#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! icon-matches-product and `named_product` (section 13.10): the gcp, aws, azure and oss products
//! tables, the longest-match rule, each defect kind beside its clean neighbor, and the examples.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::grammar::{IconClass, IconProducts};
use stencil_model::products::{icon_matches_product, named_product};
use stencil_model::{
    AwsIcon, AzureIcon, GcpIcon, Grammar, IconStem, Page, builtin_grammar, parse_page,
};

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn stem(text: &str) -> IconStem {
    IconStem::try_from(text).unwrap()
}

/// The icon of a products row; every vendor row has one.
fn row_icon(row: &IconProducts) -> &str {
    row.icon
        .as_ref()
        .expect("a vendor row has an icon")
        .as_str()
}

fn gcp_table() -> Vec<IconProducts> {
    let grammar = common::gcp();
    let product = grammar.item("product").expect("gcp has the product kind");
    product.products.clone()
}

fn page_of_items(items: Value) -> Page {
    serde_json::from_value(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "badge": "Customer",
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

/// The defect messages of a gcp page holding the one item.
fn defects_for(subtitle: Option<&str>, icon: Option<&str>) -> Vec<String> {
    defects_under(&common::gcp(), subtitle, icon)
}

/// The defect messages of a page under `grammar` holding the one item.
fn defects_under(grammar: &Grammar, subtitle: Option<&str>, icon: Option<&str>) -> Vec<String> {
    let page = page_of_items(json!([item(subtitle, icon)]));
    let report = icon_matches_product(&page, grammar);
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
    let icons: Vec<&str> = table.iter().map(row_icon).collect();
    let pack_icons: Vec<&str> = GcpIcon::ALL.iter().map(|icon| icon.as_str()).collect();
    assert_eq!(icons, pack_icons);

    let provenance = fs::read_to_string(repository_path("assets/icons/gcp/PROVENANCE.md")).unwrap();
    let mut examined = 0;
    for row in &table {
        let file_cell = format!("| `{}.svg` | `", row_icon(row));
        let line = provenance
            .lines()
            .find(|line| line.starts_with(&file_cell))
            .unwrap_or_else(|| panic!("{} has no provenance row", row_icon(row)));
        let archive_path = line.strip_prefix(&file_cell).unwrap();
        let expected = if archive_path.starts_with("Unique Icons/") {
            IconClass::Product
        } else if archive_path.starts_with("Category Icons/") {
            IconClass::Category
        } else {
            panic!("{} comes from neither archive folder", row_icon(row))
        };
        assert_eq!(row.class, expected, "{}", row_icon(row));
        examined += 1;
    }
    assert_eq!(examined, GcpIcon::ALL.len());
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
            icon: Some(stem("gke")),
            class: IconClass::Product,
            names: vec!["Alpha".to_string()],
            shape: None,
        },
        IconProducts {
            icon: Some(stem("compute")),
            class: IconClass::Category,
            names: vec!["Gamma".to_string()],
            shape: None,
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
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "badge": "Internal",
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
        "stress-dense",
    ] {
        let text = fs::read_to_string(repository_path(&format!("examples/{stem}.json"))).unwrap();
        let page = parse_page(&text).unwrap();
        let report = icon_matches_product(&page, &common::gcp());
        assert!(report.passed(), "{stem}: {:?}", report.defects);
        examined += 1;
    }
    assert_eq!(examined, 5);
}

fn aws() -> Grammar {
    builtin_grammar("aws").unwrap().unwrap()
}

#[test]
fn the_aws_example_passes_with_items_examined() {
    let text = fs::read_to_string(repository_path("examples/aws-hybrid.json")).unwrap();
    let page = parse_page(&text).unwrap();
    let report = icon_matches_product(&page, &aws());
    assert!(report.passed(), "{:?}", report.defects);
    assert!(report.examined >= 10, "examined {}", report.examined);
}

fn aws_table() -> Vec<IconProducts> {
    let grammar = aws();
    let product = grammar.item("product").expect("aws has the product kind");
    product.products.clone()
}

/// The defect messages of an aws page holding the one item.
fn aws_defects_for(subtitle: Option<&str>, icon: Option<&str>) -> Vec<String> {
    let page = page_of_items(json!([item(subtitle, icon)]));
    let report = icon_matches_product(&page, &aws());
    assert_eq!(report.examined, 1, "{subtitle:?} {icon:?}");
    report
        .defects
        .into_iter()
        .map(|defect| defect.message)
        .collect()
}

#[test]
fn the_aws_table_has_every_icon_once_in_icon_order_with_its_archive_class() {
    let table = aws_table();
    let icons: Vec<&str> = table.iter().map(row_icon).collect();
    let pack_icons: Vec<&str> = AwsIcon::ALL.iter().map(|icon| icon.as_str()).collect();
    assert_eq!(icons, pack_icons);

    let provenance = fs::read_to_string(repository_path("assets/icons/aws/PROVENANCE.md")).unwrap();
    let mut examined = 0;
    for row in &table {
        let file_cell = format!("| `{}.svg` | `", row_icon(row));
        let line = provenance
            .lines()
            .find(|line| line.starts_with(&file_cell))
            .unwrap_or_else(|| panic!("{} has no provenance row", row_icon(row)));
        let archive_path = line.strip_prefix(&file_cell).unwrap();
        let expected = if archive_path.starts_with("Architecture-Service-Icons_") {
            IconClass::Product
        } else if archive_path.starts_with("Category-Icons_") {
            IconClass::Category
        } else {
            panic!("{} comes from neither archive folder", row_icon(row))
        };
        assert_eq!(row.class, expected, "{}", row_icon(row));
        examined += 1;
    }
    assert_eq!(examined, AwsIcon::ALL.len());
}

#[test]
fn an_aws_full_service_name_and_its_short_form_name_the_same_icon() {
    let table = aws_table();
    for (full, short, icon) in [
        ("Amazon Simple Storage Service", "S3", "s3"),
        ("Amazon S3", "S3", "s3"),
        ("AWS Lambda", "Lambda", "lambda"),
        ("Amazon Relational Database Service", "RDS", "rds"),
        ("AWS Key Management Service", "KMS", "kms"),
    ] {
        assert_eq!(named_product(&table, full), Some(full));
        assert_eq!(named_product(&table, short), Some(short));
        assert!(aws_defects_for(Some(full), Some(icon)).is_empty(), "{full}");
        assert!(
            aws_defects_for(Some(short), Some(icon)).is_empty(),
            "{short}"
        );
    }
}

#[test]
fn an_aws_service_with_its_own_icon_matches_it_and_rejects_another() {
    assert!(aws_defects_for(Some("Amazon RDS for PostgreSQL"), Some("rds")).is_empty());
    assert!(aws_defects_for(Some("Amazon DynamoDB"), None).is_empty());
    assert_eq!(
        aws_defects_for(Some("Amazon S3 · exports"), Some("storage")),
        ["subtitle names Amazon S3, whose icon is s3; the item carries storage"]
    );
    assert_eq!(
        aws_defects_for(Some("Amazon Aurora"), Some("rds")),
        ["subtitle names Amazon Aurora, whose icon is aurora; the item carries rds"]
    );
}

#[test]
fn an_aws_service_without_its_own_icon_takes_a_category_icon_but_no_service_icon() {
    assert!(aws_defects_for(Some("Amazon DocumentDB"), Some("database")).is_empty());
    assert!(aws_defects_for(Some("AWS Batch"), Some("compute")).is_empty());
    assert_eq!(
        aws_defects_for(Some("Amazon DocumentDB"), Some("dynamodb")),
        [
            "subtitle names Amazon DocumentDB, which has no product icon; the item carries the product icon dynamodb"
        ]
    );
    assert_eq!(
        aws_defects_for(Some("AWS IAM Identity Center"), Some("iam")),
        [
            "subtitle names AWS IAM Identity Center, which has no product icon; the item carries the product icon iam"
        ]
    );
}

#[test]
fn an_aws_endpoint_subtitle_names_the_endpoint_not_the_service_behind_it() {
    let table = aws_table();
    assert_eq!(
        named_product(&table, "gateway endpoint · Amazon S3"),
        Some("gateway endpoint")
    );
    assert_eq!(
        named_product(&table, "Secrets Manager interface endpoint"),
        Some("interface endpoint")
    );
    assert!(aws_defects_for(Some("gateway endpoint · Amazon S3"), Some("vpc")).is_empty());
    assert!(aws_defects_for(Some("interface endpoint"), Some("privatelink")).is_empty());
    assert_eq!(
        aws_defects_for(Some("S3 gateway endpoint"), Some("s3")),
        ["subtitle names gateway endpoint, whose icon is vpc; the item carries s3"]
    );
}

#[test]
fn an_aws_subtitle_names_its_longest_service_name_and_never_an_instance() {
    let table = aws_table();
    assert_eq!(named_product(&table, "RDS instance"), Some("RDS"));
    assert_eq!(named_product(&table, "ECS on Fargate"), Some("Fargate"));
    assert_eq!(
        named_product(&table, "Route 53 Resolver inbound endpoint"),
        Some("Route 53")
    );
    assert_eq!(named_product(&table, "EC2X"), None);
    assert!(aws_defects_for(Some("RDS instance"), Some("rds")).is_empty());
}

#[test]
fn the_azure_example_passes_with_items_examined() {
    let text = fs::read_to_string(repository_path("examples/azure-hub-spoke.json")).unwrap();
    let page = parse_page(&text).unwrap();
    let report = icon_matches_product(&page, &common::azure());
    assert!(report.passed(), "{:?}", report.defects);
    assert_eq!(report.examined, 14);
}

fn azure_table() -> Vec<IconProducts> {
    let grammar = common::azure();
    let product = grammar.item("product").expect("azure has the product kind");
    product.products.clone()
}

#[test]
fn the_azure_table_has_every_icon_once_in_icon_order_each_a_product_from_one_archive() {
    let table = azure_table();
    let icons: Vec<&str> = table.iter().map(row_icon).collect();
    let pack_icons: Vec<&str> = AzureIcon::ALL.iter().map(|icon| icon.as_str()).collect();
    assert_eq!(icons, pack_icons);

    let provenance =
        fs::read_to_string(repository_path("assets/icons/azure/PROVENANCE.md")).unwrap();
    let mut examined = 0;
    for row in &table {
        let file_cell = format!("| `{}.svg` | `", row_icon(row));
        let line = provenance
            .lines()
            .find(|line| line.starts_with(&file_cell))
            .unwrap_or_else(|| panic!("{} has no provenance row", row_icon(row)));
        let archive_path = line.strip_prefix(&file_cell).unwrap();
        assert!(
            archive_path.starts_with("Icons/")
                || archive_path.starts_with("Microsoft Entra architecture icons - Oct 2023/"),
            "{} comes from neither archive",
            row_icon(row)
        );
        assert_eq!(row.class, IconClass::Product, "{}", row_icon(row));
        examined += 1;
    }
    assert_eq!(examined, AzureIcon::ALL.len());
}

#[test]
fn an_azure_name_inside_a_longer_name_of_another_row_yields_to_it() {
    let table = azure_table();
    assert_eq!(
        named_product(&table, "ExpressRoute gateway · ErGw1AZ"),
        Some("ExpressRoute gateway")
    );
    assert_eq!(
        named_product(&table, "ExpressRoute circuit"),
        Some("ExpressRoute circuit")
    );
    assert_eq!(
        named_product(&table, "Event Grid domain"),
        Some("Event Grid domain")
    );
    assert_eq!(
        named_product(&table, "Azure Monitor Logs"),
        Some("Azure Monitor Logs")
    );
    assert_eq!(
        named_product(&table, "private endpoint · Blob Storage"),
        Some("private endpoint")
    );
    assert_eq!(named_product(&table, "health monitor"), None);
    assert_eq!(named_product(&table, "firewall policy"), None);
}

#[test]
fn an_azure_product_with_its_own_icon_matches_it_or_no_icon_and_rejects_another() {
    let azure = common::azure();
    assert!(defects_under(&azure, Some("Azure Key Vault"), Some("key-vault")).is_empty());
    assert!(defects_under(&azure, Some("Azure Key Vault"), None).is_empty());
    assert!(defects_under(&azure, Some("Blob Storage"), Some("storage-accounts")).is_empty());
    assert!(defects_under(&azure, Some("ExpressRoute gateway"), Some("vpn-gateway")).is_empty());
    assert_eq!(
        defects_under(&azure, Some("Azure Key Vault"), Some("storage-accounts")),
        [
            "subtitle names Azure Key Vault, whose icon is key-vault; the item carries storage-accounts"
        ]
    );
    assert_eq!(
        defects_under(&azure, Some("ExpressRoute gateway"), Some("expressroute")),
        [
            "subtitle names ExpressRoute gateway, whose icon is vpn-gateway; the item carries expressroute"
        ]
    );
}

#[test]
fn an_azure_item_whose_subtitle_names_no_row_is_examined_and_clean_with_any_icon() {
    let azure = common::azure();
    assert!(defects_under(&azure, Some("Azure Route Server"), Some("virtual-network")).is_empty());
    assert!(defects_under(&azure, Some("AKS · internal load balancer"), Some("aks")).is_empty());
    assert!(defects_under(&azure, None, Some("key-vault")).is_empty());
}

fn oss() -> Grammar {
    builtin_grammar("oss")
        .expect("oss is a built-in grammar")
        .expect("the oss grammar is valid")
}

/// The defect messages of an oss page holding one item of `kind`.
fn oss_defects_for(kind: &str, subtitle: Option<&str>, icon: Option<&str>) -> Vec<String> {
    let mut value = item(subtitle, icon);
    value["kind"] = json!(kind);
    let page = page_of_items(json!([value]));
    let report = icon_matches_product(&page, &oss());
    assert_eq!(report.examined, 1, "{kind} {subtitle:?} {icon:?}");
    report
        .defects
        .into_iter()
        .map(|defect| defect.message)
        .collect()
}

#[test]
fn every_oss_project_with_an_icon_matches_it_on_its_kind_and_no_other_icon() {
    let grammar = oss();
    let mut examined = 0;
    for kind in grammar
        .items
        .iter()
        .filter(|kind| !kind.products.is_empty())
    {
        for row in kind.products.iter().filter(|row| row.icon.is_some()) {
            let icon = row_icon(row);
            for name in &row.names {
                assert!(
                    oss_defects_for(&kind.name, Some(name), Some(icon)).is_empty(),
                    "{} {name} {icon}",
                    kind.name
                );
                assert!(
                    oss_defects_for(&kind.name, Some(name), None).is_empty(),
                    "{} {name}",
                    kind.name
                );
                let other = kind
                    .products
                    .iter()
                    .filter_map(|other| other.icon.as_ref())
                    .find(|other| other.as_str() != icon)
                    .unwrap();
                assert_eq!(
                    oss_defects_for(&kind.name, Some(name), Some(other.as_str())),
                    [format!(
                        "subtitle names {name}, whose icon is {icon}; the item carries {other}"
                    )],
                    "{} {name}",
                    kind.name
                );
                examined += 1;
            }
        }
    }
    println!("examined {examined} product names");
    assert!(examined > 0);
}

#[test]
fn an_oss_icon_on_another_kind_is_a_defect_when_the_subtitle_names_that_kinds_project() {
    assert_eq!(
        oss_defects_for("queue", Some("Apache Kafka"), Some("postgresql")),
        ["subtitle names Apache Kafka, whose icon is kafka; the item carries postgresql"]
    );
    assert_eq!(
        oss_defects_for("network", Some("NGINX"), Some("kubernetes")),
        ["subtitle names NGINX, whose icon is nginx; the item carries kubernetes"]
    );
    assert!(oss_defects_for("queue", Some("PostgreSQL"), Some("postgresql")).is_empty());
}

#[test]
fn an_oss_project_without_a_bundled_logo_is_clean_without_an_icon_and_a_defect_with_one() {
    for name in ["MySQL", "Elasticsearch"] {
        assert!(
            oss_defects_for("store", Some(name), None).is_empty(),
            "{name}"
        );
    }
    assert_eq!(
        oss_defects_for("store", Some("MySQL"), Some("mariadb")),
        [
            "subtitle names MySQL, which has no product icon; the item carries the product icon mariadb"
        ]
    );
    assert_eq!(
        oss_defects_for("store", Some("Elasticsearch"), Some("opensearch")),
        [
            "subtitle names Elasticsearch, which has no product icon; the item carries the product icon opensearch"
        ]
    );
    let table = oss().item("store").unwrap().products.clone();
    assert_eq!(named_product(&table, "MySQL 8.4 · primary"), Some("MySQL"));
}
