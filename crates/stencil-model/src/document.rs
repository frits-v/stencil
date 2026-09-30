use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

fn is_default_theme(theme: &Theme) -> bool {
    *theme == Theme::Center
}

fn is_default_projection(projection: &Projection) -> bool {
    *projection == Projection::Flat
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
    #[serde(default, skip_serializing_if = "is_default_theme")]
    pub theme: Theme,
    #[serde(default, skip_serializing_if = "is_default_projection")]
    pub projection: Projection,
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
pub enum Theme {
    #[default]
    Center,
    Dusk,
    Wire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Projection {
    #[default]
    Flat,
    Iso,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegendEntry {
    pub kind: PipeKind,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tag")]
pub enum Node {
    Row(Row),
    Col(Col),
    Zone(Zone),
    Pcard(Pcard),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ZoneKind {
    Gcp,
    Vpc,
    RegionA,
    RegionB,
    Subnet,
    OnpremA,
    OnpremB,
    Project,
    Optional,
    K8s,
    Perimeter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    pub kind: ZoneKind,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pcard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconName>,
    #[serde(rename = "fn")]
    #[schemars(length(min = 1, max = 400))]
    pub function_name: String,
    #[serde(rename = "pn", default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub product_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub fact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub ask: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
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
pub enum PipeKind {
    Gray,
    Blue,
    Pink,
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
    pub kind: PipeKind,
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
    pub kind: PipeKind,
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
    pub kind: PipeKind,
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
            Node::Zone(_) => "Zone",
            Node::Pcard(_) => "Pcard",
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
            Node::Zone(zone) => &zone.id,
            Node::Pcard(pcard) => &pcard.id,
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

impl ZoneKind {
    pub const ALL: [ZoneKind; 11] = [
        ZoneKind::Gcp,
        ZoneKind::Vpc,
        ZoneKind::RegionA,
        ZoneKind::RegionB,
        ZoneKind::Subnet,
        ZoneKind::OnpremA,
        ZoneKind::OnpremB,
        ZoneKind::Project,
        ZoneKind::Optional,
        ZoneKind::K8s,
        ZoneKind::Perimeter,
    ];

    /// The serialized name, for example "region-a".
    pub fn as_str(self) -> &'static str {
        match self {
            ZoneKind::Gcp => "gcp",
            ZoneKind::Vpc => "vpc",
            ZoneKind::RegionA => "region-a",
            ZoneKind::RegionB => "region-b",
            ZoneKind::Subnet => "subnet",
            ZoneKind::OnpremA => "onprem-a",
            ZoneKind::OnpremB => "onprem-b",
            ZoneKind::Project => "project",
            ZoneKind::Optional => "optional",
            ZoneKind::K8s => "k8s",
            ZoneKind::Perimeter => "perimeter",
        }
    }
}

impl PipeKind {
    pub const ALL: [PipeKind; 5] = [
        PipeKind::Gray,
        PipeKind::Blue,
        PipeKind::Pink,
        PipeKind::Dash,
        PipeKind::Deny,
    ];

    /// The serialized name, for example "blue".
    pub fn as_str(self) -> &'static str {
        match self {
            PipeKind::Gray => "gray",
            PipeKind::Blue => "blue",
            PipeKind::Pink => "pink",
            PipeKind::Dash => "dash",
            PipeKind::Deny => "deny",
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
