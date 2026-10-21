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
    "Dedicated Interconnect 99.99% · two metros, two regions",
    "A reader following the lines sees four attachments, not one bundled cable.",
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

/// Section 13.11: the shared break opportunities, with the fake as the reference.
#[test]
fn dotted_and_hyphenated_names_break_as_the_fake_does() {
    use stencil_model::text::FixedMetricsMeasurer;
    let cases: [(&str, &[&str]); 4] = [
        (
            "acme-prod.analytics.raw-events",
            &["acme-", "prod.", "analytics.", "raw-", "events"],
        ),
        ("10.8.0.0/28", &["10.8.0.0/28"]),
        ("v1.2", &["v1.2"]),
        ("gs://acme/raw", &["gs://", "acme/", "raw"]),
    ];
    let mut cosmic = measurer();
    let mut fake = FixedMetricsMeasurer::default();
    for (text, expected) in cases {
        let metrics = cosmic.measure(text, &card_function(), Some(0.0)).unwrap();
        assert_eq!(line_texts(text, &metrics), expected, "{text}");
        assert!(!text.contains('\u{200B}'));
        assert_contract_invariants(text, &card_function(), &metrics);
        let faked = fake.measure(text, &card_function(), Some(0.0)).unwrap();
        assert_eq!(line_texts(text, &faked), expected, "fake {text}");
    }
}

#[test]
fn a_dotted_name_keeps_its_max_content_width_and_one_line() {
    let text = "acme-prod.analytics.raw-events";
    let mut cosmic = measurer();
    let whole = cosmic.measure(text, &card_function(), None).unwrap();
    assert_eq!(line_texts(text, &whole), [text]);
    let at_width = cosmic
        .measure(
            text,
            &card_function(),
            Some(whole.width_px + WRAP_EPSILON_PX),
        )
        .unwrap();
    assert_eq!(metric_bits(&at_width), metric_bits(&whole));
}

fn width_of(measurer: &mut CosmicTextMeasurer, text: &str) -> f32 {
    measurer
        .measure(text, &card_function(), None)
        .unwrap()
        .width_px
}

fn assert_baselines_one_line_apart(text: &str, metrics: &TextMetrics) {
    let line_height_px = card_function().line_height_px;
    for (line, next_line) in metrics.lines.iter().zip(metrics.lines.iter().skip(1)) {
        let step_px = next_line.baseline_px - line.baseline_px;
        assert!(
            (step_px - line_height_px).abs() < 0.001,
            "{text:?}: baselines {} apart",
            step_px
        );
    }
}

#[test]
fn an_indent_of_zero_measures_as_measure_does() {
    let mut measurer = measurer();
    let widths = [None, Some(0.0), Some(80.0), Some(200.0)];
    for named in &TEXT_STYLES {
        for sample in SAMPLE_STRINGS {
            let text = drawn_text(named.name, sample);
            for max_width_px in widths {
                let indented = measurer
                    .measure_indented(&text, &named.style, max_width_px, 0.0)
                    .unwrap();
                let plain = measurer.measure(&text, &named.style, max_width_px).unwrap();
                assert_eq!(
                    metric_bits(&indented),
                    metric_bits(&plain),
                    "{}: {text:?} at {max_width_px:?}",
                    named.name.as_str()
                );
            }
        }
    }
}

#[test]
fn a_first_line_indent_narrows_only_the_first_line() {
    let text = "Dedicated Interconnect attachment";
    let mut measurer = measurer();
    let max_width_px = width_of(&mut measurer, "Dedicated Interconnect") + WRAP_EPSILON_PX;
    let unindented = measurer
        .measure(text, &card_function(), Some(max_width_px))
        .unwrap();
    assert_eq!(
        line_texts(text, &unindented),
        ["Dedicated Interconnect", "attachment"]
    );

    let first_word_px = width_of(&mut measurer, "Dedicated");
    let first_line_indent_px = max_width_px - first_word_px - 1.0;
    let indented = measurer
        .measure_indented(
            text,
            &card_function(),
            Some(max_width_px),
            first_line_indent_px,
        )
        .unwrap();
    let rest_start = "Dedicated ".len();
    let rest = measurer
        .measure(&text[rest_start..], &card_function(), Some(max_width_px))
        .unwrap();
    let indented_texts = line_texts(text, &indented);
    assert_eq!(indented_texts[0], "Dedicated");
    assert_eq!(
        indented_texts[1..],
        line_texts(&text[rest_start..], &rest)[..]
    );
    assert_eq!(indented.lines[0].width_px, first_word_px);
    for (indented_line, rest_line) in indented.lines[1..].iter().zip(&rest.lines) {
        assert_eq!(indented_line.width_px, rest_line.width_px);
        assert!(indented_line.width_px <= max_width_px);
    }
    assert_eq!(indented.line_count, 1 + rest.line_count);
    assert_eq!(
        indented.height_px,
        indented.line_count as f32 * card_function().line_height_px
    );
    assert_eq!(
        indented.width_px,
        (first_line_indent_px + first_word_px).max(rest.width_px)
    );
    assert_eq!(
        indented.lines[0].baseline_px,
        unindented.lines[0].baseline_px
    );
    assert_baselines_one_line_apart(text, &indented);
}

