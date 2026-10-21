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
/// Grid reference columns of a design sheet, numbered 1 to 8 left to right (section 14.1).
pub const SHEET_COLUMNS: u8 = 8;
/// Grid reference rows of a design sheet, lettered A to F top to bottom.
pub const SHEET_ROWS: u8 = 6;
pub const SHEET_ROW_LETTERS: [char; 6] = ['A', 'B', 'C', 'D', 'E', 'F'];
/// Root padding on every side: section 2.2, and section 14.4 rule 1 on a sheet.
pub const ROOT_PADDING_PX: f32 = 20.0;
pub const SHEET_ROOT_PADDING_PX: f32 = 50.0;
pub const WORDMARK_SCALARS_MAX: usize = 24;
pub const CAPTION_SCALARS_MAX: usize = 60;
/// A View's gap between its children when it names none.
pub const VIEW_GAP_DEFAULT_PX: u16 = 16;
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
/// The radius of a routed link's bends when neither its line kind nor the page sets one.
pub const LINK_CORNER_DEFAULT_PX: f32 = 6.0;
pub const LINK_CORNER_MAX_PX: f32 = 16.0;
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

/// The canvas width layout computes and `link-via-outside` bounds: the page width plus the
/// root padding on both sides, which is wider on a design sheet (section 14.4 rule 1).
pub fn canvas_width(page: &Page) -> f32 {
    page.width as f32 + 2.0 * root_padding(page)
}

/// The root padding on every side of the page.
pub fn root_padding(page: &Page) -> f32 {
    if page.sheet.is_some() {
        SHEET_ROOT_PADDING_PX
    } else {
        ROOT_PADDING_PX
    }
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
    /// The pill before the kicker, printed uppercase, for example Draft; absent draws none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub badge: Option<String>,
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
    /// The radius of every routed link's bends; absent is 6, 0 draws square corners. A line
    /// kind of the grammar that sets its own wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0, max = 16.0))]
    pub corner: Option<f32>,
    /// How a routed link draws its bends; absent is arc. A line kind's bend wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bend: Option<Bend>,
    /// Draws the page as a design sheet (section 14): a frame with grid references, a title
    /// block and numbered Views. Flat only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<Sheet>,
    #[schemars(length(min = 1, max = 256))]
    pub body: Vec<Node>,
    #[schemars(length(max = 16))]
    pub legend: Vec<LegendEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 256))]
    pub links: Vec<Link>,
}

/// The title block of a design sheet. Its first cell is the page head: the badge and kicker,
/// the wordmark when set, the title and the lede; the other three cells follow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sheet {
    /// The product name set large in the first cell; absent draws the title in its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 24))]
    pub wordmark: Option<String>,
    pub problem: Problem,
    pub answer: Answer,
    pub attention: Attention,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Problem {
    /// The cell caption; absent prints `Problem`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 60))]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub lead: Option<String>,
    /// Numbered P1, P2 in order.
    #[schemars(length(min = 1, max = 8))]
    pub points: Vec<ProblemPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProblemPoint {
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The cell caption; absent prints `Answer`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 60))]
    pub caption: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub statement: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 4))]
    pub paragraphs: Vec<AnswerParagraph>,
}

