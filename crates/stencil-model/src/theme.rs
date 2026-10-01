//! Themes as data (section 13.4): every paint the renderer reads, by role and tone, never
//! by a grammar's kind. A built-in theme is a JSON file embedded by stencil-render; a theme
//! file is the same document read by the CLI. `#Theme` in `cue/theme.cue` is its CUE twin.

mod overrides;
mod quality;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::document::{Line, line_tint};
use crate::grammar::Tone;
use crate::pointer::NodePointer;

pub use overrides::apply_overrides;
pub use quality::{
    QualityClass, QualityRow, TOLD_APART_BY_LINE, ThemeReport, channel_contrast_ratio,
    channel_lightness, contrast_ratio, lightness, theme_quality,
};

pub const COLOR_PATTERN: &str = r"^#[0-9A-F]{6}$";
pub const THEME_NAME_PATTERN: &str = r"^[a-z][a-z0-9-]{0,31}$";
pub const TINT_NAME_PATTERN: &str = r"^[a-z]{1,12}$";
/// The widest a drawn legend label may run past its canonical label (rule 9): the
/// legend's column gap, so a relabeled entry never reaches the next one.
pub const LEGEND_RELABEL_SLACK_PX: f32 = 16.0;
pub const WIDTH_MIN_PX: f32 = 0.5;
pub const WIDTH_MAX_PX: f32 = 4.0;
pub const FACE_STEP_LIMIT: i8 = 40;
pub const SLAB_THICKNESS_MIN_PX: f32 = 2.0;
pub const SLAB_THICKNESS_MAX_PX: f32 = 16.0;
pub const SHADOW_BLUR_MAX_PX: f32 = 8.0;
pub const SHADOW_DY_MAX_PX: f32 = 8.0;
pub const CHIP_SHADOW_DY_MAX_PX: f32 = 4.0;
pub const TINT_NAME_SCALARS_MAX: usize = 12;

/// An uppercase `#RRGGBB` color.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Color(#[schemars(regex(pattern = COLOR_PATTERN))] pub String);

impl Color {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True when the value matches COLOR_PATTERN.
    pub fn is_well_formed(&self) -> bool {
        let Some(digits) = self.0.strip_prefix('#') else {
            return false;
        };
        digits.len() == 6
            && digits
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
    }

