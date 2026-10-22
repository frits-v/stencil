use std::collections::{BTreeMap, BTreeSet};

use crate::document::{
    AnswerParagraph, Arrow, BUILTIN_GRAMMARS, BUILTIN_THEMES, CAPTION_SCALARS_MAX, Chrome,
    FACTS_MAX, GeneralNotes, LADDERS_MAX, LANES_MAX, LINE_STEP_MAX, Line, Link, MarkKind, Node,
    Outline, PHASE_COLUMNS_MAX, Page, PartsList, Phases, PipeDir, PipeForm, Projection, Sheet,
    Step, StepLook, TINT_SLOTS, Tee, Text, WORDMARK_SCALARS_MAX, canvas_width, is_valid_id,
};
use crate::grammar::{Grammar, IconPack, PAGE_PARENT};
use crate::marks::{CarrierMark, PageMarks};
use crate::pointer::NodePointer;
use crate::sheet::{PageActors, SheetList, SheetSurface};
use crate::walk::{
    NodeEntry, NodeRef, TextField, body_nodes, push_node_text_fields, push_one_link_text_fields,
    push_page_head_text_fields,
};
use crate::{
    CHILDREN_MAX, DEPTH_MAX, FRAME_HEIGHT_MAX, FRAME_HEIGHT_MIN, GAP_MAX_PX, GROW_WEIGHT_MAX,
    ID_PATTERN, LEGEND_ENTRIES_MAX, LINK_CORNER_MAX_PX, LINK_VIA_MAX, LINKS_MAX, NODES_MAX,
    PAGE_WIDTH_MAX, PAGE_WIDTH_MIN, TEXT_BODY_LINES_MAX, TEXT_SCALARS_MAX,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub pointer: NodePointer,
    pub rule: VetRule,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VetRule {
    TextEmpty,
    TextTooLong,
    TextControlCharacter,
    TextUntrimmed,
    PageWidthOutOfRange,
    BodyEmpty,
    ChildrenEmpty,
    ChildrenTooMany,
    LegendTooLong,
    GapOutOfRange,
    GrowLengthMismatch,
    GrowOutOfRange,
    TeeArmNotHorizontal,
    DepthExceeded,
    NodesExceeded,
    IdMalformed,
    IdDuplicate,
    TextBodyEmpty,
    TextBodyTooLong,
    FrameHeightOutOfRange,
    LinksTooMany,
    LinkSelf,
    LinkUnknownId,
    LinkViaTooMany,
    LinkViaOutside,
    TintOutOfRange,
    GrammarUnknown,
    ThemeUnknown,
    KindUnknown,
    KindParentNotAllowed,
    IconOutsidePack,
    PipeTargetUnknown,
    PipeTargetsEqual,
    PipeTargetOnTeeArm,
    FactsTooMany,
    LanesTooMany,
    LinkOrderOutsideLanes,
    LinkOrderDuplicate,
    LanesInIso,
    CornerOutOfRange,
    NeedsSheet,
    SheetInIso,
    SheetChromeNone,
    SheetBodyNotView,
    ViewNotTopLevel,
    SheetWithoutLines,
    ListEmpty,
    ListTooLong,
    SheetTextTooLong,
    KeyDuplicate,
    ActorTintDuplicate,
    ActorUnknown,
    TintOnActorPage,
    KindTintWithoutActor,
    LineWithoutActor,
    ActorOnNeutralLine,
    ActorOnUntintableKind,
    TeeArmActorDiffers,
    LegendByActorMisused,
    PhasesChildNotStep,
    StepOutsidePhases,
    PhaseLaneTwice,
    StepLaneUnknown,
    StepPhaseUnknown,
    StepSpanOutOfRange,
    StepTextFill,
    MarkUnknown,
    MarkRepeated,
    MarkLegendUnused,
    LinkMarksWithoutTag,
    KeyedListTwice,
    PartNodeUnknown,
    PartNodeTwice,
    PartFieldFromNode,
    PartNameMissing,
    PlannedNotSolid,
    LineIntoPlanned,
    CompartmentsWithoutStore,
    OutlinePersonWithIcon,
    RailNotVertical,
    RailOutsideRow,
    RailWithTargets,
    StepOutOfRange,
    LinkStepDuplicate,
    ValveWithoutArrow,
    LaddersTooMany,
    LadderAroundNotStep,
    LadderAroundTwoPhases,
}

impl VetRule {
    /// The rules of section 14.3, in table order.
    pub const SHEET: [VetRule; 48] = [
        VetRule::NeedsSheet,
        VetRule::SheetInIso,
        VetRule::SheetChromeNone,
        VetRule::SheetBodyNotView,
        VetRule::ViewNotTopLevel,
        VetRule::SheetWithoutLines,
        VetRule::ListEmpty,
        VetRule::ListTooLong,
        VetRule::SheetTextTooLong,
        VetRule::KeyDuplicate,
        VetRule::ActorTintDuplicate,
        VetRule::ActorUnknown,
        VetRule::TintOnActorPage,
        VetRule::KindTintWithoutActor,
        VetRule::LineWithoutActor,
        VetRule::ActorOnNeutralLine,
        VetRule::ActorOnUntintableKind,
        VetRule::TeeArmActorDiffers,
        VetRule::LegendByActorMisused,
        VetRule::PhasesChildNotStep,
        VetRule::StepOutsidePhases,
        VetRule::PhaseLaneTwice,
        VetRule::StepLaneUnknown,
        VetRule::StepPhaseUnknown,
        VetRule::StepSpanOutOfRange,
        VetRule::StepTextFill,
        VetRule::MarkUnknown,
        VetRule::MarkRepeated,
        VetRule::MarkLegendUnused,
        VetRule::LinkMarksWithoutTag,
        VetRule::KeyedListTwice,
        VetRule::PartNodeUnknown,
        VetRule::PartNodeTwice,
        VetRule::PartFieldFromNode,
        VetRule::PartNameMissing,
        VetRule::PlannedNotSolid,
        VetRule::LineIntoPlanned,
        VetRule::CompartmentsWithoutStore,
        VetRule::OutlinePersonWithIcon,
        VetRule::RailNotVertical,
        VetRule::RailOutsideRow,
        VetRule::RailWithTargets,
        VetRule::StepOutOfRange,
        VetRule::LinkStepDuplicate,
        VetRule::ValveWithoutArrow,
        VetRule::LaddersTooMany,
        VetRule::LadderAroundNotStep,
        VetRule::LadderAroundTwoPhases,
    ];

    /// Kebab-case name used on the CLI, as in section 1.3.
    pub fn as_str(self) -> &'static str {
        match self {
            VetRule::TextEmpty => "text-empty",
            VetRule::TextTooLong => "text-too-long",
            VetRule::TextControlCharacter => "text-control-character",
            VetRule::TextUntrimmed => "text-untrimmed",
            VetRule::PageWidthOutOfRange => "page-width-out-of-range",
            VetRule::BodyEmpty => "body-empty",
            VetRule::ChildrenEmpty => "children-empty",
            VetRule::ChildrenTooMany => "children-too-many",
            VetRule::LegendTooLong => "legend-too-long",
            VetRule::GapOutOfRange => "gap-out-of-range",
            VetRule::GrowLengthMismatch => "grow-length-mismatch",
            VetRule::GrowOutOfRange => "grow-out-of-range",
            VetRule::TeeArmNotHorizontal => "tee-arm-not-horizontal",
            VetRule::DepthExceeded => "depth-exceeded",
            VetRule::NodesExceeded => "nodes-exceeded",
            VetRule::IdMalformed => "id-malformed",
            VetRule::IdDuplicate => "id-duplicate",
            VetRule::TextBodyEmpty => "text-body-empty",
            VetRule::TextBodyTooLong => "text-body-too-long",
            VetRule::FrameHeightOutOfRange => "frame-height-out-of-range",
            VetRule::LinksTooMany => "links-too-many",
            VetRule::LinkSelf => "link-self",
            VetRule::LinkUnknownId => "link-unknown-id",
            VetRule::LinkViaTooMany => "link-via-too-many",
            VetRule::LinkViaOutside => "link-via-outside",
            VetRule::TintOutOfRange => "tint-out-of-range",
            VetRule::GrammarUnknown => "grammar-unknown",
            VetRule::ThemeUnknown => "theme-unknown",
            VetRule::KindUnknown => "kind-unknown",
            VetRule::KindParentNotAllowed => "kind-parent-not-allowed",
            VetRule::IconOutsidePack => "icon-outside-pack",
            VetRule::PipeTargetUnknown => "pipe-target-unknown",
            VetRule::PipeTargetsEqual => "pipe-targets-equal",
            VetRule::PipeTargetOnTeeArm => "pipe-target-on-tee-arm",
            VetRule::FactsTooMany => "facts-too-many",
            VetRule::LanesTooMany => "lanes-too-many",
            VetRule::LinkOrderOutsideLanes => "link-order-outside-lanes",
            VetRule::LinkOrderDuplicate => "link-order-duplicate",
            VetRule::LanesInIso => "lanes-in-iso",
            VetRule::CornerOutOfRange => "corner-out-of-range",
            VetRule::NeedsSheet => "needs-sheet",
            VetRule::SheetInIso => "sheet-in-iso",
            VetRule::SheetChromeNone => "sheet-chrome-none",
            VetRule::SheetBodyNotView => "sheet-body-not-view",
            VetRule::ViewNotTopLevel => "view-not-top-level",
            VetRule::SheetWithoutLines => "sheet-without-lines",
            VetRule::ListEmpty => "list-empty",
            VetRule::ListTooLong => "list-too-long",
            VetRule::SheetTextTooLong => "sheet-text-too-long",
            VetRule::KeyDuplicate => "key-duplicate",
            VetRule::ActorTintDuplicate => "actor-tint-duplicate",
            VetRule::ActorUnknown => "actor-unknown",
            VetRule::TintOnActorPage => "tint-on-actor-page",
            VetRule::KindTintWithoutActor => "kind-tint-without-actor",
            VetRule::LineWithoutActor => "line-without-actor",
            VetRule::ActorOnNeutralLine => "actor-on-neutral-line",
            VetRule::ActorOnUntintableKind => "actor-on-untintable-kind",
            VetRule::TeeArmActorDiffers => "tee-arm-actor-differs",
            VetRule::LegendByActorMisused => "legend-by-actor-misused",
            VetRule::PhasesChildNotStep => "phases-child-not-step",
            VetRule::StepOutsidePhases => "step-outside-phases",
            VetRule::PhaseLaneTwice => "phase-lane-twice",
            VetRule::StepLaneUnknown => "step-lane-unknown",
            VetRule::StepPhaseUnknown => "step-phase-unknown",
            VetRule::StepSpanOutOfRange => "step-span-out-of-range",
            VetRule::StepTextFill => "step-text-fill",
            VetRule::MarkUnknown => "mark-unknown",
            VetRule::MarkRepeated => "mark-repeated",
            VetRule::MarkLegendUnused => "mark-legend-unused",
            VetRule::LinkMarksWithoutTag => "link-marks-without-tag",
            VetRule::KeyedListTwice => "keyed-list-twice",
            VetRule::PartNodeUnknown => "part-node-unknown",
            VetRule::PartNodeTwice => "part-node-twice",
            VetRule::PartFieldFromNode => "part-field-from-node",
            VetRule::PartNameMissing => "part-name-missing",
            VetRule::PlannedNotSolid => "planned-not-solid",
            VetRule::LineIntoPlanned => "line-into-planned",
            VetRule::CompartmentsWithoutStore => "compartments-without-store",
            VetRule::OutlinePersonWithIcon => "outline-person-with-icon",
            VetRule::RailNotVertical => "rail-not-vertical",
            VetRule::RailOutsideRow => "rail-outside-row",
            VetRule::RailWithTargets => "rail-with-targets",
            VetRule::StepOutOfRange => "step-out-of-range",
            VetRule::LinkStepDuplicate => "link-step-duplicate",
            VetRule::ValveWithoutArrow => "valve-without-arrow",
            VetRule::LaddersTooMany => "ladders-too-many",
            VetRule::LadderAroundNotStep => "ladder-around-not-step",
            VetRule::LadderAroundTwoPhases => "ladder-around-two-phases",
        }
    }
}

