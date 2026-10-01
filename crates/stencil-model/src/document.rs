use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grammar::ContainerKind;

pub const PAGE_WIDTH_DEFAULT: u32 = 1280;
pub const PAGE_WIDTH_MIN: u32 = 640;
pub const PAGE_WIDTH_MAX: u32 = 2560;
pub const GAP_DEFAULT_PX: u16 = 8;
pub const GAP_MAX_PX: u16 = 64;
pub const GROW_WEIGHT_MAX: u16 = 100;
pub const TEXT_SCALARS_MAX: usize = 400;
pub const CHILDREN_MAX: usize = 256;
pub const LEGEND_ENTRIES_MAX: usize = 16;
pub const DEPTH_MAX: usize = 24;
pub const NODES_MAX: usize = 4096;
pub const LINKS_MAX: usize = 256;
pub const LINK_VIA_MAX: usize = 8;
pub const LINK_SEGMENTS_MAX: usize = 12;
pub const ROUTER_GRID_LINES_MAX: usize = 512;
pub const TEXT_BODY_LINES_MAX: usize = 64;
pub const FRAME_HEIGHT_DEFAULT: u16 = 200;
pub const FRAME_HEIGHT_MIN: u16 = 40;
pub const FRAME_HEIGHT_MAX: u16 = 1200;
/// Scalar values in an `id`, the `{0,63}` repetition plus the first character.
pub const ID_SCALARS_MAX: usize = 64;
/// The JSON Schema and CUE pattern for `id`. `is_valid_id` is its Rust twin.
pub const ID_PATTERN: &str = r"^[a-z0-9][a-z0-9-]{0,63}$";
pub const TINT_SLOTS: u8 = 8;
/// The slot names of the built-in center theme, in slot order. Layout measures every
/// legend label with these names (section 13.1 rule 7), so geometry never depends on the
/// theme.
pub const TINT_NAMES: [&str; 8] = [
    "blue", "pink", "teal", "amber", "violet", "green", "orange", "cyan",
];
pub const FACTS_MAX: usize = 8;
pub const LANES_MAX: usize = 32;
pub const LANE_GAP_DEFAULT_PX: u16 = 32;
/// Smallest height of one message row in a Lanes band (section 13.6).
pub const LANE_ROW_MIN_PX: f32 = 36.0;
pub const LINK_ORDER_MAX: u16 = 256;
/// A grammar kind name: a Box or Item `kind`.
pub const KIND_PATTERN: &str = r"^[a-z][a-z0-9-]{0,31}$";
pub const BUILTIN_GRAMMARS: [&str; 2] = ["gcp", "plain"];
pub const GRAMMAR_DEFAULT: &str = "gcp";
pub const GRAMMAR_REFERENCE_PATTERN: &str = r"^(gcp|plain|[^\u0000-\u001F]{1,395}\.json)$";
/// Designed built-ins first (section 13.5), then the imported tier (section 13.9).
pub const BUILTIN_THEMES: [&str; 13] = [
    "center",
    "paper",
    "dusk",
    "clear",
    "clear-dark",
    "wire",
    "tokyo-night",
    "solarized-light",
    "solarized-dark",
    "material-dark",
    "gruvbox-dark",
    "dracula",
    "nord",
];
/// The designed built-in themes, in the gallery's order (section 13.5).
pub const DESIGNED_THEMES: [&str; 6] = ["center", "paper", "dusk", "clear", "clear-dark", "wire"];
/// The imported tier, mapped from pinned base16 schemes (section 13.9).
pub const IMPORTED_THEMES: [&str; 7] = [
    "tokyo-night",
    "solarized-light",
    "solarized-dark",
    "material-dark",
    "gruvbox-dark",
    "dracula",
    "nord",
];
pub const THEME_DEFAULT: &str = "center";
pub const THEME_REFERENCE_PATTERN: &str = r"^(center|paper|dusk|clear|clear-dark|wire|tokyo-night|solarized-light|solarized-dark|material-dark|gruvbox-dark|dracula|nord|[^\u0000-\u001F]{1,395}\.json)$";
/// Largest grammar or theme file the CLI reads, in bytes.
pub const DATA_FILE_BYTES_MAX: usize = 65_536;

fn page_width_default() -> u32 {
    PAGE_WIDTH_DEFAULT
}

fn frame_height_default() -> u16 {
    FRAME_HEIGHT_DEFAULT
}

fn pipe_arrow_default() -> Arrow {
    Arrow::None
}

fn is_pipe_arrow_default(arrow: &Arrow) -> bool {
    *arrow == Arrow::None
}

fn is_default_projection(projection: &Projection) -> bool {
    *projection == Projection::Flat
}

fn is_default_chrome(chrome: &Chrome) -> bool {
    *chrome == Chrome::Full
}

