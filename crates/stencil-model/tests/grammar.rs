// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Grammars as data (sections 13.2 and 13.3).

use serde_json::{Value, json};
use stencil_model::grammar::{
    BorderPattern, GrammarError, IconClass, IconPack, LabelStyle, Role, Tone, builtin_grammar_json,
};
use stencil_model::{Grammar, IconName, Shape, builtin_grammar, grammar_schema, parse_grammar};

const COMMITTED_GRAMMAR_SCHEMA: &str = include_str!("../../../schema/grammar.schema.json");

fn gcp() -> Grammar {
    builtin_grammar("gcp").unwrap().unwrap()
}

fn gcp_value() -> Value {
    serde_json::from_str(builtin_grammar_json("gcp").unwrap()).unwrap()
}

/// (rule, pointer) of every violation of `document`, which must parse as a grammar.
fn grammar_faults(document: &Value) -> Vec<(&'static str, String)> {
    match parse_grammar(&document.to_string(), "test") {
        Ok(_) => Vec::new(),
        Err(GrammarError::Invalid { violations, .. }) => violations
            .into_iter()
            .map(|violation| {
                (
                    violation.rule.as_str(),
                    violation.pointer.as_str().to_string(),
                )
            })
            .collect(),
        Err(error) => panic!("expected an invalid grammar, got {error}"),
    }
}

#[test]
fn both_built_in_grammars_parse_and_validate() {
    for name in ["gcp", "plain"] {
        let grammar = builtin_grammar(name).unwrap().unwrap();
        assert_eq!(grammar.name, name);
    }
    assert!(builtin_grammar("gcpx").is_none());
}

/// Section 13.3's gcp table, which reproduces section 2.4: name, role, tone, tintable,
/// default tint, border pattern, border width, padding, radius, label style, parents.
#[allow(clippy::type_complexity)]
const GCP_TABLE: [(
    &str,
    Role,
    Option<Tone>,
    bool,
    Option<u8>,
    BorderPattern,
    f32,
    f32,
    f32,
    LabelStyle,
    &[&str],
); 10] = [
    (
        "gcp",
        Role::Frame,
        None,
        false,
        None,
        BorderPattern::Solid,
        3.0,
        0.0,
        10.0,
        LabelStyle::Bar,
        &["page"],
    ),
    (
        "vpc",
        Role::Boundary,
        Some(Tone::Strong),
        false,
        None,
        BorderPattern::Dashed,
        2.0,
        10.0,
        8.0,
        LabelStyle::Plain,
        &["gcp", "project", "perimeter"],
    ),
    (
        "region",
        Role::Group,
        Some(Tone::Neutral),
        true,
        Some(1),
        BorderPattern::Solid,
        1.5,
        12.0,
        8.0,
        LabelStyle::Plain,
        &["gcp", "vpc", "perimeter", "project"],
    ),
    (
        "subnet",
        Role::Group,
        Some(Tone::Cool),
        false,
        None,
        BorderPattern::Dashed,
        1.5,
        12.0,
        8.0,
        LabelStyle::Plain,
        &["region", "vpc", "project"],
    ),
    (
        "onprem",
        Role::Group,
        Some(Tone::Warm),
        true,
        None,
        BorderPattern::Solid,
        1.5,
        12.0,
        8.0,
        LabelStyle::Plain,
        &["page", "optional"],
    ),
    (
        "project",
        Role::Tile,
        Some(Tone::Highlight),
        false,
        None,
        BorderPattern::Solid,
        1.5,
        12.0,
        8.0,
        LabelStyle::Plain,
        &["page", "gcp", "vpc", "perimeter", "project"],
    ),
    (
        "optional",
        Role::Group,
        Some(Tone::Emphasis),
        false,
        None,
        BorderPattern::Dashed,
        2.0,
        12.0,
        8.0,
        LabelStyle::Plain,
        &[
            "page",
            "gcp",
            "vpc",
            "region",
            "subnet",
            "project",
            "perimeter",
        ],
    ),
    (
        "k8s",
        Role::Group,
        Some(Tone::Soft),
        false,
        None,
        BorderPattern::None,
        0.0,
        12.0,
        8.0,
        LabelStyle::Plain,
        &[
            "gcp",
            "vpc",
            "region",
            "subnet",
            "project",
            "perimeter",
            "optional",
        ],
    ),
    (
        "perimeter",
        Role::Boundary,
        Some(Tone::Accent),
        false,
        None,
        BorderPattern::Dashed,
        2.5,
        12.0,
        10.0,
        LabelStyle::Accent,
        &["gcp", "vpc", "project"],
    ),
    (
        "apis",
        Role::Group,
        Some(Tone::Neutral),
        false,
        None,
        BorderPattern::Solid,
        1.5,
        12.0,
        8.0,
        LabelStyle::Plain,
        &["gcp", "project", "perimeter"],
    ),
];

