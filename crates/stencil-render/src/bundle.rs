//! A theme bundle (section 5.2): one written SVG for several themes, in which every paint
//! value a theme controls is a CSS custom property declared once per theme, so the file
//! repaints itself from `prefers-color-scheme` or from `data-theme` on its root. The bundle
//! is built from one written file per theme: the files share one layout, so their lines
//! align, and a line that differs only in paint becomes one line with variables, while a
//! shape one theme draws and another does not is switched with `display`. Anything else
//! that differs cannot be switched in CSS and refuses the theme.

use std::collections::BTreeMap;

use serde_json::Value;
use stencil_model::Theme;

use crate::svg::{FADE_VARIABLE, fade_floor_of_page};
use crate::{RenderError, format_number};

/// One theme's written file, as `self_contained_with(FadeFloor::Variable)` or
/// `portable_with(FadeFloor::Variable)` wrote it.
#[derive(Debug, Clone, Copy)]
pub struct ThemeFile<'a> {
    pub theme: &'a Theme,
    pub svg: &'a str,
    /// True when the theme's page colour is dark, which picks the fade floor and the
    /// `prefers-color-scheme` block.
    pub dark_page: bool,
}

/// The attributes whose values are CSS properties, so a difference in one becomes a custom
/// property. Every other attribute is structure: a difference in it is a different shape.
const PAINT_ATTRIBUTES: [&str; 15] = [
    "fill",
    "stroke",
    "stroke-width",
    "stroke-dasharray",
    "stroke-opacity",
    "fill-opacity",
    "opacity",
    "filter",
    "marker-start",
    "marker-mid",
    "marker-end",
    "stop-color",
    "stop-opacity",
    "flood-color",
    "flood-opacity",
];

const VARIABLE_PREFIX: &str = "--stencil-";
/// The root attribute that lists the bundled themes and scopes the bundle's rules.
const THEMES_ATTRIBUTE: &str = "data-themes";
/// The root attribute an embedding page sets to pick one bundled theme.
const THEME_ATTRIBUTE: &str = "data-theme";
const DISPLAY_SHOWN: &str = "inline";
const DISPLAY_HIDDEN: &str = "none";
/// The longest file the aligner takes, in lines: its table holds a u16 per line pair.
const LINES_MAX: usize = u16::MAX as usize;

/// The bundle of `files`, whose first entry is the default theme (section 5.2).
pub fn bundle_themes(files: &[ThemeFile<'_>]) -> Result<String, RenderError> {
    let Some(default) = files.first() else {
        return Err(bundle_error("", "a bundle needs at least two themes"));
    };
    if files.len() < 2 {
        return Err(bundle_error(
            &default.theme.name,
            "a bundle needs at least two themes",
        ));
    }
    for (index, file) in files.iter().enumerate() {
        if files
            .iter()
            .take(index)
            .any(|earlier| earlier.theme.name == file.theme.name)
        {
            return Err(bundle_error(
                &file.theme.name,
                "two bundled themes share this name",
            ));
        }
    }

    let mut parts: Vec<FileParts<'_>> = Vec::with_capacity(files.len());
    for file in files {
        parts.push(split_file(file.svg, &file.theme.name)?);
    }
    let Some(default_parts) = parts.first() else {
        return Err(bundle_error("", "a bundle needs at least two themes"));
    };
    for (file, theme_parts) in files.iter().zip(parts.iter()).skip(1) {
        if theme_parts.root != default_parts.root {
            return Err(bundle_error(
                &file.theme.name,
                layout_difference(file, default, theme_parts.root, default_parts.root),
            ));
        }
        if theme_parts.style != default_parts.style {
            return Err(bundle_error(
                &file.theme.name,
                format!(
                    "its style block differs from {}'s, so the files were not written with the variable fade floor",
                    default.theme.name
                ),
            ));
        }
    }
    for index in 1..parts.len() {
        let (earlier, rest) = parts.split_at_mut(index);
        if let (Some(theme_parts), Some(file)) = (rest.first_mut(), files.get(index)) {
            rename_differing_ids(earlier, theme_parts, &file.theme.name);
        }
    }
    let mut bodies: Vec<Vec<ParsedLine>> = Vec::with_capacity(files.len());
    for (file, theme_parts) in files.iter().zip(parts.iter()) {
        bodies.push(parse_body(theme_parts, &file.theme.name)?);
    }

    let slots = align_bodies(files, &bodies)?;
    let mut variables = Variables::new(files);
    let mut body = String::new();
    for slot in &slots {
        body.push_str(&emit_slot(slot, files, &bodies, &mut variables)?);
        body.push('\n');
    }
    if let Some(repeated) = first_repeated_id(&body) {
        return Err(bundle_error(
            &default.theme.name,
            format!("the bundle would carry the id \"{repeated}\" twice"),
        ));
    }

    let names: Vec<&str> = files.iter().map(|file| file.theme.name.as_str()).collect();
    let Some(default_parts) = parts.first() else {
        return Err(bundle_error("", "a bundle needs at least two themes"));
    };
    let root = default_parts
        .root
        .strip_suffix('>')
        .ok_or_else(|| bundle_error(&default.theme.name, "its <svg> tag does not close"))?;
    let mut output = format!(
        "{root} {THEMES_ATTRIBUTE}=\"{}\">\n  <defs>\n    <style>\n",
        names.join(" ")
    );
    for line in variables.style_rules(files) {
        output.push_str("      ");
        output.push_str(&line);
        output.push('\n');
    }
    output.push_str("    </style>\n  </defs>\n");
    for line in &default_parts.style {
        output.push_str(line);
        output.push('\n');
    }
    output.push_str(&body);
    Ok(output)
}

fn bundle_error(theme: &str, message: impl Into<String>) -> RenderError {
    RenderError::Bundle {
        theme: theme.to_string(),
        message: message.into(),
    }
}

/// Why a theme's root tag differs from the default's: its iso slab thickness when the two
/// differ, since that changes the projected canvas, else the two tags.
fn layout_difference(
    file: &ThemeFile<'_>,
    default: &ThemeFile<'_>,
    root: &str,
    default_root: &str,
) -> String {
    let thickness = file.theme.iso.slab_thickness;
    let default_thickness = default.theme.iso.slab_thickness;
    if thickness != default_thickness {
        return format!(
            "its iso slab thickness is {} where {}'s is {}, so the projected canvas differs",
            format_number(thickness),
            default.theme.name,
            format_number(default_thickness)
        );
    }
    format!(
        "its drawing is laid out differently: its <svg> tag is {root} where {}'s is {default_root}",
        default.theme.name
    )
}

/// The first `id` value that two lines of `body` carry.
fn first_repeated_id(body: &str) -> Option<&str> {
    let mut seen = std::collections::BTreeSet::new();
    body.lines().flat_map(line_ids).find(|id| !seen.insert(*id))
}

/// A written file in three parts: its root tag, the `<defs><style>` block the writer puts
/// directly after it (empty when the file has none), and every line after that.
struct FileParts<'a> {
    root: &'a str,
    style: Vec<&'a str>,
    body: Vec<String>,
    /// The 1-based file line of the first body line.
    body_first_line: usize,
}

