//! SVG writer, PNG through resvg, and the measured JSON (SPEC sections 4.5 and 5).

pub mod iso;
pub mod palette;

mod icons;
mod measured;
mod png;
mod svg;

use std::fmt;

use stencil_model::pointer::NodePointer;
use stencil_text::FontError;

pub use icons::{icon_data_uri, icon_svg_bytes};
pub use measured::measured_json;
pub use png::{PNG_PIXELS_MAX, render_png};
pub use svg::render_svg;

#[derive(Debug, Clone, PartialEq)]
pub struct SvgDocument {
    pub svg: String,
    /// Number of <text> elements written: one per line of every TextRun.
    pub text_elements: usize,
}

/// Section 5.1 rounding. The SVG writer and measured_json both call it.
/// Callers pass finite values only; layout returns NonFinite otherwise.
pub fn format_number(value: f32) -> NumberRepr {
    debug_assert!(value.is_finite(), "format_number got {value}");
    let rounded = (f64::from(value) * 100.0).round() / 100.0;
    // -0.0 has no fractional part, so it lands in Integer(0) and never prints as "-0".
    if rounded.fract() == 0.0 {
        NumberRepr::Integer(rounded as i64)
    } else {
        NumberRepr::Decimal(rounded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberRepr {
    /// The rounded value has no fractional part.
    Integer(i64),
    /// At most 2 decimals.
    Decimal(f64),
}

impl NumberRepr {
    /// Integer through serde_json::Number from i64, Decimal through
    /// serde_json::Value::from(f64), so no Option is unwrapped.
    pub fn to_json(self) -> serde_json::Value {
        match self {
            NumberRepr::Integer(integer) => serde_json::Value::Number(integer.into()),
            NumberRepr::Decimal(decimal) => serde_json::Value::from(decimal),
        }
    }
}

/// The SVG attribute form: an integer with no decimal point, or f64 Display.
impl fmt::Display for NumberRepr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumberRepr::Integer(integer) => write!(formatter, "{integer}"),
            NumberRepr::Decimal(decimal) => write!(formatter, "{decimal}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceScale(u8);

impl DeviceScale {
    pub const DEFAULT: DeviceScale = DeviceScale(2);
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 4;

    /// Accepts 1 to 4.
    pub fn new(value: u8) -> Result<Self, RenderError> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(DeviceScale(value))
        } else {
            Err(RenderError::ScaleOutOfRange { value })
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("device scale {value} is outside 1 to 4")]
    ScaleOutOfRange { value: u8 },
    #[error("geometry node {found} does not match document node {expected}")]
    GeometryMismatch {
        expected: NodePointer,
        found: NodePointer,
    },
    #[error("generated SVG does not parse: {message}")]
    Svg { message: String },
    #[error("{count} text element(s) rendered no glyphs")]
    TextNotRendered { count: usize },
    #[error("parsed SVG holds {found} text node(s), more than the {expected} expected")]
    TextCountExceeded { expected: usize, found: usize },
    #[error("{lookups} font or glyph lookup(s) found no bundled face")]
    FontNotResolved { lookups: usize },
    #[error("cannot allocate a {width}x{height} pixmap")]
    PixmapAllocation { width: u32, height: u32 },
    #[error("PNG encoding failed: {message}")]
    PngEncode { message: String },
    #[error(transparent)]
    Fonts(#[from] FontError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_maps_negative_zero_and_rounds_to_two_decimals() {
        assert_eq!(format_number(-0.0), NumberRepr::Integer(0));
        assert_eq!(format_number(-0.001), NumberRepr::Integer(0));
        assert_eq!(format_number(1320.0), NumberRepr::Integer(1320));
        assert_eq!(format_number(652.004), NumberRepr::Integer(652));
        assert_eq!(format_number(652.4), NumberRepr::Decimal(652.4));
        assert_eq!(format_number(-3.456), NumberRepr::Decimal(-3.46));
    }

    #[test]
    fn number_display_has_no_decimal_point_for_integers_and_no_trailing_zero() {
        assert_eq!(format_number(1320.0).to_string(), "1320");
        assert_eq!(format_number(-0.0).to_string(), "0");
        assert_eq!(format_number(-0.004).to_string(), "0");
        assert_eq!(format_number(652.4).to_string(), "652.4");
        assert_eq!(format_number(0.7).to_string(), "0.7");
        assert_eq!(format_number(1320.05).to_string(), "1320.05");
    }

    #[test]
    fn number_json_is_an_integer_number_when_integral() {
        let integral = format_number(1320.0).to_json();
        assert!(integral.is_i64());
        assert_eq!(serde_json::to_string(&integral).unwrap(), "1320");
        let zero = format_number(-0.0).to_json();
        assert_eq!(serde_json::to_string(&zero).unwrap(), "0");
        let decimal = format_number(652.4).to_json();
        assert!(decimal.is_f64());
        assert_eq!(serde_json::to_string(&decimal).unwrap(), "652.4");
    }

    #[test]
    fn device_scale_accepts_one_to_four_only() {
        assert!(matches!(
            DeviceScale::new(0),
            Err(RenderError::ScaleOutOfRange { value: 0 })
        ));
        assert!(matches!(
            DeviceScale::new(5),
            Err(RenderError::ScaleOutOfRange { value: 5 })
        ));
        assert_eq!(DeviceScale::new(1).unwrap().get(), 1);
        assert_eq!(DeviceScale::new(4).unwrap().get(), 4);
        assert_eq!(DeviceScale::DEFAULT.get(), 2);
    }
}
