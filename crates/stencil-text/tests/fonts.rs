#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use sha2::{Digest, Sha256};
use stencil_model::text::FontWeight;
use stencil_text::{BUNDLED_FONTS, verify_bundled_fonts};

const FONTS_MD: &str = include_str!("../../../assets/fonts/FONTS.md");

fn lowercase_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// (file name, weight, sha256) rows of the FONTS.md table.
fn fonts_md_rows() -> Vec<(String, u16, String)> {
    FONTS_MD
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line
                .split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect();
            assert_eq!(cells.len(), 3, "unexpected FONTS.md row {line:?}");
            (
                cells[0].trim_matches('`').to_string(),
                cells[1].parse().unwrap(),
                cells[2].trim_matches('`').to_string(),
            )
        })
        .collect()
}

#[test]
fn bundled_bytes_hash_to_their_listed_sha256() {
    for font in &BUNDLED_FONTS {
        let digest = Sha256::digest(font.bytes);
        assert_eq!(
            lowercase_hex(&digest),
            font.sha256,
            "{} bytes do not match its sha256",
            font.file_name
        );
    }
}

#[test]
fn bundled_hashes_and_weights_equal_fonts_md() {
    let rows = fonts_md_rows();
    assert_eq!(rows.len(), BUNDLED_FONTS.len());
    for (font, (file_name, weight, sha256)) in BUNDLED_FONTS.iter().zip(&rows) {
        assert_eq!(font.file_name, file_name);
        assert_eq!(font.weight.css_value(), *weight);
        assert_eq!(font.sha256, sha256);
    }
}

#[test]
fn hash_check_rejects_a_changed_byte() {
    let font = &BUNDLED_FONTS[0];
    let mut changed = font.bytes.to_vec();
    changed[0] ^= 0x01;
    assert_ne!(lowercase_hex(&Sha256::digest(&changed)), font.sha256);
}

#[test]
fn bundled_fonts_verify() {
    verify_bundled_fonts().unwrap();
}

#[test]
fn bundled_fonts_cover_the_four_weights_in_order() {
    let weights: Vec<FontWeight> = BUNDLED_FONTS.iter().map(|font| font.weight).collect();
    assert_eq!(
        weights,
        vec![
            FontWeight::Regular,
            FontWeight::SemiBold,
            FontWeight::Bold,
            FontWeight::ExtraBold
        ]
    );
}
