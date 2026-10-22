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

/// The 29 styles of section 2.9, in table order.
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
    SheetLabel,
    SheetKicker,
    Wordmark,
    Motto,
    SheetTitle,
    Statement,
    ViewNumeral,
    ViewTitle,
    ViewNote,
    ViewNoteSmall,
    LeadBody,
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
            TextStyleName::SheetLabel => "sheet_label",
            TextStyleName::SheetKicker => "sheet_kicker",
            TextStyleName::Wordmark => "wordmark",
            TextStyleName::Motto => "motto",
            TextStyleName::SheetTitle => "sheet_title",
            TextStyleName::Statement => "statement",
            TextStyleName::ViewNumeral => "view_numeral",
            TextStyleName::ViewTitle => "view_title",
            TextStyleName::ViewNote => "view_note",
            TextStyleName::ViewNoteSmall => "view_note_small",
            TextStyleName::LeadBody => "lead_body",
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
            TextStyleName::SheetLabel => &SHEET_LABEL,
            TextStyleName::SheetKicker => &SHEET_KICKER,
            TextStyleName::Wordmark => &WORDMARK,
            TextStyleName::Motto => &MOTTO,
            TextStyleName::SheetTitle => &SHEET_TITLE,
            TextStyleName::Statement => &STATEMENT,
            TextStyleName::ViewNumeral => &VIEW_NUMERAL,
            TextStyleName::ViewTitle => &VIEW_TITLE,
            TextStyleName::ViewNote => &VIEW_NOTE,
            TextStyleName::ViewNoteSmall => &VIEW_NOTE_SMALL,
            TextStyleName::LeadBody => &LEAD_BODY,
        }
    }
}

/// One row of the section 2.9 table, colors excluded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamedTextStyle {
    pub name: TextStyleName,
    pub style: TextStyle,
    /// True for `badge`, `kicker`, `sheet_label`, `sheet_kicker`, `motto` and `view_title`: the
    /// string is uppercased before measurement.
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

/// The design sheet styles of section 14.4 rule 28.
const SHEET_LABEL: NamedTextStyle = inter(
    TextStyleName::SheetLabel,
    FontWeight::SemiBold,
    11.0,
    13.2,
    0.12,
    true,
);
/// The page kicker on a sheet: a small label beside the badge, on one line in the head cell.
const SHEET_KICKER: NamedTextStyle = inter(
    TextStyleName::SheetKicker,
    FontWeight::SemiBold,
    10.0,
    12.0,
    0.05,
    true,
);
/// A 46 px line would let descenders hang below the measured box; 55 holds them.
const WORDMARK: NamedTextStyle = inter(
    TextStyleName::Wordmark,
    FontWeight::ExtraBold,
    46.0,
    55.0,
    -0.02,
    false,
);
const MOTTO: NamedTextStyle = inter(
    TextStyleName::Motto,
    FontWeight::Bold,
    13.0,
    16.0,
    0.06,
    true,
);
const SHEET_TITLE: NamedTextStyle = inter(
    TextStyleName::SheetTitle,
    FontWeight::ExtraBold,
    26.0,
    30.0,
    0.0,
    false,
);
const STATEMENT: NamedTextStyle = inter(
    TextStyleName::Statement,
    FontWeight::Bold,
    18.0,
    23.4,
    0.0,
    false,
);
const VIEW_NUMERAL: NamedTextStyle = inter(
    TextStyleName::ViewNumeral,
    FontWeight::Bold,
    13.0,
    15.6,
    0.0,
    false,
);
const VIEW_TITLE: NamedTextStyle = inter(
    TextStyleName::ViewTitle,
    FontWeight::Bold,
    16.0,
    19.2,
    0.1,
    true,
);
const VIEW_NOTE: NamedTextStyle = inter(
    TextStyleName::ViewNote,
    FontWeight::Regular,
    14.0,
    18.0,
    0.0,
    false,
);
const VIEW_NOTE_SMALL: NamedTextStyle = inter(
    TextStyleName::ViewNoteSmall,
    FontWeight::Regular,
    12.0,
    15.6,
    0.0,
    false,
);
const LEAD_BODY: NamedTextStyle = inter(
    TextStyleName::LeadBody,
    FontWeight::Bold,
    12.0,
    17.4,
    0.0,
    false,
);

/// The 29 styles of section 2.9, in table order, which is TextStyleName order:
/// TEXT_STYLES[i].name as usize == i.
pub const TEXT_STYLES: [NamedTextStyle; 29] = [
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
    SHEET_LABEL,
    SHEET_KICKER,
    WORDMARK,
    MOTTO,
    SHEET_TITLE,
    STATEMENT,
    VIEW_NUMERAL,
    VIEW_TITLE,
    VIEW_NOTE,
    VIEW_NOTE_SMALL,
    LEAD_BODY,
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
    ) -> Result<TextMetrics, MeasureError> {
        self.measure_indented(text, style, max_width_px, 0.0)
    }

    /// Measures a run whose first line starts `first_line_indent_px` from the left edge and
    /// whose later lines start at the left edge, all wrapping at `max_width_px` (section 3).
    /// With an indent above 0, a first word that does not fit after the indent leaves line 0
    /// empty and starts the text on line 1.
    fn measure_indented(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width_px: Option<f32>,
        first_line_indent_px: f32,
    ) -> Result<TextMetrics, MeasureError>;
}

