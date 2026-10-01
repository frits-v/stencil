// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 13.4: the theme document, its structural rules, overrides and quality rows.

use serde_json::{Value, json};
use stencil_model::theme::{QualityClass, TOLD_APART_BY_LINE, TintCue, contrast_ratio};
use stencil_model::{
    Line, Theme, ThemeError, ThemeRule, apply_overrides, drawn_legend_label, parse_theme,
    theme_quality, theme_schema, validate_theme,
};

const COMMITTED_THEME_SCHEMA: &str = include_str!("../../../schema/theme.schema.json");

const BUILTINS: [(&str, &str); 6] = [
    (
        "center",
        include_str!("../../stencil-render/themes/center.json"),
    ),
    (
        "paper",
        include_str!("../../stencil-render/themes/paper.json"),
    ),
    (
        "dusk",
        include_str!("../../stencil-render/themes/dusk.json"),
    ),
    (
        "clear",
        include_str!("../../stencil-render/themes/clear.json"),
    ),
    (
        "clear-dark",
        include_str!("../../stencil-render/themes/clear-dark.json"),
    ),
    (
        "wire",
        include_str!("../../stencil-render/themes/wire.json"),
    ),
];

fn builtin(name: &str) -> Theme {
    let (_, json_text) = BUILTINS
        .iter()
        .find(|(builtin, _)| *builtin == name)
        .unwrap();
    parse_theme(json_text, name).unwrap()
}

fn center_value() -> Value {
    serde_json::from_str(BUILTINS[0].1).unwrap()
}

/// The (rule, pointer) pairs of an Invalid error, in order.
fn invalid(result: Result<Theme, ThemeError>) -> Vec<(ThemeRule, String)> {
    match result {
        Err(ThemeError::Invalid { violations, .. }) => violations
            .into_iter()
            .map(|violation| (violation.rule, violation.pointer.as_str().to_string()))
            .collect(),
        other => panic!("expected ThemeError::Invalid, got {other:?}"),
    }
}

fn assert_json_error(result: Result<Theme, ThemeError>, needle: &str) {
    match result {
        Err(ThemeError::Json { message, .. }) => {
            assert!(message.contains(needle), "{message}");
        }
        other => panic!("expected ThemeError::Json, got {other:?}"),
    }
}

#[test]
fn every_builtin_parses_and_passes_validate_theme() {
    for (name, json_text) in BUILTINS {
        let theme = parse_theme(json_text, name).unwrap();
        assert_eq!(theme.name, name);
        assert_eq!(validate_theme(&theme), Vec::new(), "{name}");
    }
}

#[test]
fn theme_schema_equals_the_committed_file() {
    let generated = serde_json::to_string_pretty(&theme_schema()).unwrap() + "\n";
    assert!(
        generated == COMMITTED_THEME_SCHEMA,
        "schema/theme.schema.json is out of date; regenerate it from theme_schema()"
    );
}

#[test]
fn each_structural_fault_is_rejected_with_its_rule_and_pointer() {
    let faults: [(&str, Value, ThemeRule); 9] = [
        ("/page", json!("#ffffff"), ThemeRule::ColorMalformed),
        ("/card/border/width", json!(5.0), ThemeRule::WidthOutOfRange),
        ("/iso/faces/left", json!(41), ThemeRule::StepOutOfRange),
        (
            "/iso/chip/shadow/opacity",
            json!(1.5),
            ThemeRule::OpacityOutOfRange,
        ),
        (
            "/iso/slab_thickness",
            json!(1.0),
            ThemeRule::SlabThicknessOutOfRange,
        ),
        ("/tints/2/name", json!("Teal"), ThemeRule::TintNameMalformed),
        ("/tints/1/name", json!("blue"), ThemeRule::TintNameDuplicate),
        (
            "/tones/strong/fill",
            json!("#000000"),
            ThemeRule::StrongToneFilled,
        ),
        ("/name", json!("Center"), ThemeRule::NameMalformed),
    ];
    for (pointer, value, rule) in &faults {
        let mut document = center_value();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        document
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.to_string(), value.clone());
        let violations = invalid(parse_theme(&document.to_string(), "fault"));
        assert_eq!(violations, [(*rule, pointer.to_string())], "{pointer}");
        assert!(rule.as_str().starts_with("theme-"));
    }
}

