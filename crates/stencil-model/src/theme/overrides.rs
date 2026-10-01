//! `Page.theme_overrides` (section 13.4 rule 8): a partial theme merged onto the resolved
//! base theme before validation.

use serde_json::{Map, Value};

use super::{
    BlockShadow, ChipShadow, Color, FrameSides, LinePattern, Theme, ThemeError, ThemeRule,
    ThemeStroke, ThemeViolation, checked, strip_location,
};
use crate::pointer::NodePointer;

/// The page field the override pointers start with.
const OVERRIDES_FIELD: &str = "theme_overrides";
/// The deepest a theme nests objects is 4 (`iso.chip.shadow.color`); the merge never
/// descends further than the base does, and this bound holds it to that.
const MERGE_DEPTH_MAX: usize = 8;

/// Deep-merges `overrides` onto `base` (rule 8), then parses and validates the result.
/// Objects merge key by key; any other value replaces the base value. `tints` takes an
/// object keyed by slot number `"1"` to `"8"`, each value merged into that slot. Every
/// pointer in an error starts with `/theme_overrides`.
pub fn apply_overrides(base: &Theme, overrides: &Map<String, Value>) -> Result<Theme, ThemeError> {
    let origin = format!("{} with /{OVERRIDES_FIELD}", base.name);
    let json_error = |error: serde_json::Error| ThemeError::Json {
        origin: origin.clone(),
        line: error.line(),
        column: error.column(),
        message: strip_location(&error),
    };
    let mut merged = serde_json::to_value(base).map_err(json_error)?;
    let shape = serde_json::to_value(full_shape(base)).map_err(json_error)?;
    let mut violations = Vec::new();
    let root = NodePointer::root().child(OVERRIDES_FIELD);
    merge_object(&mut merged, &shape, overrides, &root, 0, &mut violations);
    if !violations.is_empty() {
        return Err(ThemeError::Invalid { origin, violations });
    }
    let theme: Theme = serde_json::from_value(merged).map_err(json_error)?;
    checked(theme, &origin).map_err(|error| match error {
        ThemeError::Invalid { origin, violations } => ThemeError::Invalid {
            origin,
            violations: violations.into_iter().map(override_violation).collect(),
        },
        other => other,
    })
}

/// A violation of the merged theme, its pointer moved under `/theme_overrides` with each
/// tint index written as its slot number.
fn override_violation(violation: ThemeViolation) -> ThemeViolation {
    let mut pointer = NodePointer::root().child(OVERRIDES_FIELD);
    let mut tokens = violation.pointer.as_str().split('/').skip(1).peekable();
    while let Some(token) = tokens.next() {
        pointer = pointer.child(token);
        if token == "tints"
            && let Some(index) = tokens.next()
        {
            let slot = index
                .parse::<usize>()
                .map_or_else(|_| index.to_string(), |index| (index + 1).to_string());
            pointer = pointer.child(&slot);
        }
    }
    ThemeViolation {
        pointer,
        ..violation
    }
}

