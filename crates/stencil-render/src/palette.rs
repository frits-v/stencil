//! Every color of sections 2.4 to 2.9 and 5.2. Text colors are defined once in
//! `stencil_layout::styles`, which resolves them into each `TextRun`, and are re-exported
//! here so the palette is complete in one place.

use stencil_model::{PipeKind, ZoneKind};

pub use stencil_layout::styles::{
    BADGE_FILL_CUSTOMER, BADGE_FILL_INTERNAL, BADGE_TEXT_CUSTOMER, BADGE_TEXT_INTERNAL, TEXT_AMBER,
    TEXT_BLUE, TEXT_DARK, TEXT_DENY, TEXT_MUTED, TEXT_WHITE,
};

pub const CANVAS_FILL: &str = "#FFFFFF";

pub const GCP_BORDER: &str = "#1A73E8";
pub const GCP_FRAME_FILL: &str = "#FFFFFF";
pub const GCP_BAR_FILL: &str = "#1A73E8";
pub const GCP_BODY_FILL: &str = "#FAFBFC";
pub const VPC_BORDER: &str = "#5F6368";
pub const REGION_BORDER: &str = "#BDC1C6";
pub const SUBNET_BORDER: &str = "#9AA0A6";
pub const ONPREM_BORDER: &str = "#D7CCC8";
pub const PROJECT_BORDER: &str = "#FFE082";
pub const OPTIONAL_BORDER: &str = "#4284F3";
pub const PERIMETER_BORDER: &str = "#E37400";
pub const TINT_A_FILL: &str = "#D2E3FC";
pub const TINT_B_FILL: &str = "#FCE4EC";
pub const SUBNET_FILL: &str = "#EDE7F6";
pub const PROJECT_FILL: &str = "#FFF8E1";
pub const OPTIONAL_FILL: &str = "#F8FBFF";
pub const K8S_FILL: &str = "#FCE4EC";
pub const PERIMETER_FILL: &str = "#FFFBF5";

pub const CARD_FILL: &str = "#FFFFFF";
pub const CARD_BORDER: &str = "#DADCE0";
pub const FACT_FILL: &str = "#F1F3F4";
pub const ASK_FILL: &str = "#FEF7E0";

pub const TAG_FILL: &str = "#FFFFFF";
pub const TAG_BORDER: &str = "#DADCE0";
pub const TAG_BORDER_DENY: &str = "#F4C7C3";

pub const WIRE_GRAY: &str = "#5F6368";
pub const WIRE_BLUE: &str = "#1A73E8";
pub const WIRE_PINK: &str = "#C2185B";
pub const WIRE_DENY: &str = "#C5221F";

/// `stroke-dasharray` of every dashed border and dashed wire (section 5.2).
pub const DASH_ARRAY: &str = "6 5";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    Dashed,
}

/// A Zone's section 2.4 row, the gcp frame's border and frame fill included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneStyle {
    /// None for k8s, which has no border.
    pub border: Option<Stroke>,
    /// None for vpc, which is unfilled.
    pub fill: Option<&'static str>,
    pub radius_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width_px: f32,
    pub line: LineStyle,
    pub color: &'static str,
}

pub fn zone_style(kind: ZoneKind) -> ZoneStyle {
    let solid = |width_px, color| Stroke {
        width_px,
        line: LineStyle::Solid,
        color,
    };
    let dashed = |width_px, color| Stroke {
        width_px,
        line: LineStyle::Dashed,
        color,
    };
    match kind {
        ZoneKind::Gcp => ZoneStyle {
            border: Some(solid(3.0, GCP_BORDER)),
            fill: Some(GCP_FRAME_FILL),
            radius_px: 10.0,
        },
        ZoneKind::Vpc => ZoneStyle {
            border: Some(dashed(2.0, VPC_BORDER)),
            fill: None,
            radius_px: 8.0,
        },
        ZoneKind::RegionA => ZoneStyle {
            border: Some(solid(1.5, REGION_BORDER)),
            fill: Some(TINT_A_FILL),
            radius_px: 8.0,
        },
        ZoneKind::RegionB => ZoneStyle {
            border: Some(solid(1.5, REGION_BORDER)),
            fill: Some(TINT_B_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Subnet => ZoneStyle {
            border: Some(dashed(1.5, SUBNET_BORDER)),
            fill: Some(SUBNET_FILL),
            radius_px: 8.0,
        },
        ZoneKind::OnpremA => ZoneStyle {
            border: Some(solid(1.5, ONPREM_BORDER)),
            fill: Some(TINT_A_FILL),
            radius_px: 8.0,
        },
        ZoneKind::OnpremB => ZoneStyle {
            border: Some(solid(1.5, ONPREM_BORDER)),
            fill: Some(TINT_B_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Project => ZoneStyle {
            border: Some(solid(1.5, PROJECT_BORDER)),
            fill: Some(PROJECT_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Optional => ZoneStyle {
            border: Some(dashed(2.0, OPTIONAL_BORDER)),
            fill: Some(OPTIONAL_FILL),
            radius_px: 8.0,
        },
        ZoneKind::K8s => ZoneStyle {
            border: None,
            fill: Some(K8S_FILL),
            radius_px: 8.0,
        },
        ZoneKind::Perimeter => ZoneStyle {
            border: Some(dashed(2.5, PERIMETER_BORDER)),
            fill: Some(PERIMETER_FILL),
            radius_px: 10.0,
        },
    }
}

/// Wire, dot, spine and legend swatch color and line style (section 5.2).
pub fn wire_style(kind: PipeKind) -> (&'static str, LineStyle) {
    match kind {
        PipeKind::Gray => (WIRE_GRAY, LineStyle::Solid),
        PipeKind::Blue => (WIRE_BLUE, LineStyle::Solid),
        PipeKind::Pink => (WIRE_PINK, LineStyle::Solid),
        PipeKind::Dash => (WIRE_BLUE, LineStyle::Dashed),
        PipeKind::Deny => (WIRE_DENY, LineStyle::Dashed),
    }
}

/// Border of a Pipe tag or Tee hub (sections 2.7 and 2.8).
pub fn tag_border(kind: PipeKind) -> &'static str {
    match kind {
        PipeKind::Deny => TAG_BORDER_DENY,
        PipeKind::Gray | PipeKind::Blue | PipeKind::Pink | PipeKind::Dash => TAG_BORDER,
    }
}
