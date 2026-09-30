// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use stencil_model::text::{
    FixedMetricsMeasurer, FontFamily, FontWeight, MeasureError, TEXT_STYLES, TextLine,
    TextMeasurer, TextStyle, TextStyleName, WRAP_EPSILON_PX,
};

const ALL_STYLE_NAMES: [TextStyleName; 17] = [
    TextStyleName::Badge,
    TextStyleName::Kicker,
    TextStyleName::Title,
    TextStyleName::Lede,
    TextStyleName::ZoneLabel,
    TextStyleName::GcpBar,
    TextStyleName::PerimeterLabel,
    TextStyleName::CardFunction,
    TextStyleName::CardProduct,
    TextStyleName::Fact,
    TextStyleName::Ask,
    TextStyleName::TagLabel,
    TextStyleName::TagSub,
    TextStyleName::NoteLegend,
    TextStyleName::LegendLabel,
    TextStyleName::LegendText,
    TextStyleName::Foot,
];

/// Section 2.9: name, weight, size, line height, letter spacing, uppercase.
const SECTION_2_9: [(&str, u16, f32, f32, f32, bool); 17] = [
    ("badge", 800, 10.0, 12.0, 0.07, true),
    ("kicker", 700, 11.0, 13.2, 0.08, true),
    ("title", 700, 20.0, 24.0, -0.02, false),
    ("lede", 400, 13.0, 15.6, 0.0, false),
    ("zone_label", 700, 12.0, 14.4, 0.0, false),
    ("gcp_bar", 700, 18.0, 21.6, 0.01, false),
    ("perimeter_label", 700, 13.0, 15.6, 0.0, false),
    ("card_function", 700, 13.0, 15.6, 0.0, false),
    ("card_product", 400, 12.0, 14.4, 0.0, false),
    ("fact", 600, 12.0, 16.2, 0.0, false),
    ("ask", 600, 12.0, 16.2, 0.0, false),
    ("tag_label", 700, 12.0, 15.6, 0.0, false),
    ("tag_sub", 600, 12.0, 15.6, 0.0, false),
    ("note_legend", 400, 12.0, 14.4, 0.0, false),
    ("legend_label", 700, 12.0, 14.4, 0.0, false),
    ("legend_text", 400, 12.0, 14.4, 0.0, false),
    ("foot", 400, 11.0, 13.2, 0.0, false),
];

#[test]
fn text_styles_are_indexed_by_name() {
    for name in ALL_STYLE_NAMES {
        let index = name as usize;
        assert_eq!(TEXT_STYLES[index].name, name);
        assert_eq!(name.text_style(), &TEXT_STYLES[index]);
        assert_eq!(name.as_str(), SECTION_2_9[index].0);
    }
}

#[test]
fn text_styles_match_section_2_9() {
    for (named, expected) in TEXT_STYLES.iter().zip(SECTION_2_9) {
        let (name, weight, size_px, line_height_px, letter_spacing_em, uppercase) = expected;
        assert_eq!(named.name.as_str(), name);
        assert_eq!(named.style.family, FontFamily::Inter);
        assert_eq!(named.style.weight.css_value(), weight, "{name}");
        assert_eq!(named.style.size_px, size_px, "{name}");
        assert_eq!(named.style.line_height_px, line_height_px, "{name}");
        assert_eq!(named.style.letter_spacing_em, letter_spacing_em, "{name}");
        assert_eq!(named.uppercase, uppercase, "{name}");
    }
}

#[test]
fn font_values() {
    assert_eq!(FontFamily::Inter.css_name(), "Inter");
    let weights = [
        (FontWeight::Regular, 400),
        (FontWeight::SemiBold, 600),
        (FontWeight::Bold, 700),
        (FontWeight::ExtraBold, 800),
    ];
    for (weight, value) in weights {
        assert_eq!(weight.css_value(), value);
    }
    assert_eq!(WRAP_EPSILON_PX, 0.01);
}

fn card_function() -> TextStyle {
    TextStyleName::CardFunction.text_style().style
}

fn measure(
    text: &str,
    style: &TextStyle,
    max_width_px: Option<f32>,
) -> Result<stencil_model::text::TextMetrics, MeasureError> {
    FixedMetricsMeasurer::default().measure(text, style, max_width_px)
}

