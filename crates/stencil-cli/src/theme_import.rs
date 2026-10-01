//! `stencil theme import --base16` (section 13.9): a base16 or base24 color scheme mapped
//! onto a theme by fixed rules, written with the quality report `stencil theme check`
//! prints for it.
//!
//! The scheme is read line by line in a restricted format rather than as YAML, so the CLI
//! takes no YAML dependency: a line `base0D: "#2AC3DE"` sets one color, with or without
//! quotes and `#`, under `palette:` or at the top level.

use std::error::Error;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use stencil_model::theme::{
    Accent, BadgeRole, BlockShadow, CalloutRole, CardRole, ChipRole, Color, ContainerRole,
    DashRole, DenyRole, DotStyle, FaceSteps, FrameRole, FrameSides, GrayRole, Inks, IsoRole,
    IsoWidths, LabelInks, LanesRole, LegendRole, LinePattern, PlaceholderRole, SolidEdges,
    SolidRole, Swatch, THEME_NAME_PATTERN, TagRole, ThemeStroke, Tint, TintCue, ToneRole, Tones,
    channel_contrast_ratio, channel_lightness, is_valid_theme_name,
};
use stencil_model::{Theme, theme_quality};
use stencil_render::palette::shade;

use crate::ExitCode;
use crate::theme_command::{quality_exit_code, quality_report_lines};

/// The largest scheme file read, in bytes; a larger one is a base16 error.
pub const SCHEME_BYTES_MAX: usize = 65_536;
/// The keys every scheme sets, `base00` to `base0F`.
const BASE16_KEY_COUNT: usize = 16;
/// The base24 keys `base10` to `base17` are read and ignored.
const BASE24_KEY_LAST: u8 = 0x17;
/// Tint slot names, fixed by slot (section 13.9).
pub const SLOT_NAMES: [&str; 8] = [
    "blue", "pink", "cyan", "yellow", "red", "green", "orange", "brown",
];
/// The base16 key of each tint slot's accent.
pub const SLOT_KEYS: [usize; 8] = [0x0D, 0x0E, 0x0C, 0x0A, 0x08, 0x0B, 0x09, 0x0F];
/// L* of `base00` below which a scheme is dark.
const DARK_LIGHTNESS_MAX: f64 = 50.0;
/// The tint fill mix of a dark and a light scheme (the callout `r` of section 13.5).
const DARK_FILL_MIX: f32 = 0.18;
const LIGHT_FILL_MIX: f32 = 0.22;
/// The tint border: the accent at half strength over the page.
const BORDER_MIX: f32 = 0.5;
/// Bounds of the iso face step, the L* difference between `base00` and `base02`.
const FACE_STEP_MIN: f64 = 4.0;
const FACE_STEP_MAX: f64 = 8.0;
/// The L* difference between `base00` and `base01` below which a slab is drawn thicker.
const SLAB_CONTRAST_MIN: f64 = 4.0;
const SLAB_THICKNESS_PX: f32 = 6.0;
const SLAB_THICKNESS_LOW_CONTRAST_PX: f32 = 8.0;
/// The largest L* step from the page to the card surface. base16 leaves `base02`, the
/// selection background, anywhere from a few to 30 L* from the page; solarized puts it at
/// mid gray, where a card would outweigh every container and hide its secondary ink.
const SURFACE_LIFT_MAX: f64 = 14.0;
/// The HSL step from the frame border to the frame slab's right side (section 13.5).
const FRAME_RIGHT_SIDE_STEP: i8 = -12;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("cannot read scheme {}", path.display())]
    ReadScheme {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("base16 {}: {message}", path.display())]
    Base16 { path: PathBuf, message: String },
    #[error("cannot shade the frame border {color} of scheme {}", path.display())]
    Shade { path: PathBuf, color: String },
    #[error("cannot serialize the imported theme")]
    Serialize(#[source] serde_json::Error),
    #[error("cannot write {}", path.display())]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Section 13.12: a scheme that does not read is a defect of the input (1); an unreadable
/// scheme, an internal fault and a failed write could not run (2).
pub fn import_exit_code(error: &ImportError) -> ExitCode {
    match error {
        ImportError::Base16 { .. } => ExitCode::Defects,
        ImportError::ReadScheme { .. }
        | ImportError::Shade { .. }
        | ImportError::Serialize(_)
        | ImportError::WriteOutput { .. } => ExitCode::CouldNotRun,
    }
}

/// An sRGB color as three channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rgb([u8; 3]);

impl Rgb {
    fn color(self) -> Color {
        let [red, green, blue] = self.0;
        Color(format!("#{red:02X}{green:02X}{blue:02X}"))
    }

    fn lightness(self) -> f64 {
        channel_lightness(self.0)
    }

    fn contrast(self, ground: Rgb) -> f64 {
        channel_contrast_ratio(self.0, ground.0)
    }

    /// `self` moved `fraction` of the way to `toward` in each channel: the `mix` of
    /// section 12.6, with its rounding.
    fn mix(self, toward: Rgb, fraction: f32) -> Rgb {
        let fraction = fraction.clamp(0.0, 1.0);
        let mut channels = [0_u8; 3];
        for ((channel, start), end) in channels.iter_mut().zip(self.0).zip(toward.0) {
            let start = f32::from(start);
            let end = f32::from(end);
            // The clamp keeps the cast in range, so `as` never saturates.
            *channel = (start + (end - start) * fraction + 0.5)
                .floor()
                .clamp(0.0, 255.0) as u8;
        }
        Rgb(channels)
    }
}

/// The card surface: `surface` when it lies within SURFACE_LIFT_MAX L* of `page`, else
/// `page` mixed toward it by SURFACE_LIFT_MAX over that difference.
fn lifted_surface(page: Rgb, surface: Rgb) -> Rgb {
    let step = (page.lightness() - surface.lightness()).abs();
    if step <= SURFACE_LIFT_MAX {
        surface
    } else {
        page.mix(surface, (SURFACE_LIFT_MAX / step) as f32)
    }
}

/// Of `first` and `second`, the one with the higher contrast on `ground`; `first` on a tie.
fn higher_contrast(first: Rgb, second: Rgb, ground: Rgb) -> Rgb {
    if second.contrast(ground) > first.contrast(ground) {
        second
    } else {
        first
    }
}

/// The sixteen colors of a scheme, `base00` to `base0F`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base16Colors {
    colors: [Rgb; BASE16_KEY_COUNT],
}