/// Merges `overrides` into the object `target`, whose keys and value kinds `shape` gives.
fn merge_object(
    target: &mut Value,
    shape: &Value,
    overrides: &Map<String, Value>,
    pointer: &NodePointer,
    depth: usize,
    violations: &mut Vec<ThemeViolation>,
) {
    let (Some(target_fields), Some(shape_fields)) = (target.as_object_mut(), shape.as_object())
    else {
        return;
    };
    for (key, value) in overrides {
        let key_pointer = pointer.child(key);
        let Some(shape_value) = shape_fields.get(key) else {
            violations.push(ThemeViolation {
                pointer: key_pointer,
                rule: ThemeRule::OverrideUnknownRole,
                message: format!("\"{key}\" is not a theme role"),
            });
            continue;
        };
        if key == "tints" {
            merge_tints(
                target_fields,
                shape_value,
                value,
                &key_pointer,
                depth,
                violations,
            );
            continue;
        }
        if !shape_value.is_object() {
            target_fields.insert(key.clone(), value.clone());
            continue;
        }
        let Some(value_fields) = value.as_object() else {
            violations.push(not_object(key_pointer, key));
            continue;
        };
        if depth >= MERGE_DEPTH_MAX {
            continue;
        }
        let slot = target_fields
            .entry(key.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        merge_object(
            slot,
            shape_value,
            value_fields,
            &key_pointer,
            depth + 1,
            violations,
        );
    }
}

/// `tints` in an override is an object keyed by slot number.
fn merge_tints(
    target_fields: &mut Map<String, Value>,
    shape: &Value,
    value: &Value,
    pointer: &NodePointer,
    depth: usize,
    violations: &mut Vec<ThemeViolation>,
) {
    let Some(slots) = value.as_object() else {
        violations.push(not_object(pointer.clone(), "tints"));
        return;
    };
    let slot_shape = shape.get(0).cloned().unwrap_or(Value::Null);
    let Some(tints) = target_fields.get_mut("tints").and_then(Value::as_array_mut) else {
        return;
    };
    for (key, slot_value) in slots {
        let slot_pointer = pointer.child(key);
        let index = key
            .parse::<usize>()
            .ok()
            .filter(|slot| (1..=8).contains(slot))
            .map(|slot| slot - 1);
        let Some(tint) = index.and_then(|index| tints.get_mut(index)) else {
            violations.push(ThemeViolation {
                pointer: slot_pointer,
                rule: ThemeRule::OverrideUnknownRole,
                message: format!("\"{key}\" is not a tint slot 1 to 8"),
            });
            continue;
        };
        let Some(slot_fields) = slot_value.as_object() else {
            violations.push(not_object(slot_pointer, key));
            continue;
        };
        merge_object(
            tint,
            &slot_shape,
            slot_fields,
            &slot_pointer,
            depth + 1,
            violations,
        );
    }
}

fn not_object(pointer: NodePointer, key: &str) -> ThemeViolation {
    ThemeViolation {
        pointer,
        rule: ThemeRule::OverrideNotObject,
        message: format!("\"{key}\" holds an object in the theme; its override must be one"),
    }
}

/// `base` with every optional role present, so its serialization names every key a theme
/// may hold and which of them hold objects.
fn full_shape(base: &Theme) -> Theme {
    let color = base.page.clone();
    let stroke = ThemeStroke {
        color: color.clone(),
        width: 1.0,
        pattern: LinePattern::Solid,
    };
    let mut shape = base.clone();
    shape.badge.border = Some(stroke.clone());
    shape.icon_chip = Some(color.clone());
    shape.frame.bar_rule = Some(stroke.clone());
    for tone in [
        &mut shape.tones.neutral,
        &mut shape.tones.warm,
        &mut shape.tones.cool,
        &mut shape.tones.soft,
        &mut shape.tones.strong,
        &mut shape.tones.highlight,
        &mut shape.tones.emphasis,
        &mut shape.tones.accent,
    ] {
        tone.fill = Some(color.clone());
        tone.border = Some(color.clone());
        tone.label_ink = Some(color.clone());
    }
    shape.containers.draw_width = Some(1.0);
    shape.containers.frame_draw_width = Some(1.0);
    shape.containers.borderless_outline = Some(stroke.clone());
    shape.iso.frame_sides = Some(FrameSides {
        left: color.clone(),
        right: color.clone(),
    });
    shape.iso.ring = Some(stroke.clone());
    shape.iso.block_outline = Some(stroke);
    shape.iso.chip.ring = Some(color.clone());
    shape.iso.chip.shadow = Some(ChipShadow {
        color: color.clone(),
        opacity: 1.0,
        dy: 1.0,
    });
    shape.iso.shadow = Some(BlockShadow {
        color: Color(color.0),
        opacity: 1.0,
        blur: 1.0,
        dy: 1.0,
    });
    shape
}
