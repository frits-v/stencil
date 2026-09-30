//! Text measurement contract (section 3): style values, the measurer trait and the
//! fixed-metrics fake that layout tests run against.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFamily {
    Inter,
}

impl FontFamily {
    pub fn css_name(self) -> &'static str {
        match self {
            FontFamily::Inter => "Inter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Regular,
    SemiBold,
    Bold,
    ExtraBold,
}

impl FontWeight {
    /// 400, 600, 700, 800.
    pub fn css_value(self) -> u16 {
        match self {
            FontWeight::Regular => 400,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub family: FontFamily,
    pub weight: FontWeight,
    pub size_px: f32,
    pub line_height_px: f32,
    pub letter_spacing_em: f32,
}

/// Added to every definite width layout passes to a measurer (section 2.1).
pub const WRAP_EPSILON_PX: f32 = 0.01;

pub const FONT_SIZE_MIN_PX: f32 = 6.0;
pub const FONT_SIZE_MAX_PX: f32 = 96.0;
pub const LETTER_SPACING_MIN_EM: f32 = -0.2;
pub const LETTER_SPACING_MAX_EM: f32 = 0.5;

/// The 18 styles of section 2.9, in table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextStyleName {
    Badge,
    Kicker,
    Title,
    Lede,
    ZoneLabel,
    GcpBar,
    PerimeterLabel,
    CardFunction,
    CardProduct,
    Fact,
    Ask,
    TagLabel,
    TagSub,
    NoteLegend,
    LegendLabel,
    LegendText,
    Foot,
    BlockBody,
}

impl TextStyleName {
    /// The section 2.9 name, for example "tag_label".
    pub fn as_str(self) -> &'static str {
        match self {
            TextStyleName::Badge => "badge",
            TextStyleName::Kicker => "kicker",
            TextStyleName::Title => "title",
            TextStyleName::Lede => "lede",
            TextStyleName::ZoneLabel => "zone_label",
            TextStyleName::GcpBar => "gcp_bar",
            TextStyleName::PerimeterLabel => "perimeter_label",
            TextStyleName::CardFunction => "card_function",
            TextStyleName::CardProduct => "card_product",
            TextStyleName::Fact => "fact",
            TextStyleName::Ask => "ask",
            TextStyleName::TagLabel => "tag_label",
            TextStyleName::TagSub => "tag_sub",
            TextStyleName::NoteLegend => "note_legend",
            TextStyleName::LegendLabel => "legend_label",
            TextStyleName::LegendText => "legend_text",
            TextStyleName::Foot => "foot",
            TextStyleName::BlockBody => "block_body",
        }
    }

    /// The TEXT_STYLES entry whose `name` is self.
    pub fn text_style(self) -> &'static NamedTextStyle {
        // Each arm borrows a named const, which promotes to 'static; a runtime index into
        // the TEXT_STYLES const would borrow a temporary instead.
        match self {
            TextStyleName::Badge => &BADGE,
            TextStyleName::Kicker => &KICKER,
            TextStyleName::Title => &TITLE,
            TextStyleName::Lede => &LEDE,
            TextStyleName::ZoneLabel => &ZONE_LABEL,
            TextStyleName::GcpBar => &GCP_BAR,
            TextStyleName::PerimeterLabel => &PERIMETER_LABEL,
            TextStyleName::CardFunction => &CARD_FUNCTION,
            TextStyleName::CardProduct => &CARD_PRODUCT,
            TextStyleName::Fact => &FACT,
            TextStyleName::Ask => &ASK,
            TextStyleName::TagLabel => &TAG_LABEL,
            TextStyleName::TagSub => &TAG_SUB,
            TextStyleName::NoteLegend => &NOTE_LEGEND,
            TextStyleName::LegendLabel => &LEGEND_LABEL,
            TextStyleName::LegendText => &LEGEND_TEXT,
            TextStyleName::Foot => &FOOT,
            TextStyleName::BlockBody => &BLOCK_BODY,
        }
    }
}

/// One row of the section 2.9 table, colors excluded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamedTextStyle {
    pub name: TextStyleName,
    pub style: TextStyle,
    /// True for `badge` and `kicker`: the string is uppercased before measurement.
    pub uppercase: bool,
}

const fn inter(
    name: TextStyleName,
    weight: FontWeight,
    size_px: f32,
    line_height_px: f32,
    letter_spacing_em: f32,
    uppercase: bool,
) -> NamedTextStyle {
    NamedTextStyle {
        name,
        style: TextStyle {
            family: FontFamily::Inter,
            weight,
            size_px,
            line_height_px,
            letter_spacing_em,
        },
        uppercase,
    }
}