/// One line of a scheme that sets a color.
struct KeyLine {
    key: u8,
    channels: [u8; 3],
}

/// `0` to `F` and `a` to `f` as their value.
fn hex_digit(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// The line when it matches
/// `^\s*(base[0-9A-Fa-f]{2})\s*:\s*"?#?([0-9A-Fa-f]{6})"?\s*(#.*)?$`, else None.
fn key_line(line: &str) -> Option<KeyLine> {
    let rest = line.trim_start().strip_prefix("base")?;
    let bytes = rest.as_bytes();
    let key = hex_digit(*bytes.first()?)? * 16 + hex_digit(*bytes.get(1)?)?;
    let rest = rest.get(2..)?.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"').unwrap_or(rest);
    let rest = rest.strip_prefix('#').unwrap_or(rest);
    let digits = rest.as_bytes();
    let mut channels = [0_u8; 3];
    for (index, channel) in channels.iter_mut().enumerate() {
        let high = hex_digit(*digits.get(index * 2)?)?;
        let low = hex_digit(*digits.get(index * 2 + 1)?)?;
        *channel = high * 16 + low;
    }
    let rest = rest.get(6..)?;
    let rest = rest.strip_prefix('"').unwrap_or(rest).trim_start();
    if rest.is_empty() || rest.starts_with('#') {
        Some(KeyLine { key, channels })
    } else {
        None
    }
}

/// The value of a `system:` line with its quotes and trailing comment removed, or None
/// when the line is not one.
fn system_line(line: &str) -> Option<&str> {
    let value = line
        .trim_start()
        .strip_prefix("system")?
        .trim_start()
        .strip_prefix(':')?;
    let value = match value.find(" #") {
        Some(comment) => value.get(..comment)?,
        None => value,
    };
    let value = value.trim();
    let unquoted = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        });
    Some(unquoted.unwrap_or(value))
}

/// `base0D`: the key as written in the spec, its digit uppercase.
fn key_name(key: usize) -> String {
    format!("base{key:02X}")
}

