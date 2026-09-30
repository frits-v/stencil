use std::collections::HashMap;

use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, LayoutRun, LineIter, Metrics, Shaping, Weight, Wrap,
};
use stencil_model::text::{
    MeasureError, TextLine, TextMeasurer, TextMetrics, TextStyle, check_measure_input,
};

use crate::{FontError, bundled_font_database, verify_bundled_fonts};

/// Entries kept before the cache is emptied. A g7 layout measures a few hundred distinct
/// (text, style, width) triples, so the bound only guards long-running callers.
const MEASURE_CACHE_ENTRIES_MAX: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MeasureKey {
    text: String,
    family: &'static str,
    weight: u16,
    size_bits: u32,
    line_height_bits: u32,
    letter_spacing_bits: u32,
    max_width_bits: Option<u32>,
}

impl MeasureKey {
    fn new(text: &str, style: &TextStyle, max_width_px: Option<f32>) -> Self {
        MeasureKey {
            text: text.to_string(),
            family: style.family.css_name(),
            weight: style.weight.css_value(),
            size_bits: style.size_px.to_bits(),
            line_height_bits: style.line_height_px.to_bits(),
            letter_spacing_bits: style.letter_spacing_em.to_bits(),
            max_width_bits: max_width_px.map(f32::to_bits),
        }
    }
}

/// `TextMeasurer` over a cosmic-text `FontSystem` that holds only the bundled Inter faces.
pub struct CosmicTextMeasurer {
    font_system: FontSystem,
    cache: HashMap<MeasureKey, TextMetrics>,
}

impl std::fmt::Debug for CosmicTextMeasurer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CosmicTextMeasurer")
            .field("faces", &self.font_system.db().len())
            .field("cached_measurements", &self.cache.len())
            .finish()
    }
}

impl CosmicTextMeasurer {
    /// Calls verify_bundled_fonts, then builds a FontSystem over the four faces only.
    pub fn new() -> Result<Self, FontError> {
        verify_bundled_fonts()?;
        let database = bundled_font_database()?;
        let font_system = FontSystem::new_with_locale_and_db("en-US".into(), database);
        Ok(CosmicTextMeasurer {
            font_system,
            cache: HashMap::new(),
        })
    }