struct Violations(Vec<Violation>);

impl Violations {
    fn push(&mut self, pointer: NodePointer, rule: VetRule, message: String) {
        self.0.push(Violation {
            pointer,
            rule,
            message,
        });
    }
}

/// The bound of `corner` on a page and on a grammar's line kind.
pub fn is_corner_in_range(corner: f32) -> bool {
    corner.is_finite() && (0.0..=LINK_CORNER_MAX_PX).contains(&corner)
}

/// True when `reference` is a built-in grammar name or a string ending in `.json`, the
/// condition of `grammar-unknown`.
pub fn is_grammar_reference(reference: &str) -> bool {
    BUILTIN_GRAMMARS.contains(&reference) || reference.ends_with(".json")
}

/// True when `reference` is a built-in theme name or a string ending in `.json`, the
/// condition of `theme-unknown`.
pub fn is_theme_reference(reference: &str) -> bool {
    BUILTIN_THEMES.contains(&reference) || reference.ends_with(".json")
}

/// All vet violations against `grammar`, the grammar `page.grammar` resolved to, in
/// document order (section 4.2). Empty means valid.
pub fn validate_page(page: &Page, grammar: &Grammar) -> Vec<Violation> {
    let mut violations = Violations(Vec::new());
    let root = NodePointer::root();

    let mut head_fields = Vec::new();
    push_page_head_text_fields(page, &mut head_fields);
    check_text_fields(&head_fields, &mut violations);

    if !(PAGE_WIDTH_MIN..=PAGE_WIDTH_MAX).contains(&page.width) {
        violations.push(
            root.child("width"),
            VetRule::PageWidthOutOfRange,
            format!(
                "width {} is outside {PAGE_WIDTH_MIN} to {PAGE_WIDTH_MAX}",
                page.width
            ),
        );
    }

    if let Some(reference) = &page.grammar
        && !is_grammar_reference(reference)
    {
        violations.push(
            root.child("grammar"),
            VetRule::GrammarUnknown,
            format!("grammar \"{reference}\" is neither a built-in grammar nor a .json path"),
        );
    }

    if let Some(reference) = &page.theme
        && !is_theme_reference(reference)
    {
        violations.push(
            root.child("theme"),
            VetRule::ThemeUnknown,
            format!("theme \"{reference}\" is neither a built-in theme nor a .json path"),
        );
    }

    let entries = body_nodes(page);
    let has_lanes = entries
        .iter()
        .any(|entry| matches!(entry.node, NodeRef::Node(Node::Lanes(_))));
    if has_lanes && page.projection == Projection::Iso {
        violations.push(
            root.child("projection"),
            VetRule::LanesInIso,
            "a page with Lanes cannot be drawn in iso".to_string(),
        );
    }

    if let Some(corner) = page.corner
        && !is_corner_in_range(corner)
    {
        violations.push(
            root.child("corner"),
            VetRule::CornerOutOfRange,
            format!("corner {corner} is outside 0 to {LINK_CORNER_MAX_PX}"),
        );
    }

    if let Some(sheet) = &page.sheet {
        check_sheet(page, sheet, &entries, &mut violations);
    }

    let body_pointer = root.child("body");
    if page.body.is_empty() {
        violations.push(
            body_pointer.clone(),
            VetRule::BodyEmpty,
            "body has no nodes".to_string(),
        );
    } else if page.body.len() > CHILDREN_MAX {
        violations.push(
            body_pointer.clone(),
            VetRule::ChildrenTooMany,
            children_too_many_message(page.body.len()),
        );
    }
    if entries.len() > NODES_MAX {
        violations.push(
            body_pointer,
            VetRule::NodesExceeded,
            format!("more than {NODES_MAX} nodes"),
        );
    }
    // Well-formed ids of the walked nodes, each with the pointer of the first node holding
    // it. Bounded by the walk, like every other per-node rule.
    let mut ids: BTreeMap<&str, NodePointer> = BTreeMap::new();
    // The nearest Box kind above each walked container, keyed by its pointer, so a child
    // reads its parent's entry. Row, Col and Lanes pass their parent's value through.
    let mut box_context: BTreeMap<NodePointer, &str> = BTreeMap::new();
    // A pipe target may name a node later in document order, so targets are checked
    // against every well-formed id of the walk.
    let all_ids: BTreeSet<&str> = entries
        .iter()
        .filter_map(|entry| entry.node.id())
        .filter(|id| is_valid_id(id))
        .collect();
    // Off a sheet, every sheet surface but the sheet itself is `needs-sheet` where it occurs:
    // `/actors` before the body, a node or its `actor` field with the node, and a legend or
    // link field after the body.
    let mut surfaces: Vec<(NodePointer, SheetSurface)> = Vec::new();
    if page.sheet.is_none() {
        for surface in SheetSurface::ALL {
            if surface == SheetSurface::Sheet {
                continue;
            }
            for pointer in surface.occurrences(page, grammar) {
                surfaces.push((pointer, surface));
            }
        }
    }
    let mut reported: BTreeSet<NodePointer> = BTreeSet::new();
    let actors_pointer = SheetList::Actors.pointer();
    report_needs_sheet(&surfaces, &mut reported, &mut violations, |pointer| {
        *pointer == actors_pointer
    });
    let mut sheet_nodes = SheetNodes {
        actors: PageActors::of(page),
        marks: PageMarks::of(page),
        node_tags: entries
            .iter()
            .filter_map(|entry| entry.node.id().map(|id| (id, entry.node.tag_name())))
            .collect(),
        keyed_lists: BTreeMap::new(),
        phases: BTreeMap::new(),
        on_sheet: page.sheet.is_some(),
        planned_ids: planned_ids(&entries),
        rows: BTreeSet::new(),
        steps: BTreeMap::new(),
        step_parents: step_parents(&entries),
        ladders: 0,
    };
    for entry in &entries {
        let own_fields = [
            entry.pointer.child("actor"),
            entry.pointer.child("marks"),
            entry.pointer.child("corner_mark"),
            entry.pointer.child("outline"),
            entry.pointer.child("compartments"),
            entry.pointer.child("planned"),
            entry.pointer.child("kind"),
            entry.pointer.child("form"),
            entry.pointer.child("step"),
        ];
        let lines_prefix = format!("{}/", entry.pointer.child("lines").as_str());
        let rungs_prefix = format!("{}/", entry.pointer.child("rungs").as_str());
        report_needs_sheet(&surfaces, &mut reported, &mut violations, |pointer| {
            *pointer == entry.pointer
                || own_fields.contains(pointer)
                || pointer.as_str().starts_with(&lines_prefix)
                || pointer.as_str().starts_with(&rungs_prefix)
        });
        check_sheet_placement(page, entry, &mut violations);
        let context = enclosing_box_kind(entry, &box_context);
        check_entry(entry, context, grammar, &mut ids, &mut violations);
        check_pipe_targets(entry, &all_ids, &mut violations);
        if sheet_nodes.on_sheet {
            check_sheet_node(page, entry, grammar, &mut sheet_nodes, &mut violations);
        }
        match entry.node {
            // A Box of an unknown kind is reported once and is transparent for its
            // children, so they are not reported again against a kind that does not exist.
            NodeRef::Node(Node::Box(box_node)) if grammar.container(&box_node.kind).is_some() => {
                box_context.insert(entry.pointer.clone(), box_node.kind.as_str());
            }
            NodeRef::Node(Node::Box(_)) => {
                box_context.insert(entry.pointer.clone(), context);
            }
            NodeRef::Node(
                Node::Row(_) | Node::Col(_) | Node::Lanes(_) | Node::View(_) | Node::Phases(_),
            ) => {
                box_context.insert(entry.pointer.clone(), context);
            }
            _ => {}
        }
    }
    report_needs_sheet(&surfaces, &mut reported, &mut violations, |_| true);

    if page.legend.len() > LEGEND_ENTRIES_MAX {
        violations.push(
            root.child("legend"),
            VetRule::LegendTooLong,
            format!(
                "legend has {} entries, above {LEGEND_ENTRIES_MAX}",
                page.legend.len()
            ),
        );
    }
    let legend_pointer = root.child("legend");
    for (index, entry) in page.legend.iter().enumerate().take(LEGEND_ENTRIES_MAX + 1) {
        check_tint(entry.tint, &legend_pointer.index(index), &mut violations);
        if sheet_nodes.on_sheet {
            check_legend_actor(page, entry, &legend_pointer.index(index), &mut violations);
            check_planned_solid(
                entry.line,
                entry.planned,
                &legend_pointer.index(index),
                &mut violations,
            );
        }
        check_text(
            &TextField {
                pointer: legend_pointer.index(index).child("text"),
                text: &entry.text,
            },
            &mut violations,
        );
    }

    let lane_heads = lane_heads(&entries);
    check_links(page, &ids, &lane_heads, &sheet_nodes, &mut violations);

    violations.0
}

