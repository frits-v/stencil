//! The figure description (section 14.1): Markdown an agent reads without rendering, and
//! `description-covers-drawing` (section 14.6), which parses it back against the units the
//! page and its geometry define.

use std::collections::{BTreeMap, HashMap, HashSet};

use stencil_layout::{
    DescriptionUnits, NodeTag, PageGeometry, PipeEndSide, RouteStatus, UnitKind, description_units,
    title_cell_caption, where_used,
};
use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;
use stencil_model::products::{item_shape, named_product};
use stencil_model::{
    Arrow, BandLook, CalloutKind, Chrome, Grammar, KeyColour, LADDERS_MAX, LINKS_MAX, Ladder,
    LegendKey, Line, ListKind, Node, NodeRef, NoteKind, Page, PageActors, PageMarks, PartsList,
    Phases, Pipe, PipeDir, PipeForm, Projection, Sheet, SheetList, Step, StepLook, TINT_NAMES, Tee,
    body_nodes, canonical_legend_label, entry_key, line_key,
};

use crate::format_number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    pub text: String,
}

/// The last sentence of a sheet's figure paragraph (section 14.1 rule 13).
const REFERENCES_SENTENCE: &str = "References: a reference such as B3 or B2-C5 names the grid cells under an element's top-left and bottom-right corners, columns 1 to 8 left to right, rows A to F top to bottom. The reference before a pointer is where that element is drawn; a Zones cell lists the cells of every balloon, flag or diamond keyed to its entry.";

/// The sentence a `chrome: none` page writes after its uncaptioned head lines.
const CAPTIONED_HEAD: &str =
    "The title, kicker and lede caption the figure in its document and are not drawn.";

/// The Markdown description of a laid-out page. Takes no Theme, Palette or IsoScene, so the
/// text is the same under every theme by construction.
pub fn describe(page: &Page, grammar: &Grammar, geometry: &PageGeometry) -> Description {
    let units = description_units(page, geometry);
    let writer = Writer::new(page, grammar, geometry, &units);
    let mut blocks: Vec<String> = Vec::new();
    writer.head(&mut blocks);
    if let Some(sheet) = &page.sheet {
        writer.sheet(sheet, &mut blocks);
        let mut text = blocks.join("\n\n");
        text.push('\n');
        return Description { text };
    }
    blocks.push("## Figure".to_string());
    blocks.push(writer.figure_paragraph());
    blocks.push(format!("## Drawing {}", span(&body_pointer())));
    blocks.push(writer.outline());
    let lines = writer.lines_rows();
    if !lines.is_empty() {
        blocks.push("## Lines".to_string());
        blocks.push(table(
            &[
                "Line", "Kind", "From", "To", "Style", "Ends", "Means", "Pointer",
            ],
            &lines,
        ));
    }
    let legend_pointer = NodePointer::root().child("legend");
    let has_legend_node = geometry.node(&legend_pointer).is_some();
    if !page.legend.is_empty() || has_legend_node {
        let heading = if has_legend_node {
            format!("## Legend {}", span(&legend_pointer))
        } else {
            "## Legend".to_string()
        };
        blocks.push(heading);
        blocks.push(table(
            &["Key", "Center label", "Means", "Uses", "Pointer"],
            &writer.legend_rows(),
        ));
    }
    if let Some(foot) = &page.foot {
        let foot_pointer = NodePointer::root().child("foot");
        blocks.push(writer.with_pointer(format!("Foot: {}", escape_markdown(foot)), &foot_pointer));
    }
    let mut text = blocks.join("\n\n");
    text.push('\n');
    Description { text }
}

