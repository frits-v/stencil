//! Stencil document model: the section 1.2 types, vet rules, JSON Schema, node walks,
//! the text measurement contract and the geometry-free checks.

pub mod checks;
pub mod document;
pub mod pointer;
pub mod text;

mod vet;
mod walk;

pub use document::*;
pub use pointer::NodePointer;
pub use vet::{VetRule, Violation, validate_page};
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

/// serde_json parse followed by validate_page.
pub fn parse_page(json_text: &str) -> Result<Page, ModelError> {
    let page: Page = serde_json::from_str(json_text).map_err(|error| json_error(&error))?;
    let violations = validate_page(&page);
    if violations.is_empty() {
        Ok(page)
    } else {
        Err(ModelError::Invalid(violations))
    }
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