/// Reads the sixteen colors of a scheme. A missing or repeated key from `base00` to
/// `base0F`, or a `system:` other than base16 or base24, is an error naming it; `base10`
/// to `base17` are ignored, as are lines that set no color.
pub fn read_base16(scheme_text: &str) -> Result<Base16Colors, String> {
    let mut colors: [Option<(Rgb, usize)>; BASE16_KEY_COUNT] = [None; BASE16_KEY_COUNT];
    // The text is at most SCHEME_BYTES_MAX bytes, which bounds the lines.
    for (index, line) in scheme_text.lines().enumerate() {
        let line_number = index + 1;
        if let Some(system) = system_line(line) {
            if system != "base16" && system != "base24" {
                return Err(format!(
                    "line {line_number}: system {system:?} is neither base16 nor base24"
                ));
            }
            continue;
        }
        let Some(KeyLine { key, channels }) = key_line(line) else {
            continue;
        };
        if key > BASE24_KEY_LAST {
            continue;
        }
        let Some(slot) = colors.get_mut(usize::from(key)) else {
            continue;
        };
        if let Some((_, first_line)) = slot {
            return Err(format!(
                "{} is given twice, on lines {first_line} and {line_number}",
                key_name(usize::from(key))
            ));
        }
        *slot = Some((Rgb(channels), line_number));
    }
    let missing: Vec<String> = colors
        .iter()
        .enumerate()
        .filter(|(_, color)| color.is_none())
        .map(|(key, _)| key_name(key))
        .collect();
    if !missing.is_empty() {
        return Err(format!("missing {}", missing.join(", ")));
    }
    let mut read = [Rgb([0, 0, 0]); BASE16_KEY_COUNT];
    for (target, source) in read.iter_mut().zip(colors) {
        if let Some((color, _)) = source {
            *target = color;
        }
    }
    Ok(Base16Colors { colors: read })
}