/// Backslash-escapes document text so that no string opens a heading, list, emphasis,
/// strikethrough, entity, link, HTML block, table cell or code span (section 14.1 rule 12).
pub fn escape_markdown(text: &str) -> String {
    let leading_digits = text
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    let mut escaped = String::with_capacity(text.len() + 8);
    for (position, character) in text.chars().enumerate() {
        let always = matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '|' | '#' | '!' | '&' | '~'
        );
        let opens_line = position == 0 && matches!(character, '-' | '+' | '=');
        let numbers_line =
            leading_digits > 0 && position == leading_digits && matches!(character, '.' | ')');
        if always || opens_line || numbers_line {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// The legend key the description prints: the line's output key, or `<line>-by-actor` for
/// a by-actor key, followed by `-band` for a band entry.
pub fn legend_output_key(key: &LegendKey) -> String {
    let line = match key.colour {
        KeyColour::Slot(tint) => line_key(key.line, tint).to_string(),
        KeyColour::ByActor => format!("{}-by-actor", key.line.as_str()),
    };
    let form = match key.form {
        PipeForm::Tube => line,
        PipeForm::Band => format!("{line}-band"),
        PipeForm::Rail => format!("{line}-rail"),
    };
    if key.planned {
        format!("{form}-planned")
    } else {
        form
    }
}

/// A value inside a list-valued table cell: escaped, and escaped again before `;`, which
/// joins the cell's values (section 14.1 rule 12).
fn cell_value(text: &str) -> String {
    escape_markdown(text).replace(';', "\\;")
}

fn body_pointer() -> NodePointer {
    NodePointer::root().child("body")
}

fn links_pointer() -> NodePointer {
    NodePointer::root().child("links")
}

/// A pointer span: the pointer between backticks.
fn span(pointer: &NodePointer) -> String {
    format!("`{}`", pointer.as_str())
}

fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let mut lines = vec![
        format!("| {} |", header.join(" | ")),
        format!("|{}", "---|".repeat(header.len())),
    ];
    for row in rows {
        lines.push(format!("| {} |", row.join(" | ")));
    }
    lines.join("\n")
}

fn count_words(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

struct Writer<'a> {
    page: &'a Page,
    grammar: &'a Grammar,
    geometry: &'a PageGeometry,
    units: &'a DescriptionUnits,
    nodes: HashMap<&'a str, NodeRef<'a>>,
    ids: HashMap<&'a str, NodePointer>,
    children: Vec<Vec<usize>>,
    actors: PageActors<'a>,
    page_marks: PageMarks<'a>,
}

impl<'a> Writer<'a> {
    fn new(
        page: &'a Page,
        grammar: &'a Grammar,
        geometry: &'a PageGeometry,
        units: &'a DescriptionUnits,
    ) -> Self {
        let mut nodes = HashMap::new();
        let mut ids = HashMap::new();
        for entry in body_nodes(page) {
            if let Some(node) = geometry.node(&entry.pointer) {
                nodes.insert(node.pointer.as_str(), entry.node);
            }
            if let Some(id) = entry.node.id() {
                ids.insert(id, entry.pointer.clone());
            }
        }
        let mut children = vec![Vec::new(); geometry.nodes.len()];
        for (index, node) in geometry.nodes.iter().enumerate() {
            if let Some(siblings) = node.parent.and_then(|parent| children.get_mut(parent)) {
                siblings.push(index);
            }
        }
        Writer {
            page,
            grammar,
            geometry,
            units,
            nodes,
            ids,
            children,
            actors: PageActors::of(page),
            page_marks: PageMarks::of(page),
        }
    }

    /// An actor's name, or its id when the page does not declare it.
    fn actor_name(&self, id: &str) -> String {
        self.page
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .map_or_else(|| id.to_string(), |actor| actor.name.clone())
    }

    /// The effective actor of the line drawn by the geometry node `node`: a Pipe, a Tee
    /// spine or a Tee arm.
    fn node_line_actor(&self, node: &stencil_layout::NodeGeometry) -> Option<&'a str> {
        match self.nodes.get(node.pointer.as_str()).copied()? {
            NodeRef::Node(Node::Pipe(pipe)) => self.actors.pipe_actor(pipe),
            NodeRef::Node(Node::Tee(tee)) => self.actors.tee_actor(tee),
            NodeRef::TeeArm(arm) => {
                let parent = self.geometry.nodes.get(node.parent?)?;
                match self.nodes.get(parent.pointer.as_str()).copied()? {
                    NodeRef::Node(Node::Tee(tee)) => self.actors.arm_actor(tee, arm),
                    _ => None,
                }
            }
            NodeRef::Node(_) => None,
        }
    }

    /// The legend key of a use: by actor when its effective actor is declared.
    fn use_key(
        &self,
        (line, tint, form): (Line, Option<u8>, PipeForm),
        actor: Option<&str>,
    ) -> LegendKey {
        match self.actors.slot(actor) {
            Some(_) => LegendKey::by_actor(line, form),
            None => LegendKey::new(line, tint, form),
        }
    }

    /// The Style cell of a use: its slot and actor name when it acts for an actor.
    fn use_style(&self, key: &LegendKey, actor: Option<&str>) -> String {
        let style = match (actor, self.actors.slot(actor)) {
            (Some(actor), Some(slot)) => {
                actor_style(key.line, slot, &escape_markdown(&self.actor_name(actor)))
            }
            _ => style(key),
        };
        // A planned line names itself so before its colour word.
        if key.planned {
            style.replacen("; ", ", planned; ", 1)
        } else {
            style
        }
    }

    /// The Actor cell of a line on a sheet.
    fn actor_cell(&self, actor: Option<&str>) -> String {
        match actor.filter(|actor| self.actors.tint(actor).is_some()) {
            Some(actor) => escape_markdown(&self.actor_name(actor)),
            None => "none".to_string(),
        }
    }

    /// `text` followed by the spans of `pointer` when geometry has a node there.
    fn with_pointer(&self, text: String, pointer: &NodePointer) -> String {
        if self.geometry.node(pointer).is_some() {
            format!("{text} {}", self.spans(pointer))
        } else {
            text
        }
    }

    /// The pointer span, led on a sheet by the reference span of the unit it names.
    fn spans(&self, pointer: &NodePointer) -> String {
        match self.units.unit(pointer).and_then(|unit| unit.zone) {
            Some(zone) => format!("`{zone}` {}", span(pointer)),
            None => span(pointer),
        }
    }

    /// A mark as the description writes it: its unit name, then its spans.
    fn mark_text(&self, pointer: &NodePointer) -> String {
        let name = self
            .units
            .unit(pointer)
            .map(|unit| unit.name.clone())
            .unwrap_or_default();
        format!("{} {}", cell_value(&name), self.spans(pointer))
    }

    /// The Marks cell of a table row: each mark of `marks`, joined by `; `, or `none`.
    fn marks_cell(&self, marks: &[stencil_model::CarrierMark]) -> String {
        if marks.is_empty() {
            return "none".to_string();
        }
        marks
            .iter()
            .map(|mark| self.mark_text(&mark.pointer))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// One nested `Mark` line per mark `place` selects.
    fn mark_lines(
        &self,
        marks: &[stencil_model::CarrierMark],
        select: impl Fn(stencil_model::MarkPlace) -> bool,
        indent: &str,
        lines: &mut Vec<String>,
    ) {
        for mark in marks.iter().filter(|mark| select(mark.place)) {
            let name = self
                .units
                .unit(&mark.pointer)
                .map(|unit| unit.name.clone())
                .unwrap_or_default();
            lines.push(format!(
                "{indent}- Mark {} {}",
                escape_markdown(&name),
                self.spans(&mark.pointer)
            ));
        }
    }

    /// The where-used cells of the key at `key`, or `none`.
    fn zones_cell(&self, key: &NodePointer) -> String {
        let zones = where_used(self.units, key);
        if zones.is_empty() {
            "none".to_string()
        } else {
            zones
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    /// The marks of the body node at `pointer`.
    fn node_marks(&self, pointer: &NodePointer) -> Vec<stencil_model::CarrierMark> {
        match self.nodes.get(pointer.as_str()).copied() {
            Some(node) => self.page_marks.marks_at(pointer, node),
            None => Vec::new(),
        }
    }

    /// The unit name of the node at `pointer`, or its tag word for a structural node.
    fn name_of(&self, pointer: &NodePointer) -> String {
        if let Some(unit) = self.units.unit(pointer) {
            return unit.name.clone();
        }
        self.geometry
            .node(pointer)
            .map(|node| node.tag.as_str().to_string())
            .unwrap_or_default()
    }

    fn head(&self, blocks: &mut Vec<String>) {
        let root = NodePointer::root();
        let kicker = match &self.page.badge {
            Some(badge) => format!(
                "{} · {}",
                escape_markdown(badge),
                escape_markdown(&self.page.kicker)
            ),
            None => escape_markdown(&self.page.kicker),
        };
        blocks.push(self.with_pointer(
            format!("# {}", escape_markdown(&self.page.title)),
            &root.child("title"),
        ));
        blocks.push(self.with_pointer(kicker, &root.child("kicker")));
        blocks.push(self.with_pointer(escape_markdown(&self.page.lede), &root.child("lede")));
        match self.page.chrome {
            Chrome::Full => {}
            Chrome::None => blocks.push(CAPTIONED_HEAD.to_string()),
        }
    }

    fn figure_paragraph(&self) -> String {
        let count_of = |kind: UnitKind| {
            self.units
                .units
                .iter()
                .filter(|unit| unit.kind == kind)
                .count()
        };
        let link_count = count_of(UnitKind::Link);
        let entry_count = count_of(UnitKind::Entry);
        let mark_count = count_of(UnitKind::Mark);
        let node_count = self.units.units.len() - link_count - entry_count - mark_count;
        let width = format_number(self.geometry.canvas.width);
        let height = format_number(self.geometry.canvas.height);
        let grammar = escape_markdown(&self.grammar.name);
        let canvas = match self.page.projection {
            Projection::Flat => {
                format!("Canvas {width} x {height} px, projection flat, grammar {grammar}.")
            }
            Projection::Iso => format!(
                "Laid out at {width} x {height} px, projection iso, grammar {grammar}; the projected drawing is larger, by an amount that depends on the theme."
            ),
        };
        let counts = match self.page.sheet {
            Some(_) => format!(
                "{}, {}, {}, {}",
                count_words(node_count, "node", "nodes"),
                count_words(link_count, "link", "links"),
                count_words(entry_count, "entry", "entries"),
                count_words(mark_count, "mark", "marks"),
            ),
            None => format!(
                "{}, {}",
                count_words(node_count, "node", "nodes"),
                count_words(link_count, "link", "links"),
            ),
        };
        let mut paragraph = format!(
            "Drawn elements: {} ({counts}). {canvas} Line styles name a tint slot by number; the colour word is the one the center theme draws it in.",
            self.units.units.len(),
        );
        if self.page.sheet.is_some() {
            paragraph.push(' ');
            paragraph.push_str(REFERENCES_SENTENCE);
        }
        paragraph
    }

    /// One list line per body node in geometry order, nested by geometry parent.
    fn outline(&self) -> String {
        match self
            .geometry
            .nodes
            .iter()
            .position(|node| node.pointer == body_pointer())
        {
            Some(body) => self.outline_under(body),
            None => String::new(),
        }
    }

    /// The outline of the descendants of the node at `root`.
    /// A Phases node's lane-by-phase table follows the whole list, so no list item after it
    /// can read as an indented code block.
    fn outline_under(&self, root: usize) -> String {
        let mut lines = Vec::new();
        let mut tables = Vec::new();
        let mut pending: Vec<(usize, usize)> = self
            .children_of(root)
            .iter()
            .rev()
            .map(|&child| (child, 0))
            .collect();
        for _ in 0..self.geometry.nodes.len() {
            let Some((index, depth)) = pending.pop() else {
                break;
            };
            self.outline_node(index, depth, &mut lines);
            if let (Some(node), Some(NodeRef::Node(Node::Phases(phases)))) = (
                self.geometry.nodes.get(index),
                self.geometry
                    .nodes
                    .get(index)
                    .and_then(|node| self.nodes.get(node.pointer.as_str()).copied()),
            ) {
                tables.push(self.phases_table(node, phases));
            }
            if let (Some(node), Some(NodeRef::Node(Node::PartsList(list)))) = (
                self.geometry.nodes.get(index),
                self.geometry
                    .nodes
                    .get(index)
                    .and_then(|node| self.nodes.get(node.pointer.as_str()).copied()),
            ) {
                tables.push(self.parts_table(node, list));
            }
            if let Some(NodeRef::Node(Node::HowTo(how_to))) = self
                .geometry
                .nodes
                .get(index)
                .and_then(|node| self.nodes.get(node.pointer.as_str()).copied())
            {
                tables.push(self.how_to_blocks(index, how_to));
            }
            for &child in self.children_of(index).iter().rev() {
                pending.push((child, depth + 1));
            }
        }
        let mut blocks = vec![lines.join("\n")];
        blocks.extend(tables);
        blocks.retain(|block| !block.is_empty());
        blocks.join("\n\n")
    }

    /// One section per how-to of the HowTo at geometry node `index`: its heading with the
    /// number the sheet draws, its command runs in drawn order, its text and its terminal
    /// in a fence, one line per terminal line, each led by its kind's prefix.
    fn how_to_blocks(&self, index: usize, how_to: &stencil_model::HowTo) -> String {
        let mut blocks = Vec::new();
        let items_pointer = SheetList::HowToItems.pointer_in(
            &self
                .geometry
                .nodes
                .get(index)
                .map(|node| node.pointer.clone())
                .unwrap_or_else(NodePointer::root),
        );
        for &child in self.children_of(index) {
            let Some(node) = self.geometry.nodes.get(child) else {
                continue;
            };
            let Some(item) = how_to
                .items
                .iter()
                .enumerate()
                .take(SheetList::HowToItems.max() + 1)
                .find(|(position, _)| items_pointer.index(*position) == node.pointer)
                .map(|(_, item)| item)
            else {
                continue;
            };
            let number = node
                .part(stencil_layout::PartName::HowToNumber)
                .and_then(|part| part.text.as_ref())
                .map(|run| run.text.clone())
                .unwrap_or_default();
            let mut block = vec![format!(
                "### How-to {}: {} {}",
                escape_markdown(&number),
                escape_markdown(&item.title),
                self.spans(&node.pointer)
            )];
            let runs: Vec<String> = item
                .commands
                .iter()
                .take(SheetList::CommandRuns.max() + 1)
                .map(|run| match run {
                    stencil_model::CommandRun::Chip(text) => {
                        format!("- Command {}.", escape_markdown(text))
                    }
                    stencil_model::CommandRun::Words(words) => {
                        format!("- Words {}.", escape_markdown(&words.words))
                    }
                })
                .collect();
            if !runs.is_empty() {
                block.push(runs.join("\n"));
            }
            if item.beside && item.text.is_some() && !item.terminal.is_empty() {
                block.push("Text beside the terminal.".to_string());
            }
            if let Some(text) = &item.text {
                block.push(escape_markdown(text));
            }
            if !item.terminal.is_empty() {
                block.push(terminal_fence(&item.terminal));
            }
            blocks.push(block.join("\n\n"));
        }
        blocks.join("\n\n")
    }

    fn children_of(&self, index: usize) -> &[usize] {
        self.children
            .get(index)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn outline_node(&self, index: usize, depth: usize, lines: &mut Vec<String>) {
        let Some(node) = self.geometry.nodes.get(index) else {
            return;
        };
        let indent = "  ".repeat(depth);
        let nested = "  ".repeat(depth + 1);
        let pointer = self.spans(&node.pointer);
        let reference = self.nodes.get(node.pointer.as_str()).copied();
        let text = match (node.tag, reference) {
            (NodeTag::Row, _) => format!("Row, left to right. {pointer}"),
            (NodeTag::Col, _) => format!("Col, top to bottom. {pointer}"),
            (NodeTag::Lanes, _) => format!("Lanes, heads left to right. {pointer}"),
            (NodeTag::Zone, Some(NodeRef::Node(Node::Box(box_node)))) => {
                let mut text = format!(
                    "Box {}; kind {}",
                    escape_markdown(&box_node.label),
                    escape_markdown(&box_node.kind)
                );
                if let Some(tint) = node.tint {
                    text.push_str(&format!(", tint {tint}"));
                }
                if let Some(actor) = &box_node.actor {
                    text.push_str(&format!(
                        ", actor {}",
                        escape_markdown(&self.actor_name(actor))
                    ));
                }
                if box_node.planned {
                    text.push_str(", planned");
                }
                lines.push(format!("{indent}- {text}. {pointer}"));
                self.mark_lines(&self.node_marks(&node.pointer), |_| true, &nested, lines);
                return;
            }
            (NodeTag::Pcard, Some(NodeRef::Node(Node::Item(item)))) => {
                let mut text = format!("Item {}", escape_markdown(&item.title));
                if let Some(subtitle) = &item.subtitle {
                    text.push_str(&format!(": {}", escape_markdown(subtitle)));
                }
                text.push_str(&format!("; kind {}", escape_markdown(&item.kind)));
                if let Some(icon) = item.icon {
                    text.push_str(&format!(", icon {}", icon.as_str()));
                }
                let kind = self.grammar.item(&item.kind);
                let table = kind
                    .map(|kind| kind.products.as_slice())
                    .unwrap_or_default();
                if let Some(product) = item
                    .subtitle
                    .as_deref()
                    .and_then(|subtitle| named_product(table, subtitle))
                {
                    text.push_str(&format!(", product {}", escape_markdown(product)));
                }
                if self.page.projection == Projection::Iso {
                    text.push_str(&format!(", shape {}", item_shape(kind, item).as_str()));
                }
                if item.outline != stencil_model::Outline::Card {
                    text.push_str(&format!(", outline {}", item.outline.as_str()));
                }
                if let Some(actor) = &item.actor {
                    text.push_str(&format!(
                        ", actor {}",
                        escape_markdown(&self.actor_name(actor))
                    ));
                }
                if item.planned {
                    text.push_str(", planned");
                }
                lines.push(format!("{indent}- {text}. {pointer}"));
                let marks = self.node_marks(&node.pointer);
                self.mark_lines(
                    &marks,
                    |place| !matches!(place, stencil_model::MarkPlace::Compartment(_)),
                    &nested,
                    lines,
                );
                for fact in &item.facts {
                    lines.push(format!(
                        "{nested}- Fact, {}: {}.",
                        fact.source.as_str(),
                        escape_markdown(&fact.text)
                    ));
                }
                let deeper = "  ".repeat(depth + 2);
                for (index, compartment) in item.compartments.iter().enumerate() {
                    lines.push(format!(
                        "{nested}- Compartment {}.",
                        escape_markdown(compartment.text())
                    ));
                    self.mark_lines(
                        &marks,
                        |place| place == stencil_model::MarkPlace::Compartment(index),
                        &deeper,
                        lines,
                    );
                }
                return;
            }
            (NodeTag::Fact, Some(NodeRef::Node(Node::Fact(fact)))) => {
                format!(
                    "Fact, {}: {}. {pointer}",
                    fact.source.as_str(),
                    escape_markdown(&fact.text)
                )
            }
            (NodeTag::Note, Some(NodeRef::Node(Node::Note(note)))) => format!(
                "Note, {}: {}. {pointer}",
                note_kind(note.kind),
                escape_markdown(&note.text)
            ),
            (NodeTag::Text, Some(NodeRef::Node(Node::Text(text_block)))) => {
                let name = self.name_of(&node.pointer);
                lines.push(format!(
                    "{indent}- Text {}. {pointer}",
                    escape_markdown(&name)
                ));
                for (position, line) in text_block.body.iter().enumerate() {
                    let marker = match text_block.list {
                        ListKind::Numbered => format!("{}.", position + 1),
                        ListKind::Plain | ListKind::Bulleted => "-".to_string(),
                    };
                    lines.push(format!("{nested}{marker} {}", escape_markdown(line)));
                }
                return;
            }
            (NodeTag::Callout, Some(NodeRef::Node(Node::Callout(callout)))) => match &callout.title
            {
                Some(title) => format!(
                    "Callout, {}: {}. {} {pointer}",
                    callout_kind(callout.kind),
                    escape_markdown(title),
                    escape_markdown(&callout.text)
                ),
                None => format!(
                    "Callout, {}: {} {pointer}",
                    callout_kind(callout.kind),
                    escape_markdown(&callout.text)
                ),
            },
            (NodeTag::Frame, Some(NodeRef::Node(Node::Frame(frame)))) => format!(
                "Frame {}: {} px placeholder. {pointer}",
                escape_markdown(&frame.label),
                frame.height
            ),
            (NodeTag::Pipe, Some(NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe))) => {
                format!("Pipe {}.", pipe_line_name(pipe))
            }
            (NodeTag::Tee, Some(NodeRef::Node(Node::Tee(tee)))) => {
                format!("Tee {}.", escape_markdown(&tee.hub))
            }
            (NodeTag::Phases, Some(NodeRef::Node(Node::Phases(phases)))) => {
                self.phases_outline(node, phases, depth, lines);
                return;
            }
            (NodeTag::Step, Some(NodeRef::Node(Node::Step(step)))) => {
                lines.push(format!("{indent}- {} {pointer}", self.step_line(step)));
                let marks = self.node_marks(&node.pointer);
                self.mark_lines(
                    &marks,
                    |place| !matches!(place, stencil_model::MarkPlace::Line(_)),
                    &nested,
                    lines,
                );
                let deeper = "  ".repeat(depth + 2);
                for (line_index, line) in step.lines.iter().enumerate() {
                    lines.push(format!("{nested}- Line {}.", escape_markdown(line.text())));
                    self.mark_lines(
                        &marks,
                        |place| place == stencil_model::MarkPlace::Line(line_index),
                        &deeper,
                        lines,
                    );
                }
                return;
            }
            (NodeTag::PartsList, Some(NodeRef::Node(Node::PartsList(list)))) => {
                let mut text = escape_markdown(&list.title);
                if let Some(note) = &list.note {
                    text.push_str(&format!(": {}", escape_markdown(note)));
                }
                format!("{text}. {pointer}")
            }
            (NodeTag::Ladder, Some(NodeRef::Node(Node::Ladder(ladder)))) => {
                let mut text = format!(
                    "Ladder, detail {}: {}",
                    self.detail_letter_of(ladder),
                    escape_markdown(&ladder.title)
                );
                if let Some(note) = &ladder.note {
                    text.push_str(&format!("; {}", escape_markdown(note)));
                }
                if let Some(fill) = ladder.rung_fill {
                    text.push_str(&format!("; rung fill {}", fill.as_str()));
                }
                format!(
                    "{text}. Frames {}. {pointer}",
                    self.framed_titles(ladder).join(", ")
                )
            }
            (NodeTag::Rung, _) => {
                let Some((ladder, ladder_pointer, index)) = self.rung_at(&node.pointer) else {
                    return;
                };
                let Some(rung) = ladder.rungs.get(index) else {
                    return;
                };
                let count = ladder.rungs.len().min(SheetList::Rungs.max() + 1);
                let place = match index {
                    _ if count < 2 => "",
                    0 => " (bottom)",
                    _ if index + 1 == count => " (top)",
                    _ => "",
                };
                let mut text =
                    format!("Rung {}{place} {}", index + 1, escape_markdown(&rung.title));
                if let Some(who) = &rung.who {
                    text.push_str(&format!(", {}", escape_markdown(who)));
                }
                if let Some(actor) = &rung.actor {
                    text.push_str(&format!(
                        "; actor {}",
                        escape_markdown(&self.actor_name(actor))
                    ));
                }
                if rung.planned {
                    text.push_str("; planned");
                }
                lines.push(format!(
                    "{indent}- {text}: {} {pointer}",
                    escape_markdown(&rung.text)
                ));
                let marks = stencil_model::PageMarks::rung_marks(ladder, &ladder_pointer, index);
                self.mark_lines(&marks, |_| true, &nested, lines);
                return;
            }
            (NodeTag::HowTo, _) => format!("How-to column. {pointer}"),
            // Each item is a section after the outline, with its terminal in a fence.
            (NodeTag::HowToItem, _) => return,
            (NodeTag::GeneralNotes, Some(NodeRef::Node(Node::GeneralNotes(notes)))) => {
                let mut text = escape_markdown(&notes.title);
                if let Some(note) = &notes.note {
                    text.push_str(&format!(": {}", escape_markdown(note)));
                }
                lines.push(format!("{indent}- {text}. {pointer}"));
                let notes_pointer = SheetList::GeneralNotes.pointer_in(&node.pointer);
                for (index, note) in notes.notes.iter().enumerate() {
                    let note_pointer = notes_pointer.index(index);
                    let mut text = format!(
                        "Note {} ({}): {}",
                        index + 1,
                        escape_markdown(&note.id),
                        escape_markdown(&note.text)
                    );
                    if let Some(detail) = &note.detail {
                        text.push(' ');
                        text.push_str(&escape_markdown(detail));
                    }
                    lines.push(format!(
                        "{nested}- {text} Zones {}. {}",
                        self.zones_cell(&note_pointer),
                        self.spans(&note_pointer)
                    ));
                }
                if let Some(foot) = &notes.foot {
                    lines.push(format!("{nested}- Foot: {}", escape_markdown(foot)));
                }
                return;
            }
            (
                NodeTag::Zone
                | NodeTag::Pcard
                | NodeTag::Fact
                | NodeTag::Note
                | NodeTag::Text
                | NodeTag::Callout
                | NodeTag::Frame
                | NodeTag::Pipe
                | NodeTag::Tee
                | NodeTag::Phases
                | NodeTag::Step
                | NodeTag::PartsList
                | NodeTag::GeneralNotes
                | NodeTag::Ladder,
                _,
            ) => format!(
                "{} {}. {pointer}",
                node.tag.as_str(),
                escape_markdown(&self.name_of(&node.pointer))
            ),
            // A View is a section of its own and the title block's nodes are written by
            // the title block section.
            (
                NodeTag::Page
                | NodeTag::Kicker
                | NodeTag::Title
                | NodeTag::Lede
                | NodeTag::Body
                | NodeTag::Legend
                | NodeTag::LegendEntry
                | NodeTag::Foot
                | NodeTag::TitleBlock
                | NodeTag::TitleCell
                | NodeTag::ProblemPoint
                | NodeTag::View
                | NodeTag::ActorChip,
                _,
            ) => return,
        };
        lines.push(format!("{indent}- {text}"));
    }

    /// A Step's outline line: `Step <title>. Lane <actor>, phase <n> <title>, column <c>[ to
    /// <d>][, look <look>][, fill <fill>].` The fill is named when the Step sets one or
    /// when the one it takes differs from its look's.
    fn step_line(&self, step: &Step) -> String {
        let phases = self.phases_of(step);
        let fill = step.fill_under(phases.and_then(|phases| phases.step_fill));
        let fill_text = if step.look != StepLook::Text
            && (step.fill.is_some() || fill != step.look.default_fill())
        {
            format!(", fill {}", fill.as_str())
        } else {
            String::new()
        };
        let phase = phases.and_then(|phases| {
            phases
                .phases
                .iter()
                .position(|phase| phase.id == step.phase)
                .and_then(|index| Some((index + 1, phases.phases.get(index)?)))
        });
        let phase_text = match phase {
            Some((number, phase)) => format!("phase {number} {}", escape_markdown(&phase.title)),
            None => format!("phase {}", escape_markdown(&step.phase)),
        };
        let last = u16::from(step.col) + u16::from(step.span) - 1;
        let columns = if step.span > 1 {
            format!("column {} to {last}", step.col)
        } else {
            format!("column {}", step.col)
        };
        let look = match step.look {
            StepLook::Box => String::new(),
            other => format!(", look {}", other.as_str()),
        };
        format!(
            "Step {}. Lane {}, {phase_text}, {columns}{look}{fill_text}.",
            escape_markdown(&step.title),
            escape_markdown(&self.actor_name(&step.lane))
        )
    }

    /// The page's Ladders in body order, up to the eight vet allows.
    fn ladders(&self) -> Vec<&'a Ladder> {
        body_nodes(self.page)
            .into_iter()
            .filter_map(|entry| match entry.node {
                NodeRef::Node(Node::Ladder(ladder)) => Some(ladder),
                _ => None,
            })
            .take(LADDERS_MAX)
            .collect()
    }

    /// The letter a ladder's detail is drawn with: A for the first Ladder in body order.
    fn detail_letter_of(&self, ladder: &Ladder) -> String {
        let index = self
            .ladders()
            .iter()
            .position(|candidate| std::ptr::eq(*candidate, ladder))
            .unwrap_or_default();
        let offset = u8::try_from(index).unwrap_or(u8::MAX).min(25);
        char::from(b'A' + offset).to_string()
    }

    /// The Steps a ladder frames, in `around` order.
    fn framed_steps(&self, ladder: &Ladder) -> Vec<&'a Step> {
        ladder
            .around
            .iter()
            .take(SheetList::Around.max() + 1)
            .filter_map(|id| {
                self.nodes.values().copied().find_map(|node| match node {
                    NodeRef::Node(Node::Step(step)) if step.id.as_deref() == Some(id) => Some(step),
                    _ => None,
                })
            })
            .collect()
    }

    fn framed_titles(&self, ladder: &Ladder) -> Vec<String> {
        self.framed_steps(ladder)
            .iter()
            .map(|step| escape_markdown(&step.title))
            .collect()
    }

    /// The Ladder holding the rung at `pointer`, its pointer and the rung's index.
    fn rung_at(&self, pointer: &NodePointer) -> Option<(&'a Ladder, NodePointer, usize)> {
        let rungs = SheetList::Rungs;
        body_nodes(self.page).into_iter().find_map(|entry| {
            let NodeRef::Node(Node::Ladder(ladder)) = entry.node else {
                return None;
            };
            (0..ladder.rungs.len().min(rungs.max() + 1))
                .find(|index| rungs.pointer_in(&entry.pointer).index(*index) == *pointer)
                .map(|index| (ladder, entry.pointer.clone(), index))
        })
    }

    /// The Phases node that holds a Step.
    fn phases_of(&self, step: &Step) -> Option<&'a Phases> {
        self.nodes.values().copied().find_map(|node| match node {
            NodeRef::Node(Node::Phases(phases))
                if phases.children.iter().any(|child| match child {
                    Node::Step(candidate) => std::ptr::eq(candidate, step),
                    _ => false,
                }) =>
            {
                Some(phases)
            }
            _ => None,
        })
    }

    /// The Phases node line, its Steps nested under it, then a table of lanes by phases
    /// whose cells list each lane's Steps in that phase in column order. The outline goes
    /// on in a new list after the table.
    fn phases_outline(
        &self,
        node: &stencil_layout::NodeGeometry,
        phases: &Phases,
        depth: usize,
        lines: &mut Vec<String>,
    ) {
        let indent = "  ".repeat(depth);
        let lanes: Vec<String> = phases
            .lanes
            .iter()
            .map(|lane| escape_markdown(&self.actor_name(lane)))
            .collect();
        let band = match phases.band {
            BandLook::Wash => String::new(),
            other => format!("; band {}", other.as_str()),
        };
        let step_fill = phases.step_fill.map_or(String::new(), |fill| {
            format!("; step fill {}", fill.as_str())
        });
        let mut details = String::new();
        for ladder in self.ladders() {
            let framed = self.framed_steps(ladder);
            let in_node = framed.first().is_some_and(|step| {
                phases.children.iter().any(|child| match child {
                    Node::Step(candidate) => std::ptr::eq(candidate, *step),
                    _ => false,
                })
            });
            if in_node {
                details.push_str(&format!(
                    "; detail {} frames {}",
                    self.detail_letter_of(ladder),
                    self.framed_titles(ladder).join(", ")
                ));
            }
        }
        lines.push(format!(
            "{indent}- Phases: lanes {}; phases {}{band}{step_fill}{details}. {}",
            lanes.join(", "),
            phase_titles(phases).join(", "),
            self.spans(&node.pointer)
        ));
    }

    /// A table with one row per lane and one column per phase, whose header cells carry
    /// the phase spans and lane cells the lane spans, each cell listing that lane's Steps
    /// in the phase in column order.
    fn phases_table(&self, node: &stencil_layout::NodeGeometry, phases: &Phases) -> String {
        let lanes_pointer = SheetList::Lanes.pointer_in(&node.pointer);
        let phases_pointer = SheetList::Phases.pointer_in(&node.pointer);
        let mut header = vec!["Lane".to_string()];
        for (index, title) in phase_titles(phases).iter().enumerate() {
            header.push(format!(
                "{title} {}",
                self.spans(&phases_pointer.index(index))
            ));
        }
        let mut rows = Vec::new();
        for (lane_index, lane) in phases.lanes.iter().enumerate() {
            let mut row = vec![format!(
                "{} {}",
                escape_markdown(&self.actor_name(lane)),
                self.spans(&lanes_pointer.index(lane_index))
            )];
            for phase in &phases.phases {
                let mut steps: Vec<&Step> = phases
                    .children
                    .iter()
                    .filter_map(|child| match child {
                        Node::Step(step) if &step.lane == lane && step.phase == phase.id => {
                            Some(step)
                        }
                        _ => None,
                    })
                    .collect();
                steps.sort_by_key(|step| step.col);
                let titles: Vec<String> =
                    steps.iter().map(|step| cell_value(&step.title)).collect();
                row.push(if titles.is_empty() {
                    "none".to_string()
                } else {
                    titles.join("; ")
                });
            }
            rows.push(row);
        }
        let header: Vec<&str> = header.iter().map(String::as_str).collect();
        table(&header, &rows)
    }

    /// `### Parts list` and a row per part: its number, name, what, why, actor, zones and
    /// pointer.
    fn parts_table(&self, node: &stencil_layout::NodeGeometry, list: &PartsList) -> String {
        let parts_pointer = SheetList::Parts.pointer_in(&node.pointer);
        let rows: Vec<Vec<String>> = list
            .parts
            .iter()
            .enumerate()
            .take(SheetList::Parts.max() + 1)
            .map(|(index, part)| {
                let pointer = parts_pointer.index(index);
                let mut name = escape_markdown(&self.page_marks.part_name(part));
                if self.page_marks.part_planned(part) {
                    name.push_str(", planned");
                }
                vec![
                    (index + 1).to_string(),
                    name,
                    self.page_marks
                        .part_what(part)
                        .map_or_else(|| "none".to_string(), |what| escape_markdown(&what)),
                    escape_markdown(&part.why),
                    self.actor_cell(self.page_marks.part_actor(part)),
                    self.zones_cell(&pointer),
                    self.spans(&pointer),
                ]
            })
            .collect();
        format!(
            "### {}\n\n{}",
            escape_markdown(&list.title),
            table(
                &["Part", "Name", "What", "Why", "Actor", "Zones", "Pointer"],
                &rows
            )
        )
    }

    /// The legend key of every line use: pipes, Tee spines and arms in body order, then
    /// links in index order.
    fn line_uses(&self) -> Vec<LegendKey> {
        let mut uses = Vec::new();
        for node in &self.geometry.nodes {
            let actor = self.node_line_actor(node);
            match self.nodes.get(node.pointer.as_str()).copied() {
                Some(NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe)) => {
                    uses.push(
                        self.use_key((pipe.line, pipe.tint, pipe.form), actor)
                            .with_planned(pipe.planned),
                    );
                }
                Some(NodeRef::Node(Node::Tee(tee))) => {
                    uses.push(self.use_key((tee.line, tee.tint, PipeForm::Tube), actor));
                }
                Some(NodeRef::Node(
                    Node::Row(_)
                    | Node::Col(_)
                    | Node::Lanes(_)
                    | Node::Box(_)
                    | Node::Item(_)
                    | Node::Fact(_)
                    | Node::Note(_)
                    | Node::Text(_)
                    | Node::Callout(_)
                    | Node::Frame(_)
                    | Node::View(_)
                    | Node::Phases(_)
                    | Node::Step(_)
                    | Node::PartsList(_)
                    | Node::GeneralNotes(_)
                    | Node::Ladder(_)
                    | Node::HowTo(_),
                ))
                | None => {}
            }
        }
        for route in self.geometry.links.iter().take(LINKS_MAX + 1) {
            if let Some(link) = self.page.links.get(route.index) {
                let actor = self.actors.link_actor(link);
                uses.push(
                    self.use_key((link.line, link.tint, PipeForm::Tube), actor)
                        .with_planned(link.planned),
                );
            }
        }
        uses
    }

    fn means(&self, key: &LegendKey) -> String {
        self.page
            .legend
            .iter()
            .find(|entry| entry_key(entry) == *key)
            .map_or_else(
                || "no legend entry".to_string(),
                |entry| escape_markdown(&entry.text),
            )
    }

    /// The Lines table rows of section 14.1 rule 9.
    fn lines_rows(&self) -> Vec<Vec<String>> {
        self.scoped_lines_rows()
            .into_iter()
            .map(|(_, row)| row)
            .collect()
    }

    /// The Lines table rows, each with the geometry index of the View that holds the line:
    /// a pipe's View, or the View that holds both ends of a link. None off a sheet and for
    /// a link between two Views.
    fn scoped_lines_rows(&self) -> Vec<(Option<usize>, Vec<String>)> {
        let mut rows = Vec::new();
        for (index, node) in self.geometry.nodes.iter().enumerate() {
            let pointer = self.spans(&node.pointer);
            let view = self.view_of(index);
            match self.nodes.get(node.pointer.as_str()).copied() {
                Some(NodeRef::Node(Node::Pipe(pipe))) => {
                    let kind = match pipe.dir {
                        PipeDir::Horizontal => "pipe h",
                        PipeDir::Vertical => "pipe v",
                    };
                    let actor = self.node_line_actor(node);
                    rows.push((
                        view,
                        self.pipe_row(&node.pointer, (pipe, actor), kind, pointer),
                    ));
                }
                Some(NodeRef::TeeArm(pipe)) => {
                    let actor = self.node_line_actor(node);
                    rows.push((
                        view,
                        self.pipe_row(&node.pointer, (pipe, actor), "tee arm", pointer),
                    ));
                }
                Some(NodeRef::Node(Node::Tee(tee))) => {
                    let actor = self.node_line_actor(node);
                    rows.push((view, self.spine_row((tee, actor), pointer)));
                }
                Some(NodeRef::Node(
                    Node::Row(_)
                    | Node::Col(_)
                    | Node::Lanes(_)
                    | Node::Box(_)
                    | Node::Item(_)
                    | Node::Fact(_)
                    | Node::Note(_)
                    | Node::Text(_)
                    | Node::Callout(_)
                    | Node::Frame(_)
                    | Node::View(_)
                    | Node::Phases(_)
                    | Node::Step(_)
                    | Node::PartsList(_)
                    | Node::GeneralNotes(_)
                    | Node::Ladder(_)
                    | Node::HowTo(_),
                ))
                | None => {}
            }
        }
        for route in self.geometry.links.iter().take(LINKS_MAX + 1) {
            let Some(link) = self.page.links.get(route.index) else {
                continue;
            };
            let mut line = match &link.label {
                Some(label) => labelled(label, link.sub.as_deref()),
                None => "unlabelled".to_string(),
            };
            if let Some(step) = link.step.filter(|_| self.page.sheet.is_some()) {
                line = format!("step {step}: {line}");
            }
            let kind = match link.order {
                Some(order) => format!("message, order {order}"),
                None => "link".to_string(),
            };
            let endpoint = |index: usize, id: &str| match self.geometry.nodes.get(index) {
                Some(node) => format!(
                    "{} ({})",
                    escape_markdown(&self.name_of(&node.pointer)),
                    escape_markdown(id)
                ),
                None => format!("({})", escape_markdown(id)),
            };
            let mut ends = self.link_ends(route);
            if link.valve && self.page.sheet.is_some() {
                ends.push_str(", valve before the end");
            }
            if route.status == RouteStatus::Fallback {
                ends.push_str(", fallback route");
            }
            let actor = self.actors.link_actor(link);
            let key = self
                .use_key((link.line, link.tint, PipeForm::Tube), actor)
                .with_planned(link.planned);
            let from_view = self.view_of(route.from_node);
            let view = from_view.filter(|_| from_view == self.view_of(route.to_node));
            rows.push((
                view,
                self.line_row(
                    vec![
                        line,
                        kind,
                        endpoint(route.from_node, &link.from),
                        endpoint(route.to_node, &link.to),
                        self.use_style(&key, actor),
                        ends,
                        self.means(&key),
                    ],
                    (actor, &PageMarks::link_marks(link, route.index)),
                    self.spans(&links_pointer().index(route.index)),
                ),
            ));
        }
        rows
    }

    /// The Ends cell of a link: its drawn arrowheads, with `dot start` or `dot end` for an
    /// end on a rail, which draws a dot in place of an arrowhead.
    fn link_ends(&self, route: &stencil_layout::LinkRoute) -> String {
        let on_rail = |index: usize| {
            self.geometry
                .nodes
                .get(index)
                .and_then(stencil_layout::rail_bar)
                .is_some()
        };
        let (start_dot, end_dot) = (on_rail(route.from_node), on_rail(route.to_node));
        if !start_dot && !end_dot {
            return arrow_ends(route.arrow).to_string();
        }
        let mut ends = Vec::new();
        if start_dot {
            ends.push("dot start");
        }
        match route.arrow {
            Arrow::None => {}
            arrow => ends.push(arrow_ends(arrow)),
        }
        if end_dot {
            ends.push("dot end");
        }
        ends.join(", ")
    }

    /// The From or To cell of a rail: `rail`, then each link that ends on it (`in from`) or
    /// starts on it (`out to`), in link order.
    fn rail_cell(&self, rail: &NodePointer, incoming: bool) -> String {
        let mut cell = "rail".to_string();
        for route in self.geometry.links.iter().take(LINKS_MAX + 1) {
            let Some(link) = self.page.links.get(route.index) else {
                continue;
            };
            let (here, there, there_id, word) = if incoming {
                (route.to_node, route.from_node, &link.from, "in from")
            } else {
                (route.from_node, route.to_node, &link.to, "out to")
            };
            if self.geometry.nodes.get(here).map(|node| &node.pointer) != Some(rail) {
                continue;
            }
            let name = self
                .geometry
                .nodes
                .get(there)
                .map(|node| self.name_of(&node.pointer))
                .unwrap_or_default();
            cell.push_str(&format!(
                "; {word} {} ({})",
                cell_value(&name),
                cell_value(there_id)
            ));
        }
        cell
    }

    /// A Lines row: its cells, then on a sheet the Actor and Marks cells, then the pointer.
    fn line_row(
        &self,
        mut cells: Vec<String>,
        (actor, marks): (Option<&str>, &[stencil_model::CarrierMark]),
        pointer: String,
    ) -> Vec<String> {
        if self.page.sheet.is_some() {
            cells.push(self.actor_cell(actor));
            cells.push(self.marks_cell(marks));
        }
        cells.push(pointer);
        cells
    }

    /// The geometry index of the View that holds the node at `index`, if any.
    fn view_of(&self, index: usize) -> Option<usize> {
        let mut current = Some(index);
        for _ in 0..self.geometry.nodes.len() {
            let node_index = current?;
            let node = self.geometry.nodes.get(node_index)?;
            if node.tag == NodeTag::View {
                return Some(node_index);
            }
            current = node.parent;
        }
        None
    }

    /// A design sheet in place of the figure, drawing, lines and legend sections: the
    /// sheet paragraph, the title block, the legend, then each View with its outline and
    /// its lines.
    fn sheet(&self, sheet: &Sheet, blocks: &mut Vec<String>) {
        blocks.push("## Sheet".to_string());
        blocks.push(self.figure_paragraph());
        self.title_block(sheet, blocks);
        self.who_acts(blocks);
        let legend_pointer = NodePointer::root().child("legend");
        if self.geometry.node(&legend_pointer).is_some() {
            blocks.push(format!("## Legend {}", self.spans(&legend_pointer)));
            blocks.push(table(
                &["Key", "Center label", "Means", "Uses", "Pointer"],
                &self.legend_rows(),
            ));
        }
        let kinds = stencil_layout::marks::sampled_kinds(self.page);
        let draws_steps = stencil_layout::marks::draws_step_badges(self.page);
        if !kinds.is_empty() || draws_steps {
            let mut kinds: Vec<String> = kinds
                .into_iter()
                .map(|kind| {
                    let text = sheet
                        .marks
                        .text(kind)
                        .unwrap_or_else(|| kind.default_legend());
                    format!(
                        "{} ({}, {})",
                        escape_markdown(text),
                        kind.glyph(),
                        kind.list()
                    )
                })
                .collect();
            if draws_steps {
                let text = sheet
                    .marks
                    .step
                    .as_deref()
                    .unwrap_or(stencil_layout::marks::STEP_SAMPLE_TEXT);
                kinds.push(format!(
                    "{} (step badge, numbered lines)",
                    escape_markdown(text)
                ));
            }
            blocks.push(format!("Mark kinds: {}.", kinds.join("; ")));
        }
        let scoped = self.scoped_lines_rows();
        let header = [
            "Line", "Kind", "From", "To", "Style", "Ends", "Means", "Actor", "Marks", "Pointer",
        ];
        let mut view_number = 1;
        for (index, node) in self.geometry.nodes.iter().enumerate() {
            let Some(NodeRef::Node(Node::View(view))) =
                self.nodes.get(node.pointer.as_str()).copied()
            else {
                continue;
            };
            view_number += 1;
            blocks.push(format!(
                "## View {view_number}: {} {}",
                escape_markdown(&view.title),
                self.spans(&node.pointer)
            ));
            let mut aside = Vec::new();
            if let Some(subtitle) = &view.subtitle {
                aside.push(format!("Subtitle: {}.", escape_markdown(subtitle)));
            }
            if let Some(note) = &view.note {
                aside.push(format!("Note: {}.", escape_markdown(note)));
            }
            if !aside.is_empty() {
                blocks.push(aside.join(" "));
            }
            let outline = self.outline_under(index);
            if !outline.is_empty() {
                blocks.push(outline);
            }
            let rows: Vec<Vec<String>> = scoped
                .iter()
                .filter(|(scope, _)| *scope == Some(index))
                .map(|(_, row)| row.clone())
                .collect();
            if !rows.is_empty() {
                blocks.push(format!("### Lines in view {view_number}"));
                blocks.push(table(&header, &rows));
            }
        }
        let between: Vec<Vec<String>> = scoped
            .iter()
            .filter(|(scope, _)| scope.is_none())
            .map(|(_, row)| row.clone())
            .collect();
        if !between.is_empty() {
            blocks.push("## Lines between views".to_string());
            blocks.push(table(&header, &between));
        }
        if let Some(foot) = &self.page.foot {
            let foot_pointer = NodePointer::root().child("foot");
            blocks
                .push(self.with_pointer(format!("Foot: {}", escape_markdown(foot)), &foot_pointer));
        }
    }

    /// The title block, view 1: the wordmark line, then the problem, answer and attention
    /// cells, each under its caption.
    fn title_block(&self, sheet: &Sheet, blocks: &mut Vec<String>) {
        let sheet_pointer = NodePointer::root().child("sheet");
        blocks.push("## Title block (view 1)".to_string());
        let wordmark = sheet.wordmark.as_deref().unwrap_or(&self.page.title);
        blocks.push(format!(
            "Wordmark {}. {}",
            escape_markdown(wordmark),
            self.spans(&sheet_pointer)
        ));

        let problem = sheet_pointer.child("problem");
        blocks.push(self.cell_heading(&problem));
        if let Some(lead) = &sheet.problem.lead {
            blocks.push(escape_markdown(lead));
        }
        let points: Vec<Vec<String>> = sheet
            .problem
            .points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                vec![
                    format!("P{}", index + 1),
                    escape_markdown(&point.text),
                    self.marks_cell(&self.page_marks.point_marks(self.page, index)),
                    self.spans(&SheetList::ProblemPoints.pointer().index(index)),
                ]
            })
            .collect();
        blocks.push(table(&["Point", "Text", "Marks", "Pointer"], &points));

        let answer = sheet_pointer.child("answer");
        blocks.push(self.cell_heading(&answer));
        blocks.push(escape_markdown(&sheet.answer.statement));
        for paragraph in &sheet.answer.paragraphs {
            blocks.push(match paragraph.lead() {
                Some(lead) => format!(
                    "{} {}",
                    escape_markdown(lead),
                    escape_markdown(paragraph.text())
                ),
                None => escape_markdown(paragraph.text()),
            });
        }

        let attention = sheet_pointer.child("attention");
        blocks.push(self.cell_heading(&attention));
        let items: Vec<Vec<String>> = sheet
            .attention
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let pointer = SheetList::AttentionItems.pointer().index(index);
                vec![
                    format!("diamond {}", index + 1),
                    format!(
                        "{} {}",
                        escape_markdown(&item.lead),
                        escape_markdown(&item.text)
                    ),
                    self.actor_cell(item.actor.as_deref()),
                    self.zones_cell(&pointer),
                    self.spans(&pointer),
                ]
            })
            .collect();
        blocks.push(table(
            &["Mark", "Item", "Actor", "Zones", "Pointer"],
            &items,
        ));
    }

    /// `## Who acts`: one row per actor, its pointer the chip's in the legend strip.
    fn who_acts(&self, blocks: &mut Vec<String>) {
        if self.page.actors.is_empty() {
            return;
        }
        blocks.push("## Who acts".to_string());
        let rows: Vec<Vec<String>> = self
            .page
            .actors
            .iter()
            .enumerate()
            .take(SheetList::Actors.max() + 1)
            .map(|(index, actor)| {
                let colour = TINT_NAMES
                    .get(usize::from(actor.tint).wrapping_sub(1))
                    .copied()
                    .unwrap_or("blue");
                vec![
                    escape_markdown(&actor.name),
                    escape_markdown(&actor.id),
                    format!("tint {}; {colour} in center", actor.tint),
                    actor
                        .text
                        .as_deref()
                        .map_or_else(|| "none".to_string(), escape_markdown),
                    self.spans(&SheetList::Actors.pointer().index(index)),
                ]
            })
            .collect();
        blocks.push(table(&["Actor", "Id", "Colour", "Text", "Pointer"], &rows));
    }

    /// `### <caption>` with the cell's spans.
    fn cell_heading(&self, cell: &NodePointer) -> String {
        let caption = title_cell_caption(self.page, cell).unwrap_or_default();
        format!("### {} {}", escape_markdown(&caption), self.spans(cell))
    }

    fn pipe_row(
        &self,
        pointer: &NodePointer,
        (pipe, actor): (&Pipe, Option<&str>),
        kind: &str,
        pointer_span: String,
    ) -> Vec<String> {
        let kind = match pipe.form {
            PipeForm::Tube => kind.to_string(),
            PipeForm::Band => format!("{kind}, band"),
            PipeForm::Rail => format!("{kind}, rail"),
        };
        let key = self
            .use_key((pipe.line, pipe.tint, pipe.form), actor)
            .with_planned(pipe.planned);
        let (from, to) = match pipe.form {
            PipeForm::Rail => (
                self.rail_cell(pointer, true),
                self.rail_cell(pointer, false),
            ),
            PipeForm::Tube | PipeForm::Band => (
                self.pipe_end(pointer, PipeEndSide::Start, pipe.from.as_deref()),
                self.pipe_end(pointer, PipeEndSide::End, pipe.to.as_deref()),
            ),
        };
        let ends = match pipe.form {
            PipeForm::Rail => "no arrowheads".to_string(),
            PipeForm::Tube | PipeForm::Band => arrow_ends(pipe.arrow).to_string(),
        };
        let mut name = pipe_line_name(pipe);
        if let Some(step) = pipe.step.filter(|_| self.page.sheet.is_some()) {
            name = format!("step {step}: {name}");
        }
        self.line_row(
            vec![
                name,
                kind,
                from,
                to,
                self.use_style(&key, actor),
                ends,
                self.means(&key),
            ],
            (actor, &self.node_marks(pointer)),
            pointer_span,
        )
    }

    fn spine_row(&self, (tee, actor): (&Tee, Option<&str>), pointer_span: String) -> Vec<String> {
        let arms: Vec<String> = tee
            .arms
            .iter()
            .map(|arm| {
                let stencil_model::TeeArm::Pipe(pipe) = arm;
                escape_markdown(&pipe.label)
            })
            .collect();
        let key = self.use_key((tee.line, tee.tint, PipeForm::Tube), actor);
        self.line_row(
            vec![
                escape_markdown(&tee.hub),
                "tee spine".to_string(),
                format!("hub {}", escape_markdown(&tee.hub)),
                arms.join(" and "),
                self.use_style(&key, actor),
                arrow_ends(Arrow::None).to_string(),
                self.means(&key),
            ],
            (actor, &[]),
            pointer_span,
        )
    }

    /// One end of a pipe: its target by name and id, else what rule 11 lands it on.
    fn pipe_end(&self, pipe: &NodePointer, end: PipeEndSide, target: Option<&str>) -> String {
        if let Some(id) = target {
            let name = self
                .ids
                .get(id)
                .map(|pointer| self.name_of(pointer))
                .unwrap_or_default();
            return format!("{} ({})", escape_markdown(&name), escape_markdown(id));
        }
        let Some(landing) = self.units.landing(pipe, end) else {
            return "open".to_string();
        };
        match (&landing.node, landing.relation) {
            (None, _) => "open".to_string(),
            (Some(node), "hub") => format!("hub {}", escape_markdown(&self.name_of(node))),
            (Some(node), relation) => {
                format!("{} ({relation})", escape_markdown(&self.name_of(node)))
            }
        }
    }

    /// The Legend table rows of section 14.1 rule 10.
    fn legend_rows(&self) -> Vec<Vec<String>> {
        let uses = self.line_uses();
        let legend_pointer = NodePointer::root().child("legend");
        self.page
            .legend
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let key = entry_key(entry);
                let used = uses.iter().filter(|used_key| **used_key == key).count();
                vec![
                    legend_output_key(&key),
                    canonical_legend_label(entry).to_string(),
                    escape_markdown(&entry.text),
                    used.to_string(),
                    self.spans(&legend_pointer.index(index)),
                ]
            })
            .collect()
    }
}

