//! Grammars as data (section 13.2): the container and item kinds a figure may use, how
//! each container is drawn and where each kind may sit, and the remembered constants.
//! The source of a built-in grammar is a CUE file under `cue/grammars/`; its JSON export
//! is embedded here, and Rust never evaluates CUE.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Shape;
use crate::document::{IconName, KIND_PATTERN};
use crate::pointer::NodePointer;

/// The kind name a `parents` list uses for the top level of the page body.
pub const PAGE_PARENT: &str = "page";
pub const GRAMMAR_KINDS_MAX: usize = 32;
pub const GRAMMAR_REMEMBERED_MAX: usize = 64;
pub const GRAMMAR_PRODUCTS_MAX: usize = 64;
pub const GRAMMAR_PRODUCT_NAMES_MAX: usize = 64;
pub const GRAMMAR_PARENTS_MAX: usize = 64;
pub const BORDER_WIDTH_MAX_PX: f32 = 4.0;
pub const KIND_PADDING_MAX_PX: f32 = 32.0;
pub const KIND_RADIUS_MAX_PX: f32 = 16.0;

const GCP_JSON: &str = include_str!("../grammars/gcp.json");
const PLAIN_JSON: &str = include_str!("../grammars/plain.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Grammar {
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub name: String,
    #[schemars(length(min = 1, max = 32))]
    pub containers: Vec<ContainerKind>,
    #[schemars(length(min = 1, max = 32))]
    pub items: Vec<ItemKind>,
    #[schemars(length(max = 64))]
    pub remembered: Vec<Remembered>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The outermost system or cloud: a bar holding the label over a body.
    Frame,
    /// A network, trust or security edge.
    Boundary,
    /// A locality, cluster or site.
    Group,
    /// An ownership scope: a project, folder or account.
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Neutral,
    Warm,
    Cool,
    Soft,
    Strong,
    Highlight,
    Emphasis,
    Accent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BorderPattern {
    Solid,
    Dashed,
    Dotted,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KindBorder {
    pub pattern: BorderPattern,
    #[schemars(range(min = 0.0, max = 4.0))]
    pub width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LabelStyle {
    /// The zone label style.
    Plain,
    /// The perimeter label style.
    Accent,
    /// The gcp bar label, frame kinds only.
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContainerKind {
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub name: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
    pub tintable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub default_tint: Option<u8>,
    pub border: KindBorder,
    #[schemars(range(min = 0.0, max = 32.0))]
    pub padding: f32,
    #[schemars(range(min = 0.0, max = 16.0))]
    pub radius: f32,
    pub label: LabelStyle,
    #[schemars(length(min = 1))]
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IconPack {
    Gcp,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IconClass {
    /// From the archive's Unique Icons: stands for one product.
    Product,
    /// From the archive's Category Icons: stands for a product family.
    Category,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IconProducts {
    pub icon: IconName,
    pub class: IconClass,
    #[schemars(length(min = 1))]
    pub names: Vec<String>,
    /// Under iso, the solid an item with this icon stands as; absent is card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemKind {
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub name: String,
    pub icons: IconPack,
    pub products: Vec<IconProducts>,
    #[schemars(length(min = 1))]
    pub parents: Vec<String>,
    /// Under iso, the solid an item of this kind stands as when neither the item nor its
    /// icon's row names one; absent is card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Remembered {
    #[schemars(length(min = 1, max = 64))]
    pub literal: String,
    #[schemars(length(min = 1, max = 400))]
    pub reason: String,
}

impl Grammar {
    pub fn container(&self, kind: &str) -> Option<&ContainerKind> {
        self.containers
            .iter()
            .take(GRAMMAR_KINDS_MAX)
            .find(|container| container.name == kind)
    }

    pub fn item(&self, kind: &str) -> Option<&ItemKind> {
        self.items
            .iter()
            .take(GRAMMAR_KINDS_MAX)
            .find(|item| item.name == kind)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrammarViolation {
    /// RFC 6901 pointer into the grammar document.
    pub pointer: NodePointer,
    pub rule: GrammarRule,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarRule {
    NameMalformed,
    KindsOutOfRange,
    NameDuplicate,
    ParentUnknown,
    NoTopLevel,
    RoleMismatch,
    DefaultTintUntintable,
    DefaultTintOutOfRange,
    BorderMismatch,
    BoxOutOfRange,
    IconTable,
    RememberedOutOfRange,
    RememberedDuplicate,
}

impl GrammarRule {
    /// Kebab-case name with a `grammar-` prefix, as printed on the CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            GrammarRule::NameMalformed => "grammar-name-malformed",
            GrammarRule::KindsOutOfRange => "grammar-kinds-out-of-range",
            GrammarRule::NameDuplicate => "grammar-name-duplicate",
            GrammarRule::ParentUnknown => "grammar-parent-unknown",
            GrammarRule::NoTopLevel => "grammar-no-top-level",
            GrammarRule::RoleMismatch => "grammar-role-mismatch",
            GrammarRule::DefaultTintUntintable => "grammar-default-tint-untintable",
            GrammarRule::DefaultTintOutOfRange => "grammar-default-tint-out-of-range",
            GrammarRule::BorderMismatch => "grammar-border-mismatch",
            GrammarRule::BoxOutOfRange => "grammar-box-out-of-range",
            GrammarRule::IconTable => "grammar-icon-table",
            GrammarRule::RememberedOutOfRange => "grammar-remembered-out-of-range",
            GrammarRule::RememberedDuplicate => "grammar-remembered-duplicate",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GrammarError {
    #[error(
        "grammar {origin} is not valid grammar JSON at line {line}, column {column}: {message}"
    )]
    Json {
        origin: String,
        line: usize,
        column: usize,
        message: String,
    },
    #[error("grammar {origin} violates {} rule(s)", .violations.len())]
    Invalid {
        origin: String,
        violations: Vec<GrammarViolation>,
    },
}

/// The embedded built-in of that name, parsed and validated; None for any other name.
pub fn builtin_grammar(name: &str) -> Option<Result<Grammar, GrammarError>> {
    let json_text = match name {
        "gcp" => GCP_JSON,
        "plain" => PLAIN_JSON,
        _ => return None,
    };
    Some(parse_grammar(json_text, name))
}

/// The embedded JSON of a built-in grammar, exactly as committed.
pub fn builtin_grammar_json(name: &str) -> Option<&'static str> {
    match name {
        "gcp" => Some(GCP_JSON),
        "plain" => Some(PLAIN_JSON),
        _ => None,
    }
}

/// serde_json parse followed by validate_grammar. `origin` names the file or built-in in
/// error messages.
pub fn parse_grammar(json_text: &str, origin: &str) -> Result<Grammar, GrammarError> {
    let grammar: Grammar = serde_json::from_str(json_text).map_err(|error| GrammarError::Json {
        origin: origin.to_string(),
        line: error.line(),
        column: error.column(),
        message: strip_location(&error),
    })?;
    let violations = validate_grammar(&grammar);
    if violations.is_empty() {
        Ok(grammar)
    } else {
        Err(GrammarError::Invalid {
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

pub fn grammar_schema() -> schemars::Schema {
    schemars::schema_for!(Grammar)
}

/// True when `name` matches KIND_PATTERN: a lowercase ASCII letter, then 0 to 31 lowercase
/// letters, digits or `-`.
pub fn is_valid_kind_name(name: &str) -> bool {
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

struct Violations(Vec<GrammarViolation>);

impl Violations {
    fn push(&mut self, pointer: NodePointer, rule: GrammarRule, message: String) {
        self.0.push(GrammarViolation {
            pointer,
            rule,
            message,
        });
    }
}

/// Every grammar rule of section 13.2, reported in field order. Empty means valid.
pub fn validate_grammar(grammar: &Grammar) -> Vec<GrammarViolation> {
    let mut violations = Violations(Vec::new());
    let root = NodePointer::root();
    if !is_valid_kind_name(&grammar.name) {
        violations.push(
            root.child("name"),
            GrammarRule::NameMalformed,
            format!("name \"{}\" does not match {KIND_PATTERN}", grammar.name),
        );
    }

    let mut names: BTreeSet<&str> = BTreeSet::new();
    let mut container_names: BTreeSet<&str> = BTreeSet::new();
    for container in grammar.containers.iter().take(GRAMMAR_KINDS_MAX) {
        container_names.insert(container.name.as_str());
    }
    let mut top_level = false;

    let containers_pointer = root.child("containers");
    check_kind_count(
        &containers_pointer,
        grammar.containers.len(),
        &mut violations,
    );
    for (index, container) in grammar
        .containers
        .iter()
        .enumerate()
        .take(GRAMMAR_KINDS_MAX)
    {
        let pointer = containers_pointer.index(index);
        check_kind_name(&pointer, &container.name, &mut names, &mut violations);
        check_container(&pointer, container, &mut violations);
        top_level |= check_parents(
            &pointer,
            &container.parents,
            &container_names,
            &mut violations,
        );
    }

    let items_pointer = root.child("items");
    check_kind_count(&items_pointer, grammar.items.len(), &mut violations);
    for (index, item) in grammar.items.iter().enumerate().take(GRAMMAR_KINDS_MAX) {
        let pointer = items_pointer.index(index);
        check_kind_name(&pointer, &item.name, &mut names, &mut violations);
        check_icon_table(&pointer, item, &mut violations);
        top_level |= check_parents(&pointer, &item.parents, &container_names, &mut violations);
    }

    if !top_level {
        violations.push(
            root.clone(),
            GrammarRule::NoTopLevel,
            format!("no kind lists \"{PAGE_PARENT}\" among its parents"),
        );
    }

    check_remembered(&root.child("remembered"), grammar, &mut violations);
    violations.0
}

fn check_kind_count(pointer: &NodePointer, count: usize, violations: &mut Violations) {
    if count == 0 || count > GRAMMAR_KINDS_MAX {
        violations.push(
            pointer.clone(),
            GrammarRule::KindsOutOfRange,
            format!("{count} kinds, outside 1 to {GRAMMAR_KINDS_MAX}"),
        );
    }
}

fn check_kind_name<'a>(
    pointer: &NodePointer,
    name: &'a str,
    names: &mut BTreeSet<&'a str>,
    violations: &mut Violations,
) {
    let name_pointer = pointer.child("name");
    if !is_valid_kind_name(name) {
        violations.push(
            name_pointer,
            GrammarRule::NameMalformed,
            format!("kind \"{name}\" does not match {KIND_PATTERN}"),
        );
    } else if name == PAGE_PARENT {
        violations.push(
            name_pointer,
            GrammarRule::NameMalformed,
            format!("kind \"{PAGE_PARENT}\" names the top level, not a kind"),
        );
    } else if !names.insert(name) {
        violations.push(
            name_pointer,
            GrammarRule::NameDuplicate,
            format!("kind \"{name}\" is declared twice"),
        );
    }
}

fn check_container(pointer: &NodePointer, container: &ContainerKind, violations: &mut Violations) {
    let is_frame = container.role == Role::Frame;
    if is_frame && container.tone.is_some() {
        violations.push(
            pointer.child("tone"),
            GrammarRule::RoleMismatch,
            format!("frame kind {} has a tone", container.name),
        );
    }
    if !is_frame && container.tone.is_none() {
        violations.push(
            pointer.child("role"),
            GrammarRule::RoleMismatch,
            format!(
                "{} kind {} has no tone",
                role_name(container.role),
                container.name
            ),
        );
    }
    if let Some(default_tint) = container.default_tint {
        if !container.tintable {
            violations.push(
                pointer.child("default_tint"),
                GrammarRule::DefaultTintUntintable,
                format!(
                    "kind {} is not tintable but has a default tint",
                    container.name
                ),
            );
        } else if !(1..=crate::document::TINT_SLOTS).contains(&default_tint) {
            violations.push(
                pointer.child("default_tint"),
                GrammarRule::DefaultTintOutOfRange,
                format!("default tint {default_tint} is outside 1 to 8"),
            );
        }
    }
    let border = container.border;
    let width_in_range =
        border.width.is_finite() && (0.0..=BORDER_WIDTH_MAX_PX).contains(&border.width);
    if !width_in_range {
        violations.push(
            pointer.child("border").child("width"),
            GrammarRule::BoxOutOfRange,
            format!(
                "border width {} is outside 0 to {BORDER_WIDTH_MAX_PX}",
                border.width
            ),
        );
    } else if (border.pattern == BorderPattern::None) != (border.width == 0.0) {
        violations.push(
            pointer.child("border"),
            GrammarRule::BorderMismatch,
            format!(
                "border pattern {} with width {}: width is 0 exactly when the pattern is none",
                pattern_name(border.pattern),
                border.width
            ),
        );
    }
    if !(container.padding.is_finite() && (0.0..=KIND_PADDING_MAX_PX).contains(&container.padding))
    {
        violations.push(
            pointer.child("padding"),
            GrammarRule::BoxOutOfRange,
            format!(
                "padding {} is outside 0 to {KIND_PADDING_MAX_PX}",
                container.padding
            ),
        );
    }
    if !(container.radius.is_finite() && (0.0..=KIND_RADIUS_MAX_PX).contains(&container.radius)) {
        violations.push(
            pointer.child("radius"),
            GrammarRule::BoxOutOfRange,
            format!(
                "radius {} is outside 0 to {KIND_RADIUS_MAX_PX}",
                container.radius
            ),
        );
    }
    if is_frame != (container.label == LabelStyle::Bar) {
        violations.push(
            pointer.child("label"),
            GrammarRule::RoleMismatch,
            format!(
                "{} kind {} has label {}: the bar label belongs to frame kinds alone",
                role_name(container.role),
                container.name,
                label_name(container.label)
            ),
        );
    }
}

/// Reports each unknown parent; returns true when the list names the page.
fn check_parents(
    pointer: &NodePointer,
    parents: &[String],
    container_names: &BTreeSet<&str>,
    violations: &mut Violations,
) -> bool {
    let parents_pointer = pointer.child("parents");
    if parents.is_empty() || parents.len() > GRAMMAR_PARENTS_MAX {
        violations.push(
            parents_pointer.clone(),
            GrammarRule::KindsOutOfRange,
            format!(
                "{} parents, outside 1 to {GRAMMAR_PARENTS_MAX}",
                parents.len()
            ),
        );
    }
    let mut top_level = false;
    for (index, parent) in parents.iter().enumerate().take(GRAMMAR_PARENTS_MAX) {
        if parent == PAGE_PARENT {
            top_level = true;
        } else if !container_names.contains(parent.as_str()) {
            violations.push(
                parents_pointer.index(index),
                GrammarRule::ParentUnknown,
                format!("parent \"{parent}\" is neither a container kind nor \"{PAGE_PARENT}\""),
            );
        }
    }
    top_level
}

fn check_icon_table(pointer: &NodePointer, item: &ItemKind, violations: &mut Violations) {
    let products_pointer = pointer.child("products");
    if item.icons == IconPack::None && !item.products.is_empty() {
        violations.push(
            products_pointer.clone(),
            GrammarRule::IconTable,
            format!("item kind {} takes no icon but lists products", item.name),
        );
    }
    if item.products.len() > GRAMMAR_PRODUCTS_MAX {
        violations.push(
            products_pointer.clone(),
            GrammarRule::IconTable,
            format!(
                "{} product rows, above {GRAMMAR_PRODUCTS_MAX}",
                item.products.len()
            ),
        );
    }
    let mut icons: BTreeSet<&'static str> = BTreeSet::new();
    for (index, row) in item.products.iter().enumerate().take(GRAMMAR_PRODUCTS_MAX) {
        let row_pointer = products_pointer.index(index);
        if !icons.insert(row.icon.file_name()) {
            violations.push(
                row_pointer.child("icon"),
                GrammarRule::IconTable,
                format!(
                    "icon {} appears twice in item kind {}",
                    row.icon.as_str(),
                    item.name
                ),
            );
        }
        if row.names.is_empty() || row.names.len() > GRAMMAR_PRODUCT_NAMES_MAX {
            violations.push(
                row_pointer.child("names"),
                GrammarRule::IconTable,
                format!(
                    "{} names, outside 1 to {GRAMMAR_PRODUCT_NAMES_MAX}",
                    row.names.len()
                ),
            );
        }
        for (name_index, name) in row.names.iter().enumerate().take(GRAMMAR_PRODUCT_NAMES_MAX) {
            let scalars = name.chars().count();
            if scalars == 0 || scalars > 64 {
                violations.push(
                    row_pointer.child("names").index(name_index),
                    GrammarRule::IconTable,
                    format!("product name has {scalars} scalar values, outside 1 to 64"),
                );
            }
        }
    }
}

fn check_remembered(pointer: &NodePointer, grammar: &Grammar, violations: &mut Violations) {
    if grammar.remembered.len() > GRAMMAR_REMEMBERED_MAX {
        violations.push(
            pointer.clone(),
            GrammarRule::RememberedOutOfRange,
            format!(
                "{} remembered literals, above {GRAMMAR_REMEMBERED_MAX}",
                grammar.remembered.len()
            ),
        );
    }
    let mut literals: BTreeSet<&str> = BTreeSet::new();
    for (index, remembered) in grammar
        .remembered
        .iter()
        .enumerate()
        .take(GRAMMAR_REMEMBERED_MAX)
    {
        let entry_pointer = pointer.index(index);
        let literal_scalars = remembered.literal.chars().count();
        if literal_scalars == 0 || literal_scalars > 64 {
            violations.push(
                entry_pointer.child("literal"),
                GrammarRule::RememberedOutOfRange,
                format!("literal has {literal_scalars} scalar values, outside 1 to 64"),
            );
        }
        let reason_scalars = remembered.reason.chars().count();
        if reason_scalars == 0 || reason_scalars > 400 {
            violations.push(
                entry_pointer.child("reason"),
                GrammarRule::RememberedOutOfRange,
                format!("reason has {reason_scalars} scalar values, outside 1 to 400"),
            );
        }
        if !literals.insert(remembered.literal.as_str()) {
            violations.push(
                entry_pointer.child("literal"),
                GrammarRule::RememberedDuplicate,
                format!("literal \"{}\" is listed twice", remembered.literal),
            );
        }
    }
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Frame => "frame",
        Role::Boundary => "boundary",
        Role::Group => "group",
        Role::Tile => "tile",
    }
}

fn label_name(label: LabelStyle) -> &'static str {
    match label {
        LabelStyle::Plain => "plain",
        LabelStyle::Accent => "accent",
        LabelStyle::Bar => "bar",
    }
}

fn pattern_name(pattern: BorderPattern) -> &'static str {
    match pattern {
        BorderPattern::Solid => "solid",
        BorderPattern::Dashed => "dashed",
        BorderPattern::Dotted => "dotted",
        BorderPattern::None => "none",
    }
}