/// The width of an indented run (section 3): the widest line, line 0 counted from the left
/// edge with its indent when it holds text.
pub fn indented_run_width(lines: &[TextLine], first_line_indent_px: f32) -> f32 {
    lines
        .iter()
        .enumerate()
        .fold(0.0_f32, |widest, (line_index, line)| {
            let starts_after_indent = line_index == 0 && line.byte_start < line.byte_end;
            let x_px = if starts_after_indent {
                first_line_indent_px
            } else {
                0.0
            };
            widest.max(x_px + line.width_px)
        })
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MeasureError {
    #[error("text is empty")]
    EmptyText,
    #[error("text style is out of range: {reason}")]
    InvalidStyle { reason: &'static str },
    #[error("max width {max_width_px} is negative or not finite")]
    InvalidMaxWidth { max_width_px: f32 },
    #[error("first-line indent {first_line_indent_px} is negative or not finite")]
    InvalidIndent { first_line_indent_px: f32 },
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
/// a style outside the supported ranges, a negative or non-finite max width, and a negative
/// or non-finite first-line indent.
pub fn check_measure_input(
    text: &str,
    style: &TextStyle,
    max_width_px: Option<f32>,
    first_line_indent_px: f32,
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
    if !first_line_indent_px.is_finite() || first_line_indent_px < 0.0 {
        return Err(MeasureError::InvalidIndent {
            first_line_indent_px,
        });
    }
    Ok(())
}

/// A measurer with a fixed advance per Unicode scalar value that breaks at the
/// opportunities of `break_opportunities` (sections 3.1 and 13.11).
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

/// Byte offsets at which a line may break, ascending (section 13.11): after a run of
/// U+0020 that follows other text, after a `-` or `/` followed by an ASCII letter, and after
/// a `.` followed by an ASCII letter. Each offset is where the next line would start, and a
/// character before it stays on the earlier line. A dot between digits and a hyphen before a
/// digit are not opportunities, so `10.8.0.0` and `-29` never split.
pub fn break_opportunities(text: &str) -> Vec<usize> {
    let mut opportunities = Vec::new();
    let mut seen_text = false;
    let mut characters = text.char_indices().peekable();
    while let Some((byte_index, character)) = characters.next() {
        let next = characters.peek().map(|&(_, next)| next);
        let after_space_run = character == ' ' && seen_text && next.is_some_and(|next| next != ' ');
        let after_joiner = matches!(character, '-' | '/' | '.')
            && next.is_some_and(|next| next.is_ascii_alphabetic());
        if after_space_run || after_joiner {
            opportunities.push(byte_index + character.len_utf8());
        }
        seen_text |= character != ' ';
    }
    opportunities
}

/// Byte range of a word: the text between two consecutive break opportunities, without its
/// spaces.
#[derive(Debug, Clone, Copy)]
struct Word {
    byte_start: usize,
    byte_end: usize,
}

/// The words of `text` in order. A segment that holds only spaces is not a word.
fn words(text: &str) -> Vec<Word> {
    let mut segment_starts = vec![0];
    segment_starts.extend(break_opportunities(text));
    let segment_ends = segment_starts
        .iter()
        .skip(1)
        .copied()
        .chain(std::iter::once(text.len()));
    segment_starts
        .iter()
        .zip(segment_ends)
        .filter_map(|(&segment_start, segment_end)| {
            let segment = text.get(segment_start..segment_end)?;
            let leading = segment.len() - segment.trim_start_matches(' ').len();
            let trimmed = segment.trim_matches(' ');
            (!trimmed.is_empty()).then(|| Word {
                byte_start: segment_start + leading,
                byte_end: segment_start + leading + trimmed.len(),
            })
        })
        .collect()
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
    fn measure_indented(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width_px: Option<f32>,
        first_line_indent_px: f32,
    ) -> Result<TextMetrics, MeasureError> {
        check_measure_input(text, style, max_width_px, first_line_indent_px)?;
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
        let words = words(text);

        let first_line_room_px = max_width_px.map(|max_width| max_width - first_line_indent_px);
        let room_px = |line_ranges: &[(usize, usize)]| {
            if line_ranges.is_empty() {
                first_line_room_px
            } else {
                max_width_px
            }
        };

        // Pairs of (byte_start, byte_end). Leading spaces stay on the first line; the spaces
        // at a break are trailing spaces of the earlier line and are dropped.
        let mut line_ranges: Vec<(usize, usize)> = Vec::new();
        let mut current: Option<(usize, usize)> = None;
        for word in &words {
            current = match current {
                None => {
                    let first_word_width = Self::line_width(text, 0, word.byte_end, advance_px);
                    if first_line_indent_px > 0.0
                        && first_line_room_px.is_some_and(|room| first_word_width > room)
                    {
                        line_ranges.push((0, 0));
                    }
                    Some((0, word.byte_end))
                }
                Some((line_start, line_end)) => {
                    let candidate_width =
                        Self::line_width(text, line_start, word.byte_end, advance_px);
                    match room_px(&line_ranges) {
                        Some(room) if candidate_width > room => {
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
        let width_px = indented_run_width(&lines, first_line_indent_px);
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