/// `needs-sheet` for each surface `select` picks that has not been reported yet.
fn report_needs_sheet(
    surfaces: &[(NodePointer, SheetSurface)],
    reported: &mut BTreeSet<NodePointer>,
    violations: &mut Violations,
    select: impl Fn(&NodePointer) -> bool,
) {
    for (pointer, surface) in surfaces {
        if select(pointer) && !reported.contains(pointer) {
            reported.insert(pointer.clone());
            violations.push(
                pointer.clone(),
                VetRule::NeedsSheet,
                needs_sheet_message(*surface),
            );
        }
    }
}

/// What the actor and Phases rules read while the body is walked on a sheet.
struct SheetNodes<'a> {
    actors: PageActors<'a>,
    /// The keyed lists the marks name, and the balloon each part puts on its node.
    marks: PageMarks<'a>,
    /// Every well-formed node id with its tag, for `part-node-unknown`.
    node_tags: BTreeMap<&'a str, &'static str>,
    /// The first PartsList and GeneralNotes node walked, for `keyed-list-twice`.
    keyed_lists: BTreeMap<&'static str, NodePointer>,
    /// Every Phases node walked so far, by pointer; a Step reads its parent here.
    phases: BTreeMap<NodePointer, &'a Phases>,
    on_sheet: bool,
    /// The ids of planned work: an Item or Box with `planned`, a Step with look planned.
    planned_ids: BTreeSet<&'a str>,
    /// Every Row walked so far, so a rail reads its parent's tag.
    rows: BTreeSet<NodePointer>,
    /// Each step badge number with the pointer of the first pipe or link that draws it.
    steps: BTreeMap<u16, NodePointer>,
    /// Every Step id of the page with its parent's pointer, for a ladder's `around`.
    step_parents: BTreeMap<&'a str, NodePointer>,
    /// Ladders walked so far, for `ladders-too-many`.
    ladders: usize,
}

/// Each Step id with its parent's pointer, the first Step holding an id winning.
fn step_parents<'a>(entries: &[NodeEntry<'a>]) -> BTreeMap<&'a str, NodePointer> {
    let mut parents = BTreeMap::new();
    for entry in entries {
        if let (NodeRef::Node(Node::Step(step)), Some(parent)) = (entry.node, &entry.parent)
            && let Some(id) = step.id.as_deref()
        {
            parents.entry(id).or_insert_with(|| parent.clone());
        }
    }
    parents
}

/// A ladder's rules in field order: `ladders-too-many` on the ninth, the length of
/// `around`, each `around` id naming a Step of one Phases node, the length of `rungs`, then
/// each rung's actor and marks.
fn check_ladder<'a>(
    page: &'a Page,
    entry: &NodeEntry<'a>,
    ladder: &'a crate::document::Ladder,
    sheet_nodes: &mut SheetNodes<'a>,
    violations: &mut Violations,
) {
    sheet_nodes.ladders += 1;
    if sheet_nodes.ladders > LADDERS_MAX {
        violations.push(
            entry.pointer.clone(),
            VetRule::LaddersTooMany,
            format!(
                "{} ladders, above {LADDERS_MAX}: details are lettered A to H",
                sheet_nodes.ladders
            ),
        );
    }
    let around = SheetList::Around;
    check_list_length_in(around, &entry.pointer, ladder.around.len(), violations);
    let mut framed: Option<&NodePointer> = None;
    for (index, id) in ladder.around.iter().enumerate().take(around.max() + 1) {
        let pointer = around.pointer_in(&entry.pointer).index(index);
        match sheet_nodes.step_parents.get(id.as_str()) {
            None => violations.push(
                pointer,
                VetRule::LadderAroundNotStep,
                format!("\"{id}\" names no Step"),
            ),
            Some(parent) => match framed {
                Some(first) if first != parent => violations.push(
                    pointer,
                    VetRule::LadderAroundTwoPhases,
                    "a detail frame lies in one Phases node".to_string(),
                ),
                Some(_) => {}
                None => framed = Some(parent),
            },
        }
    }
    let rungs = SheetList::Rungs;
    check_list_length_in(rungs, &entry.pointer, ladder.rungs.len(), violations);
    for (index, rung) in ladder.rungs.iter().enumerate().take(rungs.max() + 1) {
        if let Some(actor) = &rung.actor {
            check_actor_known(
                page,
                &sheet_nodes.actors,
                actor,
                rungs.pointer_in(&entry.pointer).index(index).child("actor"),
                violations,
            );
        }
        check_carrier_marks(
            &sheet_nodes.marks,
            &PageMarks::rung_marks(ladder, &entry.pointer, index),
            violations,
        );
    }
}

/// The ids of planned work on a page: an Item or Box with `planned`, a Step with look
/// planned (section 14.3, `line-into-planned`).
fn planned_ids<'a>(entries: &[NodeEntry<'a>]) -> BTreeSet<&'a str> {
    entries
        .iter()
        .filter(|entry| match entry.node {
            NodeRef::Node(Node::Item(item)) => item.planned,
            NodeRef::Node(Node::Box(box_node)) => box_node.planned,
            NodeRef::Node(Node::Step(step)) => step.look == crate::document::StepLook::Planned,
            NodeRef::Node(_) | NodeRef::TeeArm(_) => false,
        })
        .filter_map(|entry| entry.node.id())
        .collect()
}

/// `planned-not-solid` for a planned line that is not solid.
fn check_planned_solid(
    line: Line,
    planned: bool,
    holder: &NodePointer,
    violations: &mut Violations,
) {
    if planned && line != Line::Solid {
        violations.push(
            holder.child("planned"),
            VetRule::PlannedNotSolid,
            "a planned line is a solid line drawn hollow; set line solid".to_string(),
        );
    }
}

/// `line-into-planned` for a built solid or dash line whose end names planned work: the
/// `to` end first, then the `from` end.
fn check_line_into_planned(
    (line, planned): (Line, bool),
    (from, to): (Option<&str>, Option<&str>),
    planned_ids: &BTreeSet<&str>,
    holder: &NodePointer,
    violations: &mut Violations,
) {
    if planned || !line.takes_tint() {
        return;
    }
    for (field, end) in [("to", to), ("from", from)] {
        let Some(id) = end.filter(|id| planned_ids.contains(id)) else {
            continue;
        };
        let message = match line {
            Line::Dash => format!(
                "\"{id}\" is planned, and a dash line cannot be planned; draw it solid and planned, or remove it"
            ),
            Line::Solid | Line::Gray | Line::Deny => {
                format!("\"{id}\" is planned, so a line to it is planned too; set planned")
            }
        };
        violations.push(holder.child(field), VetRule::LineIntoPlanned, message);
        return;
    }
}

/// `step-out-of-range` and `link-step-duplicate` for a pipe's or link's step badge.
fn check_line_step(
    step: Option<u16>,
    holder: &NodePointer,
    steps: &mut BTreeMap<u16, NodePointer>,
    violations: &mut Violations,
) {
    let Some(step) = step else {
        return;
    };
    let pointer = holder.child("step");
    if !(1..=LINE_STEP_MAX).contains(&step) {
        violations.push(
            pointer,
            VetRule::StepOutOfRange,
            format!("step {step} is outside 1 to {LINE_STEP_MAX}"),
        );
        return;
    }
    match steps.get(&step) {
        Some(first) => violations.push(
            pointer,
            VetRule::LinkStepDuplicate,
            format!("step {step} is already used by {}", first.as_str()),
        ),
        None => {
            steps.insert(step, holder.clone());
        }
    }
}

/// The rail rules of a Pipe or Tee arm with form rail: it stands vertical, as a direct
/// child of a Row, and names no targets.
fn check_rail(
    pipe: &crate::document::Pipe,
    entry: &NodeEntry<'_>,
    rows: &BTreeSet<NodePointer>,
    violations: &mut Violations,
) {
    if pipe.form != PipeForm::Rail {
        return;
    }
    if pipe.dir != PipeDir::Vertical {
        violations.push(
            entry.pointer.child("dir"),
            VetRule::RailNotVertical,
            "a rail stands vertical".to_string(),
        );
    }
    let in_row = matches!(entry.node, NodeRef::Node(Node::Pipe(_)))
        && entry
            .parent
            .as_ref()
            .is_some_and(|parent| rows.contains(parent));
    if !in_row {
        violations.push(
            entry.pointer.clone(),
            VetRule::RailOutsideRow,
            "a rail is a column of a Row".to_string(),
        );
    }
    for (field, target) in [("from", &pipe.from), ("to", &pipe.to)] {
        if target.is_some() {
            violations.push(
                entry.pointer.child(field),
                VetRule::RailWithTargets,
                "a rail is attached by links, not targets".to_string(),
            );
        }
    }
}

/// The declared actor ids, for the `actor-unknown` message.
fn declared_actors(page: &Page) -> String {
    let ids: Vec<&str> = page
        .actors
        .iter()
        .take(SheetList::Actors.max() + 1)
        .map(|actor| actor.id.as_str())
        .collect();
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.join(", ")
    }
}