#[test]
fn worked_example() {
    let metrics = measure("On-prem router 1", &card_function(), None).unwrap();
    assert_eq!(metrics.width_px, 104.0);
    assert_eq!(metrics.height_px, 15.6);
    assert_eq!(metrics.line_count, 1);
    assert_eq!(
        metrics.lines,
        vec![TextLine {
            byte_start: 0,
            byte_end: 16,
            width_px: 104.0,
            baseline_px: 0.8 * 15.6,
        }]
    );
}

#[test]
fn max_content_width_plus_epsilon_stays_on_one_line() {
    let metrics = measure(
        "On-prem router 1",
        &card_function(),
        Some(104.0 + WRAP_EPSILON_PX),
    )
    .unwrap();
    assert_eq!(metrics.line_count, 1);
    let exact = measure("On-prem router 1", &card_function(), Some(104.0)).unwrap();
    assert_eq!(exact.line_count, 1);
}

#[test]
fn wraps_greedily_between_words() {
    // 6.5 px per scalar. "On-prem router" is 14 scalars, 91 px; adding " 1" makes 104.
    let metrics = measure("On-prem router 1", &card_function(), Some(100.0)).unwrap();
    assert_eq!(metrics.line_count, 2);
    assert_eq!(metrics.lines.len(), 2);
    assert_eq!(
        (metrics.lines[0].byte_start, metrics.lines[0].byte_end),
        (0, 14)
    );
    assert_eq!(metrics.lines[0].width_px, 91.0);
    assert_eq!(
        (metrics.lines[1].byte_start, metrics.lines[1].byte_end),
        (15, 16)
    );
    assert_eq!(metrics.lines[1].width_px, 6.5);
    assert_eq!(metrics.width_px, 91.0);
    assert_eq!(metrics.height_px, 2.0 * 15.6);
    assert_eq!(metrics.lines[1].baseline_px, 15.6 + 0.8 * 15.6);
}

#[test]
fn width_that_splits_two_words() {
    let metrics = measure("alpha beta", &card_function(), Some(40.0)).unwrap();
    let texts: Vec<&str> = metrics
        .lines
        .iter()
        .map(|line| &"alpha beta"[line.byte_start..line.byte_end])
        .collect();
    assert_eq!(texts, ["alpha", "beta"]);
    assert_eq!(metrics.width_px, 5.0 * 6.5);
}

#[test]
fn word_wider_than_max_width_stays_whole() {
    let metrics = measure("Interconnect", &card_function(), Some(20.0)).unwrap();
    assert_eq!(metrics.line_count, 1);
    assert_eq!(metrics.width_px, 12.0 * 6.5);
    assert!(metrics.width_px > 20.0);
}

#[test]
fn zero_width_gives_the_widest_word() {
    let metrics = measure("On-prem router 1", &card_function(), Some(0.0)).unwrap();
    assert_eq!(metrics.line_count, 3);
    assert_eq!(metrics.width_px, 7.0 * 6.5);
}

#[test]
fn interior_space_runs_count_and_trailing_spaces_do_not() {
    let metrics = measure("a  b", &card_function(), None).unwrap();
    assert_eq!(metrics.width_px, 4.0 * 6.5);
    let wrapped = measure("a  b", &card_function(), Some(0.0)).unwrap();
    assert_eq!(wrapped.line_count, 2);
    assert_eq!(
        (wrapped.lines[0].byte_start, wrapped.lines[0].byte_end),
        (0, 1)
    );
    assert_eq!(wrapped.lines[0].width_px, 6.5);
    assert_eq!(
        (wrapped.lines[1].byte_start, wrapped.lines[1].byte_end),
        (3, 4)
    );
}

#[test]
fn letter_spacing_adds_n_times_em_times_size() {
    let text = "ABCDEFGHIJKLMNOPQRST";
    let spaced = TextStyleName::Kicker.text_style().style;
    let unspaced = TextStyle {
        letter_spacing_em: 0.0,
        ..spaced
    };
    let with_spacing = measure(text, &spaced, None).unwrap();
    let without_spacing = measure(text, &unspaced, None).unwrap();
    let expected = 20.0 * 0.08 * 11.0;
    assert!((with_spacing.width_px - without_spacing.width_px - expected).abs() < 1e-3);
}

