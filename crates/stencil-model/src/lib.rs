//! Stencil document model: the section 1.2 types, vet rules, JSON Schema, node walks,
//! the text measurement contract and the geometry-free checks.

pub mod checks;
pub mod document;
pub mod grammar;
pub mod pointer;
pub mod text;
pub mod theme;

mod vet;
mod walk;

pub use document::*;
pub use grammar::{
    Grammar, GrammarError, GrammarRule, GrammarViolation, builtin_grammar, grammar_schema,
    parse_grammar, validate_grammar,
};
pub use pointer::NodePointer;
pub use theme::{
    Theme, ThemeError, ThemeRule, ThemeViolation, apply_overrides, drawn_legend_label, parse_theme,
    theme_quality, theme_schema, validate_theme,
};
pub use vet::{VetRule, Violation, is_grammar_reference, is_theme_reference, validate_page};
pub use walk::{NodeEntry, NodeRef, TextField, body_nodes, text_fields};

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("document is not valid stencil JSON at line {line}, column {column}: {message}")]
    Json {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("document violates {} vet rule(s)", .0.len())]
    Invalid(Vec<Violation>),
}

/// serde_json parsing alone. The grammar a page names is known only after parsing, so
/// callers resolve it and then run `validate_page`, or call `parse_and_vet`.
pub fn parse_page(json_text: &str) -> Result<Page, ModelError> {
    serde_json::from_str(json_text).map_err(|error| json_error(&error))
}

/// `parse_page` followed by `validate_page` against `grammar`.
pub fn parse_and_vet(json_text: &str, grammar: &Grammar) -> Result<Page, ModelError> {
    let page = parse_page(json_text)?;
    vet_page(page, grammar)
}

/// `validate_page`, returning the page when it is valid.
pub fn vet_page(page: Page, grammar: &Grammar) -> Result<Page, ModelError> {
    let violations = validate_page(&page, grammar);
    if violations.is_empty() {
        Ok(page)
    } else {
        Err(ModelError::Invalid(violations))
    }
}

/// The grammar reference a page names, `gcp` when it names none.
pub fn grammar_reference(page: &Page) -> &str {
    page.grammar.as_deref().unwrap_or(GRAMMAR_DEFAULT)
}

/// The theme reference a page names, `center` when it names none.
pub fn theme_reference(page: &Page) -> &str {
    page.theme.as_deref().unwrap_or(THEME_DEFAULT)
}

/// serde_json's Display appends " at line L column C"; the location is carried in its own
/// fields, so the message keeps only the cause.
fn json_error(error: &serde_json::Error) -> ModelError {
    let line = error.line();
    let column = error.column();
    let full_message = error.to_string();
    let location_suffix = format!(" at line {line} column {column}");
    let message = full_message
        .strip_suffix(&location_suffix)
        .unwrap_or(&full_message)
        .to_string();
    ModelError::Json {
        line,
        column,
        message,
    }
}

pub fn page_schema() -> schemars::Schema {
    schemars::schema_for!(Page)
}
