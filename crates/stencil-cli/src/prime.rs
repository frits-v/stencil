//! `stencil prime`: the briefing an agent reads before it authors a figure. The prose lives
//! in `prime/*.md`; the vocabulary table is rendered from `page_schema()` at run time, so
//! tag names, field names, bounds and enum values come from the model, and each grammar
//! briefing's kind tables are rendered from the grammar's data.

use serde::Serialize;
use serde_json::{Map, Value};
use stencil_model::grammar::{BorderPattern, IconPack, LabelStyle, Role, Tone};
use stencil_model::{
    Arrow, BUILTIN_GRAMMARS, Bend, Chrome, FactSource, GAP_DEFAULT_PX, GRAMMAR_DEFAULT,
    GRAMMAR_REFERENCE_PATTERN, GROW_WEIGHT_MAX, Grammar, GrammarError, ID_PATTERN, Justify,
    KIND_PATTERN, LANE_GAP_DEFAULT_PX, LINK_CORNER_DEFAULT_PX, SheetSurface, SurfaceVocabulary,
    TEXT_SCALARS_MAX, THEME_DEFAULT, THEME_REFERENCE_PATTERN, VIEW_GAP_DEFAULT_PX, builtin_grammar,
};

const BASE_TEXT: &str = include_str!("../prime/base.md");
const VOCABULARY_MARKER: &str = "{{vocabulary}}";
const TOPICS_MARKER: &str = "{{topics}}";
const GRAMMARS_MARKER: &str = "{{grammars}}";
const KINDS_MARKER: &str = "{{kinds}}";

/// Largest `stencil prime` output, in bytes.
pub const BASE_BYTES_MAX: usize = 7000;
/// Largest `stencil prime <topic>` and `stencil prime grammar <name>` output, in bytes, for
/// every topic except `example`.
pub const TOPIC_BYTES_MAX: usize = 5000;

/// Schema definitions that get their own vocabulary row after Page and the node tags.
pub const ROW_DEFINITIONS: [&str; 3] = ["FactEntry", "LegendEntry", "Link"];
/// Structs of the design sheet that a topic prints as rows of its own field table.
pub const TOPIC_ROW_DEFINITIONS: [&str; 14] = [
    "Sheet",
    "Problem",
    "ProblemPoint",
    "Answer",
    "Attention",
    "AttentionItem",
    "Actor",
    "Phase",
    "MarkLegend",
    "PartEntry",
    "GeneralNote",
    "Rung",
    "HowToItem",
    "TerminalLine",
];
/// Definitions written inline where a field uses them, never as a row or an enum line: a
/// point as `{x,y}`, a Tee arm as its tag, a paragraph as `text or {lead,text}`, a mark as
/// `{part/note/attention}`, a marked line as `text or {text,marks}` and a command run as
/// `text or {words}`.
pub const INLINE_DEFINITIONS: [&str; 9] = [
    "PagePoint",
    "TeeArm",
    "AnswerParagraph",
    "LedParagraph",
    "Mark",
    "MarkedText",
    "MarkedLine",
    "CommandRun",
    "CommandWords",
];
/// A topic marker that fills in the vocabulary of the names it lists:
/// `{{vocabulary:View,Sheet}}`.
const TOPIC_VOCABULARY_MARKER: &str = "{{vocabulary:";

/// Upper bound on `$ref` hops while describing one field, so a self-referencing schema
/// cannot loop.
const REFERENCE_DEPTH_MAX: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    Themes,
    Links,
    Blocks,
    Layout,
    Iso,
    Checks,
    Cue,
    Sheet,
    Phases,
    Marks,
    Assembly,
    HowTo,
    Example,
}

