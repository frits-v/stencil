// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use stencil_model::text::{
    FixedMetricsMeasurer, FontFamily, FontWeight, MeasureError, TEXT_STYLES, TextLine,
    TextMeasurer, TextStyle, TextStyleName, WRAP_EPSILON_PX,
};

const ALL_STYLE_NAMES: [TextStyleName; 39] = [
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
    TextStyleName::BlockBody,
    TextStyleName::SheetLabel,
    TextStyleName::SheetKicker,
    TextStyleName::Wordmark,
    TextStyleName::Motto,
    TextStyleName::SheetTitle,
    TextStyleName::Statement,
    TextStyleName::ViewNumeral,
    TextStyleName::ViewTitle,
    TextStyleName::ViewNote,
    TextStyleName::ViewNoteSmall,
    TextStyleName::LeadBody,
    TextStyleName::ActorName,
    TextStyleName::LaneName,
    TextStyleName::StepTitle,
    TextStyleName::StepText,
    TextStyleName::ListTitle,
    TextStyleName::MarkNumeral,
    TextStyleName::MarkNumeralSmall,
    TextStyleName::FlagNumeral,
    TextStyleName::RungTitle,
    TextStyleName::RungWho,
];

/// Section 2.9: name, weight, size, line height, letter spacing, uppercase.
const SECTION_2_9: [(&str, u16, f32, f32, f32, bool); 39] = [
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
    ("block_body", 400, 12.0, 17.4, 0.0, false),
    ("sheet_label", 600, 11.0, 13.2, 0.12, true),
    ("sheet_kicker", 600, 10.0, 12.0, 0.05, true),
    ("wordmark", 800, 46.0, 55.0, -0.02, false),
    ("motto", 700, 13.0, 16.0, 0.06, true),
    ("sheet_title", 800, 26.0, 30.0, 0.0, false),
    ("statement", 700, 18.0, 23.4, 0.0, false),
    ("view_numeral", 700, 13.0, 15.6, 0.0, false),
    ("view_title", 700, 16.0, 19.2, 0.1, true),
    ("view_note", 400, 14.0, 18.0, 0.0, false),
    ("view_note_small", 400, 12.0, 15.6, 0.0, false),
    ("lead_body", 700, 12.0, 17.4, 0.0, false),
    ("actor_name", 700, 11.0, 13.2, 0.08, true),
    ("lane_name", 700, 13.0, 15.6, 0.1, true),
    ("step_title", 700, 12.0, 15.0, 0.0, false),
    ("step_text", 400, 11.5, 15.0, 0.0, false),
    ("list_title", 700, 13.0, 15.6, 0.12, true),
    ("mark_numeral", 700, 10.0, 10.0, 0.0, false),
    ("mark_numeral_small", 700, 9.5, 9.5, 0.0, false),
    ("flag_numeral", 700, 9.0, 9.0, 0.0, false),
    ("rung_title", 700, 12.0, 14.4, 0.08, true),
    ("rung_who", 400, 12.0, 14.4, 0.0, false),
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
    assert_eq!(TEXT_STYLES.len(), SECTION_2_9.len());
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
    let text = "On-prem router 1";
    let metrics = measure(text, &card_function(), Some(0.0)).unwrap();
    assert_eq!(line_texts(text, &metrics), ["On-", "prem", "router", "1"]);
    assert_eq!(metrics.width_px, 6.0 * 6.5);
    assert_eq!(metrics.lines[0].width_px, 3.0 * 6.5);
}

fn line_texts<'a>(text: &'a str, metrics: &stencil_model::text::TextMetrics) -> Vec<&'a str> {
    metrics
        .lines
        .iter()
        .map(|line| &text[line.byte_start..line.byte_end])
        .collect()
}

#[test]
fn a_bucket_name_breaks_after_each_hyphen_and_dot_before_a_letter() {
    let text = "acme-prod.analytics.raw-events";
    let metrics = measure(text, &card_function(), Some(0.0)).unwrap();
    assert_eq!(
        line_texts(text, &metrics),
        ["acme-", "prod.", "analytics.", "raw-", "events"]
    );
    assert_eq!(metrics.width_px, 10.0 * 6.5);
}

#[test]
fn addresses_versions_and_ranges_stay_whole() {
    for text in ["10.8.0.0/28", "v1.2", "-29", "a.1", "x-9"] {
        let metrics = measure(text, &card_function(), Some(0.0)).unwrap();
        assert_eq!(line_texts(text, &metrics), [text], "{text}");
    }
}

#[test]
fn a_storage_path_breaks_after_each_slash_before_a_letter() {
    let text = "gs://acme/raw";
    let metrics = measure(text, &card_function(), Some(0.0)).unwrap();
    assert_eq!(line_texts(text, &metrics), ["gs://", "acme/", "raw"]);
}

