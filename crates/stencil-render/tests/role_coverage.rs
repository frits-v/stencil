#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 13.4: every `#Theme` role is read by the renderer, and no color is named in
//! renderer code. Each leaf of a theme with every optional role set is changed in turn; the
//! change must show in the SVG of a figure that draws every role, flat or iso.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use stencil_layout::layout_page;
use stencil_model::grammar::{BorderPattern, KindBorder};
use stencil_model::text::FixedMetricsMeasurer;
use stencil_model::{Grammar, Page, Theme, validate_theme};
use stencil_render::render_svg;

fn pipe(line: &str, tint: Option<u8>, arrow: &str) -> Value {
    let mut node = json!({ "tag": "Pipe", "dir": "h", "line": line, "label": "tag", "sub": "sub", "arrow": arrow });
    if let Some(tint) = tint {
        node["tint"] = json!(tint);
    }
    node
}

fn boxed(kind: &str, tint: Option<u8>, children: Value) -> Value {
    let mut node = json!({ "tag": "Box", "kind": kind, "label": kind, "children": children });
    if let Some(tint) = tint {
        node["tint"] = json!(tint);
    }
    node
}

fn fact() -> Value {
    json!([{ "tag": "Fact", "text": "fact" }])
}

/// A gcp figure that draws every container tone, every tint slot, every line in every slot,
/// an item with an icon, a doc fact and an ask, a Fact, each block and the full chrome.
fn every_role_document(canvas: &str, projection: &str) -> Value {
    let mut pipes = vec![
        pipe("gray", None, "none"),
        pipe("deny", None, "end"),
        pipe("dash", Some(1), "end"),
    ];
    for slot in 1..=8 {
        pipes.push(pipe("solid", Some(slot), "end"));
        pipes.push(pipe("solid", Some(slot), "none"));
        pipes.push(pipe("dash", Some(slot), "none"));
    }
    let regions: Vec<Value> = (1..=8)
        .map(|slot| boxed("region", Some(slot), fact()))
        .collect();
    let mut legend = vec![
        json!({ "line": "gray", "text": "gray" }),
        json!({ "line": "deny", "text": "deny" }),
    ];
    for slot in 1..=8 {
        legend.push(json!({ "line": "solid", "tint": slot, "text": "solid" }));
    }
    for slot in 1..=6 {
        legend.push(json!({ "line": "dash", "tint": slot, "text": "dash" }));
    }
    let item = json!({
        "tag": "Item", "kind": "product", "icon": "bigquery", "title": "Warehouse",
        "subtitle": "BigQuery", "facts": [{ "text": "a fact" }, { "text": "an ask", "source": "ask" }]
    });
    let nested = boxed(
        "gcp",
        None,
        json!([boxed(
            "vpc",
            None,
            json!([boxed(
                "region",
                Some(1),
                json!([boxed("subnet", None, json!([item]))])
            )])
        )]),
    );
    let mut document = common::page_document(
        json!([
            { "tag": "Col", "children": [
                nested,
                boxed("onprem", None, fact()),
                boxed("project", None, fact()),
                boxed("optional", None, fact()),
                boxed("k8s", None, fact()),
                boxed("perimeter", None, fact()),
                boxed("apis", None, fact()),
                { "tag": "Col", "children": regions }
            ] },
            { "tag": "Col", "children": pipes },
            { "tag": "Col", "children": [
                { "tag": "Fact", "text": "a doc fact" },
                { "tag": "Text", "heading": "Notes", "list": "bulleted", "body": ["one", "two"] },
                { "tag": "Callout", "kind": "note", "text": "note" },
                { "tag": "Callout", "kind": "risk", "text": "risk" },
                { "tag": "Callout", "kind": "decision", "text": "decision" },
                { "tag": "Callout", "kind": "open", "text": "open" },
                { "tag": "Frame", "label": "Screen", "height": 80 },
                { "tag": "Note", "kind": "legend", "text": "a legend note" }
            ] }
        ]),
        json!(legend),
    );
    document["canvas"] = json!(canvas);
    document["projection"] = json!(projection);
    document["foot"] = json!("foot");
    document
}

/// The figures the coverage renders, each with the grammar it is laid out under: the gcp
/// figure flat in both canvases and in iso, the plain sequence example for the lanes, and a
/// soft-tone Box under a grammar that gives that tone a solid border, which no built-in
/// grammar does.
fn figures() -> Vec<(Page, Grammar)> {
    let page = |document: Value| -> Page { serde_json::from_value(document).unwrap() };
    let mut figures: Vec<(Page, Grammar)> = [
        every_role_document("customer", "flat"),
        every_role_document("internal", "flat"),
        every_role_document("customer", "iso"),
        serde_json::from_str(include_str!("../../../examples/sequence.json")).unwrap(),
    ]
    .into_iter()
    .map(|document| {
        let page = page(document);
        let grammar = common::grammar_for(&page);
        (page, grammar)
    })
    .collect();
    let mut bordered = common::gcp_anywhere();
    for container in &mut bordered.containers {
        if container.name == "k8s" {
            container.border = KindBorder {
                pattern: BorderPattern::Solid,
                width: 1.5,
            };
        }
    }
    let soft = page(common::page_document(
        json!([boxed("k8s", None, fact())]),
        json!([]),
    ));
    figures.push((soft, bordered));
    figures
}

