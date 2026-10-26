//! Text measurement on cosmic-text over the bundled Inter and JetBrains Mono NL faces (spec
//! sections 3.2, 4.3 and 8.1).

mod measurer;

use std::sync::Arc;

use cosmic_text::fontdb;
use stencil_model::text::{FontFamily, FontWeight};

pub use measurer::CosmicTextMeasurer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontFile {
    pub file_name: &'static str,
    pub family: FontFamily,
    pub weight: FontWeight,
    pub bytes: &'static [u8],
    /// Lowercase hex SHA-256 of `bytes`, copied from assets/fonts/FONTS.md.
    pub sha256: &'static str,
}

/// The four Inter faces, then JetBrains Mono NL Regular and Bold, via include_bytes!, in
/// FONTS.md order.
pub const BUNDLED_FONTS: [FontFile; 6] = [
    FontFile {
        file_name: "Inter-Regular.ttf",
        family: FontFamily::Inter,
        weight: FontWeight::Regular,
        bytes: include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
        sha256: "40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82",
    },
    FontFile {
        file_name: "Inter-SemiBold.ttf",
        family: FontFamily::Inter,
        weight: FontWeight::SemiBold,
        bytes: include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"),
        sha256: "78a843fade9d4612a5567302fb595b56976eb5fcebf4fea5a5912d638bafcde3",
    },
    FontFile {
        file_name: "Inter-Bold.ttf",
        family: FontFamily::Inter,
        weight: FontWeight::Bold,
        bytes: include_bytes!("../../../assets/fonts/Inter-Bold.ttf"),
        sha256: "288316099b1e0a47a4716d159098005eef7c0066921f34e3200393dbdb01947f",
    },
    FontFile {
        file_name: "Inter-ExtraBold.ttf",
        family: FontFamily::Inter,
        weight: FontWeight::ExtraBold,
        bytes: include_bytes!("../../../assets/fonts/Inter-ExtraBold.ttf"),
        sha256: "e6756ad5690b77606aa62249a7b420d9902d45cae4b0048a24911fd4324b0a22",
    },
    FontFile {
        file_name: "JetBrainsMonoNL-Regular.ttf",
        family: FontFamily::JetBrainsMonoNl,
        weight: FontWeight::Regular,
        bytes: include_bytes!("../../../assets/fonts/JetBrainsMonoNL-Regular.ttf"),
        sha256: "fb3b2575d7b0657359707993288f12a7360344d39387bb26050e276d61f6bd2a",
    },
    FontFile {
        file_name: "JetBrainsMonoNL-Bold.ttf",
        family: FontFamily::JetBrainsMonoNl,
        weight: FontWeight::Bold,
        bytes: include_bytes!("../../../assets/fonts/JetBrainsMonoNL-Bold.ttf"),
        sha256: "0198e841824025f8876e5c297f0b9b497ee8d6eb9969710a3328e1303f996ec3",
    },
];

