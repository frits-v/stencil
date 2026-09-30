#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use stencil_model::text::{
    FontFamily, FontWeight, MeasureError, TEXT_STYLES, TextMeasurer, TextMetrics, TextStyle,
    TextStyleName, WRAP_EPSILON_PX,
};
use stencil_text::CosmicTextMeasurer;

/// Strings for the per-style tests: g7 labels, interior hyphens and slashes, digits and
/// the two non-ASCII characters the stencil uses.
const SAMPLE_STRINGS: [&str; 8] = [
    "On-prem router 1",
    "link-local /29",
    "Dedicated Interconnect",
    "failover · Region A ↔ Region B",
    "VLAN 100 · BGP 169.254.0.1",
    "Type 5 · Dedicated Interconnect 99.99% · SOW / deck / exec",
    "A stakeholder following the lines still sees four attachments, not one bundled cable.",
    "x",
];

/// Strings whose only break opportunities are U+0020.
const SPACE_BROKEN_STRINGS: [&str; 4] = [
    "Dedicated Interconnect attachment",
    "Metro 1 Region A",
    "Cloud Router pair in Region B",
    "single",
];

fn measurer() -> CosmicTextMeasurer {
    CosmicTextMeasurer::new().unwrap()
}

fn drawn_text(style_name: TextStyleName, text: &str) -> String {
    if style_name.text_style().uppercase {
        text.to_uppercase()
    } else {
        text.to_string()
    }
}

fn card_function() -> TextStyle {
    TextStyleName::CardFunction.text_style().style
}

fn line_texts<'a>(text: &'a str, metrics: &TextMetrics) -> Vec<&'a str> {
    metrics
        .lines
        .iter()
        .map(|line| &text[line.byte_start..line.byte_end])
        .collect()
}

fn assert_contract_invariants(text: &str, style: &TextStyle, metrics: &TextMetrics) {
    assert!(metrics.line_count >= 1, "{text:?}");
    assert_eq!(metrics.line_count as usize, metrics.lines.len(), "{text:?}");
    let widest = metrics
        .lines
        .iter()
        .fold(0.0_f32, |widest, line| widest.max(line.width_px));
    assert_eq!(metrics.width_px, widest, "{text:?}");
    assert_eq!(
        metrics.height_px,
        metrics.line_count as f32 * style.line_height_px,
        "{text:?}"
    );
    for line in &metrics.lines {
        assert!(line.byte_start <= line.byte_end && line.byte_end <= text.len());
        assert!(!text[line.byte_start..line.byte_end].ends_with(' '));
    }
}

fn metric_bits(metrics: &TextMetrics) -> Vec<u64> {
    let mut bits = vec![
        u64::from(metrics.width_px.to_bits()),
        u64::from(metrics.height_px.to_bits()),
        u64::from(metrics.line_count),
    ];
    for line in &metrics.lines {
        bits.extend([
            line.byte_start as u64,
            line.byte_end as u64,
            u64::from(line.width_px.to_bits()),
            u64::from(line.baseline_px.to_bits()),
        ]);
    }
    bits
}

#[test]
fn on_prem_router_at_min_content_breaks_after_the_hyphen() {
    let text = "On-prem router 1";
    let metrics = measurer()
        .measure(text, &card_function(), Some(0.0))
        .unwrap();
    assert_eq!(metrics.line_count, 4);
    assert_eq!(
        line_texts(text, &metrics),
        vec!["On-", "prem", "router", "1"]
    );
    assert_contract_invariants(text, &card_function(), &metrics);
}

#[test]
fn link_local_at_min_content_breaks_after_the_hyphen_only() {
    let text = "link-local /29";
    let metrics = measurer()
        .measure(text, &card_function(), Some(0.0))
        .unwrap();
    assert_eq!(line_texts(text, &metrics), vec!["link-", "local /29"]);
}

#[test]
fn no_max_width_keeps_one_line() {
    let text = "On-prem router 1";
    let metrics = measurer().measure(text, &card_function(), None).unwrap();
    assert_eq!(metrics.line_count, 1);
    assert_eq!(line_texts(text, &metrics), vec![text]);
    assert!(metrics.width_px > 0.0);
    assert_eq!(metrics.height_px, 15.6);
    assert!(metrics.lines[0].baseline_px > 0.0 && metrics.lines[0].baseline_px < 15.6);
}

