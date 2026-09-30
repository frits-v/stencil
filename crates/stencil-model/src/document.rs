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

fn page_width_default() -> u32 {
    PAGE_WIDTH_DEFAULT
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
    #[schemars(length(min = 1, max = 256))]
    pub body: Vec<Node>,
    #[schemars(length(max = 16))]
    pub legend: Vec<LegendEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Canvas {
    Customer,
    Internal,
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
    pub dir: PipeDir,
    pub kind: PipeKind,
    #[schemars(length(min = 1, max = 400))]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub sub: Option<String>,
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
    pub kind: PipeKind,
    #[schemars(length(min = 1, max = 400))]
    pub hub: String,
    pub arms: [TeeArm; 2],
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
        }
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