/// `actor-unknown` at `pointer` when `actor` names no declared actor.
fn check_actor_known(
    page: &Page,
    actors: &PageActors<'_>,
    actor: &str,
    pointer: NodePointer,
    violations: &mut Violations,
) {
    if actors.tint(actor).is_none() {
        violations.push(
            pointer,
            VetRule::ActorUnknown,
            format!(
                "actor \"{actor}\" is not declared; declared: {}",
                declared_actors(page)
            ),
        );
    }
}

/// The actor rules of a line (a Pipe, a Tee spine or arm, or a Link) in field order:
/// `tint-on-actor-page` at its tint, then `actor-on-neutral-line` and `actor-unknown` at its
/// actor, then `line-without-actor` at its line.
fn check_line_actor(
    page: &Page,
    actors: &PageActors<'_>,
    (line, tint, actor): (Line, Option<u8>, Option<&str>),
    (effective, from): (Option<&str>, String),
    holder: &NodePointer,
    violations: &mut Violations,
) {
    if actors.any() && tint.is_some() {
        violations.push(
            holder.child("tint"),
            VetRule::TintOnActorPage,
            "a page with actors colours by actor; remove tint".to_string(),
        );
    }
    if let Some(actor) = actor {
        if line.takes_tint() {
            check_actor_known(page, actors, actor, holder.child("actor"), violations);
        } else {
            violations.push(
                holder.child("actor"),
                VetRule::ActorOnNeutralLine,
                "a gray or deny line takes no actor".to_string(),
            );
        }
    }
    if actors.any() && line.takes_tint() && effective.is_none() {
        violations.push(
            holder.child("line"),
            VetRule::LineWithoutActor,
            format!(
                "{} line {from} has no actor; set actor or leave from an actor's node",
                line.as_str()
            ),
        );
    }
}

/// How a line names its start in `line-without-actor`: the node it leaves, else its label.
fn line_origin(from: Option<&str>, label: &str) -> String {
    match from {
        Some(from) => format!("from \"{from}\""),
        None => format!("\"{label}\""),
    }
}

/// The actor and Phases rules of one body node on a sheet.
fn check_sheet_node<'a>(
    page: &'a Page,
    entry: &NodeEntry<'a>,
    grammar: &Grammar,
    sheet_nodes: &mut SheetNodes<'a>,
    violations: &mut Violations,
) {
    check_carrier_marks(
        &sheet_nodes.marks,
        &sheet_nodes.marks.node_marks(entry),
        violations,
    );
    let actors = &sheet_nodes.actors;
    match entry.node {
        NodeRef::Node(Node::PartsList(list)) => {
            if keyed_list_first(sheet_nodes, "PartsList", entry, violations) {
                check_parts_list(page, entry, list, sheet_nodes, violations);
            }
        }
        NodeRef::Node(Node::GeneralNotes(notes)) => {
            if keyed_list_first(sheet_nodes, "GeneralNotes", entry, violations) {
                check_general_notes(entry, notes, violations);
            }
        }
        NodeRef::Node(Node::Ladder(ladder)) => {
            check_ladder(page, entry, ladder, sheet_nodes, violations);
        }
        NodeRef::Node(Node::Item(item)) => {
            if let Some(actor) = &item.actor {
                check_actor_known(
                    page,
                    actors,
                    actor,
                    entry.pointer.child("actor"),
                    violations,
                );
            }
            check_list_length_in(
                SheetList::Compartments,
                &entry.pointer,
                item.compartments.len(),
                violations,
            );
            if !item.compartments.is_empty() && item.outline != Outline::Store {
                violations.push(
                    entry.pointer.child("compartments"),
                    VetRule::CompartmentsWithoutStore,
                    "compartments are drawn in a store".to_string(),
                );
            }
            if item.outline == Outline::Person && item.icon.is_some() {
                violations.push(
                    entry.pointer.child("icon"),
                    VetRule::OutlinePersonWithIcon,
                    "a person outline draws no icon".to_string(),
                );
            }
        }
        NodeRef::Node(Node::Box(box_node)) => {
            let kind = grammar.container(&box_node.kind);
            if actors.any() && box_node.tint.is_some() {
                violations.push(
                    entry.pointer.child("tint"),
                    VetRule::TintOnActorPage,
                    "a page with actors colours by actor; remove tint".to_string(),
                );
            }
            match (&box_node.actor, kind) {
                (Some(_), Some(kind)) if !kind.tintable => violations.push(
                    entry.pointer.child("actor"),
                    VetRule::ActorOnUntintableKind,
                    format!(
                        "{} takes no colour, so it cannot name an actor",
                        box_node.kind
                    ),
                ),
                (Some(actor), _) => {
                    check_actor_known(
                        page,
                        actors,
                        actor,
                        entry.pointer.child("actor"),
                        violations,
                    );
                }
                (None, Some(kind)) if actors.any() && kind.tintable => {
                    if let Some(slot) = kind.default_tint {
                        violations.push(
                            entry.pointer.clone(),
                            VetRule::KindTintWithoutActor,
                            format!(
                                "{} paints tint {slot} by default, which reads as an actor; name its actor",
                                box_node.kind
                            ),
                        );
                    }
                }
                (None, _) => {}
            }
        }
        NodeRef::Node(Node::Pipe(pipe)) => {
            check_line_actor(
                page,
                actors,
                (pipe.line, pipe.tint, pipe.actor.as_deref()),
                (
                    actors.pipe_actor(pipe),
                    line_origin(pipe.from.as_deref(), &pipe.label),
                ),
                &entry.pointer,
                violations,
            );
            check_pipe_sheet_fields(pipe, entry, sheet_nodes, violations);
        }
        NodeRef::Node(Node::Tee(tee)) => check_line_actor(
            page,
            actors,
            (tee.line, tee.tint, tee.actor.as_deref()),
            (actors.tee_actor(tee), line_origin(None, &tee.hub)),
            &entry.pointer,
            violations,
        ),
        NodeRef::TeeArm(arm) => {
            let tee = tee_of(page, entry);
            check_line_actor(
                page,
                actors,
                (arm.line, arm.tint, arm.actor.as_deref()),
                (
                    tee.and_then(|tee| actors.arm_actor(tee, arm)),
                    line_origin(None, &arm.label),
                ),
                &entry.pointer,
                violations,
            );
            if let (Some(tee), Some(actor)) = (tee, &arm.actor)
                && tee.actor.as_deref() != Some(actor.as_str())
            {
                violations.push(
                    entry.pointer.child("actor"),
                    VetRule::TeeArmActorDiffers,
                    "a Tee arm acts for its Tee's actor".to_string(),
                );
            }
            check_pipe_sheet_fields(arm, entry, sheet_nodes, violations);
        }
        NodeRef::Node(Node::Phases(phases)) => {
            check_phases(page, entry, phases, actors, violations);
            sheet_nodes.phases.insert(entry.pointer.clone(), phases);
        }
        NodeRef::Node(Node::Step(step)) => {
            let parent = entry
                .parent
                .as_ref()
                .and_then(|parent| sheet_nodes.phases.get(parent).copied());
            check_step(page, entry, step, parent, actors, violations);
        }
        NodeRef::Node(Node::Row(_)) => {
            sheet_nodes.rows.insert(entry.pointer.clone());
        }
        NodeRef::Node(
            Node::Col(_)
            | Node::Lanes(_)
            | Node::Fact(_)
            | Node::Note(_)
            | Node::Text(_)
            | Node::Callout(_)
            | Node::Frame(_)
            | Node::View(_),
        ) => {}
    }
    if let Some(parent) = &entry.parent
        && sheet_nodes.phases.contains_key(parent)
        && !matches!(entry.node, NodeRef::Node(Node::Step(_)))
    {
        violations.push(
            entry.pointer.clone(),
            VetRule::PhasesChildNotStep,
            "a Phases node holds Steps only".to_string(),
        );
    }
}

/// The planned, rail and step rules of a Pipe or Tee arm on a sheet.
fn check_pipe_sheet_fields<'a>(
    pipe: &'a crate::document::Pipe,
    entry: &NodeEntry<'a>,
    sheet_nodes: &mut SheetNodes<'a>,
    violations: &mut Violations,
) {
    check_planned_solid(pipe.line, pipe.planned, &entry.pointer, violations);
    check_line_into_planned(
        (pipe.line, pipe.planned),
        (pipe.from.as_deref(), pipe.to.as_deref()),
        &sheet_nodes.planned_ids,
        &entry.pointer,
        violations,
    );
    check_rail(pipe, entry, &sheet_nodes.rows, violations);
    check_line_step(
        pipe.step,
        &entry.pointer,
        &mut sheet_nodes.steps,
        violations,
    );
}

/// The Tee a Tee arm entry belongs to.
fn tee_of<'a>(page: &'a Page, entry: &NodeEntry<'a>) -> Option<&'a Tee> {
    let parent = entry.parent.as_ref()?;
    body_nodes(page)
        .into_iter()
        .find(|candidate| &candidate.pointer == parent)
        .and_then(|candidate| match candidate.node {
            NodeRef::Node(Node::Tee(tee)) => Some(tee),
            _ => None,
        })
}

/// A Phases node's own rules: its lanes (each a declared actor, none twice), its phases
/// (well-formed ids, none twice) and the list bounds.
fn check_phases(
    page: &Page,
    entry: &NodeEntry<'_>,
    phases: &Phases,
    actors: &PageActors<'_>,
    violations: &mut Violations,
) {
    let lanes = SheetList::Lanes;
    check_list_length_in(lanes, &entry.pointer, phases.lanes.len(), violations);
    let lanes_pointer = lanes.pointer_in(&entry.pointer);
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (index, lane) in phases.lanes.iter().enumerate().take(lanes.max() + 1) {
        let pointer = lanes_pointer.index(index);
        if !seen.insert(lane.as_str()) {
            violations.push(
                pointer,
                VetRule::PhaseLaneTwice,
                format!("actor \"{lane}\" already has a lane"),
            );
        } else {
            check_actor_known(page, actors, lane, pointer, violations);
        }
    }
    let list = SheetList::Phases;
    check_list_length_in(list, &entry.pointer, phases.phases.len(), violations);
    let phases_pointer = list.pointer_in(&entry.pointer);
    let mut phase_ids: BTreeMap<&str, NodePointer> = BTreeMap::new();
    for (index, phase) in phases.phases.iter().enumerate().take(list.max() + 1) {
        check_key_id(
            &phase.id,
            &phases_pointer.index(index),
            &mut phase_ids,
            violations,
        );
    }
    let steps = SheetList::Steps;
    if phases.children.len() > steps.max() {
        check_list_length_in(steps, &entry.pointer, phases.children.len(), violations);
    }
}

