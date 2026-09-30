//! Text measurement on cosmic-text over the bundled Inter faces (spec sections 3.2, 4.3 and 8.1).

mod measurer;

use std::sync::Arc;

use cosmic_text::fontdb;
use stencil_model::text::{FontFamily, FontWeight};

pub use measurer::CosmicTextMeasurer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontFile {
    pub file_name: &'static str,
    pub weight: FontWeight,
    pub bytes: &'static [u8],
    /// Lowercase hex SHA-256 of `bytes`, copied from assets/fonts/FONTS.md.
    pub sha256: &'static str,
}

/// Inter-Regular, Inter-SemiBold, Inter-Bold, Inter-ExtraBold, via include_bytes!.
pub const BUNDLED_FONTS: [FontFile; 4] = [
    FontFile {
        file_name: "Inter-Regular.ttf",
        weight: FontWeight::Regular,
        bytes: include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
        sha256: "40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82",
    },
    FontFile {
        file_name: "Inter-SemiBold.ttf",
        weight: FontWeight::SemiBold,
        bytes: include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"),
        sha256: "78a843fade9d4612a5567302fb595b56976eb5fcebf4fea5a5912d638bafcde3",
    },
    FontFile {
        file_name: "Inter-Bold.ttf",
        weight: FontWeight::Bold,
        bytes: include_bytes!("../../../assets/fonts/Inter-Bold.ttf"),
        sha256: "288316099b1e0a47a4716d159098005eef7c0066921f34e3200393dbdb01947f",
    },
    FontFile {
        file_name: "Inter-ExtraBold.ttf",
        weight: FontWeight::ExtraBold,
        bytes: include_bytes!("../../../assets/fonts/Inter-ExtraBold.ttf"),
        sha256: "e6756ad5690b77606aa62249a7b420d9902d45cae4b0048a24911fd4324b0a22",
    },
];

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("bundled font {file_name} does not parse")]
    Unparseable { file_name: &'static str },
    #[error("bundled font {file_name} has family {found:?}, expected \"Inter\"")]
    FamilyMismatch {
        file_name: &'static str,
        found: String,
    },
    #[error("bundled font {file_name} has weight {found}, expected {expected}")]
    WeightMismatch {
        file_name: &'static str,
        expected: u16,
        found: u16,
    },
}

/// Calls verify_fonts(&BUNDLED_FONTS).
pub fn verify_bundled_fonts() -> Result<(), FontError> {
    verify_fonts(&BUNDLED_FONTS)
}

/// Parses each file and asserts family "Inter" and the listed weight. Crate-private so the
/// font-swap test can pass a modified copy of BUNDLED_FONTS.
pub(crate) fn verify_fonts(files: &[FontFile]) -> Result<(), FontError> {
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
        if family != FontFamily::Inter.css_name() {
            return Err(FontError::FamilyMismatch {
                file_name: file.file_name,
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
    fn bundled_database_holds_exactly_the_four_faces() {
        let database = bundled_font_database().unwrap();
        let mut weights: Vec<u16> = database.faces().map(|face| face.weight.0).collect();
        weights.sort_unstable();
        assert_eq!(weights, vec![400, 600, 700, 800]);
        assert!(
            database
                .faces()
                .all(|face| face.families.first().map(|(name, _)| name.as_str()) == Some("Inter"))
        );
    }
}
