//! A design sheet's title block, the legend strip inside it, and Views (section 14.4 rules
//! 1 to 10). The frame and its grid references are added after layout by `sheet_frame`.

use crate::marks::STEP_SAMPLE_TEXT;
use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{
    Actor, AnswerParagraph, LegendEntry, Page, SHEET_ROOT_PADDING_PX, Sheet, SheetList,
    VIEW_GAP_DEFAULT_PX, View, canvas_width,
};
use taffy::prelude::{
    AlignItems, Dimension, FlexWrap, JustifyContent, LengthPercentageAuto, NodeId, Rect, Style,
    auto, length,
};

use super::{
    Builder, LeadRunLeaf, PartRecord, Placement, RunPiece, TextLeaf, TextSpec,
    apply_flex_placement, base_style, fixed_size, flex_column, flex_row, margins, sides,
};
use crate::marks::WHERE_USED_PREFIX;
use crate::styles::{text_color, text_style_for};
use crate::{Axis, LayoutError, NodeTag, PartName, TextAlign};
use stencil_model::MarkKind;
use stencil_model::marks::MarkPlace;

/// Space between the title block and the first View.
/// The styles of a lead run and the fields its two pieces come from.
pub(super) struct RunStyles {
    pub lead: TextStyleName,
    pub body: TextStyleName,
    pub lead_field: &'static str,
    pub body_field: &'static str,
}

/// An attention item's or an answer paragraph's lead and text.
const ITEM_RUN: RunStyles = RunStyles {
    lead: TextStyleName::LeadBody,
    body: TextStyleName::BlockBody,
    lead_field: "lead",
    body_field: "text",
};

pub(crate) const BODY_MARGIN_TOP_PX: f32 = 14.0;
/// The title block's border, which the writer strokes at the same width.
pub const SHEET_BLOCK_BORDER_PX: f32 = 1.5;
const CELL_RULE_PX: f32 = 1.0;
/// Cell padding: top, sides, bottom.
const CELL_PADDING_PX: (f32, f32, f32) = (11.0, 14.0, 12.0);
/// Grow weights of the head, problem, answer and attention cells, which are their widths in
/// a block whose cells share 1494 px (a 1500 px page). The head cell holds the kicker on one
/// line and the motto on two.
const CELL_WEIGHTS: [f32; 4] = [316.0, 446.0, 394.0, 338.0];
/// Between the kicker row and the wordmark's 55 px line, which holds its 46 px type with
/// 4.5 px above and below.
const WORDMARK_GAP_PX: f32 = 6.0;
/// Between the wordmark's line and the motto, air under the descenders.
const TITLE_GAP_PX: f32 = 6.0;
/// The gap above the title when no wordmark stands over it.
const HEAD_TITLE_GAP_PX: f32 = 8.0;
const LEDE_GAP_PX: f32 = 6.0;
const CAPTION_GAP_PX: f32 = 8.0;
const CELL_ITEM_GAP_PX: f32 = 6.0;
const POINT_LABEL_PX: f32 = 24.0;
const POINT_RULE_PX: f32 = 1.0;
const POINT_TEXT_GAP_PX: f32 = 5.0;
/// Drops `P<n>` in `sheet_label` onto the baseline of its `block_body` text.
const POINT_LABEL_DROP_PX: f32 = 2.5;
/// The hairline between the cells and the legend strip, across the block.
const STRIP_RULE_PX: f32 = 1.0;
const STRIP_PADDING_PX: (f32, f32) = (7.0, 14.0);
const STRIP_GAP_PX: f32 = 9.0;
const STRIP_ROW_GAP_PX: f32 = 6.0;
const ACTOR_SWATCH_PX: (f32, f32) = (18.0, 12.0);
const ACTOR_CHIP_GAP_PX: f32 = 6.0;
const VIEW_MARGIN_TOP_PX: f32 = 13.0;
const VIEW_HEADING_GAP_PX: f32 = 12.0;
const VIEW_HEADING_MARGIN_BOTTOM_PX: f32 = 8.0;
const VIEW_NUMBER_PX: f32 = 28.0;
const VIEW_SUBTITLE_GAP_PX: f32 = 8.0;
const VIEW_RULE_PX: f32 = 1.0;
/// The column an attention item's diamond stands in, and the gap above its zones line.
const ATTENTION_DIAMOND_COLUMN_PX: f32 = 26.0;
const ATTENTION_ZONES_GAP_PX: f32 = 1.0;
/// The title block is view 1, so the first View is view 2.
pub(super) const FIRST_VIEW_NUMBER: usize = 2;