/// A Step's rules: it sits in a Phases node, in one of its lanes and phases, its columns
/// stay inside the phase, and a text look sets no fill it would not draw.
fn check_step(
    page: &Page,
    entry: &NodeEntry<'_>,
    step: &Step,
    parent: Option<&Phases>,
    actors: &PageActors<'_>,
    violations: &mut Violations,
) {
    if step.look == StepLook::Text && step.fill.is_some() {
        violations.push(
            entry.pointer.child("fill"),
            VetRule::StepTextFill,
            "a text Step draws no box, so it takes no fill".to_string(),
        );
    }
    let Some(phases) = parent else {
        violations.push(
            entry.pointer.clone(),
            VetRule::StepOutsidePhases,
            "a Step sits directly in a Phases node".to_string(),
        );
        return;
    };
    let lane_pointer = entry.pointer.child("lane");
    if actors.tint(&step.lane).is_none() {
        check_actor_known(page, actors, &step.lane, lane_pointer, violations);
    } else if !phases.lanes.contains(&step.lane) {
        violations.push(
            lane_pointer,
            VetRule::StepLaneUnknown,
            format!("lane \"{}\" is not a lane of this Phases node", step.lane),
        );
    }
    if !phases.phases.iter().any(|phase| phase.id == step.phase) {
        violations.push(
            entry.pointer.child("phase"),
            VetRule::StepPhaseUnknown,
            format!(
                "phase \"{}\" is not a phase of this Phases node",
                step.phase
            ),
        );
    }
    let end = u16::from(step.col) + u16::from(step.span);
    let in_range = |value: u8| (1..=PHASE_COLUMNS_MAX).contains(&value);
    if !in_range(step.col) || !in_range(step.span) || end > u16::from(PHASE_COLUMNS_MAX) + 1 {
        let field = if in_range(step.col) { "span" } else { "col" };
        violations.push(
            entry.pointer.child(field),
            VetRule::StepSpanOutOfRange,
            format!(
                "columns {} to {} run past column {PHASE_COLUMNS_MAX}",
                step.col,
                end.saturating_sub(1)
            ),
        );
    }
    check_list_length_in(
        SheetList::StepLines,
        &entry.pointer,
        step.lines.len(),
        violations,
    );
}

/// `tint-on-actor-page` and `legend-by-actor-misused` for one legend entry on a sheet.
fn check_legend_actor(
    page: &Page,
    entry: &crate::document::LegendEntry,
    pointer: &NodePointer,
    violations: &mut Violations,
) {
    if !page.actors.is_empty() && entry.tint.is_some() {
        violations.push(
            pointer.child("tint"),
            VetRule::TintOnActorPage,
            "a page with actors colours by actor; remove tint".to_string(),
        );
    }
    if entry.by_actor && (page.actors.is_empty() || !entry.line.takes_tint()) {
        violations.push(
            pointer.child("by_actor"),
            VetRule::LegendByActorMisused,
            "by_actor names solid or dashed lines on a page with actors".to_string(),
        );
    }
}

/// On a sheet, `sheet-body-not-view` for a body child that is not a View and `view-not-top-level` for a
/// View below the body.
fn check_sheet_placement(page: &Page, entry: &NodeEntry<'_>, violations: &mut Violations) {
    let is_view = matches!(entry.node, NodeRef::Node(Node::View(_)));
    match (&page.sheet, is_view, entry.depth == 1) {
        (Some(_), false, true) => violations.push(
            entry.pointer.clone(),
            VetRule::SheetBodyNotView,
            format!(
                "a design sheet holds Views; wrap {} in a View",
                entry.node.tag_name()
            ),
        ),
        (Some(_), true, false) => violations.push(
            entry.pointer.clone(),
            VetRule::ViewNotTopLevel,
            "a View is a body child of a sheet".to_string(),
        ),
        (None, _, _) | (Some(_), true, true) | (Some(_), false, false) => {}
    }
}

fn needs_sheet_message(surface: SheetSurface) -> String {
    format!(
        "{} is drawn on a design sheet only; add sheet or remove it",
        surface.as_str()
    )
}