impl Topic {
    pub const ALL: [Topic; 13] = [
        Topic::Themes,
        Topic::Links,
        Topic::Blocks,
        Topic::Layout,
        Topic::Iso,
        Topic::Checks,
        Topic::Cue,
        Topic::Sheet,
        Topic::Phases,
        Topic::Marks,
        Topic::Assembly,
        Topic::HowTo,
        Topic::Example,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Topic::Themes => "themes",
            Topic::Links => "links",
            Topic::Blocks => "blocks",
            Topic::Layout => "layout",
            Topic::Iso => "iso",
            Topic::Checks => "checks",
            Topic::Cue => "cue",
            Topic::Sheet => "sheet",
            Topic::Phases => "phases",
            Topic::Marks => "marks",
            Topic::Assembly => "assembly",
            Topic::HowTo => "howto",
            Topic::Example => "example",
        }
    }

    pub fn from_name(name: &str) -> Option<Topic> {
        Topic::ALL.into_iter().find(|topic| topic.name() == name)
    }

    /// The topic as printed: its text with every vocabulary marker filled from the schema.
    /// `example` is `examples/g7.json` verbatim.
    pub fn rendered(self) -> Result<String, PrimeError> {
        let template = self.template();
        if !template.contains(TOPIC_VOCABULARY_MARKER) {
            return Ok(template.to_string());
        }
        let schema = stencil_model::page_schema();
        let mut text = String::new();
        let mut rest = template;
        for _ in 0..template.len() {
            let Some(start) = rest.find(TOPIC_VOCABULARY_MARKER) else {
                break;
            };
            let after = rest
                .get(start + TOPIC_VOCABULARY_MARKER.len()..)
                .unwrap_or_default();
            let end = after.find("}}").ok_or(PrimeError::MissingMarker {
                marker: TOPIC_VOCABULARY_MARKER,
            })?;
            let names: Vec<&str> = after.get(..end).unwrap_or_default().split(',').collect();
            text.push_str(rest.get(..start).unwrap_or_default());
            text.push_str(&topic_vocabulary_in(
                schema.as_value(),
                &names,
                Some(self.name()),
            )?);
            rest = after.get(end + 2..).unwrap_or_default();
        }
        text.push_str(rest);
        Ok(text)
    }

    fn template(self) -> &'static str {
        match self {
            Topic::Themes => include_str!("../prime/themes.md"),
            Topic::Links => include_str!("../prime/links.md"),
            Topic::Blocks => include_str!("../prime/blocks.md"),
            Topic::Layout => include_str!("../prime/layout.md"),
            Topic::Iso => include_str!("../prime/iso.md"),
            Topic::Checks => include_str!("../prime/checks.md"),
            Topic::Cue => include_str!("../prime/cue.md"),
            Topic::Sheet => include_str!("../prime/sheet.md"),
            Topic::Phases => include_str!("../prime/phases.md"),
            Topic::Marks => include_str!("../prime/marks.md"),
            Topic::Assembly => include_str!("../prime/assembly.md"),
            Topic::HowTo => include_str!("../prime/howto.md"),
            Topic::Example => include_str!("../../../examples/g7.json"),
        }
    }

    /// Whether the text is held to TOPIC_BYTES_MAX. The example is a whole document.
    pub fn has_byte_budget(self) -> bool {
        match self {
            Topic::Themes
            | Topic::Links
            | Topic::Blocks
            | Topic::Layout
            | Topic::Iso
            | Topic::Checks
            | Topic::Cue
            | Topic::Sheet
            | Topic::Phases
            | Topic::Marks
            | Topic::Assembly
            | Topic::HowTo => true,
            Topic::Example => false,
        }
    }
}

/// `themes, links, ...`, in `Topic::ALL` order.
pub fn topic_names() -> String {
    Topic::ALL
        .iter()
        .map(|topic| topic.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The grammar briefing text of a built-in grammar, before its kind tables are filled in.
fn grammar_briefing_text(name: &str) -> Option<&'static str> {
    match name {
        "gcp" => Some(include_str!("../prime/grammars/gcp.md")),
        "plain" => Some(include_str!("../prime/grammars/plain.md")),
        _ => None,
    }
}

/// `gcp, plain`, the built-in grammar names in `BUILTIN_GRAMMARS` order.
pub fn grammar_names() -> String {
    BUILTIN_GRAMMARS.join(", ")
}

/// `stencil prime grammar <name>`: the grammar's briefing with its kind tables rendered from
/// its data. None for a name that is not a built-in grammar.
pub fn grammar_text(name: &str) -> Option<Result<String, PrimeError>> {
    let text = grammar_briefing_text(name)?;
    let grammar = match builtin_grammar(name)? {
        Ok(grammar) => grammar,
        Err(source) => return Some(Err(PrimeError::Grammar(source))),
    };
    Some(fill(text, KINDS_MARKER, &kind_tables(&grammar)))
}

/// The container kinds and item kinds of a grammar as two markdown tables.
pub fn kind_tables(grammar: &Grammar) -> String {
    let mut text = String::from(
        "| Container kind | Role | Tone | Tint | Border | Label | Parents |\n|---|---|---|---|---|---|---|\n",
    );
    for container in &grammar.containers {
        let tint = match (container.tintable, container.default_tint) {
            (true, Some(slot)) => format!("yes, default {slot}"),
            (true, None) => "yes, no default".to_string(),
            (false, _) => "no".to_string(),
        };
        text.push_str(&format!(
            "| {} | {} | {} | {tint} | {} {} | {} | {} |\n",
            container.name,
            role_name(container.role),
            container.tone.map_or("", tone_name),
            pattern_name(container.border.pattern),
            container.border.width,
            label_name(container.label),
            container.parents.join(", ")
        ));
    }
    text.push_str("\n| Item kind | Icons | Parents |\n|---|---|---|\n");
    for item in &grammar.items {
        let icons = match item.icons {
            IconPack::Gcp => "gcp",
            IconPack::None => "none",
        };
        text.push_str(&format!(
            "| {} | {icons} | {} |\n",
            item.name,
            item.parents.join(", ")
        ));
    }
    text.trim_end().to_string()
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Frame => "frame",
        Role::Boundary => "boundary",
        Role::Group => "group",
        Role::Tile => "tile",
    }
}