#[test]
fn gcp_container_kinds_equal_the_section_13_3_table_field_for_field() {
    let grammar = gcp();
    assert_eq!(grammar.containers.len(), GCP_TABLE.len());
    for (container, row) in grammar.containers.iter().zip(GCP_TABLE) {
        let (
            name,
            role,
            tone,
            tintable,
            default_tint,
            pattern,
            width,
            padding,
            radius,
            label,
            parents,
        ) = row;
        assert_eq!(container.name, name);
        assert_eq!(container.role, role, "{name}");
        assert_eq!(container.tone, tone, "{name}");
        assert_eq!(container.tintable, tintable, "{name}");
        assert_eq!(container.default_tint, default_tint, "{name}");
        assert_eq!(container.border.pattern, pattern, "{name}");
        assert_eq!(container.border.width, width, "{name}");
        assert_eq!(container.padding, padding, "{name}");
        assert_eq!(container.radius, radius, "{name}");
        assert_eq!(container.label, label, "{name}");
        assert_eq!(container.parents, parents, "{name}");
    }
}

#[test]
fn gcp_has_a_product_kind_with_every_icon_once_in_icon_order_and_a_person_and_a_device() {
    let grammar = gcp();
    let names: Vec<&str> = grammar
        .items
        .iter()
        .map(|item| item.name.as_str())
        .collect();
    assert_eq!(names, ["product", "person", "device"]);
    for (name, shape) in [("person", Shape::Figure), ("device", Shape::Laptop)] {
        let kind = grammar.item(name).unwrap();
        assert_eq!(kind.icons, IconPack::None, "{name}");
        assert!(kind.products.is_empty(), "{name}");
        assert_eq!(kind.shape, Some(shape), "{name}");
    }
    let product = &grammar.items[0];
    assert_eq!(product.name, "product");
    assert_eq!(product.shape, Some(Shape::Block));
    assert_eq!(product.icons, IconPack::Gcp);
    let mut expected_parents: Vec<&str> = GCP_TABLE.iter().map(|row| row.0).collect();
    expected_parents.push("page");
    assert_eq!(product.parents, expected_parents);
    let icons: Vec<IconName> = product.products.iter().map(|row| row.icon).collect();
    assert_eq!(icons, IconName::ALL);
    let product_class: Vec<&str> = product
        .products
        .iter()
        .filter(|row| row.class == IconClass::Product)
        .map(|row| row.icon.as_str())
        .collect();
    assert_eq!(
        product_class,
        [
            "bigquery",
            "cloud-run-flat",
            "cloud-run",
            "cloud-sql",
            "cloud-storage",
            "compute-engine",
            "gke",
            "scc",
            "vertex-ai"
        ]
    );
}

#[test]
fn plain_has_four_containers_and_five_icon_free_items() {
    let grammar = builtin_grammar("plain").unwrap().unwrap();
    let containers: Vec<(&str, Role)> = grammar
        .containers
        .iter()
        .map(|container| (container.name.as_str(), container.role))
        .collect();
    assert_eq!(
        containers,
        [
            ("system", Role::Frame),
            ("boundary", Role::Boundary),
            ("group", Role::Group),
            ("tile", Role::Tile)
        ]
    );
    let items: Vec<&str> = grammar
        .items
        .iter()
        .map(|item| item.name.as_str())
        .collect();
    assert_eq!(items, ["service", "store", "external", "person", "device"]);
    assert_eq!(grammar.item("person").unwrap().shape, Some(Shape::Figure));
    assert_eq!(grammar.item("device").unwrap().shape, Some(Shape::Laptop));
    for name in ["service", "store", "external"] {
        assert_eq!(
            grammar.item(name).unwrap().shape,
            Some(Shape::Block),
            "{name}"
        );
    }
    assert!(
        grammar
            .items
            .iter()
            .all(|item| item.icons == IconPack::None && item.products.is_empty())
    );
    assert!(grammar.remembered.is_empty());
}