fn split_file<'a>(svg: &'a str, theme: &str) -> Result<FileParts<'a>, RenderError> {
    let mut lines = svg.lines();
    let root = lines
        .next()
        .filter(|line| line.starts_with("<svg ") && line.ends_with('>'))
        .ok_or_else(|| bundle_error(theme, "its file does not open with an <svg> tag"))?;
    let rest: Vec<&str> = lines.collect();
    let mut style = Vec::new();
    let mut body_start = 0;
    let opens_style = rest.first().map(|line| line.trim_start()) == Some("<defs>")
        && rest.get(1).map(|line| line.trim_start()) == Some("<style>");
    if opens_style {
        let end = rest
            .iter()
            .position(|line| line.trim_start() == "</defs>")
            .ok_or_else(|| bundle_error(theme, "its style block does not close"))?;
        style.extend(rest.iter().take(end + 1).copied());
        body_start = end + 1;
    }
    let body: Vec<String> = rest
        .iter()
        .skip(body_start)
        .map(|line| (*line).to_string())
        .collect();
    if body.len() > LINES_MAX {
        return Err(bundle_error(
            theme,
            format!("its file has more than {LINES_MAX} lines"),
        ));
    }
    Ok(FileParts {
        root,
        style,
        body,
        body_first_line: body_start + 2,
    })
}

/// Every `id` a line carries, as the writer writes them: ` id="…"`, so a `data-id` is
/// never one. A one-line `<defs>` holds the ids of every definition inside it.
fn line_ids(line: &str) -> Vec<&str> {
    let mut ids = Vec::new();
    let mut rest = line;
    while let Some((_, after)) = rest.split_once(" id=\"") {
        let Some((id, following)) = after.split_once('"') else {
            break;
        };
        ids.push(id);
        rest = following;
    }
    ids
}