/// Center with every optional role set, so every role has a value to change.
fn full_theme() -> Value {
    let mut theme = serde_json::to_value(common::theme("center")).unwrap();
    let stroke = json!({ "color": "#3C4043", "width": 1.0, "pattern": "solid" });
    theme["badge"]["border"] = stroke.clone();
    theme["icon_chip"] = json!("#F1F3F4");
    theme["frame"]["bar_rule"] = stroke.clone();
    for tone in [
        "neutral",
        "warm",
        "cool",
        "soft",
        "highlight",
        "emphasis",
        "accent",
    ] {
        theme["tones"][tone]["fill"] = json!("#F8F9FA");
        theme["tones"][tone]["border"] = json!("#80868B");
        theme["tones"][tone]["label_ink"] = json!("#3C4043");
    }
    theme["tones"]["strong"]["label_ink"] = json!("#3C4043");
    theme["containers"]["draw_width"] = json!(1.75);
    theme["containers"]["frame_draw_width"] = json!(2.25);
    theme["containers"]["borderless_outline"] = stroke.clone();
    theme["iso"]["ring"] = json!({ "color": "#999999", "width": 1.5, "pattern": "dotted" });
    theme["iso"]["block_outline"] = stroke;
    theme
}

/// The full theme drawn with outlined solid edges and a neutral tone without a border, so
/// `edge_width` and the slot borders are read.
fn outlined_theme() -> Value {
    let mut theme = full_theme();
    theme["iso"]["solid_edges"] = json!("outline");
    theme["tones"]["neutral"]
        .as_object_mut()
        .unwrap()
        .remove("border");
    theme
}

/// Every leaf of `value` with its pointer.
fn leaves(value: &Value, pointer: String, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(fields) => {
            for (key, field) in fields {
                leaves(field, format!("{pointer}/{key}"), out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                leaves(item, format!("{pointer}/{index}"), out);
            }
        }
        leaf => out.push((pointer, leaf.clone())),
    }
}

/// A different, still valid value for the leaf at `pointer`.
fn mutated(pointer: &str, value: &Value) -> Value {
    match value {
        Value::Bool(flag) => json!(!flag),
        Value::String(text) if text.starts_with('#') => json!("#010203"),
        Value::String(text) => match text.as_str() {
            "solid" => json!("dashed"),
            "dashed" => json!("dotted"),
            "dotted" => json!("solid"),
            "filled" => json!("hollow"),
            "hollow" => json!("none"),
            "none" if pointer.ends_with("solid_edges") => json!("outline"),
            "none" => json!("filled"),
            "outline" => json!("none"),
            "color" => json!("line"),
            "line" => json!("color"),
            _ => json!("mutant"),
        },
        Value::Number(number) if pointer.contains("/faces/") => {
            json!(number.as_i64().unwrap() + 1)
        }
        Value::Number(number) if pointer.ends_with("opacity") => {
            json!(number.as_f64().unwrap() - 0.05)
        }
        Value::Number(number) if number.as_f64().unwrap() >= 3.9 => {
            json!(number.as_f64().unwrap() - 0.25)
        }
        Value::Number(number) => json!(number.as_f64().unwrap() + 0.25),
        other => panic!("{pointer}: unexpected leaf {other}"),
    }
}

fn svgs(theme: &Theme, figures: &[(Page, Grammar)]) -> Vec<String> {
    figures
        .iter()
        .map(|(page, grammar)| {
            let geometry =
                layout_page(page, grammar, &mut FixedMetricsMeasurer::default()).unwrap();
            render_svg(page, theme, &geometry).unwrap().svg
        })
        .collect()
}

fn parse(value: &Value) -> Theme {
    let theme: Theme = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(validate_theme(&theme), Vec::new());
    theme
}

#[test]
fn every_theme_role_changes_the_drawing() {
    let pages = figures();
    let variants = [full_theme(), outlined_theme()];
    let mut all_leaves = Vec::new();
    leaves(&variants[0], String::new(), &mut all_leaves);
    let mut read: Vec<String> = Vec::new();
    for variant in &variants {
        let base = svgs(&parse(variant), &pages);
        let mut variant_leaves = Vec::new();
        leaves(variant, String::new(), &mut variant_leaves);
        for (pointer, value) in variant_leaves {
            if read.contains(&pointer) {
                continue;
            }
            let mut changed = variant.clone();
            *changed.pointer_mut(&pointer).unwrap() = mutated(&pointer, &value);
            if svgs(&parse(&changed), &pages) != base {
                read.push(pointer);
            }
        }
    }
    let unread: Vec<&str> = all_leaves
        .iter()
        .map(|(pointer, _)| pointer.as_str())
        .filter(|pointer| !read.iter().any(|read| read == pointer))
        .collect();
    assert_eq!(unread, Vec::<&str>::new(), "roles the renderer never reads");
    assert!(read.len() > 150, "examined only {} roles", read.len());
}

/// Every `.rs` file under `directory`, at most `limit` deep.
fn rust_files(directory: &Path, limit: usize, out: &mut Vec<PathBuf>) {
    if limit == 0 {
        return;
    }
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, limit - 1, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_renderer_source_names_a_color_literal() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&source, 4, &mut files);
    assert!(files.len() >= 10, "examined only {} files", files.len());
    let mut literals = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        let bytes = text.as_bytes();
        for (index, byte) in bytes.iter().enumerate() {
            let digits = bytes.get(index + 1..index + 7);
            if *byte == b'#'
                && digits.is_some_and(|digits| digits.iter().all(u8::is_ascii_hexdigit))
            {
                let line = text[..index].lines().count();
                literals.push(format!("{}:{line}", file.display()));
            }
        }
    }
    assert_eq!(literals, Vec::<String>::new());
}