    /// The three channels, or None when malformed.
    pub fn channels(&self) -> Option<[u8; 3]> {
        if !self.is_well_formed() {
            return None;
        }
        let channel = |start: usize| {
            self.0
                .get(start..start + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
        };
        Some([channel(1)?, channel(3)?, channel(5)?])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TintCue {
    /// Tints are told apart by color; legend labels name the color.
    Color,
    /// Tints are told apart by end dots and pattern; legend labels name the line.
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LinePattern {
    Solid,
    Dashed,
    Dotted,
}

impl LinePattern {
    /// The capitalized word a legend label starts with (rule 9).
    pub fn label_word(self) -> &'static str {
        match self {
            LinePattern::Solid => "Solid",
            LinePattern::Dashed => "Dashed",
            LinePattern::Dotted => "Dotted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DotStyle {
    /// A disc in the wire color.
    Filled,
    /// A ring in the wire color around the page color.
    Hollow,
    /// No end dot.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SolidEdges {
    /// A solid container border is not drawn on the slab; the shaded sides carry the edge.
    None,
    /// A solid border is drawn at `edge_width` on the top face and the sides.
    Outline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ThemeStroke {
    pub color: Color,
    #[schemars(range(min = 0.5, max = 4.0))]
    pub width: f32,
    pub pattern: LinePattern,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Swatch {
    pub fill: Color,
    pub ink: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Accent {
    pub accent: Color,
    pub fill: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToneRole {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ink: Option<Color>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tint {
    #[schemars(regex(pattern = TINT_NAME_PATTERN))]
    pub name: String,
    pub fill: Color,
    pub border: Color,
    pub ink: Color,
    pub wire: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inks {
    /// Title, item title, block body, list bullets.
    pub primary: Color,
    /// Lede, item subtitle.
    pub secondary: Color,
    /// An untinted container's label, a Frame block's label.
    pub zone_label: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BadgeRole {
    pub customer: Swatch,
    pub internal: Swatch,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<ThemeStroke>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CardRole {
    pub fill: Color,
    pub border: ThemeStroke,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TagRole {
    pub fill: Color,
    pub border: ThemeStroke,
    pub ink: Color,
    pub sub_ink: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegendRole {
    pub label_ink: Color,
    /// Legend text and the Note kind legend.
    pub text_ink: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalloutRole {
    pub note: Accent,
    pub risk: Accent,
    pub decision: Accent,
    pub open: Accent,
}

/// The Frame block of section 11.3, a wireframe placeholder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceholderRole {
    pub border: Color,
    pub diagonal: Color,
}

/// The frame role: the outermost container, the gcp frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrameRole {
    pub border: Color,
    pub frame_fill: Color,
    pub bar_fill: Color,
    pub bar_ink: Color,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_rule: Option<ThemeStroke>,
    pub body_fill: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tones {
    pub neutral: ToneRole,
    pub warm: ToneRole,
    pub cool: ToneRole,
    pub soft: ToneRole,
    /// Never filled (rule 3): a container of this tone is a ring under iso.
    pub strong: ToneRole,
    pub highlight: ToneRole,
    pub emphasis: ToneRole,
    pub accent: ToneRole,
}

impl Tones {
    pub fn get(&self, tone: Tone) -> &ToneRole {
        match tone {
            Tone::Neutral => &self.neutral,
            Tone::Warm => &self.warm,
            Tone::Cool => &self.cool,
            Tone::Soft => &self.soft,
            Tone::Strong => &self.strong,
            Tone::Highlight => &self.highlight,
            Tone::Emphasis => &self.emphasis,
            Tone::Accent => &self.accent,
        }
    }

    /// Every tone with its field name, in declaration order.
    pub fn all(&self) -> [(&'static str, &ToneRole); 8] {
        [
            ("neutral", &self.neutral),
            ("warm", &self.warm),
            ("cool", &self.cool),
            ("soft", &self.soft),
            ("strong", &self.strong),
            ("highlight", &self.highlight),
            ("emphasis", &self.emphasis),
            ("accent", &self.accent),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContainerRole {
    /// Every container border but a frame's is drawn at this width; absent draws the
    /// grammar's width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.5, max = 4.0))]
    pub draw_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.5, max = 4.0))]
    pub frame_draw_width: Option<f32>,
    /// A border for containers whose grammar pattern is none; absent draws none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub borderless_outline: Option<ThemeStroke>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LanesRole {
    pub lifeline: ThemeStroke,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrayRole {
    pub color: Color,
    #[schemars(range(min = 0.5, max = 4.0))]
    pub width: f32,
    pub pattern: LinePattern,
    pub dot: DotStyle,
}

/// Solid and dash lines take their color from the tint slot's wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SolidRole {
    #[schemars(range(min = 0.5, max = 4.0))]
    pub width: f32,
    pub dots: [DotStyle; 8],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DashRole {
    #[schemars(range(min = 0.5, max = 4.0))]
    pub width: f32,
    pub pattern: LinePattern,
    pub dot: DotStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DenyRole {
    pub color: Color,
    #[schemars(range(min = 0.5, max = 4.0))]
    pub width: f32,
    pub pattern: LinePattern,
    pub dot: DotStyle,
    pub tag_border: Color,
    pub tag_ink: Color,
}

/// HSL lightness steps of the isometric faces, in percentage points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FaceSteps {
    #[schemars(range(min = -40, max = 40))]
    pub top: i8,
    #[schemars(range(min = -40, max = 40))]
    pub left: i8,
    #[schemars(range(min = -40, max = 40))]
    pub right: i8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrameSides {
    pub left: Color,
    pub right: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabelInks {
    /// The frame's name on its floor.
    pub frame: Color,
    /// The name of a Box that has no tint; a tinted Box takes its tint's ink.
    pub zone: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChipShadow {
    pub color: Color,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f32,
    #[schemars(range(min = 0.0, max = 4.0))]
    pub dy: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChipRole {
    pub fill: Color,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ring: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ChipShadow>,
}

/// A soft shadow under every opaque block (section 13.11).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BlockShadow {
    pub color: Color,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f32,
    #[schemars(range(min = 0.0, max = 8.0))]
    pub blur: f32,
    #[schemars(range(min = 0.0, max = 8.0))]
    pub dy: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IsoWidths {
    /// Solid slot 1.
    #[schemars(range(min = 0.5, max = 4.0))]
    pub primary: f32,
    /// Solid slots 2 to 8, dash, deny.
    #[schemars(range(min = 0.5, max = 4.0))]
    pub secondary: f32,
    #[schemars(range(min = 0.5, max = 4.0))]
    pub gray: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IsoRole {
    pub faces: FaceSteps,
    #[schemars(range(min = 2.0, max = 16.0))]
    pub slab_thickness: f32,
    /// The frame slab's side faces; absent shades the frame body fill like any slab.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_sides: Option<FrameSides>,
    #[schemars(range(min = 0.5, max = 4.0))]
    pub frame_outline: f32,
    pub solid_edges: SolidEdges,
    #[schemars(range(min = 0.5, max = 4.0))]
    pub edge_width: f32,
    /// A ring (a container with no drawn fill); absent draws its flat border.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ring: Option<ThemeStroke>,
    /// The outline of a block whose flat drawing has no border; absent draws none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_outline: Option<ThemeStroke>,
    /// The ink of a Box name lying on its slab (section 12.4).
    pub labels: LabelInks,
    pub chip: ChipRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<BlockShadow>,
    pub widths: IsoWidths,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[schemars(regex(pattern = THEME_NAME_PATTERN))]
    pub name: String,
    pub tint_cue: TintCue,
    pub page: Color,
    pub ink: Inks,
    pub kicker: Color,
    pub badge: BadgeRole,
    pub card: CardRole,
    pub fact: Swatch,
    pub ask: Swatch,
    pub tag: TagRole,
    pub legend: LegendRole,
    pub foot: Color,
    pub callout: CalloutRole,
    pub placeholder: PlaceholderRole,
    /// The flat chip under every icon; absent draws none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_chip: Option<Color>,
    pub frame: FrameRole,
    pub tones: Tones,
    pub containers: ContainerRole,
    pub lanes: LanesRole,
    pub gray: GrayRole,
    pub solid: SolidRole,
    pub dash: DashRole,
    pub deny: DenyRole,
    pub tints: [Tint; 8],
    pub iso: IsoRole,
}

impl Theme {
    /// The tint of slot `slot` (1 to 8); slot 1 for any other value.
    pub fn tint(&self, slot: u8) -> &Tint {
        let index = usize::from(slot.saturating_sub(1));
        self.tints.get(index).unwrap_or(&self.tints[0])
    }
}

/// The legend label the renderer draws for a line (rule 9). Under `tint_cue: color` it
/// names the pattern and the color, under `line` it names the line.
pub fn drawn_legend_label(theme: &Theme, line: Line, tint: Option<u8>) -> String {
    let slot = line_tint(line, tint).unwrap_or(1);
    match theme.tint_cue {
        TintCue::Color => match line {
            Line::Gray => format!("{} gray", theme.gray.pattern.label_word()),
            Line::Deny => format!("{} red", theme.deny.pattern.label_word()),
            Line::Solid => format!("Solid {}", theme.tint(slot).name),
            Line::Dash => format!(
                "{} {}",
                theme.dash.pattern.label_word(),
                theme.tint(slot).name
            ),
        },
        TintCue::Line => match line {
            Line::Gray => "Thin line".to_string(),
            Line::Solid => match solid_dot(theme, slot) {
                DotStyle::Filled => "Solid line".to_string(),
                DotStyle::Hollow => "Ringed line".to_string(),
                DotStyle::None => "Plain line".to_string(),
            },
            Line::Dash => "Dashed line".to_string(),
            Line::Deny => format!("{} line", theme.deny.pattern.label_word()),
        },
    }
}

/// The end dot of a solid line in slot `slot`.
pub fn solid_dot(theme: &Theme, slot: u8) -> DotStyle {
    let index = usize::from(slot.saturating_sub(1));
    theme
        .solid
        .dots
        .get(index)
        .copied()
        .unwrap_or(theme.solid.dots[0])
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeViolation {
    /// RFC 6901 pointer into the theme document, or into `/theme_overrides` of the page.
    pub pointer: NodePointer,
    pub rule: ThemeRule,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeRule {
    ColorMalformed,
    WidthOutOfRange,
    StepOutOfRange,
    OpacityOutOfRange,
    SlabThicknessOutOfRange,
    ShadowOutOfRange,
    NameMalformed,
    TintNameMalformed,
    TintNameDuplicate,
    StrongToneFilled,
    OverrideNotObject,
    OverrideUnknownRole,
    LegendLabelTooWide,
}

impl ThemeRule {
    /// Kebab-case name with a `theme-` prefix, as printed on the CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeRule::ColorMalformed => "theme-color-malformed",
            ThemeRule::WidthOutOfRange => "theme-width-out-of-range",
            ThemeRule::StepOutOfRange => "theme-step-out-of-range",
            ThemeRule::OpacityOutOfRange => "theme-opacity-out-of-range",
            ThemeRule::SlabThicknessOutOfRange => "theme-slab-thickness-out-of-range",
            ThemeRule::ShadowOutOfRange => "theme-shadow-out-of-range",
            ThemeRule::NameMalformed => "theme-name-malformed",
            ThemeRule::TintNameMalformed => "theme-tint-name-malformed",
            ThemeRule::TintNameDuplicate => "theme-tint-name-duplicate",
            ThemeRule::StrongToneFilled => "theme-strong-tone-filled",
            ThemeRule::OverrideNotObject => "theme-override-not-object",
            ThemeRule::OverrideUnknownRole => "theme-override-unknown-role",
            ThemeRule::LegendLabelTooWide => "theme-legend-label-too-wide",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("theme {origin} is not valid theme JSON at line {line}, column {column}: {message}")]
    Json {
        origin: String,
        line: usize,
        column: usize,
        message: String,
    },
    #[error("theme {origin} violates {} rule(s)", .violations.len())]
    Invalid {
        origin: String,
        violations: Vec<ThemeViolation>,
    },
}

impl ThemeError {
    /// The violations of an `Invalid` error, empty for a JSON error.
    pub fn violations(&self) -> &[ThemeViolation] {
        match self {
            ThemeError::Json { .. } => &[],
            ThemeError::Invalid { violations, .. } => violations,
        }
    }
}

/// serde_json parse followed by validate_theme. `origin` names the file or built-in.
pub fn parse_theme(json_text: &str, origin: &str) -> Result<Theme, ThemeError> {
    let theme: Theme = serde_json::from_str(json_text).map_err(|error| ThemeError::Json {
        origin: origin.to_string(),
        line: error.line(),
        column: error.column(),
        message: strip_location(&error),
    })?;
    checked(theme, origin)
}

/// `theme` when it passes validate_theme, else an Invalid error naming `origin`.
fn checked(theme: Theme, origin: &str) -> Result<Theme, ThemeError> {
    let violations = validate_theme(&theme);
    if violations.is_empty() {
        Ok(theme)
    } else {
        Err(ThemeError::Invalid {
            origin: origin.to_string(),
            violations,
        })
    }
}

fn strip_location(error: &serde_json::Error) -> String {
    let full_message = error.to_string();
    let suffix = format!(" at line {} column {}", error.line(), error.column());
    full_message
        .strip_suffix(&suffix)
        .unwrap_or(&full_message)
        .to_string()
}

pub fn theme_schema() -> schemars::Schema {
    schemars::schema_for!(Theme)
}

/// True when `name` matches THEME_NAME_PATTERN.
pub fn is_valid_theme_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && name.len() <= 32
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

/// True when `name` matches TINT_NAME_PATTERN.
pub fn is_valid_tint_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= TINT_NAME_SCALARS_MAX
        && name.bytes().all(|byte| byte.is_ascii_lowercase())
}

/// Collects violations in the order the fields are visited.
struct Checker {
    violations: Vec<ThemeViolation>,
}

impl Checker {
    fn push(&mut self, pointer: NodePointer, rule: ThemeRule, message: String) {
        self.violations.push(ThemeViolation {
            pointer,
            rule,
            message,
        });
    }

    fn color(&mut self, pointer: NodePointer, color: &Color) {
        if !color.is_well_formed() {
            let message = format!("color \"{}\" is not an uppercase #RRGGBB", color.as_str());
            self.push(pointer, ThemeRule::ColorMalformed, message);
        }
    }

    fn optional_color(&mut self, pointer: NodePointer, color: Option<&Color>) {
        if let Some(color) = color {
            self.color(pointer, color);
        }
    }

    fn range(&mut self, pointer: NodePointer, value: f32, bounds: (f32, f32), rule: ThemeRule) {
        let (low, high) = bounds;
        if !(value.is_finite() && (low..=high).contains(&value)) {
            let message = format!("{value} is outside {low} to {high}");
            self.push(pointer, rule, message);
        }
    }

    fn width(&mut self, pointer: NodePointer, width: f32) {
        self.range(
            pointer,
            width,
            (WIDTH_MIN_PX, WIDTH_MAX_PX),
            ThemeRule::WidthOutOfRange,
        );
    }

    fn optional_width(&mut self, pointer: NodePointer, width: Option<f32>) {
        if let Some(width) = width {
            self.width(pointer, width);
        }
    }

    fn opacity(&mut self, pointer: NodePointer, opacity: f32) {
        self.range(pointer, opacity, (0.0, 1.0), ThemeRule::OpacityOutOfRange);
    }

    fn step(&mut self, pointer: NodePointer, step: i8) {
        if !(-FACE_STEP_LIMIT..=FACE_STEP_LIMIT).contains(&step) {
            let message = format!("step {step} is outside -{FACE_STEP_LIMIT} to {FACE_STEP_LIMIT}");
            self.push(pointer, ThemeRule::StepOutOfRange, message);
        }
    }

    fn stroke(&mut self, pointer: &NodePointer, stroke: &ThemeStroke) {
        self.color(pointer.child("color"), &stroke.color);
        self.width(pointer.child("width"), stroke.width);
    }

    fn optional_stroke(&mut self, pointer: &NodePointer, stroke: Option<&ThemeStroke>) {
        if let Some(stroke) = stroke {
            self.stroke(pointer, stroke);
        }
    }

    fn swatch(&mut self, pointer: &NodePointer, swatch: &Swatch) {
        self.color(pointer.child("fill"), &swatch.fill);
        self.color(pointer.child("ink"), &swatch.ink);
    }

    fn accent(&mut self, pointer: &NodePointer, accent: &Accent) {
        self.color(pointer.child("accent"), &accent.accent);
        self.color(pointer.child("fill"), &accent.fill);
    }

    fn tone(&mut self, pointer: &NodePointer, tone: &ToneRole) {
        self.optional_color(pointer.child("fill"), tone.fill.as_ref());
        self.optional_color(pointer.child("border"), tone.border.as_ref());
        self.optional_color(pointer.child("label_ink"), tone.label_ink.as_ref());
    }
}

/// Structural rules serde does not express (rule 3), every violation in field order. Empty
/// means loadable.
pub fn validate_theme(theme: &Theme) -> Vec<ThemeViolation> {
    let mut checker = Checker {
        violations: Vec::new(),
    };
    let root = NodePointer::root();
    if !is_valid_theme_name(&theme.name) {
        checker.push(
            root.child("name"),
            ThemeRule::NameMalformed,
            format!(
                "name \"{}\" does not match {THEME_NAME_PATTERN}",
                theme.name
            ),
        );
    }
    checker.color(root.child("page"), &theme.page);
    let ink = root.child("ink");
    checker.color(ink.child("primary"), &theme.ink.primary);
    checker.color(ink.child("secondary"), &theme.ink.secondary);
    checker.color(ink.child("zone_label"), &theme.ink.zone_label);
    checker.color(root.child("kicker"), &theme.kicker);
    let badge = root.child("badge");
    checker.swatch(&badge.child("customer"), &theme.badge.customer);
    checker.swatch(&badge.child("internal"), &theme.badge.internal);
    checker.optional_stroke(&badge.child("border"), theme.badge.border.as_ref());
    let card = root.child("card");
    checker.color(card.child("fill"), &theme.card.fill);
    checker.stroke(&card.child("border"), &theme.card.border);
    checker.swatch(&root.child("fact"), &theme.fact);
    checker.swatch(&root.child("ask"), &theme.ask);
    let tag = root.child("tag");
    checker.color(tag.child("fill"), &theme.tag.fill);
    checker.stroke(&tag.child("border"), &theme.tag.border);
    checker.color(tag.child("ink"), &theme.tag.ink);
    checker.color(tag.child("sub_ink"), &theme.tag.sub_ink);
    let legend = root.child("legend");
    checker.color(legend.child("label_ink"), &theme.legend.label_ink);
    checker.color(legend.child("text_ink"), &theme.legend.text_ink);
    checker.color(root.child("foot"), &theme.foot);
    let callout = root.child("callout");
    checker.accent(&callout.child("note"), &theme.callout.note);
    checker.accent(&callout.child("risk"), &theme.callout.risk);
    checker.accent(&callout.child("decision"), &theme.callout.decision);
    checker.accent(&callout.child("open"), &theme.callout.open);
    let placeholder = root.child("placeholder");
    checker.color(placeholder.child("border"), &theme.placeholder.border);
    checker.color(placeholder.child("diagonal"), &theme.placeholder.diagonal);
    checker.optional_color(root.child("icon_chip"), theme.icon_chip.as_ref());
    let frame = root.child("frame");
    checker.color(frame.child("border"), &theme.frame.border);
    checker.color(frame.child("frame_fill"), &theme.frame.frame_fill);
    checker.color(frame.child("bar_fill"), &theme.frame.bar_fill);
    checker.color(frame.child("bar_ink"), &theme.frame.bar_ink);
    checker.optional_stroke(&frame.child("bar_rule"), theme.frame.bar_rule.as_ref());
    checker.color(frame.child("body_fill"), &theme.frame.body_fill);
    let tones = root.child("tones");
    for (name, tone) in theme.tones.all() {
        let pointer = tones.child(name);
        checker.tone(&pointer, tone);
        if name == "strong" && tone.fill.is_some() {
            checker.push(
                pointer.child("fill"),
                ThemeRule::StrongToneFilled,
                "the strong tone never fills: its containers are rings under iso".to_string(),
            );
        }
    }
    let containers = root.child("containers");
    checker.optional_width(containers.child("draw_width"), theme.containers.draw_width);
    checker.optional_width(
        containers.child("frame_draw_width"),
        theme.containers.frame_draw_width,
    );
    checker.optional_stroke(
        &containers.child("borderless_outline"),
        theme.containers.borderless_outline.as_ref(),
    );
    checker.stroke(
        &root.child("lanes").child("lifeline"),
        &theme.lanes.lifeline,
    );
    let gray = root.child("gray");
    checker.color(gray.child("color"), &theme.gray.color);
    checker.width(gray.child("width"), theme.gray.width);
    checker.width(root.child("solid").child("width"), theme.solid.width);
    checker.width(root.child("dash").child("width"), theme.dash.width);
    let deny = root.child("deny");
    checker.color(deny.child("color"), &theme.deny.color);
    checker.width(deny.child("width"), theme.deny.width);
    checker.color(deny.child("tag_border"), &theme.deny.tag_border);
    checker.color(deny.child("tag_ink"), &theme.deny.tag_ink);
    check_tints(&mut checker, &root.child("tints"), &theme.tints);
    check_iso(&mut checker, &root.child("iso"), &theme.iso);
    checker.violations
}

fn check_tints(checker: &mut Checker, pointer: &NodePointer, tints: &[Tint; 8]) {
    for (index, tint) in tints.iter().enumerate() {
        let slot = pointer.index(index);
        if !is_valid_tint_name(&tint.name) {
            checker.push(
                slot.child("name"),
                ThemeRule::TintNameMalformed,
                format!(
                    "tint name \"{}\" does not match {TINT_NAME_PATTERN}",
                    tint.name
                ),
            );
        } else if let Some(first) = tints
            .iter()
            .take(index)
            .position(|earlier| earlier.name == tint.name)
        {
            checker.push(
                slot.child("name"),
                ThemeRule::TintNameDuplicate,
                format!("tint name \"{}\" is already slot {}", tint.name, first + 1),
            );
        }
        checker.color(slot.child("fill"), &tint.fill);
        checker.color(slot.child("border"), &tint.border);
        checker.color(slot.child("ink"), &tint.ink);
        checker.color(slot.child("wire"), &tint.wire);
    }
}

fn check_iso(checker: &mut Checker, pointer: &NodePointer, iso: &IsoRole) {
    let faces = pointer.child("faces");
    checker.step(faces.child("top"), iso.faces.top);
    checker.step(faces.child("left"), iso.faces.left);
    checker.step(faces.child("right"), iso.faces.right);
    checker.range(
        pointer.child("slab_thickness"),
        iso.slab_thickness,
        (SLAB_THICKNESS_MIN_PX, SLAB_THICKNESS_MAX_PX),
        ThemeRule::SlabThicknessOutOfRange,
    );
    if let Some(sides) = &iso.frame_sides {
        let sides_pointer = pointer.child("frame_sides");
        checker.color(sides_pointer.child("left"), &sides.left);
        checker.color(sides_pointer.child("right"), &sides.right);
    }
    checker.width(pointer.child("frame_outline"), iso.frame_outline);
    checker.width(pointer.child("edge_width"), iso.edge_width);
    checker.optional_stroke(&pointer.child("ring"), iso.ring.as_ref());
    checker.optional_stroke(&pointer.child("block_outline"), iso.block_outline.as_ref());
    let labels = pointer.child("labels");
    checker.color(labels.child("frame"), &iso.labels.frame);
    checker.color(labels.child("zone"), &iso.labels.zone);
    let chip = pointer.child("chip");
    checker.color(chip.child("fill"), &iso.chip.fill);
    checker.optional_color(chip.child("ring"), iso.chip.ring.as_ref());
    if let Some(shadow) = &iso.chip.shadow {
        let shadow_pointer = chip.child("shadow");
        checker.color(shadow_pointer.child("color"), &shadow.color);
        checker.opacity(shadow_pointer.child("opacity"), shadow.opacity);
        checker.range(
            shadow_pointer.child("dy"),
            shadow.dy,
            (0.0, CHIP_SHADOW_DY_MAX_PX),
            ThemeRule::ShadowOutOfRange,
        );
    }
    if let Some(shadow) = &iso.shadow {
        let shadow_pointer = pointer.child("shadow");
        checker.color(shadow_pointer.child("color"), &shadow.color);
        checker.opacity(shadow_pointer.child("opacity"), shadow.opacity);
        if !(shadow.blur.is_finite() && shadow.blur > 0.0 && shadow.blur <= SHADOW_BLUR_MAX_PX) {
            checker.push(
                shadow_pointer.child("blur"),
                ThemeRule::ShadowOutOfRange,
                format!(
                    "blur {} is not above 0 and at most {SHADOW_BLUR_MAX_PX}",
                    shadow.blur
                ),
            );
        }
        checker.range(
            shadow_pointer.child("dy"),
            shadow.dy,
            (0.0, SHADOW_DY_MAX_PX),
            ThemeRule::ShadowOutOfRange,
        );
    }
    let widths = pointer.child("widths");
    checker.width(widths.child("primary"), iso.widths.primary);
    checker.width(widths.child("secondary"), iso.widths.secondary);
    checker.width(widths.child("gray"), iso.widths.gray);
}