#[test]
fn break_opportunities_follow_section_13_11() {
    use stencil_model::text::break_opportunities;
    assert_eq!(break_opportunities("On-prem router 1"), [3, 8, 15]);
    assert_eq!(break_opportunities("a  b"), [3]);
    assert_eq!(break_opportunities("  lead"), Vec::<usize>::new());
    assert_eq!(break_opportunities("trail  "), Vec::<usize>::new());
    assert_eq!(break_opportunities("10.8.0.0/28 -29"), [12]);
    assert_eq!(break_opportunities("acme.Raw/x-Y"), [5, 9, 11]);
    assert_eq!(break_opportunities("a·b-c"), [5]);
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

fn measure_indented(
    text: &str,
    max_width_px: Option<f32>,
    first_line_indent_px: f32,
) -> Result<stencil_model::text::TextMetrics, MeasureError> {
    FixedMetricsMeasurer::default().measure_indented(
        text,
        &card_function(),
        max_width_px,
        first_line_indent_px,
    )
}

#[test]
fn an_indent_of_zero_measures_as_measure_does() {
    let texts = [
        "On-prem router 1",
        "acme-prod.analytics.raw-events",
        "Interconnect",
        "  leading spaces",
    ];
    let widths = [None, Some(0.0), Some(20.0), Some(50.0), Some(100.0)];
    for text in texts {
        for max_width_px in widths {
            assert_eq!(
                measure_indented(text, max_width_px, 0.0).unwrap(),
                measure(text, &card_function(), max_width_px).unwrap(),
                "{text:?} at {max_width_px:?}"
            );
        }
    }
}

#[test]
fn a_first_line_indent_narrows_only_the_first_line() {
    // 6.5 px per scalar. Unindented at 100 the lines are "On-prem router" and "1". After 13
    // px the first line has 87: "On-prem router" (91) does not fit, "On-prem" (45.5) does,
    // and the second line wraps at the full 100.
    let text = "On-prem router 1";
    let metrics = measure_indented(text, Some(100.0), 13.0).unwrap();
    assert_eq!(line_texts(text, &metrics), ["On-prem", "router 1"]);
    assert_eq!(metrics.lines[0].width_px, 45.5);
    assert_eq!(metrics.lines[1].width_px, 52.0);
    assert_eq!(metrics.width_px, 13.0 + 45.5);
    assert_eq!(metrics.line_count, 2);
    assert_eq!(metrics.height_px, 2.0 * 15.6);
    assert_eq!(metrics.lines[0].baseline_px, 0.8 * 15.6);
    assert_eq!(metrics.lines[1].baseline_px, 15.6 + 0.8 * 15.6);
}

#[test]
fn a_body_that_fits_after_the_indent_stays_on_one_line_and_its_width_counts_the_indent() {
    let text = "On-prem router 1";
    for max_width_px in [Some(117.0), None] {
        let metrics = measure_indented(text, max_width_px, 13.0).unwrap();
        assert_eq!(line_texts(text, &metrics), [text], "{max_width_px:?}");
        assert_eq!(metrics.lines[0].width_px, 104.0);
        assert_eq!(metrics.width_px, 117.0);
    }
    let one_short = measure_indented(text, Some(116.99), 13.0).unwrap();
    assert_eq!(line_texts(text, &one_short), ["On-prem router", "1"]);
}

#[test]
fn a_first_word_that_does_not_fit_after_the_indent_starts_on_line_1() {
    // The first word, "On-", is 19.5 px. An indent of 85 leaves 15 on the first line; one of
    // 120 is wider than the line itself. Either way line 0 is empty, the text wraps at the
    // full width from line 1, and the width leaves the indent out.
    let text = "On-prem router 1";
    let unindented = measure(text, &card_function(), Some(100.0)).unwrap();
    for first_line_indent_px in [85.0, 100.0, 120.0] {
        let metrics = measure_indented(text, Some(100.0), first_line_indent_px).unwrap();
        assert_eq!(
            line_texts(text, &metrics),
            ["", "On-prem router", "1"],
            "{first_line_indent_px}"
        );
        assert_eq!(
            (metrics.lines[0].byte_start, metrics.lines[0].byte_end),
            (0, 0)
        );
        assert_eq!(metrics.lines[0].width_px, 0.0);
        assert_eq!(metrics.width_px, unindented.width_px);
        assert_eq!(metrics.line_count, 3);
        assert_eq!(metrics.height_px, 3.0 * 15.6);
        assert_eq!(metrics.lines[0].baseline_px, 0.8 * 15.6);
        assert_eq!(metrics.lines[2].baseline_px, 2.0 * 15.6 + 0.8 * 15.6);
    }
    let exact_fit = measure_indented(text, Some(100.0), 80.5).unwrap();
    assert_eq!(line_texts(text, &exact_fit), ["On-", "prem router 1"]);
}

#[test]
fn an_indented_word_wider_than_the_line_overflows_on_line_1() {
    let metrics = measure_indented("Interconnect", Some(20.0), 4.0).unwrap();
    assert_eq!(line_texts("Interconnect", &metrics), ["", "Interconnect"]);
    assert_eq!(metrics.width_px, 12.0 * 6.5);
    let at_min_content = measure_indented("On-prem router 1", Some(0.0), 4.0).unwrap();
    assert_eq!(
        line_texts("On-prem router 1", &at_min_content),
        ["", "On-", "prem", "router", "1"]
    );
}

#[test]
fn a_negative_or_non_finite_indent_is_rejected() {
    assert_eq!(
        measure_indented("a", Some(100.0), -1.0),
        Err(MeasureError::InvalidIndent {
            first_line_indent_px: -1.0
        })
    );
    assert!(matches!(
        measure_indented("a", Some(100.0), f32::NAN),
        Err(MeasureError::InvalidIndent { first_line_indent_px }) if first_line_indent_px.is_nan()
    ));
    assert!(matches!(
        measure_indented("a", None, f32::INFINITY),
        Err(MeasureError::InvalidIndent { .. })
    ));
}