    fn measure_uncached(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width_px: Option<f32>,
    ) -> Result<TextMetrics, MeasureError> {
        let metrics = Metrics::new(style.size_px, style.line_height_px);
        let attributes = Attrs::new()
            .family(Family::Name(style.family.css_name()))
            .weight(Weight(style.weight.css_value()))
            .letter_spacing(style.letter_spacing_em);

        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_wrap(Wrap::Word);
        buffer.set_size(max_width_px, None);
        buffer.set_text(text, &attributes, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Glyph clusters are byte offsets into their own paragraph; cosmic-text splits the
        // input at line endings the same way LineIter does.
        let paragraph_starts: Vec<usize> =
            LineIter::new(text).map(|(range, _)| range.start).collect();

        let mut lines = Vec::new();
        for run in buffer.layout_runs() {
            let paragraph_start =
                paragraph_starts
                    .get(run.line_i)
                    .copied()
                    .ok_or_else(|| MeasureError::Backend {
                        message: format!(
                            "layout run refers to paragraph {} of {}",
                            run.line_i,
                            paragraph_starts.len()
                        ),
                    })?;
            check_glyphs(text, style, &run, paragraph_start)?;
            let extent = line_extent(text, &run, paragraph_start)?;
            // f32::max below skips a NaN, so each line is checked before it is kept.
            if !extent.width_px.is_finite() || !run.line_y.is_finite() {
                return Err(MeasureError::Backend {
                    message: format!(
                        "cosmic-text reported line {} with width {} and baseline {}",
                        lines.len(),
                        extent.width_px,
                        run.line_y
                    ),
                });
            }
            lines.push(TextLine {
                byte_start: extent.byte_start,
                byte_end: extent.byte_end,
                width_px: extent.width_px,
                baseline_px: run.line_y,
            });
        }

        if lines.is_empty() {
            return Err(MeasureError::Backend {
                message: "cosmic-text produced no layout runs".to_string(),
            });
        }
        let line_count = u32::try_from(lines.len()).map_err(|_| MeasureError::Backend {
            message: format!("{} lines do not fit in u32", lines.len()),
        })?;
        let width_px = lines
            .iter()
            .fold(0.0_f32, |widest, line| widest.max(line.width_px));

        Ok(TextMetrics {
            width_px,
            height_px: line_count as f32 * style.line_height_px,
            line_count,
            lines,
        })
    }
}

/// Reports the first glyph the bundled faces lack. cosmic-text marks it with glyph id 0.
fn check_glyphs(
    text: &str,
    style: &TextStyle,
    run: &LayoutRun<'_>,
    paragraph_start: usize,
) -> Result<(), MeasureError> {
    let Some(missing) = run.glyphs.iter().find(|glyph| glyph.glyph_id == 0) else {
        return Ok(());
    };
    let missing_start = byte_offset(paragraph_start, missing.start)?;
    let character = text
        .get(missing_start..)
        .and_then(|rest| rest.chars().next())
        .ok_or_else(|| MeasureError::Backend {
            message: format!("missing glyph cluster at byte {missing_start} is outside the text"),
        })?;
    Err(MeasureError::MissingGlyph {
        family: style.family.css_name(),
        character,
        codepoint: u32::from(character),
    })
}

/// `base + offset` for byte offsets that come from cosmic-text, which are not trusted to
/// stay inside the text.
fn byte_offset(base: usize, offset: usize) -> Result<usize, MeasureError> {
    base.checked_add(offset)
        .ok_or_else(|| MeasureError::Backend {
            message: format!("byte offset {base} + {offset} overflows"),
        })
}

/// One layout run in terms of the whole measured string.
struct LineExtent {
    byte_start: usize,
    byte_end: usize,
    width_px: f32,
}

/// Spans a run from its first to its last cluster with trailing U+0020 removed. cosmic-text
/// leaves the first space at a break out of `line_w` but keeps any further ones, and keeps
/// spaces at the end of the text, so their advances are subtracted to match the section 3
/// contract. A run with no glyphs gets an empty range at its paragraph start.
fn line_extent(
    text: &str,
    run: &LayoutRun<'_>,
    paragraph_start: usize,
) -> Result<LineExtent, MeasureError> {
    let cluster_start = run.glyphs.iter().map(|glyph| glyph.start).min();
    let cluster_end = run.glyphs.iter().map(|glyph| glyph.end).max();
    let (Some(cluster_start), Some(cluster_end)) = (cluster_start, cluster_end) else {
        return Ok(LineExtent {
            byte_start: paragraph_start,
            byte_end: paragraph_start,
            width_px: run.line_w,
        });
    };
    let byte_start = byte_offset(paragraph_start, cluster_start)?;
    let untrimmed_end = byte_offset(paragraph_start, cluster_end)?;
    let line_text = text
        .get(byte_start..untrimmed_end)
        .ok_or_else(|| MeasureError::Backend {
            message: format!(
                "layout run bytes {byte_start}..{untrimmed_end} are outside the text or split a character"
            ),
        })?;
    let trimmed_length = line_text.trim_end_matches(' ').len();
    let trailing_cluster_start = byte_offset(cluster_start, trimmed_length)?;
    let trailing_space_px: f32 = run
        .glyphs
        .iter()
        .filter(|glyph| glyph.start >= trailing_cluster_start)
        .map(|glyph| glyph.w)
        .sum();
    let width_px = if trailing_space_px > 0.0 {
        (run.line_w - trailing_space_px).max(0.0)
    } else {
        run.line_w
    };
    Ok(LineExtent {
        byte_start,
        byte_end: byte_offset(byte_start, trimmed_length)?,
        width_px,
    })
}

impl TextMeasurer for CosmicTextMeasurer {
    fn measure(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width_px: Option<f32>,
    ) -> Result<TextMetrics, MeasureError> {
        check_measure_input(text, style, max_width_px)?;
        let key = MeasureKey::new(text, style, max_width_px);
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached.clone());
        }
        let metrics = self.measure_uncached(text, style, max_width_px)?;
        if self.cache.len() >= MEASURE_CACHE_ENTRIES_MAX {
            self.cache.clear();
        }
        self.cache.insert(key, metrics.clone());
        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stencil_model::text::TextStyleName;

    #[test]
    fn font_system_holds_only_the_bundled_faces() {
        let measurer = CosmicTextMeasurer::new().unwrap();
        assert_eq!(measurer.font_system.db().len(), 4);
    }

    #[test]
    fn cache_stays_within_its_bound() {
        let mut measurer = CosmicTextMeasurer::new().unwrap();
        let style = TextStyleName::TagLabel.text_style().style;
        for vlan in 0..=MEASURE_CACHE_ENTRIES_MAX {
            measurer
                .measure(&format!("VLAN {vlan}"), &style, None)
                .unwrap();
            assert!(measurer.cache.len() <= MEASURE_CACHE_ENTRIES_MAX);
        }
        assert_eq!(measurer.cache.len(), 1);
    }

    #[test]
    fn errors_are_not_cached() {
        let mut measurer = CosmicTextMeasurer::new().unwrap();
        let style = TextStyleName::TagLabel.text_style().style;
        measurer.measure("\u{4E00}", &style, None).unwrap_err();
        assert!(measurer.cache.is_empty());
    }

    #[test]
    fn a_byte_offset_that_overflows_is_a_backend_error() {
        assert_eq!(byte_offset(3, 4).unwrap(), 7);
        assert!(matches!(
            byte_offset(usize::MAX, 1),
            Err(MeasureError::Backend { .. })
        ));
    }
}