#[test]
fn a_body_that_fits_after_the_indent_stays_on_one_line_and_its_width_counts_the_indent() {
    let text = "Metro 1 Region A";
    let mut measurer = measurer();
    let natural = measurer.measure(text, &card_function(), None).unwrap();
    for max_width_px in [Some(natural.width_px + 40.0 + WRAP_EPSILON_PX), None] {
        let indented = measurer
            .measure_indented(text, &card_function(), max_width_px, 40.0)
            .unwrap();
        assert_eq!(line_texts(text, &indented), [text], "{max_width_px:?}");
        assert_eq!(indented.lines, natural.lines);
        assert_eq!(indented.width_px, 40.0 + natural.width_px);
    }
    let one_short = measurer
        .measure_indented(
            text,
            &card_function(),
            Some(natural.width_px + 40.0 - 1.0),
            40.0,
        )
        .unwrap();
    assert_eq!(one_short.line_count, 2);
}

#[test]
fn a_first_word_that_does_not_fit_after_the_indent_starts_on_line_1() {
    let text = "Dedicated Interconnect";
    let mut measurer = measurer();
    let max_width_px = width_of(&mut measurer, text) - 1.0;
    let unindented = measurer
        .measure(text, &card_function(), Some(max_width_px))
        .unwrap();
    assert_eq!(line_texts(text, &unindented), ["Dedicated", "Interconnect"]);
    let first_word_px = width_of(&mut measurer, "Dedicated");
    for first_line_indent_px in [
        max_width_px - first_word_px + 1.0,
        max_width_px,
        max_width_px + 30.0,
    ] {
        let indented = measurer
            .measure_indented(
                text,
                &card_function(),
                Some(max_width_px),
                first_line_indent_px,
            )
            .unwrap();
        assert_eq!(
            line_texts(text, &indented),
            ["", "Dedicated", "Interconnect"],
            "{first_line_indent_px}"
        );
        assert_eq!(
            (indented.lines[0].byte_start, indented.lines[0].byte_end),
            (0, 0)
        );
        assert_eq!(indented.lines[0].width_px, 0.0);
        for (indented_line, unindented_line) in indented.lines[1..].iter().zip(&unindented.lines) {
            assert_eq!(indented_line.width_px, unindented_line.width_px);
        }
        assert_eq!(indented.width_px, unindented.width_px);
        assert_eq!(indented.line_count, 3);
        assert_eq!(
            indented.lines[0].baseline_px,
            unindented.lines[0].baseline_px
        );
        assert_baselines_one_line_apart(text, &indented);
    }
}

#[test]
fn an_indented_run_breaks_as_the_fake_does() {
    use stencil_model::text::FixedMetricsMeasurer;
    let cases: [(&str, &[&str]); 3] = [
        (
            "acme-prod.analytics.raw-events",
            &["", "acme-", "prod.", "analytics.", "raw-", "events"],
        ),
        ("10.8.0.0/28", &["", "10.8.0.0/28"]),
        ("On-prem router 1", &["", "On-", "prem", "router", "1"]),
    ];
    let mut cosmic = measurer();
    let mut fake = FixedMetricsMeasurer::default();
    for (text, expected) in cases {
        let indented = cosmic
            .measure_indented(text, &card_function(), Some(0.0), 4.0)
            .unwrap();
        assert_eq!(line_texts(text, &indented), expected, "{text}");
        let faked = fake
            .measure_indented(text, &card_function(), Some(0.0), 4.0)
            .unwrap();
        assert_eq!(line_texts(text, &faked), expected, "fake {text}");

        let unwrapped = cosmic
            .measure_indented(text, &card_function(), None, 4.0)
            .unwrap();
        assert_eq!(line_texts(text, &unwrapped), [text]);
        assert_eq!(unwrapped.width_px, 4.0 + width_of(&mut cosmic, text));
        let faked_unwrapped = fake
            .measure_indented(text, &card_function(), None, 4.0)
            .unwrap();
        assert_eq!(line_texts(text, &faked_unwrapped), [text], "fake {text}");
    }
}

#[test]
fn a_negative_or_non_finite_indent_is_rejected() {
    let mut measurer = measurer();
    for first_line_indent_px in [-1.0, f32::NAN, f32::INFINITY] {
        let error = measurer
            .measure_indented("Metro", &card_function(), Some(100.0), first_line_indent_px)
            .unwrap_err();
        assert!(
            matches!(error, MeasureError::InvalidIndent { .. }),
            "{first_line_indent_px}: {error:?}"
        );
    }
}