/// Renames, in `theme_parts`, every id of a one-line element whose line differs from an
/// earlier theme's line with the same id, in the definition and every reference, so the
/// bundle can keep every definition under its own id.
fn rename_differing_ids(earlier: &[FileParts<'_>], theme_parts: &mut FileParts<'_>, theme: &str) {
    let mut earlier_lines: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for line in earlier.iter().flat_map(|parts| parts.body.iter()) {
        if is_one_element(line) {
            for id in line_ids(line) {
                earlier_lines.entry(id).or_default().push(line.as_str());
            }
        }
    }
    let mut renames: Vec<(String, String)> = Vec::new();
    for line in theme_parts.body.iter().filter(|line| is_one_element(line)) {
        for id in line_ids(line) {
            let differs = earlier_lines
                .get(id)
                .is_some_and(|lines| lines.iter().any(|earlier_line| *earlier_line != line));
            if differs && !renames.iter().any(|(old, _)| old == id) {
                renames.push((id.to_string(), format!("{id}-{theme}")));
            }
        }
    }
    for line in &mut theme_parts.body {
        for (old, new) in &renames {
            *line = line
                .replace(&format!(" id=\"{old}\""), &format!(" id=\"{new}\""))
                .replace(&format!("href=\"#{old}\""), &format!("href=\"#{new}\""))
                .replace(&format!("url(#{old})"), &format!("url(#{new})"));
        }
    }
}

/// True for a line that opens and closes one element: self-closing, or closed on its line.
fn is_one_element(line: &str) -> bool {
    let element = line.trim_start();
    element.starts_with('<')
        && !element.starts_with("</")
        && (element.ends_with("/>") || element.ends_with('>') && element.contains("</"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// `<rect …/>`.
    SelfClosing,
    /// `<text …>…</text>`, or a `<defs>…</defs>` written on one line.
    Complete,
    /// `<g …>`: closed on a later line.
    Opener,
    /// `</g>`.
    Closer,
}

/// The elements the writer opens on one line and closes on a later one.
const OPENER_ELEMENTS: [&str; 3] = ["g", "a", "defs"];

/// The elements that are referenced rather than drawn, so one may stand per theme under
/// its own id without a `display` switch.
const DEFINITION_ELEMENTS: [&str; 9] = [
    "defs",
    "marker",
    "filter",
    "linearGradient",
    "radialGradient",
    "pattern",
    "symbol",
    "clipPath",
    "mask",
];

#[derive(Debug, Clone)]
struct ParsedLine {
    /// The 1-based line of the file, for messages.
    number: usize,
    indent: String,
    form: Form,
    element: String,
    attributes: Vec<(String, String)>,
    /// Everything after the open tag: text and the closing tag of a complete element.
    rest: String,
    /// The line with every paint value blanked: lines with equal keys align.
    key: String,
    /// True inside a `<defs>` block that spans lines, where every element is a definition.
    in_defs: bool,
}

impl ParsedLine {
    /// The line as written, with `attributes` in place of the parsed ones.
    fn written(&self, attributes: &[(String, String)]) -> String {
        if self.form == Form::Closer {
            return format!("{}</{}>", self.indent, self.element);
        }
        let mut line = format!("{}<{}", self.indent, self.element);
        for (name, value) in attributes {
            line.push_str(&format!(" {name}=\"{value}\""));
        }
        match self.form {
            Form::SelfClosing => line.push_str("/>"),
            Form::Complete | Form::Opener => line.push('>'),
            Form::Closer => {}
        }
        line.push_str(&self.rest);
        line
    }

    /// `<rect …>` for messages: the element with its first attribute.
    fn describe(&self) -> String {
        match (self.form, self.attributes.first()) {
            (Form::Closer, _) => format!("</{}>", self.element),
            (_, Some((name, value))) => format!("<{} {name}=\"{value}\" …>", self.element),
            (_, None) => format!("<{}>", self.element),
        }
    }

    fn has_id(&self) -> bool {
        self.attributes.iter().any(|(name, _)| name == "id") || !line_ids(&self.rest).is_empty()
    }

    /// True for an element that is referenced, never drawn on its own.
    fn is_definition(&self) -> bool {
        self.in_defs || DEFINITION_ELEMENTS.contains(&self.element.as_str())
    }
}

fn parse_body(parts: &FileParts<'_>, theme: &str) -> Result<Vec<ParsedLine>, RenderError> {
    let mut lines = Vec::with_capacity(parts.body.len());
    let mut defs_depth: usize = 0;
    for (index, line) in parts.body.iter().enumerate() {
        let mut parsed = parse_line(line, parts.body_first_line + index, theme)?;
        if parsed.element == "defs" {
            match parsed.form {
                Form::Opener => defs_depth += 1,
                Form::Closer => defs_depth = defs_depth.saturating_sub(1),
                Form::SelfClosing | Form::Complete => {}
            }
        } else {
            parsed.in_defs = defs_depth > 0;
        }
        lines.push(parsed);
    }
    Ok(lines)
}

fn parse_line(line: &str, number: usize, theme: &str) -> Result<ParsedLine, RenderError> {
    let malformed = |what: &str| {
        bundle_error(
            theme,
            format!("line {number} of its file is not one element per line ({what}): {line:.80}"),
        )
    };
    let element_start = line.trim_start();
    let indent = line
        .get(..line.len() - element_start.len())
        .unwrap_or_default()
        .to_string();
    if let Some(closer) = element_start.strip_prefix("</") {
        let element = closer
            .strip_suffix('>')
            .ok_or_else(|| malformed("closing tag"))?;
        let parsed = ParsedLine {
            number,
            indent,
            form: Form::Closer,
            element: element.to_string(),
            attributes: Vec::new(),
            rest: String::new(),
            key: String::new(),
            in_defs: false,
        };
        return Ok(ParsedLine {
            key: parsed.written(&[]),
            ..parsed
        });
    }
    let after_bracket = element_start
        .strip_prefix('<')
        .ok_or_else(|| malformed("no element"))?;
    let name_end = after_bracket
        .find([' ', '>', '/'])
        .ok_or_else(|| malformed("no tag end"))?;
    let element = after_bracket
        .get(..name_end)
        .unwrap_or_default()
        .to_string();
    let mut remaining = after_bracket.get(name_end..).unwrap_or_default();
    let mut attributes = Vec::new();
    let (form, rest) = loop {
        remaining = remaining.trim_start();
        if let Some(rest) = remaining.strip_prefix("/>") {
            break (Form::SelfClosing, rest);
        }
        if let Some(rest) = remaining.strip_prefix('>') {
            let closing = format!("</{element}>");
            let form = if rest.ends_with(&closing) {
                Form::Complete
            } else if rest.is_empty() && OPENER_ELEMENTS.contains(&element.as_str()) {
                Form::Opener
            } else {
                return Err(malformed("opens an element it does not close on its line"));
            };
            break (form, rest);
        }
        let (name, after_name) = remaining
            .split_once("=\"")
            .ok_or_else(|| malformed("an attribute has no value"))?;
        let (value, after_value) = after_name
            .split_once('"')
            .ok_or_else(|| malformed("an attribute is not closed"))?;
        attributes.push((name.to_string(), value.to_string()));
        remaining = after_value;
    };
    if form == Form::SelfClosing && !rest.is_empty() {
        return Err(malformed("text after a self-closing tag"));
    }
    let parsed = ParsedLine {
        number,
        indent,
        form,
        element,
        attributes,
        rest: rest.to_string(),
        key: String::new(),
        in_defs: false,
    };
    let masked: Vec<(String, String)> = parsed
        .attributes
        .iter()
        .map(|(name, value)| {
            let value = if is_paint_attribute(name) {
                String::new()
            } else {
                value.clone()
            };
            (name.clone(), value)
        })
        .collect();
    Ok(ParsedLine {
        key: parsed.written(&masked),
        ..parsed
    })
}

fn is_paint_attribute(name: &str) -> bool {
    PAINT_ATTRIBUTES.contains(&name)
}

/// One output line: the index of the line each theme contributes, None for a theme that
/// does not draw it.
#[derive(Debug, Clone)]
struct Slot {
    lines: Vec<Option<usize>>,
    key: String,
}

impl Slot {
    fn new(theme_count: usize, key: &str) -> Self {
        Slot {
            lines: vec![None; theme_count],
            key: key.to_string(),
        }
    }

    fn set(&mut self, theme_index: usize, line_index: usize) {
        if let Some(entry) = self.lines.get_mut(theme_index) {
            *entry = Some(line_index);
        }
    }
}

/// The longest common subsequence of the keys, as pairs of line indices in order. The
/// files share almost every line, so the common prefix and suffix are paired outright and
/// the table covers the middle alone.
fn align(default_keys: &[u32], theme_keys: &[u32]) -> Vec<(usize, usize)> {
    let prefix = default_keys
        .iter()
        .zip(theme_keys)
        .take_while(|(default_key, theme_key)| default_key == theme_key)
        .count();
    let suffix = default_keys
        .get(prefix..)
        .unwrap_or_default()
        .iter()
        .rev()
        .zip(theme_keys.get(prefix..).unwrap_or_default().iter().rev())
        .take_while(|(default_key, theme_key)| default_key == theme_key)
        .count();
    let default_middle = default_keys
        .get(prefix..default_keys.len() - suffix)
        .unwrap_or_default();
    let theme_middle = theme_keys
        .get(prefix..theme_keys.len() - suffix)
        .unwrap_or_default();
    let mut pairs: Vec<(usize, usize)> = (0..prefix).map(|index| (index, index)).collect();
    pairs.extend(
        align_middle(default_middle, theme_middle)
            .into_iter()
            .map(|(row, column)| (row + prefix, column + prefix)),
    );
    let default_tail = default_keys.len() - suffix;
    let theme_tail = theme_keys.len() - suffix;
    pairs.extend((0..suffix).map(|index| (default_tail + index, theme_tail + index)));
    pairs
}

/// The table holds the subsequence length of each pair of suffixes, so the walk pairs
/// lines from the front and a line a theme adds lands after the last line it shares.
fn align_middle(default_keys: &[u32], theme_keys: &[u32]) -> Vec<(usize, usize)> {
    let rows = default_keys.len();
    let columns = theme_keys.len();
    let width = columns + 1;
    let mut table = vec![0u16; (rows + 1) * width];
    let at = |table: &[u16], row: usize, column: usize| -> u16 {
        table.get(row * width + column).copied().unwrap_or(0)
    };
    for (row, default_key) in default_keys.iter().enumerate().rev() {
        for (column, theme_key) in theme_keys.iter().enumerate().rev() {
            let value = if default_key == theme_key {
                at(&table, row + 1, column + 1).saturating_add(1)
            } else {
                at(&table, row + 1, column).max(at(&table, row, column + 1))
            };
            if let Some(cell) = table.get_mut(row * width + column) {
                *cell = value;
            }
        }
    }
    let mut pairs = Vec::new();
    let (mut row, mut column) = (0, 0);
    while row < rows && column < columns {
        let same_key = default_keys.get(row) == theme_keys.get(column);
        if same_key && at(&table, row, column) == at(&table, row + 1, column + 1) + 1 {
            pairs.push((row, column));
            row += 1;
            column += 1;
        } else if at(&table, row + 1, column) >= at(&table, row, column + 1) {
            row += 1;
        } else {
            column += 1;
        }
    }
    pairs
}

/// Every theme's lines merged onto the default's: one slot per default line, with the
/// lines a theme inserts before it in slots of their own.
fn align_bodies(
    files: &[ThemeFile<'_>],
    bodies: &[Vec<ParsedLine>],
) -> Result<Vec<Slot>, RenderError> {
    let theme_count = files.len();
    let mut key_ids: BTreeMap<String, u32> = BTreeMap::new();
    let mut keys_of = |body: &[ParsedLine]| -> Vec<u32> {
        body.iter()
            .map(|line| {
                let next = key_ids.len() as u32;
                *key_ids.entry(line.key.clone()).or_insert(next)
            })
            .collect()
    };
    let Some(default_body) = bodies.first() else {
        return Ok(Vec::new());
    };
    let default_keys = keys_of(default_body);
    let mut spine: Vec<Slot> = default_body
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let mut slot = Slot::new(theme_count, &line.key);
            slot.set(0, index);
            slot
        })
        .collect();
    let mut inserts_before: Vec<Vec<Slot>> = vec![Vec::new(); default_body.len() + 1];

    for (theme_index, body) in bodies.iter().enumerate().skip(1) {
        let theme_keys = keys_of(body);
        let pairs = align(&default_keys, &theme_keys);
        let mut next_theme_line = 0;
        let mut pairs = pairs.into_iter().peekable();
        for anchor in 0..=default_body.len() {
            let aligned = pairs.next_if(|(default_index, _)| *default_index == anchor);
            // Lines before an aligned one are inserts at its anchor; a default line the
            // theme lacks anchors nothing, and what remains after the last pair goes last.
            let insert_end = match aligned {
                Some((_, theme_line)) => theme_line,
                None if anchor == default_body.len() => body.len(),
                None => continue,
            };
            let anchored = inserts_before.get_mut(anchor).ok_or_else(|| {
                bundle_error(
                    files.get(theme_index).map_or("", |file| &file.theme.name),
                    "alignment ran past the end",
                )
            })?;
            let mut cursor = 0;
            for theme_line in next_theme_line..insert_end {
                let key = body.get(theme_line).map_or("", |line| line.key.as_str());
                let found = anchored
                    .iter()
                    .enumerate()
                    .skip(cursor)
                    .find(|(_, slot)| slot.key == key && slot.lines.get(theme_index) == Some(&None))
                    .map(|(position, _)| position);
                let position = match found {
                    Some(position) => position,
                    None => {
                        anchored.insert(cursor, Slot::new(theme_count, key));
                        cursor
                    }
                };
                if let Some(slot) = anchored.get_mut(position) {
                    slot.set(theme_index, theme_line);
                }
                cursor = position + 1;
            }
            if let Some((_, theme_line)) = aligned {
                if let Some(slot) = spine.get_mut(anchor) {
                    slot.set(theme_index, theme_line);
                }
                next_theme_line = theme_line + 1;
            }
        }
    }

    let mut slots = Vec::with_capacity(spine.len() + inserts_before.len());
    for (anchor, spine_slot) in spine.into_iter().enumerate() {
        if let Some(inserted) = inserts_before.get_mut(anchor) {
            slots.append(inserted);
        }
        slots.push(spine_slot);
    }
    if let Some(trailing) = inserts_before.last_mut() {
        slots.append(trailing);
    }
    Ok(slots)
}

/// The bundle's line for one slot.
fn emit_slot(
    slot: &Slot,
    files: &[ThemeFile<'_>],
    bodies: &[Vec<ParsedLine>],
    variables: &mut Variables,
) -> Result<String, RenderError> {
    let line_of = |theme_index: usize| -> Option<&ParsedLine> {
        let line_index = (*slot.lines.get(theme_index)?)?;
        bodies.get(theme_index)?.get(line_index)
    };
    let present: Vec<usize> = (0..files.len())
        .filter(|theme_index| line_of(*theme_index).is_some())
        .collect();
    let Some(representative) = present
        .first()
        .and_then(|theme_index| line_of(*theme_index))
    else {
        return Err(bundle_error("", "an empty slot"));
    };
    let all_present = present.len() == files.len();
    if !all_present {
        if !matches!(representative.form, Form::SelfClosing | Form::Complete) {
            // The refused theme is the one that disagrees with the default.
            let (drawn_by, missing_in) = match slot.lines.first() {
                Some(Some(_)) => (0, present_complement(&present, files.len())),
                _ => (*present.first().unwrap_or(&0), 0),
            };
            let name_of = |theme_index: usize| {
                files
                    .get(theme_index)
                    .map_or("", |file| file.theme.name.as_str())
            };
            return Err(bundle_error(
                name_of(drawn_by.max(missing_in)),
                format!(
                    "{} draws {} at line {} of its file and {} does not, and a group cannot be switched in CSS",
                    name_of(drawn_by),
                    representative.describe(),
                    representative.number,
                    name_of(missing_in)
                ),
            ));
        }
        if representative.has_id() {
            if representative.is_definition() {
                return Ok(representative.written(&representative.attributes));
            }
            let drawer = *present.first().unwrap_or(&0);
            return Err(bundle_error(
                files.get(drawer).map_or("", |file| &file.theme.name),
                format!(
                    "{} at line {} carries an id and is not drawn by every theme, and a drawn element with an id cannot be switched",
                    representative.describe(),
                    representative.number
                ),
            ));
        }
        if representative
            .attributes
            .iter()
            .any(|(name, _)| name == "display")
        {
            return Err(bundle_error(
                files
                    .get(*present.first().unwrap_or(&0))
                    .map_or("", |file| &file.theme.name),
                format!(
                    "{} at line {} already sets display",
                    representative.describe(),
                    representative.number
                ),
            ));
        }
    }
    let mut attributes: Vec<(String, String)> =
        Vec::with_capacity(representative.attributes.len() + 1);
    for (position, (name, value)) in representative.attributes.iter().enumerate() {
        let written_value = if is_paint_attribute(name) {
            let values: Vec<String> = (0..files.len())
                .map(|theme_index| {
                    line_of(theme_index)
                        .and_then(|line| line.attributes.get(position))
                        .map_or(value.clone(), |(_, own)| own.clone())
                })
                .collect();
            if values.iter().all(|own| own == value) {
                value.clone()
            } else {
                format!("var({})", variables.name_for(name, values))
            }
        } else {
            value.clone()
        };
        attributes.push((name.clone(), written_value));
    }
    if !all_present {
        let values: Vec<String> = (0..files.len())
            .map(|theme_index| {
                if present.contains(&theme_index) {
                    DISPLAY_SHOWN.to_string()
                } else {
                    DISPLAY_HIDDEN.to_string()
                }
            })
            .collect();
        attributes.push((
            "display".to_string(),
            format!("var({})", variables.name_for("display", values)),
        ));
    }
    Ok(representative.written(&attributes))
}

/// The first theme index not in `present`.
fn present_complement(present: &[usize], theme_count: usize) -> usize {
    (0..theme_count)
        .find(|theme_index| !present.contains(theme_index))
        .unwrap_or(0)
}

/// The custom properties of a bundle: one per distinct tuple of values over the themes.
struct Variables {
    /// Each declared variable with its value per theme, in declaration order.
    declared: Vec<(String, Vec<String>)>,
    /// The variable of each attribute and value tuple already named.
    by_attribute_values: BTreeMap<(String, Vec<String>), String>,
    /// The tuple each colour role holds over the themes, by the first role that holds it.
    roles: BTreeMap<Vec<String>, String>,
    counters: BTreeMap<String, usize>,
}

impl Variables {
    fn new(files: &[ThemeFile<'_>]) -> Self {
        let per_theme: Vec<Vec<(String, String)>> =
            files.iter().map(|file| colour_roles(file.theme)).collect();
        let mut roles = BTreeMap::new();
        if let Some(default_roles) = per_theme.first() {
            // The sheet and terminal roles derive from the shallower ones, so a tuple is
            // named after the shortest path that holds it.
            let mut ordered: Vec<&String> = default_roles.iter().map(|(role, _)| role).collect();
            ordered.sort_by_key(|role| (role.matches('-').count(), role.as_str()));
            for role in ordered {
                let values: Option<Vec<String>> = per_theme
                    .iter()
                    .map(|theme_roles| {
                        theme_roles
                            .iter()
                            .find(|(name, _)| name == role)
                            .map(|(_, colour)| colour.clone())
                    })
                    .collect();
                if let Some(values) = values {
                    roles.entry(values).or_insert_with(|| role.clone());
                }
            }
        }
        let fade: Vec<String> = files
            .iter()
            .map(|file| fade_floor_of_page(file.dark_page).to_string())
            .collect();
        Variables {
            declared: vec![(FADE_VARIABLE.to_string(), fade)],
            by_attribute_values: BTreeMap::new(),
            roles,
            counters: BTreeMap::new(),
        }
    }

    /// The variable holding `values` for `attribute`, declared on first use: named after
    /// the colour role that holds the same values in every theme, which a fill and a stroke
    /// of that role share, else after the attribute and a count, which no other attribute
    /// shares.
    fn name_for(&mut self, attribute: &str, values: Vec<String>) -> String {
        let key = (attribute.to_string(), values.clone());
        if let Some(name) = self.by_attribute_values.get(&key) {
            return name.clone();
        }
        let name = match self.roles.get(&values) {
            Some(role) => format!("{VARIABLE_PREFIX}{role}"),
            None => {
                let count = self.counters.entry(attribute.to_string()).or_insert(0);
                *count += 1;
                format!("{VARIABLE_PREFIX}{attribute}-{count}")
            }
        };
        if !self.declared.iter().any(|(declared, _)| *declared == name) {
            self.declared.push((name.clone(), values));
        }
        self.by_attribute_values.insert(key, name.clone());
        name
    }

    fn declarations(&self, theme_index: usize) -> String {
        self.declared
            .iter()
            .map(|(name, values)| {
                format!(
                    "{name}: {};",
                    values.get(theme_index).map_or("", |value| value.as_str())
                )
            })
            .collect::<Vec<String>>()
            .join(" ")
    }

    /// The rules of the bundle's style block, one per line.
    fn style_rules(&self, files: &[ThemeFile<'_>]) -> Vec<String> {
        let mut rules = vec![format!(
            "svg[{THEMES_ATTRIBUTE}] {{ {} }}",
            self.declarations(0)
        )];
        let default_dark = files.first().is_some_and(|file| file.dark_page);
        let opposite = files
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, file)| file.dark_page != default_dark);
        if let Some((theme_index, _)) = opposite {
            let scheme = if default_dark { "light" } else { "dark" };
            rules.push(format!(
                "@media (prefers-color-scheme: {scheme}) {{ svg[{THEMES_ATTRIBUTE}]:not([{THEME_ATTRIBUTE}]) {{ {} }} }}",
                self.declarations(theme_index)
            ));
        }
        for (theme_index, file) in files.iter().enumerate() {
            rules.push(format!(
                "svg[{THEMES_ATTRIBUTE}][{THEME_ATTRIBUTE}=\"{}\"] {{ {} }}",
                file.theme.name,
                self.declarations(theme_index)
            ));
        }
        rules
    }
}

/// Every colour leaf of `theme` with its role name, `ink-primary` or `tints-3-fill` (the
/// slot number), in the theme's serialized key order.
fn colour_roles(theme: &Theme) -> Vec<(String, String)> {
    let mut roles = Vec::new();
    if let Ok(value) = serde_json::to_value(theme) {
        collect_colours(&value, &mut Vec::new(), &mut roles);
    }
    roles
}

fn collect_colours(value: &Value, path: &mut Vec<String>, roles: &mut Vec<(String, String)>) {
    match value {
        Value::Object(fields) => {
            for (key, field) in fields {
                path.push(key.clone());
                collect_colours(field, path, roles);
                path.pop();
            }
        }
        Value::Array(items) => {
            let slots = path.last().is_some_and(|key| key == "tints");
            for (index, item) in items.iter().enumerate() {
                path.push((if slots { index + 1 } else { index }).to_string());
                collect_colours(item, path, roles);
                path.pop();
            }
        }
        Value::String(text) if is_colour(text) => roles.push((path.join("-"), text.clone())),
        _ => {}
    }
}

fn is_colour(text: &str) -> bool {
    text.strip_prefix('#').is_some_and(|digits| {
        digits.len() == 6
            && digits
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A built-in theme under another name: `center` is light, `dusk` dark. No test names
    /// a colour literal; every colour is read from the theme.
    fn theme(name: &str, builtin: &str) -> Theme {
        let mut theme = crate::builtin_theme(builtin).unwrap().unwrap();
        theme.name = name.to_string();
        theme
    }

    const ROOT: &str =
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10">"#;

    fn file(lines: &[String]) -> String {
        let mut svg = ROOT.to_string();
        svg.push('\n');
        for line in lines {
            svg.push_str(line);
            svg.push('\n');
        }
        svg.push_str("</svg>\n");
        svg
    }

    fn bundle(entries: &[(&Theme, &str, bool)]) -> Result<String, RenderError> {
        let files: Vec<ThemeFile<'_>> = entries
            .iter()
            .map(|(theme, svg, dark_page)| ThemeFile {
                theme,
                svg,
                dark_page: *dark_page,
            })
            .collect();
        bundle_themes(&files)
    }

    fn style_lines(bundled: &str) -> Vec<&str> {
        bundled
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("svg[") || line.starts_with("@media"))
            .collect()
    }

    fn page_lines(theme: &Theme) -> Vec<String> {
        vec![
            format!(
                r#"  <rect x="0" y="0" width="10" height="10" fill="{}"/>"#,
                theme.page.as_str()
            ),
            format!(
                r#"  <text x="1" y="2" fill="{}">a</text>"#,
                theme.ink.primary.as_str()
            ),
            format!(
                r#"  <line x1="0" y1="0" x2="1" y2="1" stroke="{}" stroke-width="2"/>"#,
                theme.ink.primary.as_str()
            ),
        ]
    }

    #[test]
    fn a_differing_fill_becomes_a_variable_named_after_its_role() {
        let light = theme("light", "center");
        let dark = theme("dark", "dusk");
        let light_svg = file(&page_lines(&light));
        let dark_svg = file(&page_lines(&dark));
        let bundled = bundle(&[(&light, &light_svg, false), (&dark, &dark_svg, true)]).unwrap();
        assert!(bundled.starts_with(&format!(
            r#"{} data-themes="light dark">"#,
            ROOT.strip_suffix('>').unwrap()
        )));
        assert!(
            bundled.contains(
                r#"<rect x="0" y="0" width="10" height="10" fill="var(--stencil-page)"/>"#
            )
        );
        assert!(
            bundled.contains(r#"<text x="1" y="2" fill="var(--stencil-ink-primary)">a</text>"#)
        );
        assert!(bundled.contains(r#"stroke="var(--stencil-ink-primary)" stroke-width="2"/>"#));
        let drawing = bundled.split_once("</defs>").unwrap().1;
        assert!(!drawing.contains('#'), "{drawing}");
        let declarations = |theme: &Theme, fade: &str| {
            format!(
                "--stencil-fade: {fade}; --stencil-page: {}; --stencil-ink-primary: {};",
                theme.page.as_str(),
                theme.ink.primary.as_str()
            )
        };
        assert_eq!(
            style_lines(&bundled),
            vec![
                format!("svg[data-themes] {{ {} }}", declarations(&light, "0.3")),
                format!(
                    "@media (prefers-color-scheme: dark) {{ svg[data-themes]:not([data-theme]) {{ {} }} }}",
                    declarations(&dark, "0.35")
                ),
                format!(
                    r#"svg[data-themes][data-theme="light"] {{ {} }}"#,
                    declarations(&light, "0.3")
                ),
                format!(
                    r#"svg[data-themes][data-theme="dark"] {{ {} }}"#,
                    declarations(&dark, "0.35")
                ),
            ]
        );
    }

    #[test]
    fn a_shape_one_theme_lacks_is_switched_with_display_and_a_derived_colour_is_numbered() {
        let first = theme("first", "center");
        let second = theme("second", "center");
        let third = theme("third", "center");
        let first_svg = file(&[
            r#"  <g data-id="/a">"#.to_string(),
            r#"    <rect x="1" y="1" width="2" height="2" fill="red"/>"#.to_string(),
            "  </g>".to_string(),
        ]);
        let with_chip = file(&[
            r#"  <g data-id="/a">"#.to_string(),
            r#"    <rect x="1" y="1" width="2" height="2" fill="blue"/>"#.to_string(),
            r#"    <rect x="2" y="2" width="1" height="1" fill="white"/>"#.to_string(),
            "  </g>".to_string(),
        ]);
        let bundled = bundle(&[
            (&first, &first_svg, false),
            (&second, &with_chip, false),
            (&third, &with_chip, false),
        ])
        .unwrap();
        let body: Vec<&str> = bundled
            .lines()
            .skip_while(|line| !line.starts_with("  <g"))
            .collect();
        assert_eq!(
            body,
            vec![
                r#"  <g data-id="/a">"#,
                r#"    <rect x="1" y="1" width="2" height="2" fill="var(--stencil-fill-1)"/>"#,
                r#"    <rect x="2" y="2" width="1" height="1" fill="white" display="var(--stencil-display-1)"/>"#,
                "  </g>",
                "</svg>",
            ]
        );
        assert!(bundled.contains(
            r#"svg[data-themes][data-theme="first"] { --stencil-fade: 0.3; --stencil-fill-1: red; --stencil-display-1: none; }"#
        ));
        assert!(bundled.contains(
            r#"svg[data-themes][data-theme="third"] { --stencil-fade: 0.3; --stencil-fill-1: blue; --stencil-display-1: inline; }"#
        ));
        assert!(!bundled.contains("@media"));
    }

    #[test]
    fn a_differing_definition_is_renamed_after_its_theme_and_kept_without_a_switch() {
        let first = theme("first", "center");
        let second = theme("second", "center");
        let lines = |blur: &str| {
            vec![
                format!(
                    r#"  <defs><filter id="stencil-shadow"><feGaussianBlur stdDeviation="{blur}"/></filter></defs>"#
                ),
                r#"  <polygon points="0,0 1,1" fill="black" filter="url(#stencil-shadow)"/>"#
                    .to_string(),
            ]
        };
        let first_svg = file(&lines("2.5"));
        let second_svg = file(&lines("3"));
        let bundled =
            bundle(&[(&first, &first_svg, false), (&second, &second_svg, false)]).unwrap();
        assert!(bundled.contains(
            r#"  <defs><filter id="stencil-shadow"><feGaussianBlur stdDeviation="2.5"/></filter></defs>"#
        ));
        assert!(bundled.contains(
            r#"  <defs><filter id="stencil-shadow-second"><feGaussianBlur stdDeviation="3"/></filter></defs>"#
        ));
        assert!(!bundled.contains("display="), "{bundled}");
        assert!(bundled.contains(r#"filter="var(--stencil-filter-1)"/>"#));
        assert!(bundled.contains("--stencil-filter-1: url(#stencil-shadow);"));
        assert!(bundled.contains("--stencil-filter-1: url(#stencil-shadow-second);"));
    }

    #[test]
    fn a_definition_the_default_lacks_is_renamed_against_every_earlier_theme() {
        let plain = theme("plain", "wire");
        let second = theme("second", "center");
        let third = theme("third", "dusk");
        let lines = |blur: &str| {
            vec![
                format!(
                    r#"  <defs><filter id="stencil-shadow"><feGaussianBlur stdDeviation="{blur}"/></filter></defs>"#
                ),
                r#"  <polygon points="0,0 1,1" fill="black" filter="url(#stencil-shadow)"/>"#
                    .to_string(),
            ]
        };
        let bundled = bundle(&[
            (&plain, &file(&[]), false),
            (&second, &file(&lines("2.5")), false),
            (&third, &file(&lines("3")), true),
        ])
        .unwrap();
        assert!(
            bundled.contains(r#"<filter id="stencil-shadow"><feGaussianBlur stdDeviation="2.5"/>"#)
        );
        assert!(
            bundled.contains(
                r#"<filter id="stencil-shadow-third"><feGaussianBlur stdDeviation="3"/>"#
            )
        );
        assert!(bundled.contains("--stencil-filter-1: url(#stencil-shadow);"));
        assert!(bundled.contains("--stencil-filter-1: url(#stencil-shadow-third);"));
        assert_eq!(first_repeated_id(&bundled), None);
    }

    #[test]
    fn a_bundle_that_would_repeat_an_id_is_refused() {
        assert_eq!(
            first_repeated_id(
                "<rect id=\"a\"/>\n<g data-id=\"a\">\n<rect id=\"b\"/>\n<rect id=\"a\"/>"
            ),
            Some("a")
        );
        assert_eq!(
            first_repeated_id("<rect id=\"a\"/>\n<rect id=\"a-x\"/>"),
            None
        );
    }

    #[test]
    fn a_data_id_is_never_renamed_and_a_defs_line_renames_each_of_its_ids() {
        let first = theme("first", "center");
        let second = theme("second", "center");
        let first_svg = file(&[
            r#"  <defs><filter id="p"><feGaussianBlur stdDeviation="1"/></filter><filter id="q"><feGaussianBlur stdDeviation="1"/></filter></defs>"#.to_string(),
            r#"  <rect data-id="/a" x="0" y="0" width="1" height="1" fill="red" filter="url(#q)"/>"#.to_string(),
        ]);
        let second_svg = first_svg
            .replace(
                r#"stdDeviation="1"/></filter><filter"#,
                r#"stdDeviation="2"/></filter><filter"#,
            )
            .replace("fill=\"red\"", "fill=\"blue\"");
        let bundled =
            bundle(&[(&first, &first_svg, false), (&second, &second_svg, false)]).unwrap();
        assert!(bundled.contains(r#"<filter id="p-second">"#), "{bundled}");
        assert!(bundled.contains(r#"<filter id="q-second">"#), "{bundled}");
        assert!(bundled.contains(r#"data-id="/a" x="0""#), "{bundled}");
        assert!(!bundled.contains("data-id=\"/a-second\""), "{bundled}");
        assert!(
            bundled.contains(r#"fill="var(--stencil-fill-1)" filter="var(--stencil-filter-1)"/>"#)
        );
        assert!(bundled.contains("--stencil-filter-1: url(#q-second);"));
        assert_eq!(
            line_ids(r##"<g data-id="/a" id="b"><use href="#c" id="d"/></g>"##),
            vec!["b", "d"]
        );
    }

    #[test]
    fn a_drawn_element_with_an_id_that_one_theme_lacks_is_refused() {
        let first = theme("first", "center");
        let second = theme("second", "center");
        let first_svg =
            file(&[r#"  <rect id="r" x="0" y="0" width="1" height="1" fill="red"/>"#.to_string()]);
        let error =
            bundle(&[(&first, &first_svg, false), (&second, &file(&[]), false)]).unwrap_err();
        assert_eq!(
            error.to_string(),
            r#"theme first cannot be bundled: <rect id="r" …> at line 2 carries an id and is not drawn by every theme, and a drawn element with an id cannot be switched"#
        );
    }

    #[test]
    fn an_element_opened_and_not_closed_on_its_line_is_refused() {
        let error = parse_line(r#"  <text x="1">foo"#, 3, "t").unwrap_err();
        assert!(
            error.to_string().contains("line 3 of its file is not one element per line (opens an element it does not close on its line)"),
            "{error}"
        );
        assert!(parse_line(r#"  <g x="0"> "#, 1, "t").is_err());
        assert!(parse_line(r#"  <rect x="0">"#, 1, "t").is_err());
        assert_eq!(
            parse_line(r##"  <a href="#x">"##, 1, "t").unwrap().form,
            Form::Opener
        );
    }

    #[test]
    fn a_differing_slab_thickness_is_named_when_the_canvas_differs() {
        let first = theme("first", "center");
        let mut thick = theme("thick", "center");
        thick.iso.slab_thickness = 8.0;
        let first_svg = file(&[]);
        let wider = first_svg.replace(r#"width="10""#, r#"width="12""#);
        let error = bundle(&[(&first, &first_svg, false), (&thick, &wider, false)]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "theme thick cannot be bundled: its iso slab thickness is 8 where first's is 6, so the projected canvas differs"
        );
    }

    #[test]
    fn a_fill_and_a_stroke_width_with_equal_values_get_their_own_variables() {
        let first = theme("first", "center");
        let second = theme("second", "center");
        let lines = |value: &str| {
            vec![
                format!(r#"  <rect x="0" y="0" width="1" height="1" fill="{value}"/>"#),
                format!(
                    r#"  <line x1="0" y1="0" x2="1" y2="1" stroke="black" stroke-width="{value}"/>"#
                ),
            ]
        };
        let bundled = bundle(&[
            (&first, &file(&lines("1")), false),
            (&second, &file(&lines("2")), false),
        ])
        .unwrap();
        assert!(bundled.contains(r#"fill="var(--stencil-fill-1)""#));
        assert!(bundled.contains(r#"stroke-width="var(--stencil-stroke-width-1)""#));
    }

    #[test]
    fn a_different_canvas_and_a_group_one_theme_lacks_are_refused() {
        let first = theme("first", "center");
        let second = theme("second", "center");
        let first_svg = file(&[r#"  <g data-id="/a">"#.to_string(), "  </g>".to_string()]);
        let wider = first_svg.replace(r#"width="10""#, r#"width="12""#);
        let error = bundle(&[(&first, &first_svg, false), (&second, &wider, false)]).unwrap_err();
        let message = error.to_string();
        assert!(
            message
                .starts_with("theme second cannot be bundled: its drawing is laid out differently"),
            "{message}"
        );

        let with_group = file(&[
            r#"  <g data-id="/a">"#.to_string(),
            "  </g>".to_string(),
            r#"  <g data-id="/b">"#.to_string(),
            "  </g>".to_string(),
        ]);
        let error =
            bundle(&[(&first, &first_svg, false), (&second, &with_group, false)]).unwrap_err();
        assert_eq!(
            error.to_string(),
            r#"theme second cannot be bundled: second draws <g data-id="/b" …> at line 4 of its file and first does not, and a group cannot be switched in CSS"#
        );
        let error =
            bundle(&[(&first, &with_group, false), (&second, &first_svg, false)]).unwrap_err();
        assert_eq!(
            error.to_string(),
            r#"theme second cannot be bundled: first draws <g data-id="/b" …> at line 4 of its file and second does not, and a group cannot be switched in CSS"#
        );
    }

    #[test]
    fn a_bundle_of_one_theme_or_of_a_repeated_name_is_refused() {
        let first = theme("first", "center");
        let svg = file(&[]);
        assert!(
            bundle(&[(&first, &svg, false)])
                .unwrap_err()
                .to_string()
                .contains("at least two")
        );
        let message = bundle(&[(&first, &svg, false), (&first, &svg, false)])
            .unwrap_err()
            .to_string();
        assert_eq!(
            message,
            "theme first cannot be bundled: two bundled themes share this name"
        );
    }

    #[test]
    fn a_dark_default_takes_the_first_light_theme_for_the_light_scheme() {
        let dark = theme("dark", "dusk");
        let darker = theme("darker", "clear-dark");
        let light = theme("light", "center");
        let svg = |theme: &Theme| {
            file(&[format!(
                r#"  <rect x="0" y="0" width="10" height="10" fill="{}"/>"#,
                theme.page.as_str()
            )])
        };
        let bundled = bundle(&[
            (&dark, &svg(&dark), true),
            (&darker, &svg(&darker), true),
            (&light, &svg(&light), false),
        ])
        .unwrap();
        let rules = style_lines(&bundled);
        assert_eq!(rules.len(), 5);
        assert_eq!(
            rules[1],
            format!(
                "@media (prefers-color-scheme: light) {{ svg[data-themes]:not([data-theme]) {{ --stencil-fade: 0.3; --stencil-page: {}; }} }}",
                light.page.as_str()
            )
        );
    }

    #[test]
    fn the_aligner_pairs_equal_keys_in_order() {
        assert_eq!(
            align(&[1, 2, 3, 4], &[1, 9, 3, 4, 5]),
            vec![(0, 0), (2, 2), (3, 3)]
        );
        assert_eq!(align(&[], &[1]), Vec::new());
        assert_eq!(align(&[1, 1], &[1]), vec![(0, 0)]);
        assert_eq!(align(&[1, 2], &[1, 2, 3, 2]), vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn a_line_parses_into_its_form_attributes_and_masked_key() {
        let parsed = parse_line(r#"    <text x="1" fill="red">a &amp; b</text>"#, 7, "t").unwrap();
        assert_eq!(parsed.form, Form::Complete);
        assert_eq!(parsed.element, "text");
        assert_eq!(parsed.indent, "    ");
        assert_eq!(parsed.rest, "a &amp; b</text>");
        assert_eq!(parsed.key, r#"    <text x="1" fill="">a &amp; b</text>"#);
        assert_eq!(
            parsed.written(&parsed.attributes),
            r#"    <text x="1" fill="red">a &amp; b</text>"#
        );
        assert_eq!(parse_line("  </g>", 1, "t").unwrap().form, Form::Closer);
        assert_eq!(
            parse_line(r#"  <g data-id="/a">"#, 1, "t").unwrap().form,
            Form::Opener
        );
        assert_eq!(parse_line("  <defs>", 1, "t").unwrap().form, Form::Opener);
        assert!(parse_line(r#"  <rect x="1/>"#, 1, "t").is_err());
    }
}