/// The sheet rules at and under `/sheet`, in document order: the projection and chrome the
/// sheet needs, its lines, then the title block's text, lists and attention item ids.
fn check_sheet(page: &Page, sheet: &Sheet, entries: &[NodeEntry<'_>], violations: &mut Violations) {
    let sheet_pointer = NodePointer::root().child("sheet");
    if page.projection == Projection::Iso {
        violations.push(
            sheet_pointer.clone(),
            VetRule::SheetInIso,
            "a design sheet is drawn flat only".to_string(),
        );
    }
    if page.chrome == Chrome::None {
        violations.push(
            NodePointer::root().child("chrome"),
            VetRule::SheetChromeNone,
            "a design sheet draws its head in the title block, so chrome cannot be none"
                .to_string(),
        );
    }
    let draws_a_line = !page.links.is_empty()
        || entries.iter().any(|entry| {
            matches!(
                entry.node,
                NodeRef::Node(Node::Pipe(_) | Node::Tee(_)) | NodeRef::TeeArm(_)
            )
        });
    if !draws_a_line {
        violations.push(
            sheet_pointer.clone(),
            VetRule::SheetWithoutLines,
            "a design sheet draws at least one Pipe, Tee or Link, which its legend strip keys"
                .to_string(),
        );
    }

    if let Some(wordmark) = &sheet.wordmark {
        check_sheet_text(
            sheet_pointer.child("wordmark"),
            wordmark,
            Some(("wordmark", WORDMARK_SCALARS_MAX)),
            violations,
        );
    }
    let problem = sheet_pointer.child("problem");
    check_caption(&problem, sheet.problem.caption.as_deref(), violations);
    if let Some(lead) = &sheet.problem.lead {
        check_sheet_text(problem.child("lead"), lead, None, violations);
    }
    check_list_length(
        SheetList::ProblemPoints,
        sheet.problem.points.len(),
        violations,
    );
    let points = SheetList::ProblemPoints;
    for (index, point) in sheet
        .problem
        .points
        .iter()
        .enumerate()
        .take(points.max() + 1)
    {
        check_sheet_text(
            points.pointer().index(index).child("text"),
            &point.text,
            None,
            violations,
        );
    }

    let answer = sheet_pointer.child("answer");
    check_caption(&answer, sheet.answer.caption.as_deref(), violations);
    check_sheet_text(
        answer.child("statement"),
        &sheet.answer.statement,
        None,
        violations,
    );
    let paragraphs = SheetList::AnswerParagraphs;
    check_list_length(paragraphs, sheet.answer.paragraphs.len(), violations);
    for (index, paragraph) in sheet
        .answer
        .paragraphs
        .iter()
        .enumerate()
        .take(paragraphs.max() + 1)
    {
        let pointer = paragraphs.pointer().index(index);
        match paragraph {
            AnswerParagraph::Plain(text) => check_sheet_text(pointer, text, None, violations),
            AnswerParagraph::Led(led) => {
                check_sheet_text(pointer.child("lead"), &led.lead, None, violations);
                check_sheet_text(pointer.child("text"), &led.text, None, violations);
            }
        }
    }

    let attention = sheet_pointer.child("attention");
    check_caption(&attention, sheet.attention.caption.as_deref(), violations);
    let items = SheetList::AttentionItems;
    check_list_length(items, sheet.attention.items.len(), violations);
    let mut item_ids: BTreeMap<&str, NodePointer> = BTreeMap::new();
    for (index, item) in sheet
        .attention
        .items
        .iter()
        .enumerate()
        .take(items.max() + 1)
    {
        let pointer = items.pointer().index(index);
        check_key_id(&item.id, &pointer, &mut item_ids, violations);
        check_sheet_text(pointer.child("lead"), &item.lead, None, violations);
        check_sheet_text(pointer.child("text"), &item.text, None, violations);
        if let Some(actor) = &item.actor {
            check_actor_known(
                page,
                &PageActors::of(page),
                actor,
                pointer.child("actor"),
                violations,
            );
        }
    }

    let page_marks = PageMarks::of(page);
    for index in 0..sheet.problem.points.len().min(points.max() + 1) {
        check_carrier_marks(
            &page_marks,
            &page_marks.point_marks(page, index),
            violations,
        );
    }
    check_mark_legend(page, sheet, entries, violations);

    check_actors(page, violations);
}

/// The text of each mark kind in `sheet.marks`, then `mark-legend-unused` for the text of a
/// kind whose list the sheet does not draw. Attention items always exist.
fn check_mark_legend(
    page: &Page,
    sheet: &Sheet,
    entries: &[NodeEntry<'_>],
    violations: &mut Violations,
) {
    let pointer = NodePointer::root().child("sheet").child("marks");
    for kind in MarkKind::ALL {
        let Some(text) = sheet.marks.text(kind) else {
            continue;
        };
        check_sheet_text(pointer.child(kind.as_str()), text, None, violations);
        let has_list = match kind {
            MarkKind::Part => entries
                .iter()
                .any(|entry| matches!(entry.node, NodeRef::Node(Node::PartsList(_)))),
            MarkKind::Note => entries
                .iter()
                .any(|entry| matches!(entry.node, NodeRef::Node(Node::GeneralNotes(_)))),
            MarkKind::Attention => true,
        };
        if !has_list {
            violations.push(
                pointer.child(kind.as_str()),
                VetRule::MarkLegendUnused,
                format!("no {} key on this sheet; remove the text", kind.as_str()),
            );
        }
    }
    if let Some(text) = &sheet.marks.step {
        check_sheet_text(pointer.child("step"), text, None, violations);
        let draws_a_step = page.links.iter().any(|link| link.step.is_some())
            || entries.iter().any(|entry| {
                matches!(
                    entry.node,
                    NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) if pipe.step.is_some()
                )
            });
        if !draws_a_step {
            violations.push(
                pointer.child("step"),
                VetRule::MarkLegendUnused,
                "no step key on this sheet; remove the text".to_string(),
            );
        }
    }
}

/// The mark rules of one carrier in drawn order: `list-too-long` for a field of more than
/// MARKS_MAX marks, then for each mark `mark-unknown` when its key is not in its list and
/// `mark-repeated` when the carrier already draws that key.
fn check_carrier_marks(
    page_marks: &PageMarks<'_>,
    drawn: &[CarrierMark],
    violations: &mut Violations,
) {
    let list = SheetList::Marks;
    let mut fields: Vec<NodePointer> = Vec::new();
    for mark in drawn {
        let Some(field) = mark.pointer.parent() else {
            continue;
        };
        if field.as_str().ends_with("/marks") && !fields.contains(&field) {
            let count = drawn
                .iter()
                .filter(|other| other.pointer.parent().as_ref() == Some(&field))
                .count();
            if count > list.max() {
                violations.push(
                    field.clone(),
                    VetRule::ListTooLong,
                    format!("{count} {}, above {}", list.name(), list.max()),
                );
            }
            fields.push(field);
        }
    }
    let mut seen: Vec<&crate::Mark> = Vec::new();
    for mark in drawn {
        let kind = mark.mark.kind();
        if page_marks.position(&mark.mark).is_none() {
            violations.push(
                mark.pointer.clone(),
                VetRule::MarkUnknown,
                format!(
                    "{} \"{}\" is not in the {}",
                    kind.as_str(),
                    mark.mark.id(),
                    kind.list()
                ),
            );
        } else if seen.contains(&&mark.mark) {
            violations.push(
                mark.pointer.clone(),
                VetRule::MarkRepeated,
                format!(
                    "{} \"{}\" is already marked here",
                    kind.as_str(),
                    mark.mark.id()
                ),
            );
        }
        seen.push(&mark.mark);
    }
}

/// True for the first PartsList or GeneralNotes node of the sheet; a later one is
/// `keyed-list-twice`.
fn keyed_list_first(
    sheet_nodes: &mut SheetNodes<'_>,
    tag: &'static str,
    entry: &NodeEntry<'_>,
    violations: &mut Violations,
) -> bool {
    match sheet_nodes.keyed_lists.get(tag) {
        Some(first) => {
            let list = if tag == "PartsList" {
                MarkKind::Part.list()
            } else {
                MarkKind::Note.list()
            };
            violations.push(
                entry.pointer.clone(),
                VetRule::KeyedListTwice,
                format!("the sheet has its {list} at {first}"),
            );
            false
        }
        None => {
            sheet_nodes.keyed_lists.insert(tag, entry.pointer.clone());
            true
        }
    }
}

/// A parts list's rules, part by part in field order: the id, then `node` (it names an
/// Item, Box or Pipe that no earlier part names), the fields a node gives, and the name a
/// part without a node needs; then the list's length.
fn check_parts_list(
    page: &Page,
    entry: &NodeEntry<'_>,
    list: &PartsList,
    sheet_nodes: &SheetNodes<'_>,
    violations: &mut Violations,
) {
    let parts = SheetList::Parts;
    check_list_length_in(parts, &entry.pointer, list.parts.len(), violations);
    let parts_pointer = parts.pointer_in(&entry.pointer);
    let mut ids: BTreeMap<&str, NodePointer> = BTreeMap::new();
    let mut nodes: BTreeMap<&str, &str> = BTreeMap::new();
    for (index, part) in list.parts.iter().enumerate().take(parts.max() + 1) {
        let pointer = parts_pointer.index(index);
        check_key_id(&part.id, &pointer, &mut ids, violations);
        let Some(node) = part.node.as_deref() else {
            if part.name.is_none() {
                violations.push(
                    pointer.clone(),
                    VetRule::PartNameMissing,
                    format!("part \"{}\" names a node or writes its name", part.id),
                );
            }
            if let Some(actor) = &part.actor {
                check_actor_known(
                    page,
                    &sheet_nodes.actors,
                    actor,
                    pointer.child("actor"),
                    violations,
                );
            }
            continue;
        };
        let node_pointer = pointer.child("node");
        match sheet_nodes.node_tags.get(node) {
            Some(&("Item" | "Box" | "Pipe")) => match nodes.get(node) {
                Some(first) => violations.push(
                    node_pointer,
                    VetRule::PartNodeTwice,
                    format!("node \"{node}\" is already part \"{first}\""),
                ),
                None => {
                    nodes.insert(node, part.id.as_str());
                }
            },
            _ => violations.push(
                node_pointer,
                VetRule::PartNodeUnknown,
                format!("\"{node}\" names no Item, Box or Pipe"),
            ),
        }
        for (field, written) in [
            ("name", part.name.is_some()),
            ("what", part.what.is_some()),
            ("actor", part.actor.is_some()),
            ("planned", part.planned),
        ] {
            if written {
                violations.push(
                    pointer.child(field),
                    VetRule::PartFieldFromNode,
                    format!(
                        "part \"{}\" takes its {field} from node \"{node}\"",
                        part.id
                    ),
                );
            }
        }
    }
}

/// A general notes node's rules: each note's id, then the list's length.
fn check_general_notes(entry: &NodeEntry<'_>, notes: &GeneralNotes, violations: &mut Violations) {
    let list = SheetList::GeneralNotes;
    check_list_length_in(list, &entry.pointer, notes.notes.len(), violations);
    let notes_pointer = list.pointer_in(&entry.pointer);
    let mut ids: BTreeMap<&str, NodePointer> = BTreeMap::new();
    for (index, note) in notes.notes.iter().enumerate().take(list.max() + 1) {
        check_key_id(&note.id, &notes_pointer.index(index), &mut ids, violations);
    }
}

/// The actors list: its length, then per actor its id, name, text and tint, and
/// `actor-tint-duplicate` for a slot an earlier actor holds.
fn check_actors(page: &Page, violations: &mut Violations) {
    let list = SheetList::Actors;
    check_list_length(list, page.actors.len(), violations);
    let mut ids: BTreeMap<&str, NodePointer> = BTreeMap::new();
    let mut slots: BTreeMap<u8, &str> = BTreeMap::new();
    for (index, actor) in page.actors.iter().enumerate().take(list.max() + 1) {
        let pointer = list.pointer().index(index);
        check_key_id(&actor.id, &pointer, &mut ids, violations);
        check_sheet_text(pointer.child("name"), &actor.name, None, violations);
        if let Some(text) = &actor.text {
            check_sheet_text(pointer.child("text"), text, None, violations);
        }
        check_tint(Some(actor.tint), &pointer, violations);
        match slots.get(&actor.tint) {
            Some(first) => violations.push(
                pointer.child("tint"),
                VetRule::ActorTintDuplicate,
                format!("tint {} is already used by actor \"{first}\"", actor.tint),
            ),
            None => {
                slots.insert(actor.tint, actor.id.as_str());
            }
        }
    }
}

/// `id-malformed` and `key-duplicate` for the id of one keyed entry at `entry`.
fn check_key_id<'a>(
    id: &'a str,
    entry: &NodePointer,
    seen: &mut BTreeMap<&'a str, NodePointer>,
    violations: &mut Violations,
) {
    let id_pointer = entry.child("id");
    if !is_valid_id(id) {
        violations.push(
            id_pointer,
            VetRule::IdMalformed,
            format!("id does not match {ID_PATTERN}"),
        );
        return;
    }
    match seen.get(id) {
        Some(first) => violations.push(
            id_pointer,
            VetRule::KeyDuplicate,
            format!("\"{id}\" is already used by {first}"),
        ),
        None => {
            seen.insert(id, entry.clone());
        }
    }
}

fn check_caption(cell: &NodePointer, caption: Option<&str>, violations: &mut Violations) {
    if let Some(caption) = caption {
        check_sheet_text(
            cell.child("caption"),
            caption,
            Some(("caption", CAPTION_SCALARS_MAX)),
            violations,
        );
    }
}

/// The text rules of one sheet field, then `sheet-text-too-long` against `limit`, the
/// field's name and its largest scalar count, when it has one.
fn check_sheet_text(
    pointer: NodePointer,
    text: &str,
    limit: Option<(&str, usize)>,
    violations: &mut Violations,
) {
    let field = TextField {
        pointer: pointer.clone(),
        text,
    };
    check_text(&field, violations);
    if let Some((name, max)) = limit {
        let scalar_count = text.chars().count();
        if scalar_count > max {
            violations.push(
                pointer,
                VetRule::SheetTextTooLong,
                format!("{name} of {scalar_count} characters, above {max}"),
            );
        }
    }
}

/// `list-empty` for a list with minimum 1 that holds nothing, `list-too-long` above its
/// maximum.
fn check_list_length(list: SheetList, length: usize, violations: &mut Violations) {
    check_list_length_in(list, &NodePointer::root(), length, violations);
}

/// `check_list_length` for a list of the node at `owner`.
fn check_list_length_in(
    list: SheetList,
    owner: &NodePointer,
    length: usize,
    violations: &mut Violations,
) {
    if length < list.min() {
        violations.push(
            list.pointer_in(owner),
            VetRule::ListEmpty,
            format!("{} is empty", list.name()),
        );
    } else if length > list.max() {
        violations.push(
            list.pointer_in(owner),
            VetRule::ListTooLong,
            format!("{length} {}, above {}", list.name(), list.max()),
        );
    }
}