/// The theme name for a scheme file without `--name`: the file stem lowercased, with every
/// character outside `a-z0-9-` replaced by `-`.
pub fn name_from_stem(scheme_path: &Path) -> String {
    let stem = scheme_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    stem.chars()
        .map(|character| {
            if character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn solid_stroke(color: Color, width: f32) -> ThemeStroke {
    ThemeStroke {
        color,
        width,
        pattern: LinePattern::Solid,
    }
}

fn tone(fill: Option<Rgb>, border: Option<Rgb>) -> ToneRole {
    ToneRole {
        fill: fill.map(Rgb::color),
        border: border.map(Rgb::color),
        label_ink: None,
    }
}

/// A tint slot in channels, before it is written as a `Tint`.
#[derive(Clone, Copy)]
struct Slot {
    wire: Rgb,
    fill: Rgb,
    border: Rgb,
    ink: Rgb,
}

/// Maps a scheme onto a theme by the table of section 13.9. Every rule is a fixed function
/// of the sixteen colors, so the same scheme always gives the same theme.
pub fn base16_theme(
    scheme: &Base16Colors,
    name: &str,
    scheme_path: &Path,
) -> Result<Theme, ImportError> {
    let base = scheme.colors;
    let page = base[0x00];
    let dark = page.lightness() < DARK_LIGHTNESS_MAX;
    let fill_mix = if dark { DARK_FILL_MIX } else { LIGHT_FILL_MIX };
    let white = Rgb([0xFF, 0xFF, 0xFF]);

    let primary = higher_contrast(base[0x06], base[0x07], page);
    let secondary = base[0x05];
    let card_fill = lifted_surface(page, base[0x02]);
    let tone_fill = base[0x01].mix(card_fill, 0.5);
    let rule = base[0x03];
    let blue = base[0x0D];

    let slots = SLOT_KEYS.map(|key| {
        let wire = base.get(key).copied().unwrap_or(blue);
        let fill = page.mix(wire, fill_mix);
        Slot {
            wire,
            fill,
            border: page.mix(wire, BORDER_MIX),
            ink: higher_contrast(primary, page, fill),
        }
    });
    let [slot_1, slot_2, _, slot_4, _, slot_6, _, _] = slots;
    let swatch = |slot: Slot| Swatch {
        fill: slot.fill.color(),
        ink: slot.ink.color(),
    };
    let wire_on_fill = |slot: Slot| Accent {
        accent: slot.wire.color(),
        fill: slot.fill.color(),
    };
    let tints = std::array::from_fn(|index| {
        let slot = slots.get(index).copied().unwrap_or(slot_1);
        Tint {
            name: SLOT_NAMES.get(index).unwrap_or(&"blue").to_string(),
            fill: slot.fill.color(),
            border: slot.border.color(),
            ink: slot.ink.color(),
            wire: slot.wire.color(),
        }
    });

    let accent_fill = page.mix(base[0x09], fill_mix);
    let red = base[0x08];
    let bar_ink = higher_contrast(page, primary, blue);
    let frame_right =
        shade(blue.color().as_str(), FRAME_RIGHT_SIDE_STEP).ok_or_else(|| ImportError::Shade {
            path: scheme_path.to_path_buf(),
            color: blue.color().0,
        })?;
    let face_step = (page.lightness() - base[0x02].lightness())
        .abs()
        .round()
        .clamp(FACE_STEP_MIN, FACE_STEP_MAX) as i8;
    let slab_thickness = if (page.lightness() - base[0x01].lightness()).abs() >= SLAB_CONTRAST_MIN {
        SLAB_THICKNESS_PX
    } else {
        SLAB_THICKNESS_LOW_CONTRAST_PX
    };
    let shadow = if dark {
        BlockShadow {
            color: Rgb([0, 0, 0]).color(),
            opacity: 0.5,
            blur: 3.0,
            dy: 3.0,
        }
    } else {
        BlockShadow {
            color: primary.color(),
            opacity: 0.18,
            blur: 2.5,
            dy: 3.0,
        }
    };

    Ok(Theme {
        name: name.to_string(),
        tint_cue: TintCue::Color,
        page: page.color(),
        ink: Inks {
            primary: primary.color(),
            secondary: secondary.color(),
            zone_label: secondary.color(),
        },
        kicker: blue.color(),
        badge: BadgeRole {
            customer: swatch(slot_1),
            internal: swatch(slot_2),
            border: None,
        },
        card: CardRole {
            fill: card_fill.color(),
            border: solid_stroke(rule.color(), 1.5),
        },
        fact: Swatch {
            fill: base[0x01].color(),
            ink: secondary.color(),
        },
        ask: swatch(slot_4),
        tag: TagRole {
            fill: card_fill.color(),
            border: solid_stroke(rule.color(), 1.5),
            ink: primary.color(),
            sub_ink: secondary.color(),
        },
        legend: LegendRole {
            label_ink: primary.color(),
            text_ink: secondary.color(),
        },
        foot: base[0x04].color(),
        callout: CalloutRole {
            note: wire_on_fill(slot_1),
            risk: Accent {
                accent: red.color(),
                fill: page.mix(red, fill_mix).color(),
            },
            decision: wire_on_fill(slot_6),
            open: wire_on_fill(slot_4),
        },
        placeholder: PlaceholderRole {
            border: rule.color(),
            diagonal: base[0x04].color(),
        },
        icon_chip: Some(white.color()),
        frame: FrameRole {
            border: blue.color(),
            frame_fill: page.color(),
            bar_fill: blue.color(),
            bar_ink: bar_ink.color(),
            bar_rule: None,
            body_fill: base[0x01].color(),
        },
        tones: Tones {
            neutral: tone(Some(tone_fill), Some(rule)),
            warm: tone(Some(tone_fill), Some(rule)),
            cool: tone(Some(tone_fill), Some(rule)),
            soft: tone(Some(tone_fill), None),
            strong: tone(None, Some(rule)),
            highlight: tone(Some(tone_fill), Some(base[0x0A])),
            emphasis: tone(Some(tone_fill), Some(blue)),
            accent: ToneRole {
                fill: Some(accent_fill.color()),
                border: Some(base[0x09].color()),
                label_ink: Some(higher_contrast(base[0x09], primary, accent_fill).color()),
            },
        },
        containers: ContainerRole {
            draw_width: None,
            frame_draw_width: None,
            borderless_outline: None,
        },
        lanes: LanesRole {
            lifeline: ThemeStroke {
                color: base[0x04].color(),
                width: 1.0,
                pattern: LinePattern::Dashed,
            },
        },
        gray: GrayRole {
            color: base[0x04].color(),
            width: 2.0,
            pattern: LinePattern::Solid,
            dot: DotStyle::Filled,
        },
        solid: SolidRole {
            width: 2.0,
            dots: [DotStyle::Filled; 8],
        },
        dash: DashRole {
            width: 2.0,
            pattern: LinePattern::Dashed,
            dot: DotStyle::Filled,
        },
        deny: DenyRole {
            color: red.color(),
            width: 2.0,
            pattern: LinePattern::Dashed,
            dot: DotStyle::Filled,
            tag_border: page.mix(red, BORDER_MIX).color(),
            tag_ink: higher_contrast(red, primary, card_fill).color(),
        },
        tints,
        iso: IsoRole {
            faces: FaceSteps {
                top: 0,
                left: -face_step,
                right: -2 * face_step,
            },
            slab_thickness,
            frame_sides: Some(FrameSides {
                left: blue.color(),
                right: Color(frame_right),
            }),
            frame_outline: 1.5,
            solid_edges: SolidEdges::None,
            edge_width: 1.5,
            ring: None,
            block_outline: None,
            labels: LabelInks {
                frame: if dark { bar_ink.color() } else { blue.color() },
                zone: if dark { primary.color() } else { rule.color() },
            },
            chip: ChipRole {
                fill: white.color(),
                ring: Some(rule.color()),
                shadow: None,
            },
            shadow: Some(shadow),
            widths: IsoWidths {
                primary: 3.75,
                secondary: 2.5,
                gray: 2.0,
            },
        },
    })
}

/// The theme JSON as written: the pretty serialization in declaration order and a newline.
pub fn theme_file_text(theme: &Theme) -> Result<String, ImportError> {
    serde_json::to_string_pretty(theme)
        .map(|text| text + "\n")
        .map_err(ImportError::Serialize)
}

/// The report file beside the theme file: `<theme file stem>.report.txt`.
pub fn report_path(theme_path: &Path) -> PathBuf {
    let stem = theme_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    theme_path.with_file_name(format!("{stem}.report.txt"))
}

/// The scheme's bytes. A file over SCHEME_BYTES_MAX is a base16 error; a file that cannot
/// be opened or read, a directory included, is a read error.
fn read_scheme(path: &Path) -> Result<String, ImportError> {
    let read_error = |source| ImportError::ReadScheme {
        path: path.to_path_buf(),
        source,
    };
    let file = fs::File::open(path).map_err(read_error)?;
    if file.metadata().map_err(read_error)?.is_dir() {
        return Err(read_error(io::Error::from(io::ErrorKind::IsADirectory)));
    }
    let limit = u64::try_from(SCHEME_BYTES_MAX).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(read_error)?;
    let base16_error = |message: String| ImportError::Base16 {
        path: path.to_path_buf(),
        message,
    };
    if bytes.len() > SCHEME_BYTES_MAX {
        return Err(base16_error(format!(
            "scheme is larger than {SCHEME_BYTES_MAX} bytes"
        )));
    }
    String::from_utf8(bytes).map_err(|error| {
        base16_error(format!(
            "scheme is not valid UTF-8 at byte {}",
            error.utf8_error().valid_up_to()
        ))
    })
}

/// A theme imported in memory, with its two files' contents.
pub struct Imported {
    pub theme_text: String,
    pub report_lines: Vec<String>,
    pub exit: ExitCode,
}

/// Reads and maps the scheme and builds both files' contents, writing nothing.
pub fn import_scheme(scheme_path: &Path, name: Option<&str>) -> Result<Imported, ImportError> {
    let scheme_text = read_scheme(scheme_path)?;
    let base16_error = |message: String| ImportError::Base16 {
        path: scheme_path.to_path_buf(),
        message,
    };
    let colors = read_base16(&scheme_text).map_err(base16_error)?;
    let name = name.map_or_else(|| name_from_stem(scheme_path), str::to_string);
    if !is_valid_theme_name(&name) {
        return Err(base16_error(format!(
            "theme name {name:?} does not match {THEME_NAME_PATTERN}; pass --name"
        )));
    }
    let theme = base16_theme(&colors, &name, scheme_path)?;
    let theme_text = theme_file_text(&theme)?;
    let report = theme_quality(&theme);
    let report_lines = quality_report_lines(&report, &name);
    let exit = quality_exit_code(&report);
    Ok(Imported {
        theme_text,
        report_lines,
        exit,
    })
}

fn write_file(path: &Path, text: &str) -> Result<(), ImportError> {
    fs::write(path, text).map_err(|source| ImportError::WriteOutput {
        path: path.to_path_buf(),
        source,
    })
}

/// The path as an absolute path, without resolving links.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `stencil theme import --base16 <scheme> [--name <name>] -o <theme file>`: writes the
/// theme and its report, prints the report lines and the two paths, and exits 1 when a
/// quality row fails. A scheme that does not read writes nothing.
pub fn import(
    scheme_path: &Path,
    name: Option<&str>,
    theme_path: &Path,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<ExitCode> {
    let written = import_scheme(scheme_path, name).and_then(|imported| {
        let report_path = report_path(theme_path);
        let mut report_text = imported.report_lines.join("\n");
        report_text.push('\n');
        write_file(theme_path, &imported.theme_text)?;
        write_file(&report_path, &report_text)?;
        Ok((imported, report_path))
    });
    match written {
        Ok((imported, report_path)) => {
            for line in &imported.report_lines {
                writeln!(stdout, "{line}")?;
            }
            writeln!(stdout, "{}", absolute(theme_path).display())?;
            writeln!(stdout, "{}", absolute(&report_path).display())?;
            Ok(imported.exit)
        }
        Err(error) => {
            let code = import_exit_code(&error);
            if code == ExitCode::Defects {
                writeln!(stdout, "error {error}")?;
            } else {
                writeln!(stderr, "stencil theme import: {error}")?;
                let mut cause = error.source();
                for _ in 0..crate::ERROR_CHAIN_MAX {
                    let Some(current) = cause else {
                        break;
                    };
                    writeln!(stderr, "  caused by: {current}")?;
                    cause = current.source();
                }
            }
            Ok(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_line_matches_the_section_13_9_pattern() {
        let read = |line: &str| key_line(line).map(|parsed| (parsed.key, parsed.channels));
        assert_eq!(
            read("  base0D: \"#2AC3DE\""),
            Some((0x0D, [0x2A, 0xC3, 0xDE]))
        );
        assert_eq!(read("base0d: 2ac3de"), Some((0x0D, [0x2A, 0xC3, 0xDE])));
        assert_eq!(
            read("base00: \"1a1b26\" # page"),
            Some((0x00, [0x1A, 0x1B, 0x26]))
        );
        assert_eq!(read("base17:#FFFFFF"), Some((0x17, [0xFF; 3])));
        assert_eq!(read("base0D: \"#2AC3DE0\""), None);
        assert_eq!(read("base0D: \"#2AC3D\""), None);
        assert_eq!(read("base0D: '#2AC3DE'"), None);
        assert_eq!(read("base0G: \"#2AC3DE\""), None);
        assert_eq!(read("name: \"Nord\""), None);
        assert_eq!(read("base0D: \"#2AC3DE\" trailing"), None);
    }

    #[test]
    fn a_system_line_is_read_without_quotes_or_comment() {
        assert_eq!(system_line("system: \"base16\""), Some("base16"));
        assert_eq!(system_line("system: base24 # superset"), Some("base24"));
        assert_eq!(system_line("  system: 'base16'"), Some("base16"));
        assert_eq!(system_line("slug: nord"), None);
    }

    #[test]
    fn mix_rounds_as_the_render_palette_does() {
        let samples = [
            ("#1A1B26", "#2AC3DE", 0.18),
            ("#FDF6E3", "#268BD2", 0.22),
            ("#16161E", "#2F3549", 0.5),
            ("#000000", "#FFFFFF", 0.5),
        ];
        for (from, toward, fraction) in samples {
            let parse = |hex: &str| {
                let parsed = key_line(&format!("base00: {hex}")).unwrap();
                Rgb(parsed.channels)
            };
            let mixed = parse(from).mix(parse(toward), fraction).color();
            let expected = stencil_render::palette::mix(from, toward, fraction).unwrap();
            assert_eq!(mixed.0, expected, "{from} {toward} {fraction}");
        }
    }

    #[test]
    fn the_name_comes_from_the_file_stem() {
        assert_eq!(name_from_stem(Path::new("schemes/Nord.yaml")), "nord");
        assert_eq!(
            name_from_stem(Path::new("tokyo_night dark.yaml")),
            "tokyo-night-dark"
        );
        assert_eq!(
            name_from_stem(Path::new("tokyo-night.base16.yaml")),
            "tokyo-night-base16"
        );
    }

    #[test]
    fn the_report_sits_beside_the_theme_file() {
        assert_eq!(
            report_path(Path::new("out/nord.json")),
            Path::new("out/nord.report.txt")
        );
    }

    #[test]
    fn exit_codes_follow_section_13_12() {
        let path = PathBuf::from("scheme.yaml");
        let base16 = ImportError::Base16 {
            path: path.clone(),
            message: "missing base0B".to_string(),
        };
        assert_eq!(import_exit_code(&base16), ExitCode::Defects);
        let unreadable = ImportError::ReadScheme {
            path: path.clone(),
            source: io::Error::from(io::ErrorKind::NotFound),
        };
        assert_eq!(import_exit_code(&unreadable), ExitCode::CouldNotRun);
        let write = ImportError::WriteOutput {
            path,
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };
        assert_eq!(import_exit_code(&write), ExitCode::CouldNotRun);
    }
}
