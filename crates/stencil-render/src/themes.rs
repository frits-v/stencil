//! The built-in themes: the six designed ones of section 13.5, embedded from
//! `crates/stencil-render/themes/`, and the imported tier of section 13.9, embedded from
//! `crates/stencil-render/themes/imported/`.

use stencil_model::{Theme, ThemeError, parse_theme};

const CENTER_JSON: &str = include_str!("../themes/center.json");
const PAPER_JSON: &str = include_str!("../themes/paper.json");
const DUSK_JSON: &str = include_str!("../themes/dusk.json");
const CLEAR_JSON: &str = include_str!("../themes/clear.json");
const CLEAR_DARK_JSON: &str = include_str!("../themes/clear-dark.json");
const WIRE_JSON: &str = include_str!("../themes/wire.json");
const TOKYO_NIGHT_JSON: &str = include_str!("../themes/imported/tokyo-night.json");
const SOLARIZED_LIGHT_JSON: &str = include_str!("../themes/imported/solarized-light.json");
const SOLARIZED_DARK_JSON: &str = include_str!("../themes/imported/solarized-dark.json");
const MATERIAL_DARK_JSON: &str = include_str!("../themes/imported/material-dark.json");
const GRUVBOX_DARK_JSON: &str = include_str!("../themes/imported/gruvbox-dark.json");
const DRACULA_JSON: &str = include_str!("../themes/imported/dracula.json");
const NORD_JSON: &str = include_str!("../themes/imported/nord.json");

/// The embedded JSON of a built-in theme, exactly as committed; None for any other name.
pub fn builtin_theme_json(name: &str) -> Option<&'static str> {
    match name {
        "center" => Some(CENTER_JSON),
        "paper" => Some(PAPER_JSON),
        "dusk" => Some(DUSK_JSON),
        "clear" => Some(CLEAR_JSON),
        "clear-dark" => Some(CLEAR_DARK_JSON),
        "wire" => Some(WIRE_JSON),
        "tokyo-night" => Some(TOKYO_NIGHT_JSON),
        "solarized-light" => Some(SOLARIZED_LIGHT_JSON),
        "solarized-dark" => Some(SOLARIZED_DARK_JSON),
        "material-dark" => Some(MATERIAL_DARK_JSON),
        "gruvbox-dark" => Some(GRUVBOX_DARK_JSON),
        "dracula" => Some(DRACULA_JSON),
        "nord" => Some(NORD_JSON),
        _ => None,
    }
}

/// The built-in theme of that name, parsed and validated on demand; None for any other
/// name. A test parses every built-in, so a broken file fails CI, never a user's run.
pub fn builtin_theme(name: &str) -> Option<Result<Theme, ThemeError>> {
    builtin_theme_json(name).map(|json_text| parse_theme(json_text, name))
}