/// `pipe-target-on-tee-arm` for a Tee arm with `from` or `to`; for a Pipe node,
/// `pipe-target-unknown` for a `from` or `to` that names no node id and `pipe-targets-equal`
/// when both name the same id. `from` and `to` follow the Pipe's other fields.
fn check_pipe_targets(
    entry: &NodeEntry<'_>,
    all_ids: &BTreeSet<&str>,
    violations: &mut Violations,
) {
    match entry.node {
        NodeRef::TeeArm(arm) => {
            for (field, target) in [("from", &arm.from), ("to", &arm.to)] {
                if target.is_some() {
                    violations.push(
                        entry.pointer.child(field),
                        VetRule::PipeTargetOnTeeArm,
                        "a Tee arm cannot name a target".to_string(),
                    );
                }
            }
        }
        NodeRef::Node(Node::Pipe(pipe)) => {
            for (field, target) in [("from", &pipe.from), ("to", &pipe.to)] {
                if let Some(target) = target
                    && !all_ids.contains(target.as_str())
                {
                    violations.push(
                        entry.pointer.child(field),
                        VetRule::PipeTargetUnknown,
                        format!("pipe target \"{target}\" names no node id"),
                    );
                }
            }
            if let (Some(from), Some(to)) = (&pipe.from, &pipe.to)
                && from == to
            {
                violations.push(
                    entry.pointer.child("to"),
                    VetRule::PipeTargetsEqual,
                    format!("pipe from and to both name \"{to}\""),
                );
            }
        }
        NodeRef::Node(_) => {}
    }
}

/// The Lanes node each lane head belongs to, keyed by the head's id. A lane head is a
/// direct child of a Lanes node (section 13.6); a node inside a head is not one.
fn lane_heads<'a>(entries: &[NodeEntry<'a>]) -> BTreeMap<&'a str, NodePointer> {
    let lanes_pointers: BTreeSet<&NodePointer> = entries
        .iter()
        .filter(|entry| matches!(entry.node, NodeRef::Node(Node::Lanes(_))))
        .map(|entry| &entry.pointer)
        .collect();
    let mut heads = BTreeMap::new();
    for entry in entries {
        let (Some(parent), Some(id)) = (&entry.parent, entry.node.id()) else {
            continue;
        };
        if lanes_pointers.contains(parent) {
            heads.entry(id).or_insert_with(|| parent.clone());
        }
    }
    heads
}

/// `link-order-outside-lanes` and `link-order-duplicate` for a link with `order`. `orders`
/// holds the first link index of each (Lanes node, order) pair seen so far.
fn check_link_order<'a>(
    (link_index, link): (usize, &Link),
    link_pointer: &NodePointer,
    lane_heads: &'a BTreeMap<&str, NodePointer>,
    orders: &mut BTreeMap<(&'a NodePointer, u16), usize>,
    violations: &mut Violations,
) {
    let Some(order) = link.order else {
        return;
    };
    let order_pointer = link_pointer.child("order");
    let from_lanes = lane_heads.get(link.from.as_str());
    let to_lanes = lane_heads.get(link.to.as_str());
    let lanes = match (from_lanes, to_lanes) {
        (Some(from_lanes), Some(to_lanes)) if from_lanes == to_lanes && link.from != link.to => {
            from_lanes
        }
        _ => {
            violations.push(
                order_pointer,
                VetRule::LinkOrderOutsideLanes,
                "an ordered link joins two lanes of one Lanes node".to_string(),
            );
            return;
        }
    };
    match orders.get(&(lanes, order)) {
        Some(&first_index) => violations.push(
            order_pointer,
            VetRule::LinkOrderDuplicate,
            format!(
                "order {order} is already used by {}",
                NodePointer::root().child("links").index(first_index)
            ),
        ),
        None => {
            orders.insert((lanes, order), link_index);
        }
    }
}

/// `links-too-many` once at `/links`, then each of the first LINKS_MAX + 1 links in field
/// order: the link itself (`link-self`), `from`, `to`, the text fields, `via`, then `order`.
fn check_links(
    page: &Page,
    ids: &BTreeMap<&str, NodePointer>,
    lane_heads: &BTreeMap<&str, NodePointer>,
    sheet_nodes: &SheetNodes<'_>,
    violations: &mut Violations,
) {
    let links_pointer = NodePointer::root().child("links");
    if page.links.len() > LINKS_MAX {
        violations.push(
            links_pointer.clone(),
            VetRule::LinksTooMany,
            format!("links has {} entries, above {LINKS_MAX}", page.links.len()),
        );
    }
    let canvas_width = canvas_width(page);
    let mut orders = BTreeMap::new();
    let mut steps = sheet_nodes.steps.clone();
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX + 1) {
        let link_pointer = links_pointer.index(index);
        if link.from == link.to {
            violations.push(
                link_pointer.clone(),
                VetRule::LinkSelf,
                "from and to name the same node".to_string(),
            );
        }
        for (field, value) in [("from", &link.from), ("to", &link.to)] {
            if !ids.contains_key(value.as_str()) {
                violations.push(
                    link_pointer.child(field),
                    VetRule::LinkUnknownId,
                    unknown_id_message(value),
                );
            }
        }
        check_tint(link.tint, &link_pointer, violations);
        if sheet_nodes.on_sheet {
            let actors = &sheet_nodes.actors;
            check_line_actor(
                page,
                actors,
                (link.line, link.tint, link.actor.as_deref()),
                (
                    actors.link_actor(link),
                    line_origin(Some(&link.from), &link.from),
                ),
                &link_pointer,
                violations,
            );
        }
        let mut text_fields = Vec::new();
        push_one_link_text_fields(link, &link_pointer, &mut text_fields);
        check_text_fields(&text_fields, violations);
        check_via(link, &link_pointer, canvas_width, violations);
        check_link_order(
            (index, link),
            &link_pointer,
            lane_heads,
            &mut orders,
            violations,
        );
        if sheet_nodes.on_sheet {
            check_planned_solid(link.line, link.planned, &link_pointer, violations);
            check_line_into_planned(
                (link.line, link.planned),
                (Some(&link.from), Some(&link.to)),
                &sheet_nodes.planned_ids,
                &link_pointer,
                violations,
            );
            check_line_step(link.step, &link_pointer, &mut steps, violations);
            if link.valve && link.arrow == Arrow::None {
                violations.push(
                    link_pointer.child("valve"),
                    VetRule::ValveWithoutArrow,
                    "a valve sits before an arrowhead".to_string(),
                );
            }
            if !link.marks.is_empty() && link.label.is_none() && link.step.is_none() && !link.valve
            {
                violations.push(
                    link_pointer.child("marks"),
                    VetRule::LinkMarksWithoutTag,
                    "a link's marks sit in its tag or under its valve; give it a label".to_string(),
                );
            }
            check_carrier_marks(
                &sheet_nodes.marks,
                &PageMarks::link_marks(link, index),
                violations,
            );
        }
    }
}

fn unknown_id_message(value: &str) -> String {
    if is_valid_id(value) {
        format!("no node has id \"{value}\"")
    } else {
        format!("no node has this id; it does not match {ID_PATTERN}")
    }
}

/// `via` above LINK_VIA_MAX points, then each of the first LINK_VIA_MAX + 1 points outside
/// the canvas. Layout alone knows the canvas height, so vet bounds y below only.
fn check_via(
    link: &Link,
    link_pointer: &NodePointer,
    canvas_width: f32,
    violations: &mut Violations,
) {
    let via_pointer = link_pointer.child("via");
    if link.via.len() > LINK_VIA_MAX {
        violations.push(
            via_pointer.clone(),
            VetRule::LinkViaTooMany,
            format!("via has {} points, above {LINK_VIA_MAX}", link.via.len()),
        );
    }
    for (index, point) in link.via.iter().enumerate().take(LINK_VIA_MAX + 1) {
        let x_inside = point.x.is_finite() && (0.0..=canvas_width).contains(&point.x);
        let y_inside = point.y.is_finite() && point.y >= 0.0;
        if !(x_inside && y_inside) {
            violations.push(
                via_pointer.index(index),
                VetRule::LinkViaOutside,
                format!(
                    "via point ({}, {}) is outside the page: x 0 to {canvas_width}, y 0 or more",
                    point.x, point.y
                ),
            );
        }
    }
}