fn top_margin(top: f32) -> Rect<LengthPercentageAuto> {
    margins(top, 0.0, 0.0, 0.0)
}

impl Builder<'_> {
    /// The root of a design sheet: the title block with the legend strip inside its border,
    /// the Views and the foot, inside a padding that leaves room for the frame.
    pub(super) fn add_sheet_page(
        &mut self,
        page: &Page,
        sheet: &Sheet,
    ) -> Result<NodeId, LayoutError> {
        let root_pointer = NodePointer::root();
        let padding = SHEET_ROOT_PADDING_PX;
        let root_style = Style {
            size: taffy::Size {
                width: length(canvas_width(page)),
                height: auto(),
            },
            padding: sides(padding, padding, padding, padding),
            ..flex_column(AlignItems::STRETCH)
        };
        let root = self
            .tree
            .new_with_children(root_style, &[])
            .map_err(Self::taffy_error(&root_pointer))?;
        let root_index =
            self.push_record(Self::record(root_pointer, NodeTag::Page, None, None, root));
        let block = self.add_title_block(page, sheet, root, root_index)?;
        self.add_sheet_legend((&page.actors, &page.legend), block)?;
        self.add_body(page, root, root_index)?;
        if let Some(foot) = &page.foot {
            self.add_page_text(
                root,
                root_index,
                (NodeTag::Foot, "foot"),
                foot,
                TextStyleName::Foot,
                top_margin(12.0),
            )?;
        }
        Ok(root)
    }

    fn cell_style(weight: f32) -> Style {
        let (top, side, bottom) = CELL_PADDING_PX;
        Style {
            flex_grow: weight,
            flex_basis: length(0.0),
            padding: sides(top, side, bottom, side),
            ..flex_column(AlignItems::STRETCH)
        }
    }

    fn cell_rule(
        &mut self,
        cells: NodeId,
        pointer: &NodePointer,
    ) -> Result<PartRecord, LayoutError> {
        let rule = self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: length(CELL_RULE_PX),
                    height: auto(),
                },
                ..base_style()
            },
            cells,
            pointer,
        )?;
        Ok(PartRecord {
            name: PartName::CellRule,
            taffy_node: rule,
            text_leaf: None,
        })
    }

    /// A text part in `parent` with a top margin.
    fn sheet_text(
        &mut self,
        parent: NodeId,
        name: PartName,
        text: &str,
        style_name: TextStyleName,
        top: f32,
        source: NodePointer,
    ) -> Result<PartRecord, LayoutError> {
        let (leaf, leaf_index) = self.text_leaf(
            Style {
                margin: top_margin(top),
                ..base_style()
            },
            Some(parent),
            TextSpec {
                text,
                style_name,
                line: None,
                align: TextAlign::Start,
                source,
            },
        )?;
        Ok(PartRecord {
            name,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        })
    }

    /// `/sheet`: a bordered column of four cells at weights 316 : 446 : 394 : 338 with a rule
    /// between each pair, and the legend strip under them. The first cell holds the page
    /// head; the others are the problem, answer and attention cells. Returns the block's
    /// taffy node and record index, which the legend strip is added to.
    fn add_title_block(
        &mut self,
        page: &Page,
        sheet: &Sheet,
        root: NodeId,
        root_index: usize,
    ) -> Result<(NodeId, usize), LayoutError> {
        let pointer = NodePointer::root().child("sheet");
        let block_style = Style {
            border: sides(
                SHEET_BLOCK_BORDER_PX,
                SHEET_BLOCK_BORDER_PX,
                SHEET_BLOCK_BORDER_PX,
                SHEET_BLOCK_BORDER_PX,
            ),
            ..flex_column(AlignItems::STRETCH)
        };
        let block = self.container(block_style, root, &pointer)?;
        let cells = self.container(flex_row(AlignItems::STRETCH), block, &pointer)?;
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::TitleBlock,
            None,
            Some(root_index),
            block,
        );
        record.parts.push(PartRecord {
            name: PartName::BlockBorder,
            taffy_node: block,
            text_leaf: None,
        });
        let block_index = self.push_record(record);

        let [head_weight, problem_weight, answer_weight, attention_weight] = CELL_WEIGHTS;
        let head = self.container(Self::cell_style(head_weight), cells, &pointer)?;
        self.add_kicker(page, head, block_index, Some("1 \u{b7} "))?;
        let title_style = match &sheet.wordmark {
            Some(wordmark) => {
                let part = self.sheet_text(
                    head,
                    PartName::Wordmark,
                    wordmark,
                    TextStyleName::Wordmark,
                    WORDMARK_GAP_PX,
                    pointer.child("wordmark"),
                )?;
                self.push_part(block_index, part);
                TextStyleName::Motto
            }
            None => TextStyleName::SheetTitle,
        };
        let title_gap = if sheet.wordmark.is_some() {
            TITLE_GAP_PX
        } else {
            HEAD_TITLE_GAP_PX
        };
        self.add_page_text(
            head,
            block_index,
            (NodeTag::Title, "title"),
            &page.title,
            title_style,
            top_margin(title_gap),
        )?;
        self.add_page_text(
            head,
            block_index,
            (NodeTag::Lede, "lede"),
            &page.lede,
            TextStyleName::BlockBody,
            top_margin(LEDE_GAP_PX),
        )?;

        let rule = self.cell_rule(cells, &pointer)?;
        self.push_part(block_index, rule);
        self.add_problem_cell(sheet, cells, block_index, problem_weight)?;
        let rule = self.cell_rule(cells, &pointer)?;
        self.push_part(block_index, rule);
        self.add_answer_cell(sheet, cells, block_index, answer_weight)?;
        let rule = self.cell_rule(cells, &pointer)?;
        self.push_part(block_index, rule);
        self.add_attention_cell(sheet, cells, block_index, attention_weight)?;
        Ok((block, block_index))
    }

    fn push_part(&mut self, record_index: usize, part: PartRecord) {
        if let Some(record) = self.records.get_mut(record_index) {
            record.parts.push(part);
        }
    }

    /// A cell node with its caption, the authored one or the fixed word.
    fn title_cell(
        &mut self,
        cells: NodeId,
        weight: f32,
        token: &str,
        caption: (Option<&str>, &str),
    ) -> Result<(NodeId, NodePointer, Vec<PartRecord>), LayoutError> {
        let pointer = NodePointer::root().child("sheet").child(token);
        let cell = self.container(Self::cell_style(weight), cells, &pointer)?;
        let (authored, fixed) = caption;
        let source = match authored {
            Some(_) => pointer.child("caption"),
            None => pointer.clone(),
        };
        let caption = self.sheet_text(
            cell,
            PartName::Caption,
            authored.unwrap_or(fixed),
            TextStyleName::SheetLabel,
            0.0,
            source,
        )?;
        Ok((cell, pointer, vec![caption]))
    }

    fn add_problem_cell(
        &mut self,
        sheet: &Sheet,
        cells: NodeId,
        block_index: usize,
        weight: f32,
    ) -> Result<(), LayoutError> {
        let problem = &sheet.problem;
        let (cell, pointer, mut parts) = self.title_cell(
            cells,
            weight,
            "problem",
            (problem.caption.as_deref(), "Problem"),
        )?;
        let mut gap = CAPTION_GAP_PX;
        if let Some(lead) = &problem.lead {
            parts.push(self.sheet_text(
                cell,
                PartName::Lead,
                lead,
                TextStyleName::BlockBody,
                gap,
                pointer.child("lead"),
            )?);
            gap = CELL_ITEM_GAP_PX;
        }
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::TitleCell,
            None,
            Some(block_index),
            cell,
        );
        record.parts = parts;
        let cell_index = self.push_record(record);
        let points_pointer = pointer.child("points");
        for (index, point) in problem.points.iter().enumerate() {
            self.add_problem_point(
                cell,
                cell_index,
                (points_pointer.index(index), index),
                &point.text,
                gap,
            )?;
            gap = CELL_ITEM_GAP_PX;
        }
        Ok(())
    }

    /// A point: a dashed rule along its top, then `P<n>` in a 24 px column and the text.
    fn add_problem_point(
        &mut self,
        cell: NodeId,
        cell_index: usize,
        (pointer, index): (NodePointer, usize),
        text: &str,
        top: f32,
    ) -> Result<(), LayoutError> {
        let point = self.container(
            Style {
                margin: top_margin(top),
                ..flex_column(AlignItems::STRETCH)
            },
            cell,
            &pointer,
        )?;
        let rule = self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: auto(),
                    height: length(POINT_RULE_PX),
                },
                ..base_style()
            },
            point,
            &pointer,
        )?;
        let row = self.container(
            Style {
                padding: sides(POINT_TEXT_GAP_PX, 0.0, 0.0, 0.0),
                ..flex_row(AlignItems::START)
            },
            point,
            &pointer,
        )?;
        let label_text = format!("P{}", index + 1);
        let (label, label_leaf) = self.text_leaf(
            Style {
                size: taffy::Size {
                    width: length(POINT_LABEL_PX),
                    height: auto(),
                },
                margin: top_margin(POINT_LABEL_DROP_PX),
                ..base_style()
            },
            Some(row),
            TextSpec {
                text: &label_text,
                style_name: TextStyleName::SheetLabel,
                line: None,
                align: TextAlign::Start,
                source: pointer.clone(),
            },
        )?;
        let (body, body_leaf) = self.text_leaf(
            Style {
                flex_grow: 1.0,
                flex_basis: length(0.0),
                ..base_style()
            },
            Some(row),
            TextSpec {
                text,
                style_name: TextStyleName::BlockBody,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("text"),
            },
        )?;
        let marks = self.page_marks.point_marks(self.page, index);
        let glyphs = self.add_mark_row(row, &marks, MarkPlace::Trailing, &pointer)?;
        let mut record = Self::record(
            pointer,
            NodeTag::ProblemPoint,
            None,
            Some(cell_index),
            point,
        );
        record.parts = vec![
            PartRecord {
                name: PartName::PointRule,
                taffy_node: rule,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::PointLabel,
                taffy_node: label,
                text_leaf: Some(label_leaf),
            },
            PartRecord {
                name: PartName::Text,
                taffy_node: body,
                text_leaf: Some(body_leaf),
            },
        ];
        record.parts.extend(glyphs);
        self.push_record(record);
        Ok(())
    }

    fn add_answer_cell(
        &mut self,
        sheet: &Sheet,
        cells: NodeId,
        block_index: usize,
        weight: f32,
    ) -> Result<(), LayoutError> {
        let answer = &sheet.answer;
        let (cell, pointer, mut parts) = self.title_cell(
            cells,
            weight,
            "answer",
            (answer.caption.as_deref(), "Answer"),
        )?;
        parts.push(self.sheet_text(
            cell,
            PartName::Statement,
            &answer.statement,
            TextStyleName::Statement,
            CAPTION_GAP_PX,
            pointer.child("statement"),
        )?);
        let paragraphs_pointer = pointer.child("paragraphs");
        for (index, paragraph) in answer.paragraphs.iter().enumerate() {
            let paragraph_pointer = paragraphs_pointer.index(index);
            match paragraph {
                AnswerParagraph::Plain(text) => parts.push(self.sheet_text(
                    cell,
                    PartName::Paragraph,
                    text,
                    TextStyleName::BlockBody,
                    CELL_ITEM_GAP_PX,
                    paragraph_pointer,
                )?),
                AnswerParagraph::Led(led) => parts.extend(self.lead_run(
                    cell,
                    Style {
                        margin: top_margin(CELL_ITEM_GAP_PX),
                        ..base_style()
                    },
                    &ITEM_RUN,
                    [
                        (PartName::ParagraphLead, &led.lead),
                        (PartName::Paragraph, &led.text),
                    ],
                    &paragraph_pointer,
                )?),
            }
        }
        let mut record = Self::record(pointer, NodeTag::TitleCell, None, Some(block_index), cell);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    fn add_attention_cell(
        &mut self,
        sheet: &Sheet,
        cells: NodeId,
        block_index: usize,
        weight: f32,
    ) -> Result<(), LayoutError> {
        let attention = &sheet.attention;
        let (cell, pointer, mut parts) = self.title_cell(
            cells,
            weight,
            "attention",
            (attention.caption.as_deref(), "Attention"),
        )?;
        let items_pointer = pointer.child("items");
        let mut gap = CAPTION_GAP_PX;
        for (index, item) in attention.items.iter().enumerate() {
            let item_pointer = items_pointer.index(index);
            let row = self.container(
                Style {
                    margin: top_margin(gap),
                    ..flex_row(AlignItems::START)
                },
                cell,
                &item_pointer,
            )?;
            let diamond_cell = self.container(
                Style {
                    size: taffy::Size {
                        width: length(ATTENTION_DIAMOND_COLUMN_PX),
                        height: auto(),
                    },
                    ..flex_column(AlignItems::START)
                },
                row,
                &item_pointer,
            )?;
            parts.extend(self.add_glyph(
                diamond_cell,
                (PartName::AttentionDiamond, PartName::AttentionNumeral),
                (MarkKind::Attention, index + 1),
                flex_column(AlignItems::CENTER),
                &item_pointer,
            )?);
            let column = self.container(
                Style {
                    flex_grow: 1.0,
                    flex_basis: length(0.0),
                    ..flex_column(AlignItems::STRETCH)
                },
                row,
                &item_pointer,
            )?;
            parts.extend(self.lead_run(
                column,
                base_style(),
                &ITEM_RUN,
                [
                    (PartName::AttentionLead, &item.lead),
                    (PartName::AttentionText, &item.text),
                ],
                &item_pointer,
            )?);
            parts.push(self.where_used_part(
                column,
                PartName::AttentionZones,
                (MarkKind::Attention, index, WHERE_USED_PREFIX),
                Style {
                    margin: top_margin(ATTENTION_ZONES_GAP_PX),
                    ..base_style()
                },
                item_pointer.clone(),
            )?);
            gap = CELL_ITEM_GAP_PX;
        }
        let mut record = Self::record(pointer, NodeTag::TitleCell, None, Some(block_index), cell);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// A bold lead and its body as one taffy leaf, drawn as two parts: the lead in the
    /// run's lead style, the body in its body style continuing on the lead's last line.
    pub(super) fn lead_run(
        &mut self,
        parent: NodeId,
        style: Style,
        styles: &RunStyles,
        [(lead_name, lead), (body_name, body)]: [(PartName, &str); 2],
        source: &NodePointer,
    ) -> Result<[PartRecord; 2], LayoutError> {
        let lead_style_name = styles.lead;
        let body_style_name = styles.body;
        let lead_style = text_style_for(lead_style_name, false);
        let body_style = text_style_for(body_style_name, false);
        let cased = |text: &str, name: TextStyleName| {
            if name.text_style().uppercase {
                text.to_uppercase()
            } else {
                text.to_string()
            }
        };
        let lead = cased(lead, lead_style_name);
        let body = cased(body, body_style_name);
        let run = |piece| LeadRunLeaf {
            lead: lead.clone(),
            lead_style,
            body: body.clone(),
            body_style,
            piece,
        };
        let lead_index = self.text_leaves.len();
        let node = self
            .tree
            .new_leaf_with_context(style, lead_index)
            .map_err(Self::taffy_error(source))?;
        self.attach(parent, node, source)?;
        let leaf = |text: &str, style, style_name, piece, field: &str| TextLeaf {
            text: text.to_string(),
            style,
            color: text_color(style_name, None),
            align: TextAlign::Start,
            axis: Axis::X,
            min_line_px: None,
            source: source.child(field),
            lead_run: Some(run(piece)),
            wraps: true,
        };
        let lead_leaf = leaf(
            &lead,
            lead_style,
            lead_style_name,
            RunPiece::Lead,
            styles.lead_field,
        );
        let body_leaf = leaf(
            &body,
            body_style,
            body_style_name,
            RunPiece::Body,
            styles.body_field,
        );
        self.text_leaves.push(lead_leaf);
        self.text_leaves.push(body_leaf);
        Ok([
            PartRecord {
                name: lead_name,
                taffy_node: node,
                text_leaf: Some(lead_index),
            },
            PartRecord {
                name: body_name,
                taffy_node: node,
                text_leaf: Some(lead_index + 1),
            },
        ])
    }

    /// `/legend` on a sheet: a strip at the foot of the title block, inside its border, with
    /// a hairline across the block on top and a wrapping row per group: `Who acts` with a
    /// chip per actor, then `Lines` with the entries.
    fn add_sheet_legend(
        &mut self,
        (actors, legend): (&[Actor], &[LegendEntry]),
        (block, block_index): (NodeId, usize),
    ) -> Result<(), LayoutError> {
        let kinds = crate::marks::sampled_kinds(self.page);
        let draws_steps = crate::marks::draws_step_badges(self.page);
        if legend.is_empty() && actors.is_empty() && kinds.is_empty() && !draws_steps {
            return Ok(());
        }
        let pointer = NodePointer::root().child("legend");
        let strip = self.container(flex_column(AlignItems::STRETCH), block, &pointer)?;
        let rule = self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: auto(),
                    height: length(STRIP_RULE_PX),
                },
                ..base_style()
            },
            strip,
            &pointer,
        )?;
        let (vertical, horizontal) = STRIP_PADDING_PX;
        let groups = self.container(
            Style {
                gap: taffy::Size {
                    width: length(0.0),
                    height: length(STRIP_ROW_GAP_PX),
                },
                padding: sides(vertical, horizontal, vertical, horizontal),
                ..flex_column(AlignItems::STRETCH)
            },
            strip,
            &pointer,
        )?;
        let mut parts = vec![PartRecord {
            name: PartName::StripRule,
            taffy_node: rule,
            text_leaf: None,
        }];
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Legend,
            None,
            Some(block_index),
            strip,
        );
        record.content_node = groups;
        let legend_index = self.push_record(record);
        if !actors.is_empty() {
            let group = self.strip_group(groups, &pointer)?;
            parts.push(self.sheet_text(
                group,
                PartName::GroupCaption,
                "Who acts",
                TextStyleName::SheetLabel,
                0.0,
                pointer.clone(),
            )?);
            let actors_pointer = SheetList::Actors.pointer();
            for (index, actor) in actors.iter().enumerate() {
                self.add_actor_chip(actor, actors_pointer.index(index), group, legend_index)?;
            }
        }
        if !legend.is_empty() {
            let group = self.strip_group(groups, &pointer)?;
            parts.push(self.sheet_text(
                group,
                PartName::GroupCaption,
                "Lines",
                TextStyleName::SheetLabel,
                0.0,
                pointer.clone(),
            )?);
            for (index, entry) in legend.iter().enumerate() {
                self.add_legend_entry(entry, pointer.index(index), group, legend_index)?;
            }
        }
        if !kinds.is_empty() || draws_steps {
            let group = self.strip_group(groups, &pointer)?;
            parts.push(self.sheet_text(
                group,
                PartName::GroupCaption,
                "Marks",
                TextStyleName::SheetLabel,
                0.0,
                pointer.clone(),
            )?);
            let legend_texts = self.page.sheet.as_ref().map(|sheet| sheet.marks.clone());
            for kind in kinds {
                let sample = self.container(
                    Style {
                        gap: taffy::Size {
                            width: length(ACTOR_CHIP_GAP_PX),
                            height: length(0.0),
                        },
                        ..flex_row(AlignItems::CENTER)
                    },
                    group,
                    &pointer,
                )?;
                parts.extend(self.add_glyph(
                    sample,
                    (PartName::MarkSample, PartName::MarkSampleNumeral),
                    (kind, 1),
                    flex_column(AlignItems::CENTER),
                    &pointer,
                )?);
                let (text, source) = match legend_texts
                    .as_ref()
                    .and_then(|marks| marks.text(kind).map(str::to_string))
                {
                    Some(text) => (
                        text,
                        NodePointer::root()
                            .child("sheet")
                            .child("marks")
                            .child(kind.as_str()),
                    ),
                    None => (kind.default_legend().to_string(), pointer.clone()),
                };
                parts.push(self.sheet_text(
                    sample,
                    PartName::MarkSampleText,
                    &text,
                    TextStyleName::LegendText,
                    0.0,
                    source,
                )?);
            }
            if draws_steps {
                // The step badge sample reads 1, as every other sample does.
                let sample = self.container(flex_row(AlignItems::CENTER), group, &pointer)?;
                parts.extend(self.add_step_badge(sample, 1, &pointer)?);
                let (text, source) =
                    match legend_texts.as_ref().and_then(|marks| marks.step.clone()) {
                        Some(text) => (
                            text,
                            NodePointer::root()
                                .child("sheet")
                                .child("marks")
                                .child("step"),
                        ),
                        None => (STEP_SAMPLE_TEXT.to_string(), pointer.clone()),
                    };
                parts.push(self.sheet_text(
                    sample,
                    PartName::MarkSampleText,
                    &text,
                    TextStyleName::LegendText,
                    0.0,
                    source,
                )?);
            }
        }
        if let Some(record) = self.records.get_mut(legend_index) {
            record.parts = parts;
        }
        Ok(())
    }

    /// One group of the legend strip: its caption and entries in a row that wraps.
    fn strip_group(
        &mut self,
        groups: NodeId,
        pointer: &NodePointer,
    ) -> Result<NodeId, LayoutError> {
        self.container(
            Style {
                flex_wrap: FlexWrap::Wrap,
                gap: taffy::Size {
                    width: length(STRIP_GAP_PX),
                    height: length(STRIP_ROW_GAP_PX),
                },
                ..flex_row(AlignItems::CENTER)
            },
            groups,
            pointer,
        )
    }

    /// `/actors/<i>`: a swatch in the actor's colours, its name and its text, in one row.
    fn add_actor_chip(
        &mut self,
        actor: &Actor,
        pointer: NodePointer,
        group: NodeId,
        legend_index: usize,
    ) -> Result<(), LayoutError> {
        let chip = self.container(
            Style {
                gap: taffy::Size {
                    width: length(ACTOR_CHIP_GAP_PX),
                    height: length(0.0),
                },
                ..flex_row(AlignItems::CENTER)
            },
            group,
            &pointer,
        )?;
        let (swatch_width, swatch_height) = ACTOR_SWATCH_PX;
        let swatch = self.plain_leaf(
            Style {
                size: fixed_size(swatch_width, swatch_height),
                ..base_style()
            },
            chip,
            &pointer,
        )?;
        let mut parts = vec![
            PartRecord {
                name: PartName::Swatch,
                taffy_node: swatch,
                text_leaf: None,
            },
            self.sheet_text(
                chip,
                PartName::ActorName,
                &actor.name,
                TextStyleName::ActorName,
                0.0,
                pointer.child("name"),
            )?,
        ];
        if let Some(text) = &actor.text {
            parts.push(self.sheet_text(
                chip,
                PartName::ActorText,
                text,
                TextStyleName::LegendText,
                0.0,
                pointer.child("text"),
            )?);
        }
        let mut record = Self::record(pointer, NodeTag::ActorChip, None, Some(legend_index), chip);
        record.tint = Some(actor.tint);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// A View: a heading row (number, title, subtitle, a rule to the right edge and the
    /// note), then its children in a column.
    pub(super) fn add_view(
        &mut self,
        view: &View,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
        depth: usize,
    ) -> Result<(), LayoutError> {
        let earlier_views = self
            .records
            .iter()
            .filter(|record| record.tag == NodeTag::View)
            .count();
        let top = if earlier_views == 0 {
            0.0
        } else {
            VIEW_MARGIN_TOP_PX
        };
        let mut style = Style {
            margin: top_margin(top),
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let view_node = self.container(style, parent_container, &pointer)?;
        let heading = self.container(
            Style {
                gap: taffy::Size {
                    width: length(VIEW_HEADING_GAP_PX),
                    height: length(0.0),
                },
                margin: margins(0.0, 0.0, VIEW_HEADING_MARGIN_BOTTOM_PX, 0.0),
                ..flex_row(AlignItems::CENTER)
            },
            view_node,
            &pointer,
        )?;
        let number = self.container(
            Style {
                size: fixed_size(VIEW_NUMBER_PX, VIEW_NUMBER_PX),
                justify_content: Some(JustifyContent::CENTER),
                ..flex_column(AlignItems::CENTER)
            },
            heading,
            &pointer,
        )?;
        let numeral_text = (FIRST_VIEW_NUMBER + earlier_views).to_string();
        let (numeral, numeral_leaf) = self.text_leaf(
            base_style(),
            Some(number),
            TextSpec {
                text: &numeral_text,
                style_name: TextStyleName::ViewNumeral,
                line: None,
                align: TextAlign::Center,
                source: pointer.clone(),
            },
        )?;
        let mut parts = vec![
            PartRecord {
                name: PartName::ViewNumber,
                taffy_node: number,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::ViewNumeral,
                taffy_node: numeral,
                text_leaf: Some(numeral_leaf),
            },
        ];
        let (title, title_leaf) = self.text_leaf(
            base_style(),
            Some(heading),
            TextSpec {
                text: &view.title,
                style_name: TextStyleName::ViewTitle,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("title"),
            },
        )?;
        parts.push(PartRecord {
            name: PartName::ViewTitle,
            taffy_node: title,
            text_leaf: Some(title_leaf),
        });
        if let Some(subtitle) = &view.subtitle {
            let (leaf, leaf_index) = self.text_leaf(
                Style {
                    margin: margins(0.0, 0.0, 0.0, VIEW_SUBTITLE_GAP_PX - VIEW_HEADING_GAP_PX),
                    ..base_style()
                },
                Some(heading),
                TextSpec {
                    text: subtitle,
                    style_name: TextStyleName::ViewNote,
                    line: None,
                    align: TextAlign::Start,
                    source: pointer.child("subtitle"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::ViewSubtitle,
                taffy_node: leaf,
                text_leaf: Some(leaf_index),
            });
        }
        let rule = self.plain_leaf(
            Style {
                flex_grow: 1.0,
                flex_basis: length(0.0),
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: length(VIEW_RULE_PX),
                },
                ..base_style()
            },
            heading,
            &pointer,
        )?;
        parts.push(PartRecord {
            name: PartName::ViewRule,
            taffy_node: rule,
            text_leaf: None,
        });
        if let Some(note) = &view.note {
            let (leaf, leaf_index) = self.text_leaf(
                base_style(),
                Some(heading),
                TextSpec {
                    text: note,
                    style_name: TextStyleName::ViewNoteSmall,
                    line: None,
                    align: TextAlign::Start,
                    source: pointer.child("note"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::ViewNote,
                taffy_node: leaf,
                text_leaf: Some(leaf_index),
            });
        }
        let gap = f32::from(view.gap.unwrap_or(VIEW_GAP_DEFAULT_PX));
        let children = self.container(
            Style {
                gap: taffy::Size {
                    width: length(0.0),
                    height: length(gap),
                },
                ..flex_column(AlignItems::STRETCH)
            },
            view_node,
            &pointer,
        )?;
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::View,
            None,
            Some(parent_index),
            view_node,
        );
        record.content_node = children;
        record.parts = parts;
        let index = self.push_record(record);
        let children_pointer = pointer.child("children");
        for (child_index, child) in view.children.iter().enumerate() {
            self.add_body_node(
                child,
                children_pointer.index(child_index),
                index,
                children,
                Placement::ColumnItem { weight: 0 },
                depth + 1,
            )?;
        }
        Ok(())
    }
}