fn tone_name(tone: Tone) -> &'static str {
    match tone {
        Tone::Neutral => "neutral",
        Tone::Warm => "warm",
        Tone::Cool => "cool",
        Tone::Soft => "soft",
        Tone::Strong => "strong",
        Tone::Highlight => "highlight",
        Tone::Emphasis => "emphasis",
        Tone::Accent => "accent",
    }
}

fn pattern_name(pattern: BorderPattern) -> &'static str {
    match pattern {
        BorderPattern::Solid => "solid",
        BorderPattern::Dashed => "dashed",
        BorderPattern::Dotted => "dotted",
        BorderPattern::DashDot => "dash-dot",
        BorderPattern::None => "none",
    }
}

fn label_name(label: LabelStyle) -> &'static str {
    match label {
        LabelStyle::Plain => "plain",
        LabelStyle::Accent => "accent",
        LabelStyle::Bar => "bar",
        LabelStyle::Tab => "tab",
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrimeError {
    #[error(transparent)]
    Grammar(#[from] GrammarError),
    #[error("the document schema at {path} has a shape the vocabulary table does not describe")]
    SchemaShape { path: String },
    #[error("the prime text has no {marker} marker")]
    MissingMarker { marker: &'static str },
    #[error("cannot serialize the default {what}")]
    Serialize {
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },
}

/// The base briefing: `prime/base.md` with the vocabulary and topic list filled in.
pub fn base_text() -> Result<String, PrimeError> {
    let schema = stencil_model::page_schema();
    let vocabulary = vocabulary(schema.as_value())?;
    let topics = format!("Topics: `stencil prime <topic>` with {}.", topic_names());
    let grammars = format!(
        "Grammars: {}. `stencil prime grammar <name>` prints a grammar's kinds and rules.",
        grammar_names()
    );
    fill(BASE_TEXT, VOCABULARY_MARKER, &vocabulary)
        .and_then(|text| fill(&text, TOPICS_MARKER, &topics))
        .and_then(|text| fill(&text, GRAMMARS_MARKER, &grammars))
}

fn fill(text: &str, marker: &'static str, value: &str) -> Result<String, PrimeError> {
    if text.contains(marker) {
        Ok(text.replace(marker, value))
    } else {
        Err(PrimeError::MissingMarker { marker })
    }
}

/// A fact about one field that the JSON Schema does not carry: a serde default or a vet
/// bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldNote {
    pub object: &'static str,
    pub field: &'static str,
    pub note: String,
}

/// Every note is keyed by an object and field that exist in the schema; a test holds it.
pub fn field_notes() -> Result<Vec<FieldNote>, PrimeError> {
    let theme = THEME_DEFAULT;
    let justify = serialized_name("justify", Justify::Start)?;
    let pipe_arrow = serialized_name("pipe arrow", Arrow::None)?;
    let chrome = serialized_name("chrome", Chrome::default())?;
    let bend = serialized_name("bend", Bend::default())?;
    let source = serialized_name("fact source", FactSource::default())?;
    let note = |object, field, note: String| FieldNote {
        object,
        field,
        note,
    };
    let mut notes = vec![
        note(
            "Page",
            "grammar",
            format!("={GRAMMAR_DEFAULT}; a built-in name or a .json path"),
        ),
        note(
            "Page",
            "theme",
            format!("={theme}; a built-in name or a .json path"),
        ),
        note("Page", "chrome", format!("={chrome}")),
        note(
            "Page",
            "badge",
            "=none; the pill before the kicker".to_string(),
        ),
        note("Page", "corner", format!("={LINK_CORNER_DEFAULT_PX}")),
        note("Page", "bend", format!("={bend}")),
        note("Pipe", "arrow", format!("={pipe_arrow}")),
        note("Tee", "arms", "dir h".to_string()),
        note("Fact", "source", format!("={source}")),
        note("FactEntry", "source", format!("={source}")),
        note("Lanes", "gap", format!("={LANE_GAP_DEFAULT_PX}")),
        note("View", "gap", format!("={VIEW_GAP_DEFAULT_PX}")),
        note("Problem", "caption", "=Problem".to_string()),
        note("Answer", "caption", "=Answer".to_string()),
        note("Attention", "caption", "=Attention".to_string()),
    ];

    for container in ["Row", "Col"] {
        notes.push(note(container, "gap", format!("={GAP_DEFAULT_PX}")));
        notes.push(note(
            container,
            "grow",
            format!("0-{GROW_WEIGHT_MAX} per child"),
        ));
        notes.push(note(container, "justify", format!("={justify}")));
    }
    Ok(notes)
}

fn serialized_name(what: &'static str, value: impl Serialize) -> Result<String, PrimeError> {
    let serialized =
        serde_json::to_value(value).map_err(|source| PrimeError::Serialize { what, source })?;
    match serialized {
        Value::String(name) => Ok(name),
        _ => Err(PrimeError::SchemaShape {
            path: format!("default {what}"),
        }),
    }
}

/// One object of the vocabulary: its name and its schema object.
struct ObjectSchema<'a> {
    name: String,
    schema: &'a Value,
}

/// Every schema name the design sheet surfaces move out of the base briefing.
fn sheet_vocabulary() -> Vec<SurfaceVocabulary> {
    SheetSurface::ALL
        .iter()
        .flat_map(|surface| surface.vocabulary().iter().copied())
        .collect()
}

/// The fields of `object` that only a design sheet uses.
fn sheet_fields(object: &str) -> Vec<&'static str> {
    sheet_fields_in(object, None)
}