/// A node's own violations in struct declaration order: depth first (it names the node
/// itself), then `id`, then a Tee arm's `dir`, then text fields, then the container rules
/// for `gap`, `grow` and `children`, which Row and Col declare before `children` and Zone
/// declares after `label`. A Text reports its `body` length between its heading and its
/// lines, and a Frame its `height` after its label.
fn check_entry<'a>(
    entry: &NodeEntry<'a>,
    enclosing_box: &str,
    grammar: &Grammar,
    ids: &mut BTreeMap<&'a str, NodePointer>,
    violations: &mut Violations,
) {
    if entry.depth == DEPTH_MAX + 1 {
        violations.push(
            entry.pointer.clone(),
            VetRule::DepthExceeded,
            format!("depth {} is above {DEPTH_MAX}", entry.depth),
        );
    }

    if let Some(id) = entry.node.id() {
        check_id(id, entry, ids, violations);
    }

    if let NodeRef::TeeArm(pipe) = entry.node
        && pipe.dir == PipeDir::Vertical
    {
        violations.push(
            entry.pointer.child("dir"),
            VetRule::TeeArmNotHorizontal,
            "Tee arm dir is \"v\", expected \"h\"".to_string(),
        );
    }

    check_kind_and_tint(entry, enclosing_box, grammar, violations);

    let mut fields = Vec::new();
    push_node_text_fields(entry, &mut fields);
    if let NodeRef::Node(Node::Item(item)) = entry.node {
        let head_count = 1 + usize::from(item.subtitle.is_some());
        let (head_fields, fact_fields) = fields
            .split_at_checked(head_count)
            .unwrap_or((&fields, &[]));
        check_text_fields(head_fields, violations);
        if item.facts.len() > FACTS_MAX {
            violations.push(
                entry.pointer.child("facts"),
                VetRule::FactsTooMany,
                format!("{} facts, above {FACTS_MAX}", item.facts.len()),
            );
        }
        check_text_fields(fact_fields, violations);
    } else if let NodeRef::Node(Node::Text(text)) = entry.node {
        let heading_count = usize::from(text.heading.is_some());
        let (heading_fields, line_fields) = fields
            .split_at_checked(heading_count)
            .unwrap_or((&[], &fields));
        check_text_fields(heading_fields, violations);
        check_text_body_length(entry, text, violations);
        check_text_fields(line_fields, violations);
    } else {
        check_text_fields(&fields, violations);
    }

    match entry.node {
        NodeRef::Node(Node::Row(row)) => {
            check_gap_and_grow(
                entry,
                row.gap,
                row.grow.as_deref(),
                row.children.len(),
                violations,
            );
            check_children_count(entry, row.children.len(), violations);
        }
        NodeRef::Node(Node::Col(col)) => {
            check_gap_and_grow(
                entry,
                col.gap,
                col.grow.as_deref(),
                col.children.len(),
                violations,
            );
            check_children_count(entry, col.children.len(), violations);
        }
        NodeRef::Node(Node::Lanes(lanes)) => {
            check_gap_and_grow(entry, lanes.gap, None, lanes.children.len(), violations);
            check_children_count(entry, lanes.children.len(), violations);
            if lanes.children.len() > LANES_MAX {
                violations.push(
                    entry.pointer.child("children"),
                    VetRule::LanesTooMany,
                    format!("{} lanes, above {LANES_MAX}", lanes.children.len()),
                );
            }
        }
        NodeRef::Node(Node::Box(box_node)) => {
            check_children_count(entry, box_node.children.len(), violations);
        }
        NodeRef::Node(Node::View(view)) => {
            check_gap_and_grow(entry, view.gap, None, view.children.len(), violations);
            check_children_count(entry, view.children.len(), violations);
        }
        NodeRef::Node(Node::Phases(phases)) => {
            if phases.children.is_empty() {
                check_children_count(entry, 0, violations);
            }
        }
        NodeRef::Node(Node::Frame(frame)) => {
            if !(FRAME_HEIGHT_MIN..=FRAME_HEIGHT_MAX).contains(&frame.height) {
                violations.push(
                    entry.pointer.child("height"),
                    VetRule::FrameHeightOutOfRange,
                    format!(
                        "height {} is outside {FRAME_HEIGHT_MIN} to {FRAME_HEIGHT_MAX}",
                        frame.height
                    ),
                );
            }
        }
        NodeRef::Node(
            Node::Item(_)
            | Node::Fact(_)
            | Node::Note(_)
            | Node::Pipe(_)
            | Node::Tee(_)
            | Node::Text(_)
            | Node::Callout(_)
            | Node::Step(_)
            | Node::PartsList(_)
            | Node::GeneralNotes(_)
            | Node::Ladder(_),
        )
        | NodeRef::TeeArm(_) => {}
    }
}

/// The nearest Box kind above `entry`, or `page` when there is none.
fn enclosing_box_kind<'a>(
    entry: &NodeEntry<'_>,
    box_context: &BTreeMap<NodePointer, &'a str>,
) -> &'a str {
    entry
        .parent
        .as_ref()
        .and_then(|parent| box_context.get(parent).copied())
        .unwrap_or(PAGE_PARENT)
}

/// `kind-unknown`, `kind-parent-not-allowed` and `icon-outside-pack` for a Box or Item, and
/// `tint-out-of-range` for every tint carrier, in field order.
fn check_kind_and_tint(
    entry: &NodeEntry<'_>,
    enclosing_box: &str,
    grammar: &Grammar,
    violations: &mut Violations,
) {
    let kind_pointer = entry.pointer.child("kind");
    match entry.node {
        NodeRef::Node(Node::Box(box_node)) => {
            match grammar.container(&box_node.kind) {
                None => violations.push(
                    kind_pointer,
                    VetRule::KindUnknown,
                    format!(
                        "kind \"{}\" is not a container kind of grammar {}",
                        box_node.kind, grammar.name
                    ),
                ),
                Some(container) => {
                    check_parent(
                        &container.parents,
                        &box_node.kind,
                        enclosing_box,
                        kind_pointer,
                        violations,
                    );
                }
            }
            check_tint(box_node.tint, &entry.pointer, violations);
        }
        NodeRef::Node(Node::Item(item)) => match grammar.item(&item.kind) {
            None => violations.push(
                kind_pointer,
                VetRule::KindUnknown,
                format!(
                    "kind \"{}\" is not an item kind of grammar {}",
                    item.kind, grammar.name
                ),
            ),
            Some(item_kind) => {
                check_parent(
                    &item_kind.parents,
                    &item.kind,
                    enclosing_box,
                    kind_pointer,
                    violations,
                );
                if item.icon.is_some() && item_kind.icons == IconPack::None {
                    violations.push(
                        entry.pointer.child("icon"),
                        VetRule::IconOutsidePack,
                        format!("item kind {} takes no icon", item.kind),
                    );
                }
            }
        },
        NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe) => {
            check_tint(pipe.tint, &entry.pointer, violations);
        }
        NodeRef::Node(Node::Tee(tee)) => check_tint(tee.tint, &entry.pointer, violations),
        NodeRef::Node(
            Node::Row(_)
            | Node::Col(_)
            | Node::Lanes(_)
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
            | Node::Ladder(_),
        ) => {}
    }
}

fn check_parent(
    parents: &[String],
    kind: &str,
    enclosing_box: &str,
    kind_pointer: NodePointer,
    violations: &mut Violations,
) {
    if !parents.iter().any(|parent| parent == enclosing_box) {
        violations.push(
            kind_pointer,
            VetRule::KindParentNotAllowed,
            format!("{kind} cannot sit in {enclosing_box}"),
        );
    }
}

/// `tint-out-of-range` at `<holder>/tint`.
fn check_tint(tint: Option<u8>, holder: &NodePointer, violations: &mut Violations) {
    if let Some(tint) = tint
        && !(1..=TINT_SLOTS).contains(&tint)
    {
        violations.push(
            holder.child("tint"),
            VetRule::TintOutOfRange,
            format!("tint {tint} is outside 1 to {TINT_SLOTS}"),
        );
    }
}

fn check_id<'a>(
    id: &'a str,
    entry: &NodeEntry<'a>,
    ids: &mut BTreeMap<&'a str, NodePointer>,
    violations: &mut Violations,
) {
    let id_pointer = entry.pointer.child("id");
    if !is_valid_id(id) {
        violations.push(
            id_pointer,
            VetRule::IdMalformed,
            format!("id does not match {ID_PATTERN}"),
        );
        return;
    }
    if let Some(first_pointer) = ids.get(id) {
        violations.push(
            id_pointer,
            VetRule::IdDuplicate,
            format!("id \"{id}\" is already used at {first_pointer}"),
        );
    } else {
        ids.insert(id, entry.pointer.clone());
    }
}

fn check_text_body_length(entry: &NodeEntry<'_>, text: &Text, violations: &mut Violations) {
    let body_pointer = entry.pointer.child("body");
    if text.body.is_empty() {
        violations.push(
            body_pointer,
            VetRule::TextBodyEmpty,
            "body has no lines".to_string(),
        );
    } else if text.body.len() > TEXT_BODY_LINES_MAX {
        violations.push(
            body_pointer,
            VetRule::TextBodyTooLong,
            format!(
                "body has {} lines, above {TEXT_BODY_LINES_MAX}",
                text.body.len()
            ),
        );
    }
}

fn check_gap_and_grow(
    entry: &NodeEntry<'_>,
    gap: Option<u16>,
    grow: Option<&[u16]>,
    child_count: usize,
    violations: &mut Violations,
) {
    if let Some(gap) = gap
        && gap > GAP_MAX_PX
    {
        violations.push(
            entry.pointer.child("gap"),
            VetRule::GapOutOfRange,
            format!("gap {gap} is above {GAP_MAX_PX}"),
        );
    }
    if let Some(weights) = grow {
        let grow_pointer = entry.pointer.child("grow");
        if weights.len() != child_count {
            violations.push(
                grow_pointer.clone(),
                VetRule::GrowLengthMismatch,
                format!(
                    "grow has {} weights for {child_count} children",
                    weights.len()
                ),
            );
        }
        for (index, &weight) in weights.iter().enumerate().take(CHILDREN_MAX + 1) {
            if weight > GROW_WEIGHT_MAX {
                violations.push(
                    grow_pointer.index(index),
                    VetRule::GrowOutOfRange,
                    format!("grow weight {weight} is above {GROW_WEIGHT_MAX}"),
                );
            }
        }
    }
}

fn check_children_count(entry: &NodeEntry<'_>, child_count: usize, violations: &mut Violations) {
    let children_pointer = entry.pointer.child("children");
    if child_count == 0 {
        violations.push(
            children_pointer,
            VetRule::ChildrenEmpty,
            "container has no children".to_string(),
        );
    } else if child_count > CHILDREN_MAX {
        violations.push(
            children_pointer,
            VetRule::ChildrenTooMany,
            children_too_many_message(child_count),
        );
    }
}

fn children_too_many_message(child_count: usize) -> String {
    format!("{child_count} children, above {CHILDREN_MAX}")
}

fn is_forbidden_control_character(character: char) -> bool {
    matches!(character, '\u{0000}'..='\u{001F}' | '\u{007F}')
}

fn check_text_fields(fields: &[TextField<'_>], violations: &mut Violations) {
    for field in fields {
        check_text(field, violations);
    }
}

fn check_text(field: &TextField<'_>, violations: &mut Violations) {
    let text = field.text;
    if text.is_empty() {
        violations.push(
            field.pointer.clone(),
            VetRule::TextEmpty,
            "text is empty".to_string(),
        );
        return;
    }
    let scalar_count = text.chars().count();
    if scalar_count > TEXT_SCALARS_MAX {
        violations.push(
            field.pointer.clone(),
            VetRule::TextTooLong,
            format!("text has {scalar_count} scalar values, above {TEXT_SCALARS_MAX}"),
        );
    }
    if let Some(character) = text
        .chars()
        .find(|&character| is_forbidden_control_character(character))
    {
        violations.push(
            field.pointer.clone(),
            VetRule::TextControlCharacter,
            format!(
                "text contains control character U+{:04X}",
                u32::from(character)
            ),
        );
    }
    if text.starts_with(char::is_whitespace) || text.ends_with(char::is_whitespace) {
        violations.push(
            field.pointer.clone(),
            VetRule::TextUntrimmed,
            "text starts or ends with whitespace".to_string(),
        );
    }
}