fn is_default_fact_source(source: &FactSource) -> bool {
    *source == FactSource::Doc
}

/// True when `id` matches ID_PATTERN: 1 to 64 characters, lowercase ASCII letters, digits
/// and `-`, not starting with `-`.
pub fn is_valid_id(id: &str) -> bool {
    let mut characters = id.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let is_id_character = |character: char| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    };
    first != '-'
        && is_id_character(first)
        && id.len() <= ID_SCALARS_MAX
        && characters.all(is_id_character)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[schemars(length(min = 1, max = 400))]
    pub title: String,
    #[schemars(length(min = 1, max = 400))]
    pub kicker: String,
    #[schemars(length(min = 1, max = 400))]
    pub lede: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub foot: Option<String>,
    #[serde(default = "page_width_default")]
    #[schemars(range(min = 640, max = 2560))]
    pub width: u32,
    pub canvas: Canvas,
    /// A built-in grammar name or a path ending in `.json`; absent means gcp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = GRAMMAR_REFERENCE_PATTERN))]
    pub grammar: Option<String>,
    /// A built-in theme name or a path ending in `.json`; absent means center.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = THEME_REFERENCE_PATTERN))]
    pub theme: Option<String>,
    /// A partial theme merged onto the resolved theme (section 13.4 rule 8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_overrides: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "is_default_projection")]
    pub projection: Projection,
    #[serde(default, skip_serializing_if = "is_default_chrome")]
    pub chrome: Chrome,
    #[schemars(length(min = 1, max = 256))]
    pub body: Vec<Node>,
    #[schemars(length(max = 16))]
    pub legend: Vec<LegendEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 256))]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Canvas {
    Customer,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Projection {
    #[default]
    Flat,
    Iso,
}

/// Whether layout draws the badge, kicker, title and lede (section 13.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Chrome {
    #[default]
    Full,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegendEntry {
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tag")]
pub enum Node {
    Row(Row),
    Col(Col),
    Lanes(Lanes),
    Box(BoxNode),
    Item(Item),
    Fact(Fact),
    Note(Note),
    Pipe(Pipe),
    Tee(Tee),
    Text(Text),
    Callout(Callout),
    Frame(Frame),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Row {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 64))]
    pub gap: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grow: Option<Vec<u16>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Col {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 64))]
    pub gap: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grow: Option<Vec<u16>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

/// Lane heads that share one time axis (section 13.6). Each child is a lane head; a Link
/// with `order` between two heads of one Lanes node is a message, drawn as a horizontal
/// arrow in the band below the heads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lanes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 64))]
    pub gap: Option<u16>,
    /// The lane heads, left to right: 1 to 32 nodes.
    #[schemars(length(min = 1, max = 32))]
    pub children: Vec<Node>,
}

/// Any container. `kind` names one of the grammar's container kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoxNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

/// Any named leaf. `kind` names one of the grammar's item kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Item {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(regex(pattern = KIND_PATTERN))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconName>,
    #[schemars(length(min = 1, max = 400))]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 8))]
    pub facts: Vec<FactEntry>,
}

/// Where a fact comes from (section 13.7): `doc` is read from the live documentation when
/// the figure was authored, `built` is an as-built name read off the running system (a
/// bucket, a VLAN ID, a project id), and `ask` is an open question for the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum FactSource {
    #[default]
    Doc,
    Built,
    Ask,
}

/// A fact inside an Item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactEntry {
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
    #[serde(default, skip_serializing_if = "is_default_fact_source")]
    pub source: FactSource,
}

