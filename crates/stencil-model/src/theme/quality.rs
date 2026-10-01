//! The contrast and separation rows of section 13.4 rule 10. The arithmetic follows the
//! palette research checker (`accessible/scripts/palette-check.py`): WCAG 2.2 relative
//! luminance, Machado, Oliveira and Fernandes (2009) dichromacy at severity 1.0 applied in
//! linear sRGB and clipped, and CIE76 delta E in CIELAB under D65.

use super::{Color, Theme, TintCue};

pub const TEXT_CONTRAST_MIN: f64 = 4.5;
pub const NON_TEXT_CONTRAST_MIN: f64 = 3.0;
pub const WIRE_SEPARATION_NORMAL_MIN: f64 = 20.0;
pub const WIRE_SEPARATION_DICHROMACY_MIN: f64 = 12.0;
pub const FILL_SEPARATION_NORMAL_MIN: f64 = 8.0;
pub const FILL_SEPARATION_DICHROMACY_MIN: f64 = 5.0;
/// The reason a separation row is not applicable under `tint_cue: line`.
pub const TOLD_APART_BY_LINE: &str = "tints are told apart by line";
const MALFORMED_COLOR: &str = "a color is malformed";

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeReport {
    pub theme: String,
    pub rows: Vec<QualityRow>,
}