#[test]
fn grammar_schema_equals_the_committed_file() {
    let generated = serde_json::to_string_pretty(&grammar_schema()).unwrap() + "\n";
    assert!(
        generated == COMMITTED_GRAMMAR_SCHEMA,
        "schema/grammar.schema.json is out of date; regenerate it from grammar_schema()"
    );
}

#[test]
fn an_unknown_field_is_a_json_error_naming_the_origin() {
    let mut document = gcp_value();
    document["containers"][0]["color"] = json!("blue");
    let error = parse_grammar(&document.to_string(), "fixtures/odd.json").unwrap_err();
    assert!(matches!(error, GrammarError::Json { .. }));
    assert!(
        error
            .to_string()
            .starts_with("grammar fixtures/odd.json is not valid grammar JSON")
    );
}

#[test]
#[allow(clippy::type_complexity)]
fn each_grammar_fault_is_reported_with_its_rule() {
    let cases: [(&str, fn(&mut Value), &str, &str); 9] = [
        (
            "duplicate name",
            |grammar| {
                let copy = grammar["containers"][0].clone();
                grammar["containers"].as_array_mut().unwrap().push(copy);
            },
            "grammar-name-duplicate",
            "/containers/10/name",
        ),
        (
            "unknown parent",
            |grammar| grammar["containers"][1]["parents"][0] = json!("cloud"),
            "grammar-parent-unknown",
            "/containers/1/parents/0",
        ),
        (
            "frame with a tone",
            |grammar| grammar["containers"][0]["tone"] = json!("neutral"),
            "grammar-role-mismatch",
            "/containers/0/tone",
        ),
        (
            "group without a tone",
            |grammar| {
                grammar["containers"][2]
                    .as_object_mut()
                    .unwrap()
                    .remove("tone");
            },
            "grammar-role-mismatch",
            "/containers/2/role",
        ),
        (
            "default tint on an untintable kind",
            |grammar| grammar["containers"][1]["default_tint"] = json!(2),
            "grammar-default-tint-untintable",
            "/containers/1/default_tint",
        ),
        (
            "solid border of width 0",
            |grammar| grammar["containers"][2]["border"]["width"] = json!(0),
            "grammar-border-mismatch",
            "/containers/2/border",
        ),
        (
            "item with icons none and a products row",
            |grammar| grammar["items"][0]["icons"] = json!("none"),
            "grammar-icon-table",
            "/items/0/products",
        ),
        (
            "repeated icon",
            |grammar| grammar["items"][0]["products"][1]["icon"] = json!("agents"),
            "grammar-icon-table",
            "/items/0/products/1/icon",
        ),
        (
            "repeated remembered literal",
            |grammar| grammar["remembered"][1]["literal"] = json!("64512"),
            "grammar-remembered-duplicate",
            "/remembered/1/literal",
        ),
    ];
    for (name, fault, rule, pointer) in cases {
        let mut document = gcp_value();
        fault(&mut document);
        assert_eq!(
            grammar_faults(&document),
            [(rule, pointer.to_string())],
            "{name}"
        );
    }
}

#[test]
fn a_grammar_with_no_kind_under_page_is_rejected() {
    let mut document = gcp_value();
    for container in document["containers"].as_array_mut().unwrap() {
        let parents = container["parents"].as_array_mut().unwrap();
        parents.retain(|parent| parent != "page");
        if parents.is_empty() {
            parents.push(json!("vpc"));
        }
    }
    for item in document["items"].as_array_mut().unwrap() {
        item["parents"] = json!(["vpc"]);
    }
    assert_eq!(
        grammar_faults(&document),
        [("grammar-no-top-level", String::new())]
    );
}

#[test]
fn every_fault_of_one_grammar_is_reported_in_field_order() {
    let mut document = gcp_value();
    document["remembered"][1]["literal"] = json!("64512");
    document["containers"][0]["tone"] = json!("neutral");
    document["items"][0]["parents"][0] = json!("cloud");
    let pointers: Vec<String> = grammar_faults(&document)
        .into_iter()
        .map(|(_, pointer)| pointer)
        .collect();
    assert_eq!(
        pointers,
        [
            "/containers/0/tone",
            "/items/0/parents/0",
            "/remembered/1/literal"
        ]
    );
}

#[test]
fn container_and_item_lookups_find_kinds_by_name() {
    let grammar = gcp();
    assert_eq!(grammar.container("perimeter").unwrap().radius, 10.0);
    assert!(grammar.container("product").is_none());
    assert!(grammar.item("product").is_some());
    assert!(grammar.item("region").is_none());
}