const BADGE: NamedTextStyle = inter(
    TextStyleName::Badge,
    FontWeight::ExtraBold,
    10.0,
    12.0,
    0.07,
    true,
);
const KICKER: NamedTextStyle = inter(
    TextStyleName::Kicker,
    FontWeight::Bold,
    11.0,
    13.2,
    0.08,
    true,
);
const TITLE: NamedTextStyle = inter(
    TextStyleName::Title,
    FontWeight::Bold,
    20.0,
    24.0,
    -0.02,
    false,
);
const LEDE: NamedTextStyle = inter(
    TextStyleName::Lede,
    FontWeight::Regular,
    13.0,
    15.6,
    0.0,
    false,
);
const ZONE_LABEL: NamedTextStyle = inter(
    TextStyleName::ZoneLabel,
    FontWeight::Bold,
    12.0,
    14.4,
    0.0,
    false,
);
const GCP_BAR: NamedTextStyle = inter(
    TextStyleName::GcpBar,
    FontWeight::Bold,
    18.0,
    21.6,
    0.01,
    false,
);
const PERIMETER_LABEL: NamedTextStyle = inter(
    TextStyleName::PerimeterLabel,
    FontWeight::Bold,
    13.0,
    15.6,
    0.0,
    false,
);
const CARD_FUNCTION: NamedTextStyle = inter(
    TextStyleName::CardFunction,
    FontWeight::Bold,
    13.0,
    15.6,
    0.0,
    false,
);
const CARD_PRODUCT: NamedTextStyle = inter(
    TextStyleName::CardProduct,
    FontWeight::Regular,
    12.0,
    14.4,
    0.0,
    false,
);
const FACT: NamedTextStyle = inter(
    TextStyleName::Fact,
    FontWeight::SemiBold,
    12.0,
    16.2,
    0.0,
    false,
);
const ASK: NamedTextStyle = inter(
    TextStyleName::Ask,
    FontWeight::SemiBold,
    12.0,
    16.2,
    0.0,
    false,
);
const TAG_LABEL: NamedTextStyle = inter(
    TextStyleName::TagLabel,
    FontWeight::Bold,
    12.0,
    15.6,
    0.0,
    false,
);
const TAG_SUB: NamedTextStyle = inter(
    TextStyleName::TagSub,
    FontWeight::SemiBold,
    12.0,
    15.6,
    0.0,
    false,
);
const NOTE_LEGEND: NamedTextStyle = inter(
    TextStyleName::NoteLegend,
    FontWeight::Regular,
    12.0,
    14.4,
    0.0,
    false,
);
const LEGEND_LABEL: NamedTextStyle = inter(
    TextStyleName::LegendLabel,
    FontWeight::Bold,
    12.0,
    14.4,
    0.0,
    false,
);
const LEGEND_TEXT: NamedTextStyle = inter(
    TextStyleName::LegendText,
    FontWeight::Regular,
    12.0,
    14.4,
    0.0,
    false,
);
const FOOT: NamedTextStyle = inter(
    TextStyleName::Foot,
    FontWeight::Regular,
    11.0,
    13.2,
    0.0,
    false,
);

/// Body lines of the Text and Callout blocks (section 11.3): 12 px at a 1.45 line height.
const BLOCK_BODY: NamedTextStyle = inter(
    TextStyleName::BlockBody,
    FontWeight::Regular,
    12.0,
    17.4,
    0.0,
    false,
);

/// The 18 styles of section 2.9, in table order, which is TextStyleName order:
/// TEXT_STYLES[i].name as usize == i.
pub const TEXT_STYLES: [NamedTextStyle; 18] = [
    BADGE,
    KICKER,
    TITLE,
    LEDE,
    ZONE_LABEL,
    GCP_BAR,
    PERIMETER_LABEL,
    CARD_FUNCTION,
    CARD_PRODUCT,
    FACT,
    ASK,
    TAG_LABEL,
    TAG_SUB,
    NOTE_LEGEND,
    LEGEND_LABEL,
    LEGEND_TEXT,
    FOOT,
    BLOCK_BODY,
];

#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    /// Byte range of this line in the measured string, trailing spaces excluded.
    pub byte_start: usize,
    pub byte_end: usize,
    pub width_px: f32,
    /// Baseline offset from the top of the text block.
    pub baseline_px: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextMetrics {
    pub width_px: f32,
    pub height_px: f32,
    pub line_count: u32,
    pub lines: Vec<TextLine>,
}