impl ThemeReport {
    /// Rows that were examined and passed, failed, and were not applicable.
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut counts = (0, 0, 0);
        for row in &self.rows {
            match row.passed() {
                Some(true) => counts.0 += 1,
                Some(false) => counts.1 += 1,
                None => counts.2 += 1,
            }
        }
        counts
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualityRow {
    pub class: QualityClass,
    /// For example "ink.primary on page" or "wires deuteranopia".
    pub subject: String,
    /// The contrast ratio or the minimum delta E; None when not applicable.
    pub value: Option<f64>,
    pub threshold: f64,
    /// For a separation row, the closest pair of slot names.
    pub closest: Option<(String, String)>,
    pub not_applicable: Option<&'static str>,
}

impl QualityRow {
    /// None when the row is not applicable.
    pub fn passed(&self) -> Option<bool> {
        self.value.map(|value| value >= self.threshold)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityClass {
    Contrast,
    Separation,
}

impl QualityClass {
    pub fn as_str(self) -> &'static str {
        match self {
            QualityClass::Contrast => "contrast",
            QualityClass::Separation => "separation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vision {
    Normal,
    Deuteranopia,
    Protanopia,
    Tritanopia,
}

impl Vision {
    const ALL: [Vision; 4] = [
        Vision::Normal,
        Vision::Deuteranopia,
        Vision::Protanopia,
        Vision::Tritanopia,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Vision::Normal => "normal",
            Vision::Deuteranopia => "deuteranopia",
            Vision::Protanopia => "protanopia",
            Vision::Tritanopia => "tritanopia",
        }
    }

    /// Machado 2009, severity 1.0, rows applied to linear RGB.
    fn matrix(self) -> Option<[[f64; 3]; 3]> {
        match self {
            Vision::Normal => None,
            Vision::Deuteranopia => Some([
                [0.367_322, 0.860_646, -0.227_968],
                [0.280_085, 0.672_501, 0.047_413],
                [-0.011_820, 0.042_940, 0.968_881],
            ]),
            Vision::Protanopia => Some([
                [0.152_286, 1.052_583, -0.204_868],
                [0.114_503, 0.786_281, 0.099_216],
                [-0.003_882, -0.048_116, 1.051_998],
            ]),
            Vision::Tritanopia => Some([
                [1.255_528, -0.076_749, -0.178_779],
                [-0.078_411, 0.930_809, 0.147_602],
                [0.004_733, 0.691_367, 0.303_900],
            ]),
        }
    }
}

/// sRGB channels 0 to 1.
type Rgb = [f64; 3];

fn rgb(color: &Color) -> Option<Rgb> {
    color.channels().map(channel_rgb)
}

fn channel_rgb(channels: [u8; 3]) -> Rgb {
    channels.map(|channel| f64::from(channel) / 255.0)
}

fn to_linear(channel: f64) -> f64 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(channel: f64) -> f64 {
    let channel = channel.clamp(0.0, 1.0);
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

fn luminance(color: Rgb) -> f64 {
    let [red, green, blue] = color.map(to_linear);
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

/// WCAG 2.2 contrast ratio.
pub fn contrast_ratio(first: &Color, second: &Color) -> Option<f64> {
    Some(channel_contrast_ratio(
        first.channels()?,
        second.channels()?,
    ))
}

/// WCAG 2.2 contrast ratio of two colors given as sRGB channels.
pub fn channel_contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f64 {
    let first = luminance(channel_rgb(first));
    let second = luminance(channel_rgb(second));
    let (high, low) = if first >= second {
        (first, second)
    } else {
        (second, first)
    };
    (high + 0.05) / (low + 0.05)
}

fn simulate(color: Rgb, vision: Vision) -> Rgb {
    let Some(matrix) = vision.matrix() else {
        return color;
    };
    let linear = color.map(to_linear);
    matrix.map(|row| to_srgb(row[0] * linear[0] + row[1] * linear[1] + row[2] * linear[2]))
}

fn lab(color: Rgb) -> [f64; 3] {
    let [red, green, blue] = color.map(to_linear);
    let x = (0.412_456_4 * red + 0.357_576_1 * green + 0.180_437_5 * blue) / 0.950_47;
    let y = 0.212_672_9 * red + 0.715_152_2 * green + 0.072_175_0 * blue;
    let z = (0.019_333_9 * red + 0.119_192_0 * green + 0.950_304_1 * blue) / 1.088_83;
    let epsilon = (6.0_f64 / 29.0).powi(3);
    let f = |t: f64| {
        if t > epsilon {
            t.cbrt()
        } else {
            t / (3.0 * (6.0_f64 / 29.0).powi(2)) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIELAB L* of a color, 0 to 100.
pub fn lightness(color: &Color) -> Option<f64> {
    color.channels().map(channel_lightness)
}

/// CIELAB L* of a color given as sRGB channels, 0 to 100.
pub fn channel_lightness(channels: [u8; 3]) -> f64 {
    lab(channel_rgb(channels))[0]
}

fn delta_e(first: Rgb, second: Rgb) -> f64 {
    let first = lab(first);
    let second = lab(second);
    let squares: f64 = first
        .iter()
        .zip(second)
        .map(|(one, other)| (one - other).powi(2))
        .sum();
    squares.sqrt()
}

struct Rows {
    rows: Vec<QualityRow>,
}

impl Rows {
    fn contrast(&mut self, subject: String, ink: &Color, ground: &Color, threshold: f64) {
        let value = contrast_ratio(ink, ground);
        self.rows.push(QualityRow {
            class: QualityClass::Contrast,
            subject,
            value,
            threshold,
            closest: None,
            not_applicable: value.is_none().then_some(MALFORMED_COLOR),
        });
    }

    fn text(&mut self, ink: (&str, &Color), ground: (&str, &Color)) {
        let subject = format!("{} on {}", ink.0, ground.0);
        self.contrast(subject, ink.1, ground.1, TEXT_CONTRAST_MIN);
    }

    fn non_text(&mut self, mark: (&str, &Color), ground: (&str, &Color)) {
        let subject = format!("{} on {}", mark.0, ground.0);
        self.contrast(subject, mark.1, ground.1, NON_TEXT_CONTRAST_MIN);
    }
}

/// The contrast and separation rows of rule 10, in row order.
pub fn theme_quality(theme: &Theme) -> ThemeReport {
    let mut rows = Rows { rows: Vec::new() };
    text_rows(theme, &mut rows);
    non_text_rows(theme, &mut rows);
    separation_rows(theme, &mut rows);
    ThemeReport {
        theme: theme.name.clone(),
        rows: rows.rows,
    }
}

fn tone_fills(theme: &Theme) -> Vec<(String, &Color)> {
    theme
        .tones
        .all()
        .into_iter()
        .filter_map(|(name, tone)| {
            tone.fill
                .as_ref()
                .map(|fill| (format!("tones.{name}.fill"), fill))
        })
        .collect()
}

fn text_rows(theme: &Theme, rows: &mut Rows) {
    let page = ("page", &theme.page);
    for ink in [
        ("ink.primary", &theme.ink.primary),
        ("ink.secondary", &theme.ink.secondary),
        ("ink.zone_label", &theme.ink.zone_label),
        ("kicker", &theme.kicker),
        ("foot", &theme.foot),
        ("legend.label_ink", &theme.legend.label_ink),
        ("legend.text_ink", &theme.legend.text_ink),
    ] {
        rows.text(ink, page);
    }
    let card = ("card.fill", &theme.card.fill);
    rows.text(("ink.primary", &theme.ink.primary), card);
    rows.text(("ink.secondary", &theme.ink.secondary), card);
    let tag = ("tag.fill", &theme.tag.fill);
    rows.text(("tag.ink", &theme.tag.ink), tag);
    rows.text(("tag.sub_ink", &theme.tag.sub_ink), tag);
    rows.text(("deny.tag_ink", &theme.deny.tag_ink), tag);
    rows.text(
        ("fact.ink", &theme.fact.ink),
        ("fact.fill", &theme.fact.fill),
    );
    rows.text(("ask.ink", &theme.ask.ink), ("ask.fill", &theme.ask.fill));
    rows.text(
        ("badge.customer.ink", &theme.badge.customer.ink),
        ("badge.customer.fill", &theme.badge.customer.fill),
    );
    rows.text(
        ("badge.internal.ink", &theme.badge.internal.ink),
        ("badge.internal.fill", &theme.badge.internal.fill),
    );
    rows.text(
        ("frame.bar_ink", &theme.frame.bar_ink),
        ("frame.bar_fill", &theme.frame.bar_fill),
    );
    let zone_label = ("ink.zone_label", &theme.ink.zone_label);
    rows.text(zone_label, ("frame.body_fill", &theme.frame.body_fill));
    for (name, fill) in tone_fills(theme) {
        rows.text(zone_label, (&name, fill));
    }
    for (name, tone) in theme.tones.all() {
        if let (Some(fill), Some(label_ink)) = (&tone.fill, &tone.label_ink) {
            rows.text(
                (&format!("tones.{name}.label_ink"), label_ink),
                (&format!("tones.{name}.fill"), fill),
            );
        }
    }
    for (index, tint) in theme.tints.iter().enumerate() {
        let slot = index + 1;
        rows.text(
            (&format!("tints.{slot}.ink"), &tint.ink),
            (&format!("tints.{slot}.fill"), &tint.fill),
        );
    }
    for (name, accent) in callouts(theme) {
        rows.text(
            ("ink.primary", &theme.ink.primary),
            (&format!("callout.{name}.fill"), &accent.fill),
        );
    }
    rows.text(
        ("iso.labels.frame", &theme.iso.labels.frame),
        ("frame.body_fill", &theme.frame.body_fill),
    );
    rows.text(
        ("iso.labels.zone", &theme.iso.labels.zone),
        ("page", &theme.page),
    );
}

fn callouts(theme: &Theme) -> [(&'static str, &super::Accent); 4] {
    [
        ("note", &theme.callout.note),
        ("risk", &theme.callout.risk),
        ("decision", &theme.callout.decision),
        ("open", &theme.callout.open),
    ]
}

fn non_text_rows(theme: &Theme, rows: &mut Rows) {
    let mut wires = vec![
        ("gray.color".to_string(), &theme.gray.color),
        ("deny.color".to_string(), &theme.deny.color),
    ];
    for (index, tint) in theme.tints.iter().enumerate() {
        wires.push((format!("tints.{}.wire", index + 1), &tint.wire));
    }
    let mut grounds = vec![
        ("page".to_string(), &theme.page),
        ("frame.body_fill".to_string(), &theme.frame.body_fill),
    ];
    grounds.extend(tone_fills(theme));
    for (index, tint) in theme.tints.iter().enumerate() {
        grounds.push((format!("tints.{}.fill", index + 1), &tint.fill));
    }
    for (wire_name, wire) in &wires {
        for (ground_name, ground) in &grounds {
            rows.non_text((wire_name, wire), (ground_name, ground));
        }
    }
    rows.non_text(("frame.border", &theme.frame.border), ("page", &theme.page));
    for (name, accent) in callouts(theme) {
        rows.non_text(
            (&format!("callout.{name}.accent"), &accent.accent),
            (&format!("callout.{name}.fill"), &accent.fill),
        );
    }
}

fn separation_rows(theme: &Theme, rows: &mut Rows) {
    let names: Vec<&str> = theme.tints.iter().map(|tint| tint.name.as_str()).collect();
    let wires: Vec<&Color> = theme.tints.iter().map(|tint| &tint.wire).collect();
    let fills: Vec<&Color> = theme.tints.iter().map(|tint| &tint.fill).collect();
    let sets = [
        (
            "wires",
            &wires,
            WIRE_SEPARATION_NORMAL_MIN,
            WIRE_SEPARATION_DICHROMACY_MIN,
        ),
        (
            "fills",
            &fills,
            FILL_SEPARATION_NORMAL_MIN,
            FILL_SEPARATION_DICHROMACY_MIN,
        ),
    ];
    for (set_name, colors, normal_min, dichromacy_min) in sets {
        for vision in Vision::ALL {
            let threshold = if vision == Vision::Normal {
                normal_min
            } else {
                dichromacy_min
            };
            let subject = format!("{set_name} {}", vision.as_str());
            if theme.tint_cue == TintCue::Line {
                rows.rows.push(QualityRow {
                    class: QualityClass::Separation,
                    subject,
                    value: None,
                    threshold,
                    closest: None,
                    not_applicable: Some(TOLD_APART_BY_LINE),
                });
                continue;
            }
            let closest = closest_pair(colors, vision);
            rows.rows.push(QualityRow {
                class: QualityClass::Separation,
                subject,
                value: closest.map(|(value, _, _)| value),
                threshold,
                closest: closest.and_then(|(_, first, second)| {
                    Some((
                        (*names.get(first)?).to_string(),
                        (*names.get(second)?).to_string(),
                    ))
                }),
                not_applicable: closest.is_none().then_some(MALFORMED_COLOR),
            });
        }
    }
}

/// The minimum pairwise delta E under `vision` and the indices of that pair, the first
/// pair in index order on a tie. None when a color is malformed.
fn closest_pair(colors: &[&Color], vision: Vision) -> Option<(f64, usize, usize)> {
    let simulated: Vec<Rgb> = colors
        .iter()
        .map(|color| rgb(color).map(|channels| simulate(channels, vision)))
        .collect::<Option<_>>()?;
    let mut best: Option<(f64, usize, usize)> = None;
    for (first, first_color) in simulated.iter().enumerate() {
        for (second, second_color) in simulated.iter().enumerate().skip(first + 1) {
            let value = delta_e(*first_color, *second_color);
            if best.is_none_or(|(smallest, _, _)| value < smallest) {
                best = Some((value, first, second));
            }
        }
    }
    best
}