fn labelled(label: &str, sub: Option<&str>) -> String {
    match sub {
        Some(sub) => format!("{} · {}", escape_markdown(label), escape_markdown(sub)),
        None => escape_markdown(label),
    }
}

fn pipe_line_name(pipe: &Pipe) -> String {
    labelled(&pipe.label, pipe.sub.as_deref())
}

/// `<n> <title>` for each phase of a Phases node, numbered from 1.
fn phase_titles(phases: &Phases) -> Vec<String> {
    phases
        .phases
        .iter()
        .enumerate()
        .map(|(index, phase)| format!("{} {}", index + 1, escape_markdown(&phase.title)))
        .collect()
}

/// `<line>, tint <n>, <actor>; <colour> in center` for a use an actor's slot colours.
fn actor_style(line: Line, slot: u8, actor: &str) -> String {
    let colour = TINT_NAMES
        .get(usize::from(slot).wrapping_sub(1))
        .copied()
        .unwrap_or("blue");
    format!(
        "{}, tint {slot}, {actor}; {colour} in center",
        line.as_str()
    )
}

/// `<line>[, tint <n>]; <colour> in center`.
fn style(key: &LegendKey) -> String {
    let colour = match (key.line, key.tint()) {
        (Line::Gray, _) => "gray",
        (Line::Deny, _) => "red",
        (Line::Solid | Line::Dash, tint) => tint
            .and_then(|slot| TINT_NAMES.get(usize::from(slot).wrapping_sub(1)))
            .copied()
            .unwrap_or("blue"),
    };
    match key.tint() {
        Some(tint) => format!("{}, tint {tint}; {colour} in center", key.line.as_str()),
        None => format!("{}; {colour} in center", key.line.as_str()),
    }
}