/// The Fact node: a fact standing on its own in a container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
    #[serde(default, skip_serializing_if = "is_default_fact_source")]
    pub source: FactSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NoteKind {
    Kicker,
    H1,
    Lede,
    Legend,
    Foot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Note {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    pub kind: NoteKind,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum PipeDir {
    #[serde(rename = "h")]
    Horizontal,
    #[serde(rename = "v")]
    Vertical,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Line {
    Gray,
    Solid,
    Dash,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pipe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    pub dir: PipeDir,
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub sub: Option<String>,
    #[serde(
        default = "pipe_arrow_default",
        skip_serializing_if = "is_pipe_arrow_default"
    )]
    pub arrow: Arrow,
    /// The id of the node on the pipe's left (h) or upper (v) end. Layout centers the pipe's
    /// slot on its targets (section 13.8); a Tee arm cannot carry one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub from: Option<String>,
    /// The id of the node on the pipe's right (h) or lower (v) end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub to: Option<String>,
}

/// A Tee arm is written with `"tag": "Pipe"`, the same object shape as a Pipe node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tag")]
pub enum TeeArm {
    Pipe(Pipe),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tee {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[schemars(length(min = 1, max = 400))]
    pub hub: String,
    pub arms: [TeeArm; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Text {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub heading: Option<String>,
    #[schemars(length(min = 1, max = 64), inner(length(min = 1, max = 400)))]
    pub body: Vec<String>,
    #[serde(default)]
    pub list: ListKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ListKind {
    #[default]
    Plain,
    Numbered,
    Bulleted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Callout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    pub kind: CalloutKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub title: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CalloutKind {
    Note,
    Risk,
    Decision,
    Open,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[serde(default = "frame_height_default")]
    #[schemars(range(min = 40, max = 1200))]
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub line: Line,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 8))]
    pub tint: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub sub: Option<String>,
    #[serde(default)]
    pub arrow: Arrow,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_side: Option<Side>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_side: Option<Side>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 8))]
    pub via: Vec<PagePoint>,
    /// The message position along the time axis when the link joins two lane heads of one
    /// Lanes node (section 13.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 256))]
    pub order: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Arrow {
    None,
    #[default]
    End,
    Start,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PagePoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum IconName {
    Agents,
    AiMl,
    Bigquery,
    CloudRunFlat,
    CloudRun,
    CloudSql,
    CloudStorage,
    ComputeEngine,
    Compute,
    Containers,
    DataAnalytics,
    Databases,
    Devops,
    Gke,
    Hybrid,
    Integration,
    Networking,
    Observability,
    Scc,
    SecurityIdentity,
    Serverless,
    Storage,
    VertexAi,
}

impl Node {
    /// The serialized `tag` value.
    pub fn tag_name(&self) -> &'static str {
        match self {
            Node::Row(_) => "Row",
            Node::Col(_) => "Col",
            Node::Lanes(_) => "Lanes",
            Node::Box(_) => "Box",
            Node::Item(_) => "Item",
            Node::Fact(_) => "Fact",
            Node::Note(_) => "Note",
            Node::Pipe(_) => "Pipe",
            Node::Tee(_) => "Tee",
            Node::Text(_) => "Text",
            Node::Callout(_) => "Callout",
            Node::Frame(_) => "Frame",
        }
    }

    /// The node's `id`, when it has one.
    pub fn id(&self) -> Option<&str> {
        let id = match self {
            Node::Row(row) => &row.id,
            Node::Col(col) => &col.id,
            Node::Lanes(lanes) => &lanes.id,
            Node::Box(box_node) => &box_node.id,
            Node::Item(item) => &item.id,
            Node::Fact(fact) => &fact.id,
            Node::Note(note) => &note.id,
            Node::Pipe(pipe) => &pipe.id,
            Node::Tee(tee) => &tee.id,
            Node::Text(text) => &text.id,
            Node::Callout(callout) => &callout.id,
            Node::Frame(frame) => &frame.id,
        };
        id.as_deref()
    }
}

impl Line {
    pub const ALL: [Line; 4] = [Line::Gray, Line::Solid, Line::Dash, Line::Deny];

    /// The serialized name, for example "solid".
    pub fn as_str(self) -> &'static str {
        match self {
            Line::Gray => "gray",
            Line::Solid => "solid",
            Line::Dash => "dash",
            Line::Deny => "deny",
        }
    }

    /// True for the lines a tint colors: solid and dash.
    pub fn takes_tint(self) -> bool {
        matches!(self, Line::Solid | Line::Dash)
    }
}

impl FactSource {
    pub const ALL: [FactSource; 3] = [FactSource::Doc, FactSource::Built, FactSource::Ask];

    /// The serialized name, for example "built".
    pub fn as_str(self) -> &'static str {
        match self {
            FactSource::Doc => "doc",
            FactSource::Built => "built",
            FactSource::Ask => "ask",
        }
    }
}

impl IconName {
    pub const ALL: [IconName; 23] = [
        IconName::Agents,
        IconName::AiMl,
        IconName::Bigquery,
        IconName::CloudRunFlat,
        IconName::CloudRun,
        IconName::CloudSql,
        IconName::CloudStorage,
        IconName::ComputeEngine,
        IconName::Compute,
        IconName::Containers,
        IconName::DataAnalytics,
        IconName::Databases,
        IconName::Devops,
        IconName::Gke,
        IconName::Hybrid,
        IconName::Integration,
        IconName::Networking,
        IconName::Observability,
        IconName::Scc,
        IconName::SecurityIdentity,
        IconName::Serverless,
        IconName::Storage,
        IconName::VertexAi,
    ];