#[test]
fn a_zero_blur_block_shadow_is_out_of_range() {
    let mut document = center_value();
    document["iso"]["shadow"] =
        json!({ "color": "#202124", "opacity": 0.2, "blur": 0.0, "dy": 3.0 });
    let violations = invalid(parse_theme(&document.to_string(), "fault"));
    assert_eq!(
        violations,
        [(ThemeRule::ShadowOutOfRange, "/iso/shadow/blur".to_string())]
    );
}

#[test]
fn every_fault_of_one_document_is_reported_together_in_field_order() {
    let mut document = center_value();
    document["iso"]["faces"]["top"] = json!(-41);
    document["tones"]["strong"]["fill"] = json!("#000000");
    document["page"] = json!("#fff000");
    document["tints"][7]["name"] = json!("blue");
    let rules: Vec<ThemeRule> = invalid(parse_theme(&document.to_string(), "faults"))
        .into_iter()
        .map(|(rule, _)| rule)
        .collect();
    assert_eq!(
        rules,
        [
            ThemeRule::ColorMalformed,
            ThemeRule::StrongToneFilled,
            ThemeRule::TintNameDuplicate,
            ThemeRule::StepOutOfRange
        ]
    );
}

#[test]
fn seven_tints_and_unknown_roles_are_json_errors() {
    let mut document = center_value();
    document["tints"].as_array_mut().unwrap().pop();
    assert_json_error(
        parse_theme(&document.to_string(), "seven"),
        "expected an array of length 8",
    );
    let mut document = center_value();
    document["glow"] = json!("#FFFFFF");
    assert_json_error(
        parse_theme(&document.to_string(), "glow"),
        "unknown field `glow`",
    );
    let mut document = center_value();
    document["iso"]["glow"] = json!(1);
    assert_json_error(
        parse_theme(&document.to_string(), "glow"),
        "unknown field `glow`",
    );
}

fn overrides(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().unwrap().clone()
}

#[test]
fn an_override_recolors_one_slot_wire_only() {
    let base = builtin("center");
    let overridden = apply_overrides(
        &base,
        &overrides(json!({ "tints": { "3": { "wire": "#00796B" } } })),
    )
    .unwrap();
    assert_eq!(overridden.tints[2].wire.as_str(), "#00796B");
    let mut expected = base.clone();
    expected.tints[2].wire = overridden.tints[2].wire.clone();
    assert_eq!(overridden, expected);
}

#[test]
fn a_nested_override_merges_key_by_key_and_a_scalar_replaces() {
    let base = builtin("center");
    let overridden = apply_overrides(
        &base,
        &overrides(json!({
            "iso": { "chip": { "shadow": { "opacity": 0.3 } }, "plates": false },
            "icon_chip": "#F1F3F4",
            "solid": { "dots": ["hollow", "filled", "filled", "filled", "filled", "filled", "filled", "filled"] }
        })),
    )
    .unwrap();
    let shadow = overridden.iso.chip.shadow.as_ref().unwrap();
    assert_eq!(shadow.opacity, 0.3);
    assert_eq!(shadow.color.as_str(), "#202124");
    assert!(!overridden.iso.plates);
    assert_eq!(overridden.icon_chip.unwrap().as_str(), "#F1F3F4");
    assert_eq!(
        overridden.solid.dots[0],
        stencil_model::theme::DotStyle::Hollow
    );
    assert_eq!(overridden.iso.faces, base.iso.faces);
}

