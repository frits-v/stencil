//! A design sheet's title block, legend strip and Views (section 14.4 rules 1 to 10). The
//! frame and its grid references are added after layout by `sheet_frame`.

use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{
    AnswerParagraph, LegendEntry, Page, SHEET_ROOT_PADDING_PX, Sheet, VIEW_GAP_DEFAULT_PX, View,
    canvas_width,
};
use taffy::prelude::{
    AlignItems, Dimension, FlexWrap, JustifyContent, LengthPercentageAuto, NodeId, Rect, Style,
    auto, length,
};

use super::{
    Builder, LeadRunLeaf, PartRecord, Placement, RunPiece, TextLeaf, TextSpec,
    apply_flex_placement, base_style, fixed_size, flex_column, flex_row, margins, sides,
};
use crate::styles::{text_color, text_style_for};
use crate::{Axis, LayoutError, NodeTag, PartName, TextAlign};

/// Space between the legend strip and the first View.
pub(crate) const BODY_MARGIN_TOP_PX: f32 = 14.0;
const BLOCK_BORDER_PX: f32 = 2.0;
const CELL_RULE_PX: f32 = 1.0;
/// Cell padding: top, sides, bottom.
const CELL_PADDING_PX: (f32, f32, f32) = (11.0, 14.0, 12.0);
/// Grow weights of the head, problem, answer and attention cells.
const CELL_WEIGHTS: [f32; 4] = [25.0, 47.0, 41.0, 37.0];
const WORDMARK_GAP_PX: f32 = 8.0;
const TITLE_GAP_PX: f32 = 4.0;
const LEDE_GAP_PX: f32 = 6.0;
const CAPTION_GAP_PX: f32 = 8.0;
const CELL_ITEM_GAP_PX: f32 = 6.0;
const POINT_LABEL_PX: f32 = 24.0;
const POINT_RULE_PX: f32 = 1.0;
const POINT_TEXT_GAP_PX: f32 = 5.0;
const STRIP_RULE_PX: f32 = 2.0;
const STRIP_PADDING_PX: (f32, f32) = (7.0, 14.0);
const STRIP_GAP_PX: f32 = 9.0;
const STRIP_ROW_GAP_PX: f32 = 6.0;
const VIEW_MARGIN_TOP_PX: f32 = 13.0;
const VIEW_HEADING_GAP_PX: f32 = 12.0;
const VIEW_HEADING_MARGIN_BOTTOM_PX: f32 = 8.0;
const VIEW_NUMBER_PX: f32 = 28.0;
const VIEW_SUBTITLE_GAP_PX: f32 = 8.0;
const VIEW_RULE_PX: f32 = 1.0;
/// The title block is view 1, so the first View is view 2.
const FIRST_VIEW_NUMBER: usize = 2;

fn top_margin(top: f32) -> Rect<LengthPercentageAuto> {
    margins(top, 0.0, 0.0, 0.0)
}