/// A paragraph: a string, or `{"lead": ..., "text": ...}`, drawn as a bold lead with its
/// text continuing on the lead's last line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AnswerParagraph {
    Plain(#[schemars(length(min = 1, max = 400))] String),
    Led(LedParagraph),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedParagraph {
    #[schemars(length(min = 1, max = 400))]
    pub lead: String,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
}

impl AnswerParagraph {
    /// The bold lead, when the paragraph has one.
    pub fn lead(&self) -> Option<&str> {
        match self {
            AnswerParagraph::Plain(_) => None,
            AnswerParagraph::Led(led) => Some(&led.lead),
        }
    }

    /// The paragraph text after its lead.
    pub fn text(&self) -> &str {
        match self {
            AnswerParagraph::Plain(text) => text,
            AnswerParagraph::Led(led) => &led.text,
        }
    }
}

/// What the reader is asked to attend to. A sheet always draws this cell, so it holds at
/// least one item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attention {
    /// The cell caption; absent prints `Attention`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 60))]
    pub caption: Option<String>,
    #[schemars(length(min = 1, max = 8))]
    pub items: Vec<AttentionItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionItem {
    /// Unique among the attention items.
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: String,
    /// Drawn bold before `text` with one space between, so the lead carries its own
    /// punctuation.
    #[schemars(length(min = 1, max = 400))]
    pub lead: String,
    #[schemars(length(min = 1, max = 400))]
    pub text: String,
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
    /// The form of the pipes this entry names: a tube (the default) or a band. A band and
    /// a tube of one line and tint are two legend entries, and the swatch draws the form.
    #[serde(default, skip_serializing_if = "PipeForm::is_tube")]
    pub form: PipeForm,
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
    View(View),
}

/// A numbered view of a design sheet: a heading with its number, title, subtitle and note,
/// then its children in a column. The title block is view 1, so the first View is view 2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct View {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID_PATTERN))]
    pub id: Option<String>,
    #[schemars(length(min = 1, max = 400))]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub subtitle: Option<String>,
    /// Right-aligned on the heading rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 400))]
    pub note: Option<String>,
    /// The gap between children; absent is 16.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 64))]
    pub gap: Option<u16>,
    #[schemars(length(min = 1, max = 256))]
    pub children: Vec<Node>,
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
    /// Under iso, the solid the item stands as; absent takes the shape of its icon's row
    /// in the kind's product table, or card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
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

/// The solid an item stands as under iso (section 12.3): a card is the flat card raised,
/// with its content on top; every other shape carries the icon on its top and its text
/// on the floor beside it. Tile: a thin plate, for network and edge pieces. Tower: a tall
/// narrow box, for compute, clusters and nodes. Cylinder: databases and warehouses.
/// Stack: three stacked discs, for storage and buckets. Block: a card-height box of a
/// fixed footprint with its text on the floor, the default of an icon-bearing kind. Figure:
/// a standing person, for users and operators. Laptop and phone: client devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    Card,
    Tile,
    Tower,
    Cylinder,
    Stack,
    Block,
    Figure,
    Laptop,
    Phone,
}

impl Shape {
    pub fn as_str(self) -> &'static str {
        match self {
            Shape::Card => "card",
            Shape::Tile => "tile",
            Shape::Tower => "tower",
            Shape::Cylinder => "cylinder",
            Shape::Stack => "stack",
            Shape::Block => "block",
            Shape::Figure => "figure",
            Shape::Laptop => "laptop",
            Shape::Phone => "phone",
        }
    }
}

/// The reading axis of a tag under iso (section 12.4): along flat x, reading down-right on
/// screen, or along flat y, reading up-right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    Y,
}

impl Axis {
    pub fn as_str(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
        }
    }
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
    /// Under iso, the axis the tag reads along; absent follows dir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<Axis>,
    /// Under iso, a round tube on the pipe's plane (the default) or a band, a flat wide
    /// arrow lying on it (section 12.3 rule 5).
    #[serde(default, skip_serializing_if = "PipeForm::is_tube")]
    pub form: PipeForm,
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

/// How a Pipe is drawn under iso: a round tube on its plane, or a band, a flat wide arrow
/// lying on it. Flat draws both as the wire of section 2.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum PipeForm {
    #[default]
    Tube,
    Band,
}

impl PipeForm {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_tube(form: &PipeForm) -> bool {
        *form == PipeForm::Tube
    }
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
    /// Under iso, the axis the tag reads along; absent follows the leg it sits on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<Axis>,
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

/// The curve a routed link takes through a bend: a quarter circle, a cubic whose control
/// points both sit on the corner, which meets each leg with no jump in curvature, or a
/// spline: a quadratic from the middle of one leg to the middle of the next with its control
/// point on the corner, so the whole route is one smooth curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Bend {
    #[default]
    Arc,
    Curve,
    Spline,
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
            Node::View(_) => "View",
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
            Node::View(view) => &view.id,
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