#[test]
fn override_faults_point_into_theme_overrides() {
    let base = builtin("center");
    let unknown = apply_overrides(&base, &overrides(json!({ "iso": { "glow": 1 } })));
    assert_eq!(
        invalid(unknown),
        [(
            ThemeRule::OverrideUnknownRole,
            "/theme_overrides/iso/glow".to_string()
        )]
    );
    let slot = apply_overrides(
        &base,
        &overrides(json!({ "tints": { "9": { "wire": "#000000" } } })),
    );
    assert_eq!(
        invalid(slot),
        [(
            ThemeRule::OverrideUnknownRole,
            "/theme_overrides/tints/9".to_string()
        )]
    );
    let not_object = apply_overrides(&base, &overrides(json!({ "card": "#FFFFFF" })));
    assert_eq!(
        invalid(not_object),
        [(
            ThemeRule::OverrideNotObject,
            "/theme_overrides/card".to_string()
        )]
    );
    assert_eq!(
        ThemeRule::OverrideNotObject.as_str(),
        "theme-override-not-object"
    );
    let bad_color = apply_overrides(
        &base,
        &overrides(json!({ "tints": { "3": { "wire": "#00796b" } } })),
    );
    assert_eq!(
        invalid(bad_color),
        [(
            ThemeRule::ColorMalformed,
            "/theme_overrides/tints/3/wire".to_string()
        )]
    );
    let wrong_type = apply_overrides(&base, &overrides(json!({ "iso": { "plates": "yes" } })));
    assert!(matches!(wrong_type, Err(ThemeError::Json { .. })));
}

#[test]
fn quality_counts_match_section_13_5_for_every_builtin() {
    let expected = [
        ("center", 222, 1, 0),
        ("paper", 223, 0, 0),
        ("dusk", 223, 0, 0),
        ("clear", 223, 0, 0),
        ("clear-dark", 223, 0, 0),
        ("wire", 215, 0, 8),
    ];
    for (name, passed, failed, not_applicable) in expected {
        let report = theme_quality(&builtin(name));
        assert_eq!(report.rows.len(), 223, "{name}");
        assert_eq!(report.counts(), (passed, failed, not_applicable), "{name}");
    }
    let center = theme_quality(&builtin("center"));
    let failures: Vec<_> = center
        .rows
        .iter()
        .filter(|row| row.passed() == Some(false))
        .collect();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].subject, "ask.ink on ask.fill");
    assert_eq!(format!("{:.2}", failures[0].value.unwrap()), "4.34");
    let wire = theme_quality(&builtin("wire"));
    for row in wire
        .rows
        .iter()
        .filter(|row| row.class == QualityClass::Separation)
    {
        assert_eq!(row.not_applicable, Some(TOLD_APART_BY_LINE));
    }
}

/// Values printed by the palette research checker (`accessible/scripts/palette-check.py`
/// of the stencil-palettes-2026-09-30 run) for center and dusk written in its format.
const RESEARCH_CHECKER_VALUES: [(&str, &str, f64); 22] = [
    ("center", "ink.primary on page", 16.10),
    ("center", "ink.secondary on card.fill", 6.05),
    ("center", "tag.sub_ink on tag.fill", 6.05),
    ("center", "deny.tag_ink on tag.fill", 5.80),
    ("center", "fact.ink on fact.fill", 5.44),
    ("center", "badge.customer.ink on badge.customer.fill", 6.85),
    ("center", "frame.bar_ink on frame.bar_fill", 4.51),
    (
        "center",
        "tones.accent.label_ink on tones.accent.fill",
        4.51,
    ),
    ("center", "tints.1.ink on tints.1.fill", 4.65),
    ("center", "wires normal", 29.55),
    ("center", "wires deuteranopia", 13.16),
    ("center", "fills normal", 8.53),
    ("center", "fills deuteranopia", 5.38),
    ("dusk", "ink.primary on page", 15.52),
    ("dusk", "ink.primary on card.fill", 11.37),
    ("dusk", "tag.sub_ink on tag.fill", 5.17),
    ("dusk", "fact.ink on fact.fill", 8.13),
    ("dusk", "frame.bar_ink on frame.bar_fill", 10.23),
    ("dusk", "wires protanopia", 13.46),
    ("dusk", "wires tritanopia", 13.47),
    ("dusk", "fills normal", 9.98),
    ("dusk", "fills protanopia", 6.14),
];