impl Builder<'_> {
    /// The root of a design sheet: the title block, the legend strip under it, the Views and
    /// the foot, inside a padding that leaves room for the frame.
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
        let root_index = self.push_record(Self::record(
            root_pointer,
            NodeTag::Page,
            None,
            None,
            root,
        ));
        self.add_title_block(page, sheet, root, root_index)?;
        self.add_sheet_legend(&page.legend, root, root_index)?;
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

    fn cell_rule(&mut self, block: NodeId, pointer: &NodePointer) -> Result<PartRecord, LayoutError> {
        let rule = self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: length(CELL_RULE_PX),
                    height: auto(),
                },
                ..base_style()
            },
            block,
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

    /// `/sheet`: four cells at weights 25 : 47 : 41 : 37 with a rule between each pair. The
    /// first holds the page head; the others are the problem, answer and attention cells.
    fn add_title_block(
        &mut self,
        page: &Page,
        sheet: &Sheet,
        root: NodeId,
        root_index: usize,
    ) -> Result<(), LayoutError> {
        let pointer = NodePointer::root().child("sheet");
        let block_style = Style {
            border: sides(
                BLOCK_BORDER_PX,
                BLOCK_BORDER_PX,
                BLOCK_BORDER_PX,
                BLOCK_BORDER_PX,
            ),
            ..flex_row(AlignItems::STRETCH)
        };
        let block = self.container(block_style, root, &pointer)?;
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
        let head = self.container(Self::cell_style(head_weight), block, &pointer)?;
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
            WORDMARK_GAP_PX
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

        let rule = self.cell_rule(block, &pointer)?;
        self.push_part(block_index, rule);
        self.add_problem_cell(sheet, block, block_index, problem_weight)?;
        let rule = self.cell_rule(block, &pointer)?;
        self.push_part(block_index, rule);
        self.add_answer_cell(sheet, block, block_index, answer_weight)?;
        let rule = self.cell_rule(block, &pointer)?;
        self.push_part(block_index, rule);
        self.add_attention_cell(sheet, block, block_index, attention_weight)
    }

    fn push_part(&mut self, record_index: usize, part: PartRecord) {
        if let Some(record) = self.records.get_mut(record_index) {
            record.parts.push(part);
        }
    }

    /// A cell node with its caption, the authored one or the fixed word.
    fn title_cell(
        &mut self,
        block: NodeId,
        weight: f32,
        token: &str,
        caption: (Option<&str>, &str),
    ) -> Result<(NodeId, NodePointer, Vec<PartRecord>), LayoutError> {
        let pointer = NodePointer::root().child("sheet").child(token);
        let cell = self.container(Self::cell_style(weight), block, &pointer)?;
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
        block: NodeId,
        block_index: usize,
        weight: f32,
    ) -> Result<(), LayoutError> {
        let problem = &sheet.problem;
        let (cell, pointer, mut parts) =
            self.title_cell(block, weight, "problem", (problem.caption.as_deref(), "Problem"))?;
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
        self.push_record(record);
        Ok(())
    }

    fn add_answer_cell(
        &mut self,
        sheet: &Sheet,
        block: NodeId,
        block_index: usize,
        weight: f32,
    ) -> Result<(), LayoutError> {
        let answer = &sheet.answer;
        let (cell, pointer, mut parts) =
            self.title_cell(block, weight, "answer", (answer.caption.as_deref(), "Answer"))?;
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
                    CELL_ITEM_GAP_PX,
                    (PartName::ParagraphLead, &led.lead),
                    (PartName::Paragraph, &led.text),
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
        block: NodeId,
        block_index: usize,
        weight: f32,
    ) -> Result<(), LayoutError> {
        let attention = &sheet.attention;
        let (cell, pointer, mut parts) = self.title_cell(
            block,
            weight,
            "attention",
            (attention.caption.as_deref(), "Attention"),
        )?;
        let items_pointer = pointer.child("items");
        let mut gap = CAPTION_GAP_PX;
        for (index, item) in attention.items.iter().enumerate() {
            parts.extend(self.lead_run(
                cell,
                gap,
                (PartName::AttentionLead, &item.lead),
                (PartName::AttentionText, &item.text),
                &items_pointer.index(index),
            )?);
            gap = CELL_ITEM_GAP_PX;
        }
        let mut record = Self::record(pointer, NodeTag::TitleCell, None, Some(block_index), cell);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// A bold lead and its body as one taffy leaf, drawn as two parts: the lead in
    /// `lead_body`, the body in `block_body` continuing on the lead's last line.
    fn lead_run(
        &mut self,
        parent: NodeId,
        top: f32,
        (lead_name, lead): (PartName, &str),
        (body_name, body): (PartName, &str),
        source: &NodePointer,
    ) -> Result<[PartRecord; 2], LayoutError> {
        let lead_style_name = TextStyleName::LeadBody;
        let body_style_name = TextStyleName::BlockBody;
        let lead_style = text_style_for(lead_style_name, false);
        let body_style = text_style_for(body_style_name, false);
        let run = |piece| LeadRunLeaf {
            lead: lead.to_string(),
            lead_style,
            body: body.to_string(),
            body_style,
            piece,
        };
        let lead_index = self.text_leaves.len();
        let node = self
            .tree
            .new_leaf_with_context(
                Style {
                    margin: top_margin(top),
                    ..base_style()
                },
                lead_index,
            )
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
        };
        let lead_leaf = leaf(lead, lead_style, lead_style_name, RunPiece::Lead, "lead");
        let body_leaf = leaf(body, body_style, body_style_name, RunPiece::Body, "text");
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

    /// `/legend` on a sheet: a strip under the title block with a rule on top and the
    /// entries in a wrapping row after the `Lines` caption.
    fn add_sheet_legend(
        &mut self,
        legend: &[LegendEntry],
        root: NodeId,
        root_index: usize,
    ) -> Result<(), LayoutError> {
        if legend.is_empty() {
            return Ok(());
        }
        let pointer = NodePointer::root().child("legend");
        let strip = self.container(flex_column(AlignItems::STRETCH), root, &pointer)?;
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
        let group = self.container(
            Style {
                flex_wrap: FlexWrap::Wrap,
                gap: taffy::Size {
                    width: length(STRIP_GAP_PX),
                    height: length(STRIP_ROW_GAP_PX),
                },
                padding: sides(vertical, horizontal, vertical, horizontal),
                ..flex_row(AlignItems::CENTER)
            },
            strip,
            &pointer,
        )?;
        let caption = self.sheet_text(
            group,
            PartName::GroupCaption,
            "Lines",
            TextStyleName::SheetLabel,
            0.0,
            pointer.clone(),
        )?;
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Legend,
            None,
            Some(root_index),
            strip,
        );
        record.content_node = group;
        record.parts = vec![
            PartRecord {
                name: PartName::StripRule,
                taffy_node: rule,
                text_leaf: None,
            },
            caption,
        ];
        let legend_index = self.push_record(record);
        for (index, entry) in legend.iter().enumerate() {
            self.add_legend_entry(entry, pointer.index(index), group, legend_index)?;
        }
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
                    margin: margins(
                        0.0,
                        0.0,
                        0.0,
                        VIEW_SUBTITLE_GAP_PX - VIEW_HEADING_GAP_PX,
                    ),
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