/// The fields of `object` that only a design sheet uses, of the surfaces `topic` names when
/// it is given.
fn sheet_fields_in(object: &str, topic: Option<&str>) -> Vec<&'static str> {
    SheetSurface::ALL
        .iter()
        .filter(|surface| topic.is_none_or(|topic| surface.topic() == topic))
        .flat_map(|surface| surface.vocabulary().iter().copied())
        .filter_map(|item| match item {
            SurfaceVocabulary::Field {
                object: owner,
                field,
            } if owner == object => Some(field),
            SurfaceVocabulary::Field { .. }
            | SurfaceVocabulary::Tag(_)
            | SurfaceVocabulary::Value { .. }
            | SurfaceVocabulary::Definition(_) => None,
        })
        .collect()
}

/// The vocabulary table followed by one line per enumeration, without the tags, fields,
/// values and definitions only a design sheet uses (section 14.8 rule 3).
pub fn vocabulary(schema: &Value) -> Result<String, PrimeError> {
    let definitions = schema
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or_else(|| shape_error("/$defs"))?;
    let notes = field_notes()?;
    let surface_items = sheet_vocabulary();
    let sheet_definition = |name: &str| {
        surface_items.iter().any(|item| match item {
            SurfaceVocabulary::Definition(definition) => *definition == name,
            SurfaceVocabulary::Tag(_)
            | SurfaceVocabulary::Field { .. }
            | SurfaceVocabulary::Value { .. } => false,
        })
    };
    let sheet_value = |definition_name: &str, value: &str| {
        surface_items.iter().any(|item| match item {
            SurfaceVocabulary::Value {
                definition,
                value: listed,
            } => *definition == definition_name && *listed == value,
            SurfaceVocabulary::Tag(_)
            | SurfaceVocabulary::Field { .. }
            | SurfaceVocabulary::Definition(_) => false,
        })
    };

    let mut objects = vec![ObjectSchema {
        name: schema
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| shape_error("/title"))?
            .to_string(),
        schema,
    }];
    let node_variants = definitions
        .get("Node")
        .and_then(|node| node.get("oneOf"))
        .and_then(Value::as_array)
        .ok_or_else(|| shape_error("/$defs/Node/oneOf"))?;
    for (index, variant) in node_variants.iter().enumerate() {
        let tag = tag_constant(variant)
            .ok_or_else(|| shape_error(&format!("/$defs/Node/oneOf/{index}")))?;
        if is_sheet_tag(tag) {
            continue;
        }
        objects.push(ObjectSchema {
            name: tag.to_string(),
            schema: variant,
        });
    }
    for name in ROW_DEFINITIONS {
        let object = definitions
            .get(name)
            .ok_or_else(|| shape_error(&format!("/$defs/{name}")))?;
        objects.push(ObjectSchema {
            name: name.to_string(),
            schema: object,
        });
    }

    let mut rows: Vec<(Vec<String>, String)> = Vec::new();
    for object in &objects {
        let fields = describe_object(
            object,
            definitions,
            &notes,
            FieldFilter::Without(&sheet_fields(&object.name)),
        )?;
        match rows.iter_mut().find(|(_, existing)| *existing == fields) {
            Some((names, _)) => names.push(object.name.clone()),
            None => rows.push((vec![object.name.clone()], fields)),
        }
    }

    let mut text = String::from("| Tag | Fields |\n|---|---|\n");
    for (names, fields) in &rows {
        text.push_str(&format!("| {} | {fields} |\n", names.join(", ")));
    }
    text.push('\n');
    for (name, definition) in definitions {
        if sheet_definition(name) {
            continue;
        }
        if let Some(values) = definition.get("enum").and_then(Value::as_array) {
            let values = enum_values(name, values)?
                .into_iter()
                .filter(|value| !sheet_value(name, value))
                .collect::<Vec<_>>();
            text.push_str(&format!(
                "{name} ({}): {}\n",
                values.len(),
                values.join(" ")
            ));
        }
    }
    Ok(text.trim_end().to_string())
}

