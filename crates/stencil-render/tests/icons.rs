#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;
use sha2::{Digest, Sha256};
use stencil_model::{IconName, IconPack};
use stencil_render::{DeviceScale, icon_data_uri, icon_svg_bytes, render_png};

const DATA_URI_PREFIX: &str = "data:image/svg+xml;base64,";

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The section 8.2 tables of docs/SPEC.md: for each `#### Pack <name>` heading, the file
/// name and SHA-256 of every row under it, in table order.
fn section_8_2_tables() -> BTreeMap<String, Vec<(String, String)>> {
    let spec = fs::read_to_string(repository_path("docs/SPEC.md")).unwrap();
    let section = spec
        .split_once("\n### 8.2 Icons\n")
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.split_once("\n### ").map(|(section, _)| section))
        .expect("docs/SPEC.md has a section 8.2 followed by another section");
    let mut tables: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut pack: Option<String> = None;
    for line in section.lines() {
        if let Some(name) = line.strip_prefix("#### Pack ") {
            pack = Some(name.trim().to_string());
            tables.entry(name.trim().to_string()).or_default();
            continue;
        }
        let Some(row) = line.strip_prefix("| `") else {
            continue;
        };
        let cells: Vec<&str> = row
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let file_name = cells[0].trim_end_matches('`');
        let hash = cells.last().unwrap().trim_matches('`');
        let pack = pack
            .as_ref()
            .unwrap_or_else(|| panic!("row {file_name} sits under no #### Pack heading"));
        tables
            .get_mut(pack)
            .unwrap()
            .push((file_name.to_string(), hash.to_string()));
    }
    tables
}

/// Every bundled icon with its section 8.2 hash; fails on a bundled pack whose table has no
/// rows, on an icon with no row or two, and on a row for no icon.
fn icons_with_hashes() -> Vec<(IconName, String)> {
    let tables = section_8_2_tables();
    let bundled: Vec<&str> = IconPack::BUNDLED.iter().map(|pack| pack.as_str()).collect();
    let headed: Vec<&str> = tables.keys().map(String::as_str).collect();
    let mut sorted_bundled = bundled.clone();
    sorted_bundled.sort_unstable();
    assert_eq!(
        headed, sorted_bundled,
        "section 8.2 heads one table per bundled pack"
    );
    let mut icons = Vec::new();
    for pack in IconPack::BUNDLED {
        let rows = &tables[pack.as_str()];
        assert!(
            !rows.is_empty(),
            "section 8.2 has no rows for pack {}",
            pack.as_str()
        );
        let pack_icons = pack.icons();
        assert_eq!(
            rows.len(),
            pack_icons.len(),
            "rows of pack {}",
            pack.as_str()
        );
        for icon in pack_icons {
            let matching: Vec<&(String, String)> = rows
                .iter()
                .filter(|(file_name, _)| file_name == icon.file_name())
                .collect();
            assert_eq!(matching.len(), 1, "{} {}", pack.as_str(), icon.file_name());
            icons.push((icon, matching[0].1.clone()));
        }
    }
    icons
}

#[test]
fn section_8_2_has_one_row_for_every_icon_of_every_bundled_pack() {
    let icons = icons_with_hashes();
    println!(
        "examined {} icons in {} packs",
        icons.len(),
        IconPack::BUNDLED.len()
    );
    assert!(!icons.is_empty());
}

#[test]
fn embedded_icon_bytes_match_section_8_2_hashes() {
    let icons = icons_with_hashes();
    for (icon, hash) in &icons {
        assert_eq!(
            sha256_hex(icon_svg_bytes(*icon)),
            *hash,
            "{} {}",
            icon.pack().as_str(),
            icon.file_name()
        );
    }
    println!("examined {} icons", icons.len());
    assert!(!icons.is_empty());
}

#[test]
fn embedded_icon_bytes_equal_the_asset_files() {
    let mut examined = 0;
    for pack in IconPack::BUNDLED {
        let directory = repository_path("assets/icons").join(pack.as_str());
        for icon in pack.icons() {
            let on_disk = fs::read(directory.join(icon.file_name())).unwrap();
            assert_eq!(
                icon_svg_bytes(icon),
                on_disk.as_slice(),
                "{} {}",
                pack.as_str(),
                icon.file_name()
            );
            examined += 1;
        }
    }
    println!("examined {examined} icons");
    assert!(examined > 0);
}

#[test]
fn data_uri_decodes_to_the_exact_icon_bytes() {
    for (icon, hash) in icons_with_hashes() {
        let uri = icon_data_uri(icon);
        let payload = uri
            .strip_prefix(DATA_URI_PREFIX)
            .unwrap_or_else(|| panic!("{uri:.40} lacks the data URI prefix"));
        let decoded = STANDARD.decode(payload).unwrap();
        assert_eq!(sha256_hex(&decoded), hash, "{}", icon.file_name());
    }
}

#[test]
fn a_card_for_each_icon_renders_to_png() {
    let gcp_icons = IconPack::Gcp.icons();
    let cards: Vec<_> = gcp_icons
        .iter()
        .map(|icon| {
            let stem = icon.as_str();
            json!({ "tag": "Item", "kind": "product", "icon": stem, "title": format!("Card {stem}") })
        })
        .collect();
    let body = json!([
        { "tag": "Col", "children": cards },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" }
    ]);
    let legend = json!([{ "line": "solid", "tint": 1, "text": "request path" }]);
    let mut document = common::page_document(body, legend);
    document["grammar"] = json!("gcp");
    let rendered = common::render_document_with_fixed_metrics(document);

    let document = common::parse_xml(&rendered.svg.svg);
    let images: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("image"))
        .collect();
    assert_eq!(images.len(), gcp_icons.len());
    for (image, icon) in images.iter().zip(gcp_icons) {
        assert_eq!(image.attribute("href"), Some(icon_data_uri(icon).as_str()));
        assert_eq!(image.attribute("width"), Some("28"));
        assert_eq!(image.attribute("height"), Some("28"));
    }

    let png = render_png(
        &rendered.svg.svg,
        rendered.svg.text_elements,
        DeviceScale::new(1).unwrap(),
    )
    .unwrap();
    let pixmap = common::decode_png(&png);
    assert!(pixmap.width() > 0 && pixmap.height() > 0);
}