#[test]
fn advance_is_one_multiplication_per_line() {
    let measurer_advance = 0.501_f32;
    let mut measurer = FixedMetricsMeasurer {
        advance_em: measurer_advance,
        missing_glyphs: Vec::new(),
    };
    let style = TextStyleName::TagLabel.text_style().style;
    let text = "VLAN xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let metrics = measurer.measure(text, &style, None).unwrap();
    let advance_px = (measurer_advance + style.letter_spacing_em) * style.size_px;
    assert_eq!(metrics.width_px, text.chars().count() as f32 * advance_px);
}

#[test]
fn measuring_twice_is_bit_identical() {
    let mut measurer = FixedMetricsMeasurer::default();
    let style = card_function();
    let first = measurer
        .measure("Cloud Router A", &style, Some(50.0))
        .unwrap();
    let second = measurer
        .measure("Cloud Router A", &style, Some(50.0))
        .unwrap();
    assert_eq!(first, second);
}

#[test]
fn configured_character_is_a_missing_glyph() {
    let mut measurer = FixedMetricsMeasurer {
        advance_em: 0.5,
        missing_glyphs: vec!['\u{4E00}'],
    };
    let error = measurer
        .measure("a \u{4E00} b", &card_function(), None)
        .unwrap_err();
    assert_eq!(
        error,
        MeasureError::MissingGlyph {
            family: "Inter",
            character: '\u{4E00}',
            codepoint: 0x4E00,
        }
    );
    assert_eq!(
        error.to_string(),
        "font Inter has no glyph for '一' (U+4E00)"
    );
}

#[test]
fn empty_text_is_rejected() {
    assert_eq!(
        measure("", &card_function(), None),
        Err(MeasureError::EmptyText)
    );
}

#[test]
fn size_bounds() {
    for size_px in [6.0, 96.0] {
        let style = TextStyle {
            size_px,
            line_height_px: size_px,
            ..card_function()
        };
        assert!(measure("a", &style, None).is_ok(), "size {size_px}");
    }
    for size_px in [5.0, 97.0, f32::NAN] {
        let style = TextStyle {
            size_px,
            line_height_px: 120.0,
            ..card_function()
        };
        assert!(
            matches!(
                measure("a", &style, None),
                Err(MeasureError::InvalidStyle { .. })
            ),
            "size {size_px}"
        );
    }
}

#[test]
fn line_height_below_size_is_rejected() {
    let style = TextStyle {
        line_height_px: 12.9,
        ..card_function()
    };
    assert!(matches!(
        measure("a", &style, None),
        Err(MeasureError::InvalidStyle { .. })
    ));
}

#[test]
fn infinite_line_height_is_rejected() {
    let style = TextStyle {
        line_height_px: f32::INFINITY,
        ..card_function()
    };
    assert_eq!(
        measure("a", &style, None),
        Err(MeasureError::InvalidStyle {
            reason: "line height is not finite"
        })
    );
    let tall = TextStyle {
        line_height_px: 1000.0,
        ..card_function()
    };
    assert!(measure("a", &tall, None).is_ok());
}

#[test]
fn letter_spacing_bounds() {
    for letter_spacing_em in [-0.2, 0.5] {
        let style = TextStyle {
            letter_spacing_em,
            ..card_function()
        };
        assert!(
            measure("a", &style, None).is_ok(),
            "spacing {letter_spacing_em}"
        );
    }
    for letter_spacing_em in [-0.21, 0.51] {
        let style = TextStyle {
            letter_spacing_em,
            ..card_function()
        };
        assert!(
            matches!(
                measure("a", &style, None),
                Err(MeasureError::InvalidStyle { .. })
            ),
            "spacing {letter_spacing_em}"
        );
    }
}

#[test]
fn negative_or_nan_max_width_is_rejected() {
    assert_eq!(
        measure("a", &card_function(), Some(-1.0)),
        Err(MeasureError::InvalidMaxWidth { max_width_px: -1.0 })
    );
    assert!(matches!(
        measure("a", &card_function(), Some(f32::NAN)),
        Err(MeasureError::InvalidMaxWidth { max_width_px }) if max_width_px.is_nan()
    ));
    assert!(matches!(
        measure("a", &card_function(), Some(f32::INFINITY)),
        Err(MeasureError::InvalidMaxWidth { .. })
    ));
}