/// The values of an enum definition as strings.
fn enum_values<'a>(name: &str, values: &'a [Value]) -> Result<Vec<&'a str>, PrimeError> {
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| shape_error(&format!("/$defs/{name}/enum")))
        })
        .collect()
}

/// True for a node tag only a design sheet uses.
fn is_sheet_tag(tag: &str) -> bool {
    sheet_vocabulary()
        .iter()
        .any(|item| matches!(item, SurfaceVocabulary::Tag(name) if *name == tag))
}

/// A topic's vocabulary for `names`: a `| Tag | Fields |` table with a row per sheet tag
/// or `TOPIC_ROW_DEFINITIONS` entry, a `| Tag | Sheet fields |` table with the sheet
/// fields of each base row named, then one line per enum definition named. An unknown name
/// is a schema shape error.
pub fn topic_vocabulary(schema: &Value, names: &[&str]) -> Result<String, PrimeError> {
    topic_vocabulary_in(schema, names, None)
}

/// `topic_vocabulary` with the sheet fields of each base row limited to the surfaces
/// `topic` names, so each topic lists the fields it explains.
pub fn topic_vocabulary_in(
    schema: &Value,
    names: &[&str],
    topic: Option<&str>,
) -> Result<String, PrimeError> {
    let definitions = schema
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or_else(|| shape_error("/$defs"))?;
    let notes = field_notes()?;
    let node_variants = definitions
        .get("Node")
        .and_then(|node| node.get("oneOf"))
        .and_then(Value::as_array)
        .ok_or_else(|| shape_error("/$defs/Node/oneOf"))?;
    let base_object = |name: &str| -> Option<&Value> {
        if name == "Page" {
            return Some(schema);
        }
        node_variants
            .iter()
            .find(|variant| tag_constant(variant) == Some(name))
            .or_else(|| {
                ROW_DEFINITIONS
                    .contains(&name)
                    .then(|| definitions.get(name))
                    .flatten()
            })
    };
    let mut rows = Vec::new();
    let mut sheet_field_rows = Vec::new();
    let mut enum_lines = Vec::new();
    for &name in names {
        let object = if is_sheet_tag(name) {
            base_object(name)
        } else if TOPIC_ROW_DEFINITIONS.contains(&name) {
            definitions.get(name)
        } else {
            None
        };
        if let Some(object) = object {
            let fields = describe_object(
                &ObjectSchema {
                    name: name.to_string(),
                    schema: object,
                },
                definitions,
                &notes,
                FieldFilter::All,
            )?;
            rows.push(format!("| {name} | {fields} |"));
            continue;
        }
        if let Some(values) = definitions
            .get(name)
            .and_then(|definition| definition.get("enum"))
            .and_then(Value::as_array)
        {
            let values = enum_values(name, values)?;
            enum_lines.push(format!("{name} ({}): {}", values.len(), values.join(" ")));
            continue;
        }
        let fields = sheet_fields_in(name, topic);
        match base_object(name) {
            Some(object) if !fields.is_empty() => {
                let described = describe_object(
                    &ObjectSchema {
                        name: name.to_string(),
                        schema: object,
                    },
                    definitions,
                    &notes,
                    FieldFilter::Only(&fields),
                )?;
                sheet_field_rows.push(format!("| {name} | {described} |"));
            }
            _ => return Err(shape_error(&format!("topic vocabulary {name}"))),
        }
    }
    let mut blocks = Vec::new();
    if !rows.is_empty() {
        blocks.push(format!("| Tag | Fields |\n|---|---|\n{}", rows.join("\n")));
    }
    if !sheet_field_rows.is_empty() {
        blocks.push(format!(
            "| Tag | Sheet fields |\n|---|---|\n{}",
            sheet_field_rows.join("\n")
        ));
    }
    if !enum_lines.is_empty() {
        blocks.push(enum_lines.join("\n"));
    }
    Ok(blocks.join("\n\n"))
}

/// Which fields of an object a row describes.
#[derive(Debug, Clone, Copy)]
enum FieldFilter<'a> {
    All,
    Without(&'a [&'static str]),
    Only(&'a [&'static str]),
}