/// The bundled file that holds `family` at `weight`, so the SVG writer embeds exactly the
/// files the PNG renders with. JetBrains Mono NL is bundled at 400 and 700 only, and a test
/// in this crate resolves every style of TEXT_STYLES.
pub fn bundled_font(family: FontFamily, weight: FontWeight) -> Result<FontFile, FontError> {
    BUNDLED_FONTS
        .iter()
        .find(|file| file.family == family && file.weight == weight)
        .copied()
        .ok_or(FontError::NoSuchFace {
            family: family.css_name(),
            weight: weight.css_value(),
        })
}

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("bundled font {file_name} does not parse")]
    Unparseable { file_name: &'static str },
    #[error("bundled font {file_name} has family {found:?}, expected {expected:?}")]
    FamilyMismatch {
        file_name: &'static str,
        expected: &'static str,
        found: String,
    },
    #[error("bundled font {file_name} has weight {found}, expected {expected}")]
    WeightMismatch {
        file_name: &'static str,
        expected: u16,
        found: u16,
    },
    #[error("no bundled font holds {family} at weight {weight}")]
    NoSuchFace { family: &'static str, weight: u16 },
}

/// The public check over the six compiled-in faces; `verify_fonts` stays crate-private so
/// only the font-swap test can pass it other files.
pub fn verify_bundled_fonts() -> Result<(), FontError> {
    verify_fonts(&BUNDLED_FONTS)
}

/// Parses each file and asserts its listed family and weight. Crate-private so the
/// font-swap test can pass a modified copy of BUNDLED_FONTS. The array type fixes the file
/// count, so the check can never pass over an empty list.
pub(crate) fn verify_fonts(files: &[FontFile; BUNDLED_FONTS.len()]) -> Result<(), FontError> {
    for file in files {
        let mut database = fontdb::Database::new();
        load_face(&mut database, file)?;
        let Some(face) = database.faces().next() else {
            return Err(FontError::Unparseable {
                file_name: file.file_name,
            });
        };
        let family = face
            .families
            .first()
            .map(|(name, _language)| name.as_str())
            .unwrap_or_default();
        let expected_family = file.family.css_name();
        if family != expected_family {
            return Err(FontError::FamilyMismatch {
                file_name: file.file_name,
                expected: expected_family,
                found: family.to_string(),
            });
        }
        let expected = file.weight.css_value();
        if face.weight.0 != expected {
            return Err(FontError::WeightMismatch {
                file_name: file.file_name,
                expected,
                found: face.weight.0,
            });
        }
    }
    Ok(())
}

/// Loads one file into `database` and asserts it holds exactly one face. The bytes are
/// shared, not copied.
fn load_face(database: &mut fontdb::Database, file: &FontFile) -> Result<(), FontError> {
    let face_ids = database.load_font_source(fontdb::Source::Binary(Arc::new(file.bytes)));
    if face_ids.len() == 1 {
        Ok(())
    } else {
        Err(FontError::Unparseable {
            file_name: file.file_name,
        })
    }
}

/// A database holding the bundled faces and nothing else.
fn bundled_font_database() -> Result<fontdb::Database, FontError> {
    let mut database = fontdb::Database::new();
    for file in &BUNDLED_FONTS {
        load_face(&mut database, file)?;
    }
    Ok(database)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stencil_model::text::TEXT_STYLES;

    #[test]
    fn verify_fonts_rejects_swapped_weights() {
        let mut swapped = BUNDLED_FONTS;
        let regular_bytes = swapped[0].bytes;
        swapped[0].bytes = swapped[2].bytes;
        swapped[2].bytes = regular_bytes;

        let error = verify_fonts(&swapped).unwrap_err();
        assert!(
            matches!(
                error,
                FontError::WeightMismatch {
                    file_name: "Inter-Regular.ttf",
                    expected: 400,
                    found: 700,
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn verify_fonts_rejects_a_face_of_another_family() {
        let mut swapped = BUNDLED_FONTS;
        swapped[4].bytes = swapped[0].bytes;

        let error = verify_fonts(&swapped).unwrap_err();
        assert!(
            matches!(
                &error,
                FontError::FamilyMismatch {
                    file_name: "JetBrainsMonoNL-Regular.ttf",
                    expected: "JetBrains Mono NL",
                    found,
                } if found == "Inter"
            ),
            "{error:?}"
        );
    }

    #[test]
    fn verify_fonts_rejects_bytes_that_are_not_a_font() {
        let mut broken = BUNDLED_FONTS;
        broken[1].bytes = b"not a font";

        let error = verify_fonts(&broken).unwrap_err();
        assert!(
            matches!(
                error,
                FontError::Unparseable {
                    file_name: "Inter-SemiBold.ttf"
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn bundled_font_returns_the_file_of_the_asked_family_and_weight() {
        let faces = [
            (FontFamily::Inter, FontWeight::Regular),
            (FontFamily::Inter, FontWeight::SemiBold),
            (FontFamily::Inter, FontWeight::Bold),
            (FontFamily::Inter, FontWeight::ExtraBold),
            (FontFamily::JetBrainsMonoNl, FontWeight::Regular),
            (FontFamily::JetBrainsMonoNl, FontWeight::Bold),
        ];
        let file_names: Vec<&str> = faces
            .iter()
            .map(|(family, weight)| {
                let file = bundled_font(*family, *weight).unwrap();
                assert_eq!((file.family, file.weight), (*family, *weight));
                file.file_name
            })
            .collect();
        assert_eq!(
            file_names,
            BUNDLED_FONTS.map(|file| file.file_name).to_vec()
        );
    }

    #[test]
    fn bundled_font_reports_a_weight_the_family_does_not_bundle() {
        let error = bundled_font(FontFamily::JetBrainsMonoNl, FontWeight::SemiBold).unwrap_err();
        assert!(
            matches!(
                error,
                FontError::NoSuchFace {
                    family: "JetBrains Mono NL",
                    weight: 600,
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn every_text_style_names_a_bundled_face() {
        let mut examined = 0;
        for named in &TEXT_STYLES {
            let style = named.style;
            assert!(
                bundled_font(style.family, style.weight).is_ok(),
                "{}",
                named.name.as_str()
            );
            examined += 1;
        }
        assert_eq!(examined, TEXT_STYLES.len());
        assert!(examined > 0);
    }

    #[test]
    fn bundled_database_holds_exactly_the_six_faces() {
        let database = bundled_font_database().unwrap();
        let mut faces: Vec<(String, u16)> = database
            .faces()
            .map(|face| {
                let family = face.families.first().map(|(name, _)| name.clone());
                (family.unwrap_or_default(), face.weight.0)
            })
            .collect();
        faces.sort();
        let expected: Vec<(String, u16)> = [
            ("Inter", 400),
            ("Inter", 600),
            ("Inter", 700),
            ("Inter", 800),
            ("JetBrains Mono NL", 400),
            ("JetBrains Mono NL", 700),
        ]
        .iter()
        .map(|(family, weight)| (family.to_string(), *weight))
        .collect();
        assert_eq!(faces, expected);
    }
}
