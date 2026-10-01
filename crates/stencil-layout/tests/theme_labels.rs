#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

//! Section 13.4 rule 4: a drawn legend label runs at most LEGEND_RELABEL_SLACK_PX past its
//! canonical label.

use stencil_layout::theme_legend_labels;
use stencil_model::text::FixedMetricsMeasurer;
use stencil_model::{ThemeRule, parse_theme};

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

/// The imported tier (section 13.9), whose slot names are fixed by slot.
const IMPORTED: [(&str, &str); 7] = [
    (
        "tokyo-night",
        include_str!("../../stencil-render/themes/imported/tokyo-night.json"),
    ),
    (
        "solarized-light",
        include_str!("../../stencil-render/themes/imported/solarized-light.json"),
    ),
    (
        "solarized-dark",
        include_str!("../../stencil-render/themes/imported/solarized-dark.json"),
    ),
    (
        "material-dark",
        include_str!("../../stencil-render/themes/imported/material-dark.json"),
    ),
    (
        "gruvbox-dark",
        include_str!("../../stencil-render/themes/imported/gruvbox-dark.json"),
    ),
    (
        "dracula",
        include_str!("../../stencil-render/themes/imported/dracula.json"),
    ),
    (
        "nord",
        include_str!("../../stencil-render/themes/imported/nord.json"),
    ),
];

#[test]
fn every_builtin_passes_the_legend_label_rule() {
    let mut examined = 0;
    for (name, json_text) in BUILTINS.into_iter().chain(IMPORTED) {
        let theme = parse_theme(json_text, name).unwrap();
        let violations = theme_legend_labels(&theme, &mut FixedMetricsMeasurer::default()).unwrap();
        assert_eq!(violations, Vec::new(), "{name}");
        examined += 1;
    }
    assert_eq!(examined, stencil_model::BUILTIN_THEMES.len());
}

#[test]
fn a_slot_named_vermillion_is_too_wide_at_its_name() {
    let mut theme = parse_theme(BUILTINS[0].1, "center").unwrap();
    theme.tints[1].name = "vermillion".to_string();
    let violations = theme_legend_labels(&theme, &mut FixedMetricsMeasurer::default()).unwrap();
    assert!(!violations.is_empty());
    for violation in &violations {
        assert_eq!(violation.rule, ThemeRule::LegendLabelTooWide);
        assert_eq!(violation.pointer.as_str(), "/tints/1/name");
        assert_eq!(violation.rule.as_str(), "theme-legend-label-too-wide");
    }
    assert!(
        violations[0]
            .message
            .starts_with("legend label \"Solid vermillion\" runs ")
    );
}