impl FieldFilter<'_> {
    fn keeps(self, field: &str) -> bool {
        match self {
            FieldFilter::All => true,
            FieldFilter::Without(left_out) => !left_out.contains(&field),
            FieldFilter::Only(kept) => kept.contains(&field),
        }
    }
}

fn shape_error(path: &str) -> PrimeError {
    PrimeError::SchemaShape {
        path: path.to_string(),
    }
}

/// The `const` of a node variant's `tag` property.
fn tag_constant(variant: &Value) -> Option<&str> {
    variant
        .get("properties")
        .and_then(|properties| properties.get("tag"))
        .and_then(|tag| tag.get("const"))
        .and_then(Value::as_str)
}

/// `name*` for a required field, then its type, bounds, default and note. Required fields
/// come first in declaration order, then the optional ones in schema order. `tag` is
/// implied by the row and `id` is described once above the table.
fn describe_object(
    object: &ObjectSchema,
    definitions: &Map<String, Value>,
    notes: &[FieldNote],
    filter: FieldFilter<'_>,
) -> Result<String, PrimeError> {
    let properties = object
        .schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| shape_error(&format!("{}/properties", object.name)))?;
    let required: Vec<&str> = object
        .schema
        .get("required")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut field_order: Vec<(&str, bool)> = required.iter().map(|name| (*name, true)).collect();
    for name in properties.keys() {
        if !required.contains(&name.as_str()) {
            field_order.push((name.as_str(), false));
        }
    }

    let mut fields = Vec::new();
    for (name, is_required) in field_order {
        if name == "tag" || !filter.keeps(name) {
            continue;
        }
        let property = properties
            .get(name)
            .ok_or_else(|| shape_error(&format!("{}/properties/{name}", object.name)))?;
        // An optional node id is described once above the table; a keyed entry's id is
        // required and stays in its row.
        if name == "id" && !is_required && is_id_property(property) {
            continue;
        }
        let path = format!("{}/{name}", object.name);
        let mut field = name.to_string();
        if is_required {
            field.push('*');
        }
        let described = describe_property(property, definitions, &path, 0)?;
        if !described.is_empty() {
            field.push(' ');
            field.push_str(&described);
        }
        if let Some(default) = property.get("default") {
            field.push_str(&format!(" ={}", plain_value(default)));
        }
        for note in notes
            .iter()
            .filter(|note| note.object == object.name && note.field == name)
        {
            field.push(' ');
            field.push_str(&note.note);
        }
        fields.push(field);
    }
    Ok(fields.join(", "))
}

fn is_id_property(property: &Value) -> bool {
    property.get("pattern").and_then(Value::as_str) == Some(ID_PATTERN)
}

fn plain_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The type column of one field. A text field (1 to TEXT_SCALARS_MAX scalars) is written as
/// nothing, since most fields are text; the header says so.
fn describe_property(
    property: &Value,
    definitions: &Map<String, Value>,
    path: &str,
    depth: usize,
) -> Result<String, PrimeError> {
    if depth > REFERENCE_DEPTH_MAX {
        return Err(shape_error(path));
    }
    if let Some(reference) = property.get("$ref").and_then(Value::as_str) {
        return describe_reference(reference, definitions, path);
    }
    if let Some(alternatives) = property.get("anyOf").and_then(Value::as_array) {
        let non_null: Vec<&Value> = alternatives
            .iter()
            .filter(|alternative| alternative.get("type").and_then(Value::as_str) != Some("null"))
            .collect();
        return match non_null.as_slice() {
            [only] => describe_property(only, definitions, path, depth + 1),
            _ => Err(shape_error(path)),
        };
    }
    match primary_type(property) {
        Some("string") => Ok(describe_string(property)),
        Some("integer") => Ok(describe_integer(property)),
        Some("number") => Ok(describe_number(property)),
        Some("boolean") => Ok("bool".to_string()),
        Some("object") if property.get("additionalProperties") == Some(&Value::Bool(true)) => {
            Ok("object".to_string())
        }
        Some("array") => {
            let items = property
                .get("items")
                .ok_or_else(|| shape_error(&format!("{path}/items")))?;
            let item = describe_property(items, definitions, path, depth + 1)?;
            let item = if item.is_empty() {
                "text".to_string()
            } else {
                item
            };
            let minimum = property.get("minItems").and_then(Value::as_u64);
            let maximum = property.get("maxItems").and_then(Value::as_u64);
            let bounds = match (minimum, maximum) {
                (Some(low), Some(high)) if low == high => format!("[{low}]"),
                (Some(low), Some(high)) => format!("[{low}-{high}]"),
                (None, Some(high)) => format!("[0-{high}]"),
                (Some(low), None) => format!("[{low}+]"),
                (None, None) => "[]".to_string(),
            };
            Ok(format!("{item}{bounds}"))
        }
        _ => Err(shape_error(path)),
    }
}