#[test]
fn min_content_equals_widest_space_separated_word() {
    let mut measurer = measurer();
    for named in &TEXT_STYLES {
        for sample in SPACE_BROKEN_STRINGS {
            let text = drawn_text(named.name, sample);
            let widest_word = text
                .split(' ')
                .map(|word| measurer.measure(word, &named.style, None).unwrap().width_px)
                .fold(0.0_f32, f32::max);
            let min_content = measurer.measure(&text, &named.style, Some(0.0)).unwrap();
            assert_eq!(
                min_content.width_px,
                widest_word,
                "{text:?} in {}",
                named.name.as_str()
            );
            assert_eq!(
                min_content.line_count as usize,
                text.split(' ').count(),
                "{text:?}"
            );
            assert_contract_invariants(&text, &named.style, &min_content);
        }
    }
}

#[test]
fn remeasuring_at_measured_width_plus_epsilon_is_idempotent() {
    let mut measurer = measurer();
    for named in &TEXT_STYLES {
        for sample in SAMPLE_STRINGS {
            let text = drawn_text(named.name, sample);
            let max_content = measurer.measure(&text, &named.style, None).unwrap();
            let min_content = measurer.measure(&text, &named.style, Some(0.0)).unwrap();
            let halfway = (max_content.width_px + min_content.width_px) / 2.0;
            let wrapped = measurer
                .measure(&text, &named.style, Some(halfway))
                .unwrap();
            // Min-content is left out: its overflowing words sit one per line, and re-wrapping
            // at the widest word's width packs the shorter ones together.
            for first in [max_content, wrapped] {
                assert_contract_invariants(&text, &named.style, &first);
                let again = measurer
                    .measure(&text, &named.style, Some(first.width_px + WRAP_EPSILON_PX))
                    .unwrap();
                assert_eq!(
                    (again.line_count, again.width_px),
                    (first.line_count, first.width_px),
                    "{text:?} in {}",
                    named.name.as_str()
                );
            }
        }
    }
}

#[test]
fn word_wider_than_max_width_stays_whole_and_overflows() {
    let text = "Interconnect";
    let mut measurer = measurer();
    let natural = measurer.measure(text, &card_function(), None).unwrap();
    let squeezed = measurer
        .measure(text, &card_function(), Some(natural.width_px / 2.0))
        .unwrap();
    assert_eq!(squeezed.line_count, 1);
    assert_eq!(squeezed.width_px, natural.width_px);
    assert!(squeezed.width_px > natural.width_px / 2.0);
}

#[test]
fn wrapping_splits_at_a_width_between_two_words() {
    let text = "Dedicated Interconnect";
    let mut measurer = measurer();
    let whole = measurer.measure(text, &card_function(), None).unwrap();
    let wrapped = measurer
        .measure(text, &card_function(), Some(whole.width_px - 1.0))
        .unwrap();
    assert_eq!(
        line_texts(text, &wrapped),
        vec!["Dedicated", "Interconnect"]
    );
    assert!(wrapped.lines[1].baseline_px > wrapped.lines[0].baseline_px);
    let fitted = measurer
        .measure(text, &card_function(), Some(whole.width_px))
        .unwrap();
    assert_eq!(fitted.line_count, 1);
}

#[test]
fn line_widths_exclude_trailing_spaces() {
    let mut measurer = measurer();
    let word = measurer.measure("foo", &card_function(), None).unwrap();
    let trailing = measurer.measure("foo ", &card_function(), None).unwrap();
    assert_eq!(trailing.width_px, word.width_px);
    assert_eq!(line_texts("foo ", &trailing), vec!["foo"]);

    let text = "foo   bar";
    let wrapped = measurer
        .measure(text, &card_function(), Some(word.width_px + 1.0))
        .unwrap();
    assert_eq!(line_texts(text, &wrapped), vec!["foo", "bar"]);
    assert_eq!(wrapped.lines[0].width_px, word.width_px);
}

#[test]
fn byte_ranges_address_the_whole_string_across_line_endings() {
    let text = "ab\ncd ef";
    let metrics = measurer().measure(text, &card_function(), None).unwrap();
    assert_eq!(line_texts(text, &metrics), vec!["ab", "cd ef"]);
    assert_eq!(metrics.height_px, 2.0 * card_function().line_height_px);
}

#[test]
fn letter_spacing_adds_its_advance_after_every_glyph() {
    let text = "ABCDEFGHIJKLMNOPQRST";
    let plain = TextStyle {
        family: FontFamily::Inter,
        weight: FontWeight::Bold,
        size_px: 11.0,
        line_height_px: 13.2,
        letter_spacing_em: 0.0,
    };
    let spaced = TextStyle {
        letter_spacing_em: 0.08,
        ..plain
    };
    let mut measurer = measurer();
    let plain_width = measurer.measure(text, &plain, None).unwrap().width_px;
    let spaced_width = measurer.measure(text, &spaced, None).unwrap().width_px;
    let added = spaced_width - plain_width;
    assert!((added - 17.6).abs() <= 0.5, "added {added}");
}

