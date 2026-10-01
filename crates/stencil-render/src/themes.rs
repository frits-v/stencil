//! The designed built-in themes of section 13.5, embedded as the JSON files under
//! `crates/stencil-render/themes/`.

use stencil_model::{Theme, ThemeError, parse_theme};

const CENTER_JSON: &str = include_str!("../themes/center.json");
const PAPER_JSON: &str = include_str!("../themes/paper.json");
const DUSK_JSON: &str = include_str!("../themes/dusk.json");
const CLEAR_JSON: &str = include_str!("../themes/clear.json");
const CLEAR_DARK_JSON: &str = include_str!("../themes/clear-dark.json");
const WIRE_JSON: &str = include_str!("../themes/wire.json");

/// The embedded JSON of a built-in theme, exactly as committed; None for any other name.
pub fn builtin_theme_json(name: &str) -> Option<&'static str> {
    match name {
        "center" => Some(CENTER_JSON),
        "paper" => Some(PAPER_JSON),
        "dusk" => Some(DUSK_JSON),
        "clear" => Some(CLEAR_JSON),
        "clear-dark" => Some(CLEAR_DARK_JSON),
        "wire" => Some(WIRE_JSON),
        _ => None,
    }
}

/// The built-in theme of that name, parsed and validated on demand; None for any other
/// name. A test parses every built-in, so a broken file fails CI, never a user's run.
pub fn builtin_theme(name: &str) -> Option<Result<Theme, ThemeError>> {
    builtin_theme_json(name).map(|json_text| parse_theme(json_text, name))
}