/// The first non-null `type`, whether `type` is a string or a list.
fn primary_type(property: &Value) -> Option<&str> {
    match property.get("type")? {
        Value::String(name) => Some(name.as_str()),
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .find(|name| *name != "null"),
        _ => None,
    }
}

fn describe_string(property: &Value) -> String {
    let minimum = property.get("minLength").and_then(Value::as_u64);
    let maximum = property.get("maxLength").and_then(Value::as_u64);
    let text_maximum = u64::try_from(TEXT_SCALARS_MAX).ok();
    let pattern = property.get("pattern").and_then(Value::as_str);
    if minimum == Some(1) && maximum == text_maximum {
        String::new()
    } else if let (Some(1), Some(maximum), None) = (minimum, maximum, pattern) {
        format!("text 1-{maximum}")
    } else if pattern == Some(KIND_PATTERN) {
        "kind".to_string()
    } else if pattern == Some(GRAMMAR_REFERENCE_PATTERN) || pattern == Some(THEME_REFERENCE_PATTERN)
    {
        "ref".to_string()
    } else {
        // The id pattern and the link endpoints are the only other strings in the model.
        "id".to_string()
    }
}

/// `low-high`, leaving out bounds that only restate the integer format (0 to 65535 for
/// uint16), which schemars adds for types the vet rules bound more tightly.
fn describe_integer(property: &Value) -> String {
    let format_maximum = match property.get("format").and_then(Value::as_str) {
        Some("uint8") => Some(u64::from(u8::MAX)),
        Some("uint16") => Some(u64::from(u16::MAX)),
        Some("uint32") => Some(u64::from(u32::MAX)),
        _ => None,
    };
    let minimum = property.get("minimum").and_then(Value::as_u64);
    let maximum = property.get("maximum").and_then(Value::as_u64);
    match (minimum, maximum) {
        (_, Some(high)) if Some(high) == format_maximum => "int".to_string(),
        (Some(low), Some(high)) => format!("{low}-{high}"),
        _ => "int".to_string(),
    }
}

/// `low-high` when both bounds are set, else `number`.
fn describe_number(property: &Value) -> String {
    let minimum = property.get("minimum").and_then(Value::as_f64);
    let maximum = property.get("maximum").and_then(Value::as_f64);
    match (minimum, maximum) {
        (Some(low), Some(high)) => format!("{low}-{high}"),
        _ => "number".to_string(),
    }
}

/// An enumeration, `Node` or an object with its own table row by name, a tagged one-variant
/// union by its tag (a Tee arm is a `Pipe`), and any other object by its field names.
fn describe_reference(
    reference: &str,
    definitions: &Map<String, Value>,
    path: &str,
) -> Result<String, PrimeError> {
    let name = reference
        .strip_prefix("#/$defs/")
        .ok_or_else(|| shape_error(path))?;
    let definition = definitions.get(name).ok_or_else(|| shape_error(path))?;
    if definition.get("enum").is_some()
        || name == "Node"
        || ROW_DEFINITIONS.contains(&name)
        || TOPIC_ROW_DEFINITIONS.contains(&name)
    {
        return Ok(name.to_string());
    }
    if let Some(alternatives) = definition.get("anyOf").and_then(Value::as_array) {
        return text_or_object(alternatives, definitions, path);
    }
    if let Some(variants) = definition.get("oneOf").and_then(Value::as_array) {
        return match variants.as_slice() {
            [only] => tag_constant(only)
                .map(str::to_string)
                .ok_or_else(|| shape_error(path)),
            several => single_keys(several)
                .map(|keys| format!("{{{}}}", keys.join("/")))
                .ok_or_else(|| shape_error(path)),
        };
    }
    if let Some(properties) = definition.get("properties").and_then(Value::as_object) {
        let names: Vec<&str> = properties.keys().map(String::as_str).collect();
        return Ok(format!("{{{}}}", names.join(",")));
    }
    Err(shape_error(path))
}

/// The one property of each variant, when every variant is an object of one property: a
/// mark, written `{part/note/attention}`.
fn single_keys(variants: &[Value]) -> Option<Vec<&str>> {
    variants
        .iter()
        .map(|variant| {
            let properties = variant.get("properties")?.as_object()?;
            match properties.keys().collect::<Vec<_>>().as_slice() {
                [only] => Some(only.as_str()),
                _ => None,
            }
        })
        .collect()
}

