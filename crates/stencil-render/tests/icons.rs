#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;
use sha2::{Digest, Sha256};
use stencil_model::IconName;
use stencil_render::{DeviceScale, icon_data_uri, icon_svg_bytes, render_png};

/// Section 8.2: file name and SHA-256 of each icon.
const ICON_HASHES: [(&str, &str); 23] = [
    (
        "agents.svg",
        "02b04606fa62689b6add0235489e58c9dfb19b63ba54f6799e51547619e76b46",
    ),
    (
        "ai-ml.svg",
        "59b77ac8d8df60438120c83a65fcb68486e576186beba7f37dcb7bb484a0f2f0",
    ),
    (
        "bigquery.svg",
        "6c088a2a7afbfcab01918c959aad90d76835093cfc2ec8314bf7ba25b85aaae8",
    ),
    (
        "cloud-run-flat.svg",
        "42a910e5d9f5a8685227f91623bf50c8c9259c556a41eb9a97157fd1e78b0cea",
    ),
    (
        "cloud-run.svg",
        "42a910e5d9f5a8685227f91623bf50c8c9259c556a41eb9a97157fd1e78b0cea",
    ),
    (
        "cloud-sql.svg",
        "4e0d3d2049a67f64e5a2c88f836bf2d1786f809a72d82279b18906ec9d6b35c9",
    ),
    (
        "cloud-storage.svg",
        "df42fc4b652cb133a4150bea7988aa965651d2c52fdfd141b6e8a359552eec8e",
    ),
    (
        "compute-engine.svg",
        "5cf9e57e4f99e125e45f9db269f3d5d1fa0dfd187aebfe00f3f97b9caf488af2",
    ),
    (
        "compute.svg",
        "6551c59efcd0582dd4b7286ab5c031567468ad0b70921cc02fa4601d4e4af28e",
    ),
    (
        "containers.svg",
        "809477486d9ed5bfbdecacd262c9082306186a90f89e620ef4ecfee300d02459",
    ),
    (
        "data-analytics.svg",
        "b32c8b4702fb90e9669e841e0410687471b7ca97ad236527f6dad8e3d90c87d9",
    ),
    (
        "databases.svg",
        "f9ecdabe95c8c30cd6adabcb3e521f7fe462eacd63ce19021222a2a1b41a7266",
    ),
    (
        "devops.svg",
        "b08a5231acec916701acec3a2e0003cc3a4a29ddf3fc66f24f3c90e51d40587d",
    ),
    (
        "gke.svg",
        "6a33654c499bc4c38012661875a1a8c1bf2cdf01be8fdcc90d1c6504657878ec",
    ),
    (
        "hybrid.svg",
        "66389f10f6f3d8a4a05b05ce883543783ed4e407f4c696c5777bb09671a72a23",
    ),
    (
        "integration.svg",
        "f4a14bb1f46b6b8ec2dd8bd69d8e37e9fac8f161801b781cf6e1f41772fc2101",
    ),
    (
        "networking.svg",
        "c88d1b7d3bcba21ba984d92a7762e1f940b70d922f83449370870f76caaf5787",
    ),
    (
        "observability.svg",
        "f22fe17c2e91d65d3109e499bb3b5d8d60216f678bf9fa41c541de7d8254216a",
    ),
    (
        "scc.svg",
        "d7cadf3c5bd7d4fea4bab88d72d3dff276f29728c86d1b8c5d99bede796ca542",
    ),
    (
        "security-identity.svg",
        "4e233138e1170653522e4261779d51bb8f1383255845aba893bbb2381fa48e0e",
    ),
    (
        "serverless.svg",
        "971fec343a13520b4f1edb49f34d33f5611ca427a9b09e36c4537b07e9d62721",
    ),
    (
        "storage.svg",
        "d99325d1f951ec5ddff47e1d787a90acd819449ee38c6ce20a981e803f5d7ed1",
    ),
    (
        "vertex-ai.svg",
        "17922247f3110026fd637c531d0604c67a00c9a58a6e152030f2242b9fd48a8e",
    ),
];

const DATA_URI_PREFIX: &str = "data:image/svg+xml;base64,";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn expected_hash(icon: IconName) -> &'static str {
    ICON_HASHES
        .iter()
        .find(|(file_name, _)| *file_name == icon.file_name())
        .map(|(_, hash)| *hash)
        .unwrap_or_else(|| panic!("{} is not in the section 8.2 table", icon.file_name()))
}

#[test]
fn table_covers_every_icon_once() {
    assert_eq!(IconName::ALL.len(), ICON_HASHES.len());
    for icon in IconName::ALL {
        let rows = ICON_HASHES
            .iter()
            .filter(|(file_name, _)| *file_name == icon.file_name())
            .count();
        assert_eq!(rows, 1, "{}", icon.file_name());
    }
}

#[test]
fn embedded_icon_bytes_match_section_8_2_hashes() {
    for icon in IconName::ALL {
        assert_eq!(
            sha256_hex(icon_svg_bytes(icon)),
            expected_hash(icon),
            "{}",
            icon.file_name()
        );
    }
}

#[test]
fn embedded_icon_bytes_equal_the_asset_files() {
    let icons_directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons");
    for icon in IconName::ALL {
        let on_disk = std::fs::read(icons_directory.join(icon.file_name())).unwrap();
        assert_eq!(
            icon_svg_bytes(icon),
            on_disk.as_slice(),
            "{}",
            icon.file_name()
        );
    }
}

#[test]
fn data_uri_decodes_to_the_exact_icon_bytes() {
    for icon in IconName::ALL {
        let uri = icon_data_uri(icon);
        let payload = uri
            .strip_prefix(DATA_URI_PREFIX)
            .unwrap_or_else(|| panic!("{uri:.40} lacks the data URI prefix"));
        let decoded = STANDARD.decode(payload).unwrap();
        assert_eq!(
            sha256_hex(&decoded),
            expected_hash(icon),
            "{}",
            icon.file_name()
        );
    }
}

#[test]
fn a_card_for_each_icon_renders_to_png() {
    let cards: Vec<_> = IconName::ALL
        .iter()
        .map(|icon| {
            let stem = icon.file_name().trim_end_matches(".svg");
            json!({ "tag": "Item", "kind": "product", "icon": stem, "title": format!("Card {stem}") })
        })
        .collect();
    let body = json!([
        { "tag": "Col", "children": cards },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" }
    ]);
    let legend = json!([{ "line": "solid", "tint": 1, "text": "request path" }]);
    let rendered = common::render_document_with_fixed_metrics(common::page_document(body, legend));

    let document = common::parse_xml(&rendered.svg.svg);
    let images: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("image"))
        .collect();
    assert_eq!(images.len(), IconName::ALL.len());
    for (image, icon) in images.iter().zip(IconName::ALL) {
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