#[test]
fn quality_values_match_the_research_checker_to_two_decimals() {
    for (name, subject, value) in RESEARCH_CHECKER_VALUES {
        let report = theme_quality(&builtin(name));
        let row = report
            .rows
            .iter()
            .find(|row| row.subject == subject)
            .unwrap_or_else(|| panic!("{name}: no row {subject}"));
        assert_eq!(
            format!("{:.2}", row.value.unwrap()),
            format!("{value:.2}"),
            "{name} {subject}"
        );
    }
}

#[test]
fn eight_equal_wires_fail_the_wire_rows_unless_tints_are_told_apart_by_line() {
    let mut theme = builtin("center");
    for tint in &mut theme.tints {
        tint.wire = theme.gray.color.clone();
    }
    let report = theme_quality(&theme);
    let wire_rows: Vec<_> = report
        .rows
        .iter()
        .filter(|row| row.subject.starts_with("wires "))
        .collect();
    assert_eq!(wire_rows.len(), 4);
    for row in &wire_rows {
        assert_eq!(row.passed(), Some(false), "{}", row.subject);
        assert_eq!(row.value, Some(0.0));
    }
    theme.tint_cue = TintCue::Line;
    let report = theme_quality(&theme);
    for row in report
        .rows
        .iter()
        .filter(|row| row.class == QualityClass::Separation)
    {
        assert_eq!(row.passed(), None, "{}", row.subject);
    }
}

#[test]
fn contrast_ratio_is_the_wcag_ratio() {
    use stencil_model::theme::Color;
    let black = Color("#000000".to_string());
    let white = Color("#FFFFFF".to_string());
    assert_eq!(
        format!("{:.2}", contrast_ratio(&black, &white).unwrap()),
        "21.00"
    );
    assert_eq!(contrast_ratio(&white, &white), Some(1.0));
    assert_eq!(contrast_ratio(&Color("#fff".to_string()), &white), None);
}

#[test]
fn drawn_legend_labels_follow_rule_9() {
    let center = builtin("center");
    let paper = builtin("paper");
    let wire = builtin("wire");
    assert_eq!(drawn_legend_label(&center, Line::Gray, None), "Solid gray");
    assert_eq!(drawn_legend_label(&center, Line::Deny, None), "Dashed red");
    assert_eq!(
        drawn_legend_label(&center, Line::Dash, Some(2)),
        "Dashed pink"
    );
    assert_eq!(
        drawn_legend_label(&paper, Line::Solid, Some(2)),
        "Solid rose"
    );
    assert_eq!(drawn_legend_label(&wire, Line::Gray, None), "Thin line");
    assert_eq!(
        drawn_legend_label(&wire, Line::Solid, Some(1)),
        "Solid line"
    );
    assert_eq!(
        drawn_legend_label(&wire, Line::Solid, Some(2)),
        "Ringed line"
    );
    assert_eq!(
        drawn_legend_label(&wire, Line::Solid, Some(3)),
        "Plain line"
    );
    assert_eq!(
        drawn_legend_label(&wire, Line::Dash, Some(5)),
        "Dashed line"
    );
    assert_eq!(drawn_legend_label(&wire, Line::Deny, None), "Dotted line");
    for line in Line::ALL {
        for tint in [None, Some(1), Some(2), Some(5), Some(8)] {
            assert_eq!(
                drawn_legend_label(&center, line, tint),
                stencil_model::legend_label(line, tint),
                "{line:?} {tint:?}"
            );
        }
    }
}