/// A string or an object, written `text or {<fields>}`: `text or {lead,text}`.
fn text_or_object(
    alternatives: &[Value],
    definitions: &Map<String, Value>,
    path: &str,
) -> Result<String, PrimeError> {
    let [first, second] = alternatives else {
        return Err(shape_error(path));
    };
    if primary_type(first) != Some("string") {
        return Err(shape_error(path));
    }
    let object = match second.get("$ref").and_then(Value::as_str) {
        Some(reference) => reference
            .strip_prefix("#/$defs/")
            .and_then(|name| definitions.get(name))
            .ok_or_else(|| shape_error(path))?,
        None => second,
    };
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| shape_error(path))?;
    let names: Vec<&str> = properties.keys().map(String::as_str).collect();
    Ok(format!("text or {{{}}}", names.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn topic_names_round_trip() {
        for topic in Topic::ALL {
            assert_eq!(Topic::from_name(topic.name()), Some(topic));
        }
        assert_eq!(Topic::from_name("Themes"), None);
        assert_eq!(Topic::from_name(""), None);
    }

    #[test]
    fn a_text_without_its_marker_is_an_error() {
        let filled = fill("no marker here", VOCABULARY_MARKER, "table");
        assert!(matches!(
            filled,
            Err(PrimeError::MissingMarker {
                marker: VOCABULARY_MARKER
            })
        ));
        assert_eq!(
            fill("a {{vocabulary}} b", VOCABULARY_MARKER, "table").unwrap(),
            "a table b"
        );
    }

    #[test]
    fn a_schema_without_definitions_is_a_shape_error() {
        let error = vocabulary(&json!({ "title": "Page" })).unwrap_err();
        assert!(
            matches!(&error, PrimeError::SchemaShape { path } if path == "/$defs"),
            "{error}"
        );
    }

    #[test]
    fn a_field_of_an_unknown_type_is_a_shape_error() {
        let mut schema = stencil_model::page_schema().as_value().clone();
        schema["properties"]["title"] = json!({ "type": "null" });
        let error = vocabulary(&schema).unwrap_err();
        assert!(
            matches!(&error, PrimeError::SchemaShape { path } if path == "Page/title"),
            "{error}"
        );
    }

    #[test]
    fn a_boolean_field_reads_bool() {
        let definitions = Map::new();
        let described =
            describe_property(&json!({ "type": "boolean" }), &definitions, "Entry/flag", 0)
                .unwrap();
        assert_eq!(described, "bool");
    }

    #[test]
    fn a_reference_to_a_missing_definition_is_a_shape_error() {
        let mut schema = stencil_model::page_schema().as_value().clone();
        schema["properties"]["chrome"] = json!({ "$ref": "#/$defs/Missing" });
        assert!(matches!(
            vocabulary(&schema),
            Err(PrimeError::SchemaShape { .. })
        ));
    }

    #[test]
    fn integer_bounds_that_restate_the_format_are_left_out() {
        let format_bounds =
            json!({ "type": "integer", "format": "uint16", "minimum": 0, "maximum": 65535 });
        assert_eq!(describe_integer(&format_bounds), "int");
        let vet_bounds =
            json!({ "type": "integer", "format": "uint16", "minimum": 0, "maximum": 64 });
        assert_eq!(describe_integer(&vet_bounds), "0-64");
    }

    #[test]
    fn a_choice_of_one_key_objects_is_written_with_its_keys() {
        let schema = stencil_model::page_schema();
        let definitions = schema.as_value()["$defs"].as_object().unwrap();
        let described = describe_reference("#/$defs/Mark", definitions, "Item/marks").unwrap();
        assert_eq!(described, "{part/note/attention}");
        let marked = describe_reference("#/$defs/MarkedText", definitions, "Step/lines").unwrap();
        assert_eq!(marked, "text or {marks,text}");
    }

    #[test]
    fn a_string_or_an_object_is_written_text_or_its_fields() {
        let schema = stencil_model::page_schema();
        let definitions = schema.as_value()["$defs"].as_object().unwrap();
        let described =
            describe_reference("#/$defs/AnswerParagraph", definitions, "Answer/paragraphs")
                .unwrap();
        assert_eq!(described, "text or {lead,text}");
        let not_text = [json!({ "type": "integer" }), json!({ "type": "object" })];
        assert!(text_or_object(&not_text, definitions, "x").is_err());
    }

    #[test]
    fn an_unknown_topic_vocabulary_name_is_a_shape_error() {
        let schema = stencil_model::page_schema();
        assert!(matches!(
            topic_vocabulary(schema.as_value(), &["Nowhere"]),
            Err(PrimeError::SchemaShape { .. })
        ));
    }

    #[test]
    fn rows_with_the_same_fields_are_merged() {
        let text = vocabulary(stencil_model::page_schema().as_value()).unwrap();
        assert!(text.contains("\n| Row, Col | "), "{text}");
    }
}