fn arrow_ends(arrow: Arrow) -> &'static str {
    match arrow {
        Arrow::None => "no arrowheads",
        Arrow::End => "arrow end",
        Arrow::Start => "arrow start",
        Arrow::Both => "arrows both ends",
    }
}

fn note_kind(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Kicker => "kicker",
        NoteKind::H1 => "h1",
        NoteKind::Lede => "lede",
        NoteKind::Legend => "legend",
        NoteKind::Foot => "foot",
    }
}

fn callout_kind(kind: CalloutKind) -> &'static str {
    match kind {
        CalloutKind::Note => "note",
        CalloutKind::Risk => "risk",
        CalloutKind::Decision => "decision",
        CalloutKind::Open => "open",
    }
}

/// One code span of a description line.
struct Span<'t> {
    text: &'t str,
    /// Byte offsets of the opening backtick and one past the closing one.
    start: usize,
    end: usize,
    closed: bool,
}

/// A terminal in a fenced `text` block one backtick longer than its longest backtick run,
/// at least three. A line is written raw after its kind's prefix (`$ `, `# `, two spaces,
/// `ok `) and two spaces per indent level, so its first two characters give its kind.
fn terminal_fence(lines: &[stencil_model::TerminalLine]) -> String {
    let lines: Vec<&stencil_model::TerminalLine> = lines
        .iter()
        .take(SheetList::TerminalLines.max() + 1)
        .collect();
    let longest_run = lines
        .iter()
        .map(|line| longest_backtick_run(&line.text))
        .max()
        .unwrap_or(0);
    let fence = "`".repeat((longest_run + 1).max(3));
    let mut text = format!("{fence}text");
    for line in lines {
        let prefix = match line.kind {
            stencil_model::TerminalKind::Command => "$ ",
            stencil_model::TerminalKind::Comment => "# ",
            stencil_model::TerminalKind::Output => "  ",
            stencil_model::TerminalKind::Ok => "ok ",
        };
        text.push('\n');
        text.push_str(prefix);
        text.push_str(&"  ".repeat(usize::from(line.indent)));
        text.push_str(&line.text);
    }
    text.push('\n');
    text.push_str(&fence);
    text
}

fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for character in text.chars() {
        if character == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

struct ParsedLine<'t> {
    number: usize,
    text: &'t str,
    spans: Vec<Span<'t>>,
}

/// The lines outside fenced blocks with their code spans, and the count of the figure
/// paragraph when there is one.
fn parse_description(text: &str) -> (Vec<ParsedLine<'_>>, Option<u64>) {
    let mut lines = Vec::new();
    let mut fence: Option<usize> = None;
    let mut after_heading = false;
    let mut figure_count = None;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let backticks = line
            .chars()
            .take_while(|character| *character == '`')
            .count();
        match fence {
            Some(opened) => {
                if backticks >= opened && line.trim_start_matches('`').trim().is_empty() {
                    fence = None;
                }
                continue;
            }
            None if backticks >= 3 => {
                fence = Some(backticks);
                continue;
            }
            None => {}
        }
        if line == "## Figure" || line == "## Sheet" {
            after_heading = true;
        } else if after_heading && !line.is_empty() {
            after_heading = false;
            if figure_count.is_none() {
                figure_count = drawn_count(line);
            }
        }
        lines.push(ParsedLine {
            number,
            text: line,
            spans: code_spans(line),
        });
    }
    (lines, figure_count)
}

/// `n` from a paragraph that opens `Drawn elements: n`.
fn drawn_count(line: &str) -> Option<u64> {
    let rest = line.strip_prefix("Drawn elements: ")?;
    let digits: String = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// The code spans of one line. A backslash and the character after it are one escape, read
/// in pairs left to right; any other backtick opens or closes a span.
fn code_spans(line: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut open: Option<usize> = None;
    let mut characters = line.char_indices();
    while let Some((offset, character)) = characters.next() {
        match (open, character) {
            (None, '\\') => {
                characters.next();
            }
            (None, '`') => open = Some(offset),
            (Some(start), '`') => {
                spans.push(Span {
                    text: line.get(start + 1..offset).unwrap_or_default(),
                    start,
                    end: offset + 1,
                    closed: true,
                });
                open = None;
            }
            _ => {}
        }
    }
    if let Some(start) = open {
        spans.push(Span {
            text: line.get(start + 1..).unwrap_or_default(),
            start,
            end: line.len(),
            closed: false,
        });
    }
    spans
}

/// A grid reference of section 14.1 rule 13: `B3` or `B2-C5`.
fn is_reference(text: &str) -> bool {
    let cell = |cell: &str| {
        let mut characters = cell.chars();
        matches!(
            (characters.next(), characters.next(), characters.next()),
            (Some('A'..='F'), Some('1'..='8'), None)
        )
    };
    match text.split_once('-') {
        Some((first, last)) => cell(first) && cell(last),
        None => cell(text),
    }
}

/// Section 14.6: every unit's pointer on exactly one line that also holds its name, no span
/// that names nothing, and a figure paragraph whose count is the examined count.
pub fn description_covers_drawing(page: &Page, geometry: &PageGeometry, text: &str) -> CheckReport {
    let units = description_units(page, geometry);
    let (lines, figure_count) = parse_description(text);
    let known: HashSet<&str> = units
        .units
        .iter()
        .map(|unit| unit.pointer.as_str())
        .chain(units.structural.iter().map(NodePointer::as_str))
        .collect();
    let zones: HashMap<&str, Option<String>> = units
        .units
        .iter()
        .map(|unit| {
            (
                unit.pointer.as_str(),
                unit.zone.map(|zone| zone.to_string()),
            )
        })
        .collect();
    let mut lines_by_pointer: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut span_defects = Vec::new();
    for line in &lines {
        // A reference span binds to the pointer span right after it, one space apart.
        let bound_to_next = |position: usize| {
            line.spans.get(position).is_some_and(|current| {
                current.closed
                    && is_reference(current.text)
                    && line.spans.get(position + 1).is_some_and(|next| {
                        next.closed
                            && next.text.starts_with('/')
                            && line.text.get(current.end..next.start) == Some(" ")
                    })
            })
        };
        for (position, current) in line.spans.iter().enumerate() {
            if !(current.closed && current.text.starts_with('/')) {
                if !bound_to_next(position) {
                    span_defects.push(Defect {
                        pointer: NodePointer::root(),
                        message: format!(
                            "line {} has span {}, neither a pointer nor a reference",
                            line.number, current.text
                        ),
                    });
                }
                continue;
            }
            let given = position
                .checked_sub(1)
                .filter(|&before| bound_to_next(before))
                .and_then(|before| line.spans.get(before))
                .map(|reference| reference.text);
            let pointer = units
                .units
                .iter()
                .map(|unit| &unit.pointer)
                .chain(units.structural.iter())
                .find(|known| known.as_str() == current.text)
                .cloned()
                .unwrap_or_else(NodePointer::root);
            let drawn = zones.get(current.text).cloned().flatten();
            let message = match (given, drawn) {
                (Some(given), Some(drawn)) if given == drawn => None,
                (Some(given), Some(drawn)) => Some(format!(
                    "line {} gives {given}, drawn at {drawn}",
                    line.number
                )),
                (None, Some(drawn)) => Some(format!(
                    "line {} gives no reference, drawn at {drawn}",
                    line.number
                )),
                (Some(given), None) => Some(format!(
                    "line {} gives {given} where no reference belongs",
                    line.number
                )),
                (None, None) => None,
            };
            if let Some(message) = message {
                span_defects.push(Defect { pointer, message });
            }
            let numbers = lines_by_pointer.entry(current.text).or_default();
            if numbers.last() != Some(&line.number) {
                numbers.push(line.number);
            }
            if !known.contains(current.text) {
                span_defects.push(Defect {
                    pointer: NodePointer::root(),
                    message: format!(
                        "line {} names {}, which draws nothing",
                        line.number, current.text
                    ),
                });
            }
        }
    }

    let mut defects = Vec::new();
    for unit in &units.units {
        let numbers = lines_by_pointer
            .get(unit.pointer.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        match numbers {
            [] => defects.push(Defect {
                pointer: unit.pointer.clone(),
                message: "not described".to_string(),
            }),
            [number] => {
                // A name in a list-valued cell also escapes its semicolons.
                let escaped = escape_markdown(&unit.name);
                let in_cell = cell_value(&unit.name);
                let holds_name = lines
                    .iter()
                    .find(|line| line.number == *number)
                    .is_some_and(|line| {
                        line.text.contains(&escaped) || line.text.contains(&in_cell)
                    });
                if !holds_name {
                    defects.push(Defect {
                        pointer: unit.pointer.clone(),
                        message: format!("line {number} lacks \"{}\"", unit.name),
                    });
                }
            }
            several => defects.push(Defect {
                pointer: unit.pointer.clone(),
                message: format!("described on {} lines", several.len()),
            }),
        }
    }
    for pointer in &units.structural {
        let count = lines_by_pointer.get(pointer.as_str()).map_or(0, Vec::len);
        if count > 1 {
            defects.push(Defect {
                pointer: pointer.clone(),
                message: format!("described on {count} lines"),
            });
        }
    }
    for (carrier, glyphs, marks) in &units.mark_mismatches {
        defects.push(Defect {
            pointer: carrier.clone(),
            message: format!("draws {glyphs} mark glyphs for {marks} marks"),
        });
    }
    for (key, drawn, expected) in
        stencil_layout::marks::where_used_mismatches(page, geometry, &units)
    {
        defects.push(Defect {
            pointer: key,
            message: format!("where-used reads \"{drawn}\", marks at {expected}"),
        });
    }
    defects.extend(span_defects);
    let examined = u64::try_from(units.units.len()).unwrap_or(u64::MAX);
    match figure_count {
        Some(count) if count == examined => {}
        Some(count) => defects.push(Defect {
            pointer: NodePointer::root(),
            message: format!("the figure paragraph counts {count}, examined {examined}"),
        }),
        None => defects.push(Defect {
            pointer: NodePointer::root(),
            message: "no figure paragraph".to_string(),
        }),
    }
    CheckReport {
        check: CheckName::DescriptionCoversDrawing,
        examined,
        defects,
        not_applicable: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_markdown_escapes_markup_everywhere_and_list_openers_at_the_start() {
        assert_eq!(escape_markdown("a*b_c"), "a\\*b\\_c");
        assert_eq!(escape_markdown("- x - y"), "\\- x - y");
        assert_eq!(escape_markdown("12. x"), "12\\. x");
        assert_eq!(escape_markdown("1) x 2) y"), "1\\) x 2) y");
        assert_eq!(escape_markdown("a | b"), "a \\| b");
        assert_eq!(escape_markdown("\\`"), "\\\\\\`");
        assert_eq!(
            escape_markdown("two metros · 99.99%"),
            "two metros · 99.99%"
        );
    }

    #[test]
    fn code_spans_read_escapes_in_pairs() {
        let spans = code_spans("a \\\\`/body` \\` `/links/0`");
        let texts: Vec<&str> = spans.iter().map(|span| span.text).collect();
        assert_eq!(texts, ["/body", "/links/0"]);
    }

    #[test]
    fn a_reference_is_one_cell_or_a_range_of_two() {
        for reference in ["A1", "F8", "B2-C5"] {
            assert!(is_reference(reference), "{reference}");
        }
        for other in ["G1", "A9", "A1-", "B3-C", "a1", "/body"] {
            assert!(!is_reference(other), "{other}");
        }
    }
}