#[test]
fn heavier_weights_measure_wider() {
    let text = "Dedicated Interconnect";
    let mut measurer = measurer();
    let widths: Vec<f32> = [
        FontWeight::Regular,
        FontWeight::SemiBold,
        FontWeight::Bold,
        FontWeight::ExtraBold,
    ]
    .into_iter()
    .map(|weight| {
        let style = TextStyle {
            weight,
            ..card_function()
        };
        measurer.measure(text, &style, None).unwrap().width_px
    })
    .collect();
    assert!(
        widths.windows(2).all(|pair| pair[0] < pair[1]),
        "{widths:?}"
    );
}

#[test]
fn measurement_is_bit_identical_across_calls_and_instances() {
    let mut first_measurer = measurer();
    let mut second_measurer = measurer();
    for named in &TEXT_STYLES {
        for sample in SAMPLE_STRINGS {
            let text = drawn_text(named.name, sample);
            for max_width in [None, Some(0.0), Some(60.0)] {
                let first = first_measurer
                    .measure(&text, &named.style, max_width)
                    .unwrap();
                let repeated = first_measurer
                    .measure(&text, &named.style, max_width)
                    .unwrap();
                let fresh = second_measurer
                    .measure(&text, &named.style, max_width)
                    .unwrap();
                assert_eq!(metric_bits(&first), metric_bits(&repeated));
                assert_eq!(metric_bits(&first), metric_bits(&fresh));
            }
        }
    }
}

#[test]
fn missing_glyphs_are_reported_with_their_character() {
    let mut measurer = measurer();
    for (text, character) in [
        ("\u{4E00}", '\u{4E00}'),
        ("\u{1F600}", '\u{1F600}'),
        ("Region \u{4E00} A", '\u{4E00}'),
    ] {
        let error = measurer.measure(text, &card_function(), None).unwrap_err();
        assert_eq!(
            error,
            MeasureError::MissingGlyph {
                family: "Inter",
                character,
                codepoint: u32::from(character),
            }
        );
    }
}

#[test]
fn empty_text_is_rejected() {
    let error = measurer().measure("", &card_function(), None).unwrap_err();
    assert_eq!(error, MeasureError::EmptyText);
}

#[test]
fn font_size_is_accepted_at_its_limits_and_rejected_past_them() {
    let mut measurer = measurer();
    for size_px in [6.0, 96.0] {
        let style = TextStyle {
            size_px,
            line_height_px: size_px * 1.2,
            ..card_function()
        };
        measurer.measure("Metro", &style, None).unwrap();
    }
    for size_px in [5.0, 97.0] {
        let style = TextStyle {
            size_px,
            line_height_px: size_px * 1.2,
            ..card_function()
        };
        let error = measurer.measure("Metro", &style, None).unwrap_err();
        assert!(
            matches!(error, MeasureError::InvalidStyle { .. }),
            "{error:?}"
        );
    }
}

#[test]
fn line_height_below_size_and_letter_spacing_out_of_range_are_rejected() {
    let mut measurer = measurer();
    let short_line = TextStyle {
        line_height_px: 12.9,
        ..card_function()
    };
    for style in [
        short_line,
        TextStyle {
            letter_spacing_em: 0.51,
            ..card_function()
        },
        TextStyle {
            letter_spacing_em: -0.21,
            ..card_function()
        },
    ] {
        let error = measurer.measure("Metro", &style, None).unwrap_err();
        assert!(
            matches!(error, MeasureError::InvalidStyle { .. }),
            "{error:?}"
        );
    }
    for letter_spacing_em in [0.5, -0.2] {
        let style = TextStyle {
            letter_spacing_em,
            ..card_function()
        };
        measurer.measure("Metro", &style, None).unwrap();
    }
}

#[test]
fn negative_or_non_finite_max_width_is_rejected() {
    let mut measurer = measurer();
    for max_width_px in [-1.0, f32::NAN, f32::INFINITY] {
        let error = measurer
            .measure("Metro", &card_function(), Some(max_width_px))
            .unwrap_err();
        assert!(
            matches!(error, MeasureError::InvalidMaxWidth { .. }),
            "{max_width_px}: {error:?}"
        );
    }
    measurer
        .measure("Metro", &card_function(), Some(0.0))
        .unwrap();
}