pub trait TextMeasurer {
    fn measure(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width_px: Option<f32>,
    ) -> Result<TextMetrics, MeasureError>;
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MeasureError {
    #[error("text is empty")]
    EmptyText,
    #[error("text style is out of range: {reason}")]
    InvalidStyle { reason: &'static str },
    #[error("max width {max_width_px} is negative or not finite")]
    InvalidMaxWidth { max_width_px: f32 },
    #[error("font {family} has no glyph for {character:?} (U+{codepoint:04X})")]
    MissingGlyph {
        family: &'static str,
        character: char,
        codepoint: u32,
    },
    #[error("text backend failed: {message}")]
    Backend { message: String },
}

/// The boundary assertions every measurer applies before measuring (section 3): empty text,
/// a style outside the supported ranges, and a negative or non-finite max width.
pub fn check_measure_input(
    text: &str,
    style: &TextStyle,
    max_width_px: Option<f32>,
) -> Result<(), MeasureError> {
    if text.is_empty() {
        return Err(MeasureError::EmptyText);
    }
    if !(FONT_SIZE_MIN_PX..=FONT_SIZE_MAX_PX).contains(&style.size_px) {
        return Err(MeasureError::InvalidStyle {
            reason: "size is outside 6 to 96 px",
        });
    }
    if style.line_height_px.is_nan() || style.line_height_px < style.size_px {
        return Err(MeasureError::InvalidStyle {
            reason: "line height is below the size",
        });
    }
    if !style.line_height_px.is_finite() {
        return Err(MeasureError::InvalidStyle {
            reason: "line height is not finite",
        });
    }
    if !(LETTER_SPACING_MIN_EM..=LETTER_SPACING_MAX_EM).contains(&style.letter_spacing_em) {
        return Err(MeasureError::InvalidStyle {
            reason: "letter spacing is outside -0.2 to 0.5 em",
        });
    }
    if let Some(max_width_px) = max_width_px
        && (!max_width_px.is_finite() || max_width_px < 0.0)
    {
        return Err(MeasureError::InvalidMaxWidth { max_width_px });
    }
    Ok(())
}

/// A measurer with a fixed advance per Unicode scalar value and breaks only at U+0020
/// (section 3.1).
#[derive(Debug, Clone, PartialEq)]
pub struct FixedMetricsMeasurer {
    /// Advance of every character, including spaces, in em.
    pub advance_em: f32,
    /// Characters reported as MissingGlyph.
    pub missing_glyphs: Vec<char>,
}

impl Default for FixedMetricsMeasurer {
    fn default() -> Self {
        FixedMetricsMeasurer {
            advance_em: 0.5,
            missing_glyphs: Vec::new(),
        }
    }
}

/// Byte range of a maximal run of non-space characters.
#[derive(Debug, Clone, Copy)]
struct Word {
    byte_start: usize,
    byte_end: usize,
}

fn space_separated_words(text: &str) -> Vec<Word> {
    let mut words = Vec::new();
    let mut word_start: Option<usize> = None;
    for (byte_index, character) in text.char_indices() {
        match (character == ' ', word_start) {
            (true, Some(start)) => {
                words.push(Word {
                    byte_start: start,
                    byte_end: byte_index,
                });
                word_start = None;
            }
            (false, None) => word_start = Some(byte_index),
            (true, None) | (false, Some(_)) => {}
        }
    }
    if let Some(start) = word_start {
        words.push(Word {
            byte_start: start,
            byte_end: text.len(),
        });
    }
    words
}

impl FixedMetricsMeasurer {
    fn line_width(text: &str, byte_start: usize, byte_end: usize, advance_px: f32) -> f32 {
        let scalar_count = text
            .get(byte_start..byte_end)
            .map_or(0, |line| line.chars().count());
        scalar_count as f32 * advance_px
    }
}

impl TextMeasurer for FixedMetricsMeasurer {
    fn measure(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width_px: Option<f32>,
    ) -> Result<TextMetrics, MeasureError> {
        check_measure_input(text, style, max_width_px)?;
        if let Some(character) = text
            .chars()
            .find(|character| self.missing_glyphs.contains(character))
        {
            return Err(MeasureError::MissingGlyph {
                family: style.family.css_name(),
                character,
                codepoint: u32::from(character),
            });
        }

        let advance_px = (self.advance_em + style.letter_spacing_em) * style.size_px;
        let words = space_separated_words(text);

        // Pairs of (byte_start, byte_end). Leading spaces stay on the first line; the spaces
        // at a break are trailing spaces of the earlier line and are dropped.
        let mut line_ranges: Vec<(usize, usize)> = Vec::new();
        let mut current: Option<(usize, usize)> = None;
        for word in &words {
            current = match current {
                None => Some((0, word.byte_end)),
                Some((line_start, line_end)) => {
                    let candidate_width =
                        Self::line_width(text, line_start, word.byte_end, advance_px);
                    match max_width_px {
                        Some(max_width) if candidate_width > max_width => {
                            line_ranges.push((line_start, line_end));
                            Some((word.byte_start, word.byte_end))
                        }
                        _ => Some((line_start, word.byte_end)),
                    }
                }
            };
        }
        line_ranges.push(current.unwrap_or((0, 0)));

        let line_height_px = style.line_height_px;
        let lines: Vec<TextLine> = line_ranges
            .iter()
            .enumerate()
            .map(|(line_index, &(byte_start, byte_end))| TextLine {
                byte_start,
                byte_end,
                width_px: Self::line_width(text, byte_start, byte_end, advance_px),
                baseline_px: line_index as f32 * line_height_px + 0.8 * line_height_px,
            })
            .collect();
        let width_px = lines
            .iter()
            .fold(0.0_f32, |widest, line| widest.max(line.width_px));
        let line_count = u32::try_from(lines.len()).map_err(|_| MeasureError::Backend {
            message: format!("{} lines do not fit in u32", lines.len()),
        })?;

        Ok(TextMetrics {
            width_px,
            height_px: line_count as f32 * line_height_px,
            line_count,
            lines,
        })
    }
}