    /// The serialized name, for example "cloud-run".
    pub fn as_str(self) -> &'static str {
        let file_name = self.file_name();
        file_name.strip_suffix(".svg").unwrap_or(file_name)
    }

    /// The icon's file name in `assets/icons/`: the serialized stem plus `.svg`.
    pub fn file_name(self) -> &'static str {
        match self {
            IconName::Agents => "agents.svg",
            IconName::AiMl => "ai-ml.svg",
            IconName::Bigquery => "bigquery.svg",
            IconName::CloudRunFlat => "cloud-run-flat.svg",
            IconName::CloudRun => "cloud-run.svg",
            IconName::CloudSql => "cloud-sql.svg",
            IconName::CloudStorage => "cloud-storage.svg",
            IconName::ComputeEngine => "compute-engine.svg",
            IconName::Compute => "compute.svg",
            IconName::Containers => "containers.svg",
            IconName::DataAnalytics => "data-analytics.svg",
            IconName::Databases => "databases.svg",
            IconName::Devops => "devops.svg",
            IconName::Gke => "gke.svg",
            IconName::Hybrid => "hybrid.svg",
            IconName::Integration => "integration.svg",
            IconName::Networking => "networking.svg",
            IconName::Observability => "observability.svg",
            IconName::Scc => "scc.svg",
            IconName::SecurityIdentity => "security-identity.svg",
            IconName::Serverless => "serverless.svg",
            IconName::Storage => "storage.svg",
            IconName::VertexAi => "vertex-ai.svg",
        }
    }
}

/// The tint a Box is painted with (section 13.1 rule 2): its own `tint`, else the kind's
/// default, when the kind is tintable; None for a kind that takes no tint.
pub fn box_tint(kind: &ContainerKind, tint: Option<u8>) -> Option<u8> {
    if kind.tintable {
        tint.or(kind.default_tint)
    } else {
        None
    }
}

/// The tint a line is painted with: a solid or dash line's tint, or slot 1; none for gray
/// and deny.
pub fn line_tint(line: Line, tint: Option<u8>) -> Option<u8> {
    if line.takes_tint() {
        Some(tint.unwrap_or(1))
    } else {
        None
    }
}

const TINT_SUFFIXES: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

/// The output key of a Box (section 13.1 rule 6): the kind, followed by `-a` to `-h` for
/// effective tint 1 to 8.
pub fn box_key(kind: &str, tint: Option<u8>) -> String {
    match tint.and_then(|slot| TINT_SUFFIXES.get(usize::from(slot).wrapping_sub(1))) {
        Some(suffix) => format!("{kind}-{suffix}"),
        None => kind.to_string(),
    }
}

const SOLID_KEYS: [&str; 8] = [
    "blue", "pink", "solid-3", "solid-4", "solid-5", "solid-6", "solid-7", "solid-8",
];
const DASH_KEYS: [&str; 8] = [
    "dash", "dash-2", "dash-3", "dash-4", "dash-5", "dash-6", "dash-7", "dash-8",
];
const SOLID_LABELS: [&str; 8] = [
    "Solid blue",
    "Solid pink",
    "Solid teal",
    "Solid amber",
    "Solid violet",
    "Solid green",
    "Solid orange",
    "Solid cyan",
];
const DASH_LABELS: [&str; 8] = [
    "Dashed blue",
    "Dashed pink",
    "Dashed teal",
    "Dashed amber",
    "Dashed violet",
    "Dashed green",
    "Dashed orange",
    "Dashed cyan",
];

/// Index into an eight-slot table for the effective tint of a solid or dash line. A tint
/// outside 1 to 8 never passes vet; it reads as slot 1 here so the function stays total.
fn slot_index(line: Line, tint: Option<u8>) -> usize {
    let slot = line_tint(line, tint).unwrap_or(1);
    if (1..=TINT_SLOTS).contains(&slot) {
        usize::from(slot - 1)
    } else {
        0
    }
}

fn slot_entry(table: &[&'static str; 8], index: usize) -> &'static str {
    table.get(index).copied().unwrap_or(table[0])
}

/// The output key of a line (section 13.1 rule 6). Slots 1 and 2 of a solid line keep the
/// names `blue` and `pink`, and slot 1 of a dash line is `dash`.
pub fn line_key(line: Line, tint: Option<u8>) -> &'static str {
    let index = slot_index(line, tint);
    match line {
        Line::Gray => "gray",
        Line::Deny => "deny",
        Line::Solid => slot_entry(&SOLID_KEYS, index),
        Line::Dash => slot_entry(&DASH_KEYS, index),
    }
}

/// The canonical legend label of section 13.1 rule 7, for example "Solid blue". Layout
/// measures every legend label with it, whatever the theme.
pub fn legend_label(line: Line, tint: Option<u8>) -> &'static str {
    let index = slot_index(line, tint);
    match line {
        Line::Gray => "Solid gray",
        Line::Deny => "Dashed red",
        Line::Solid => slot_entry(&SOLID_LABELS, index),
        Line::Dash => slot_entry(&DASH_LABELS, index),
    }
}
