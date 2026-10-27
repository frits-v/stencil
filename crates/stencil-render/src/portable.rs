//! The portable SVG (section 5.2): the written SVG with each `<text>` drawn as its glyph
//! outlines and each icon `<image>` inlined as a nested `<svg>`, so it draws the same where
//! a content security policy blocks fonts and `data:` images. GitHub serves attachments and
//! raw files under `default-src 'none'; style-src 'unsafe-inline'`, which keeps the hover
//! rules running and blocks both.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use base64::Engine;
use std::collections::BTreeMap;

use resvg::usvg;
use skrifa::MetadataProvider;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::LocationRef;
use stencil_text::verify_bundled_fonts;

use crate::png::{bundled_font_database, data_only_image_resolver, strict_font_resolver};
use crate::{RenderError, SvgDocument, format_number};

/// Attributes of a `<text>` that only place or shape its glyphs; the outline carries none.
const TEXT_LAYOUT_ATTRIBUTES: [&str; 7] = [
    "x",
    "y",
    "xml:space",
    "font-family",
    "font-size",
    "font-weight",
    "letter-spacing",
];

/// Attributes of a `<text>` the outline carries as they are: its paint and the hover keys
/// of section 5.2, which select the outline as they select the text.
const TEXT_KEPT_ATTRIBUTES: [&str; 3] = ["fill", "data-row", "data-mark"];

/// Attributes of an icon `<image>`. The nested `<svg>` takes the four box attributes.
const IMAGE_ATTRIBUTES: [&str; 5] = ["x", "y", "width", "height", "href"];

const SVG_DATA_PREFIX: &str = "data:image/svg+xml;base64,";

impl SvgDocument {
    /// The written SVG of `self_contained` without its `@font-face` rules, each `<text>`
    /// replaced by a `<path>` of its glyph outlines followed by the text itself at
    /// `fill-opacity="0"`, which keeps it selectable and searchable, and each icon
    /// `<image>` replaced by its icon as a nested `<svg>`. The outlines are the ones the
    /// PNG rasterizes: usvg lays the text out over the bundled faces only.
    pub fn portable(&self) -> Result<String, RenderError> {
        let fontless = SvgDocument {
            font_faces: Vec::new(),
            ..self.clone()
        };
        let written = fontless.self_contained()?;
        let outlined = outline_svg(&written)?;
        if outlined.texts != self.text_elements {
            return Err(RenderError::Portable {
                message: format!(
                    "outlined {} text element(s) of the {} written",
                    outlined.texts, self.text_elements
                ),
            });
        }
        Ok(outlined.svg)
    }
}

#[derive(Debug)]
struct Outlined {
    svg: String,
    texts: usize,
}

/// Rewrites every line that holds a `<text>` or an `<image>`. The SVG writer puts each of
/// them on a line of its own, so an element that does not close on its line is refused.
/// The glyph outlines are written once each, in a `<defs>` before the closing `</svg>`.
fn outline_svg(svg: &str) -> Result<Outlined, RenderError> {
    verify_bundled_fonts()?;
    let font_misses = Arc::new(AtomicUsize::new(0));
    let options = usvg::Options {
        fontdb: Arc::new(bundled_font_database()?),
        resources_dir: None,
        font_resolver: strict_font_resolver(Arc::clone(&font_misses)),
        image_href_resolver: data_only_image_resolver(),
        ..usvg::Options::default()
    };

    let body = svg
        .trim_end()
        .strip_suffix("</svg>")
        .ok_or_else(|| portable_error("the SVG does not end with </svg>"))?;
    let mut glyphs = GlyphDefs::default();
    let mut output = String::with_capacity(svg.len());
    let mut texts = 0;
    let mut images = 0;
    for line in body.split_inclusive('\n') {
        let element = line.trim_start();
        let indent = line.get(..line.len() - element.len()).unwrap_or_default();
        if element.starts_with("<text ") {
            output.push_str(indent);
            output.push_str(&outline_text(element.trim_end(), &options, &mut glyphs)?);
            output.push('\n');
            output.push_str(indent);
            output.push_str(&hidden_text(element.trim_end())?);
            output.push('\n');
            texts += 1;
        } else if element.starts_with("<image ") {
            output.push_str(indent);
            output.push_str(&inline_image(element.trim_end(), images, &options)?);
            output.push('\n');
            images += 1;
        } else {
            output.push_str(line);
        }
    }
    let lookups = font_misses.load(Ordering::SeqCst);
    if lookups > 0 {
        return Err(RenderError::FontNotResolved { lookups });
    }
    if !glyphs.paths.is_empty() {
        output.push_str("  <defs>\n");
        for (index, path_data) in glyphs.paths.iter().enumerate() {
            output.push_str(&format!(
                "    <path id=\"{GLYPH_ID_PREFIX}{index}\" d=\"{path_data}\"/>\n"
            ));
        }
        output.push_str("  </defs>\n");
    }
    output.push_str("</svg>\n");
    Ok(Outlined { svg: output, texts })
}

const GLYPH_ID_PREFIX: &str = "stencil-glyph-";

/// The outline of each glyph the page draws, in font units, once per face and glyph.
#[derive(Default)]
struct GlyphDefs {
    index: BTreeMap<(usvg::fontdb::ID, u32), usize>,
    paths: Vec<String>,
}

impl GlyphDefs {
    /// The def index of `glyph`, reading its outline on first use; `None` for a glyph with
    /// no outline, such as a space.
    fn def_of(
        &mut self,
        database: &usvg::fontdb::Database,
        glyph: &usvg::layout::PositionedGlyph,
    ) -> Result<Option<usize>, RenderError> {
        let key = (glyph.font, glyph.id.0);
        if let Some(index) = self.index.get(&key) {
            return Ok(Some(*index));
        }
        let path_data = database
            .with_face_data(glyph.font, |data, face_index| {
                glyph_outline(data, face_index, glyph.id.0)
            })
            .ok_or_else(|| portable_error("a glyph's face is not in the font database"))??;
        if path_data.is_empty() {
            return Ok(None);
        }
        let index = self.paths.len();
        self.paths.push(path_data);
        self.index.insert(key, index);
        Ok(Some(index))
    }
}

/// The outline of glyph `glyph_id` in font units, y up, as path data, read the way usvg
/// reads it for the PNG: unhinted at the default location of every axis.
fn glyph_outline(data: &[u8], face_index: u32, glyph_id: u32) -> Result<String, RenderError> {
    let font = skrifa::FontRef::from_index(data, face_index)
        .map_err(|error| portable_error(format!("a bundled face does not read: {error}")))?;
    let Some(outline) = font.outline_glyphs().get(skrifa::GlyphId::new(glyph_id)) else {
        return Err(portable_error(format!(
            "glyph {glyph_id} has no outline entry"
        )));
    };
    let mut pen = PathDataPen::default();
    outline
        .draw(
            DrawSettings::unhinted(skrifa::prelude::Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .map_err(|error| portable_error(format!("glyph {glyph_id} does not draw: {error}")))?;
    Ok(pen.path_data)
}

#[derive(Default)]
struct PathDataPen {
    path_data: String,
}

impl PathDataPen {
    fn command(&mut self, letter: char, coordinates: &[f32]) {
        if !self.path_data.is_empty() {
            self.path_data.push(' ');
        }
        self.path_data.push(letter);
        let mut first = true;
        for coordinate in coordinates {
            if !first {
                self.path_data.push(' ');
            }
            first = false;
            self.path_data.push_str(&number(*coordinate));
        }
    }
}

impl OutlinePen for PathDataPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.command('M', &[x, y]);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.command('L', &[x, y]);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.command('Q', &[cx0, cy0, x, y]);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.command('C', &[cx0, cy0, cx1, cy1, x, y]);
    }

    fn close(&mut self) {
        self.command('Z', &[]);
    }
}

/// One `<g>` with the paint and hover keys of `element`, a `<text>` that closes on its
/// line, holding a `<use>` of the glyph def for each glyph usvg places, at the place and
/// size usvg gives it.
fn outline_text(
    element: &str,
    options: &usvg::Options<'_>,
    glyphs: &mut GlyphDefs,
) -> Result<String, RenderError> {
    let open_tag = open_tag(element, "</text>")?;
    let attributes = parse_attributes(open_tag)?;
    let mut kept = String::new();
    for (name, value) in &attributes {
        if TEXT_KEPT_ATTRIBUTES.contains(name) {
            kept.push_str(&format!(r#" {name}="{value}""#));
        } else if !TEXT_LAYOUT_ATTRIBUTES.contains(name) {
            return Err(portable_error(format!(
                "a <text> carries {name}, which the portable SVG does not map: {element:.80}"
            )));
        }
    }

    let document = format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{element}</svg>"#);
    let tree = usvg::Tree::from_str(&document, options)
        .map_err(|error| portable_error(format!("{error}: {element:.80}")))?;
    let [usvg::Node::Text(text)] = tree.root().children() else {
        return Err(portable_error(format!(
            "a <text> did not parse to one text node: {element:.80}"
        )));
    };
    let mut uses = String::new();
    for span in text.layouted() {
        if span.underline.is_some() || span.overline.is_some() || span.line_through.is_some() {
            return Err(portable_error(format!(
                "a <text> carries a decoration: {element:.80}"
            )));
        }
        for glyph in &span.positioned_glyphs {
            let Some(index) = glyphs.def_of(tree.fontdb(), glyph)? else {
                continue;
            };
            let placed = text.abs_transform().pre_concat(glyph.outline_transform());
            if placed.kx != 0.0 || placed.ky != 0.0 {
                return Err(portable_error(format!(
                    "a glyph is rotated or skewed: {element:.80}"
                )));
            }
            uses.push_str(&format!(
                r##"<use href="#{GLYPH_ID_PREFIX}{index}" transform="translate({} {}) scale({} {})"/>"##,
                number(placed.tx),
                number(placed.ty),
                scale_number(placed.sx),
                scale_number(placed.sy),
            ));
        }
    }
    if uses.is_empty() {
        return Err(RenderError::TextNotRendered { count: 1 });
    }
    Ok(format!("<g{kept}>{uses}</g>"))
}

fn number(value: f32) -> String {
    format_number(value).to_string()
}

/// A glyph scale, font size over units per em: about 0.005 for 11 px Inter, so it keeps
/// six decimals where `format_number` keeps two.
fn scale_number(value: f32) -> String {
    let written = format!("{value:.6}");
    written
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// `element` with its fill at zero opacity. The hover fade sets `opacity`, which leaves
/// `fill-opacity` alone, so the text stays unseen while its group fades.
fn hidden_text(element: &str) -> Result<String, RenderError> {
    let rest = element
        .strip_prefix("<text ")
        .ok_or_else(|| portable_error(format!("not a <text>: {element:.80}")))?;
    Ok(format!(r#"<text fill-opacity="0" {rest}"#))
}

/// The icon of `element`, an `<image>` whose href is a base64 SVG `data:` URI, as a
/// nested `<svg>` on the image's box. usvg normalizes the icon first: its styles become
/// attributes, so no `<style>` of an icon reaches the page, and every id gains the prefix
/// `stencil-icon-<index>-`, so two copies of one icon never share a gradient id.
fn inline_image(
    element: &str,
    index: usize,
    options: &usvg::Options<'_>,
) -> Result<String, RenderError> {
    let open_tag = open_tag(element, "/>")?;
    let attributes = parse_attributes(open_tag)?;
    let value = |wanted: &str| {
        attributes
            .iter()
            .find(|(name, _)| *name == wanted)
            .map(|(_, value)| *value)
            .ok_or_else(|| portable_error(format!("an <image> has no {wanted}: {element:.80}")))
    };
    if let Some((name, _)) = attributes
        .iter()
        .find(|(name, _)| !IMAGE_ATTRIBUTES.contains(name))
    {
        return Err(portable_error(format!(
            "an <image> carries {name}, which the portable SVG does not map: {element:.80}"
        )));
    }
    let encoded = value("href")?
        .strip_prefix(SVG_DATA_PREFIX)
        .ok_or_else(|| {
            portable_error(format!("an <image> is not an SVG data URI: {element:.80}"))
        })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| portable_error(format!("an icon does not decode: {error}")))?;
    let tree = usvg::Tree::from_data(&bytes, options)
        .map_err(|error| portable_error(format!("an icon does not parse: {error}")))?;
    let written = tree.to_string(&usvg::WriteOptions {
        id_prefix: Some(format!("stencil-icon-{index}-")),
        coordinates_precision: 3,
        transforms_precision: 4,
        indent: usvg::Indent::None,
        attributes_indent: usvg::Indent::None,
        ..usvg::WriteOptions::default()
    });
    let (root_tag, body) = written
        .split_once('>')
        .ok_or_else(|| portable_error("a normalized icon has no root element"))?;
    let body = body
        .trim_end()
        .strip_suffix("</svg>")
        .ok_or_else(|| portable_error("a normalized icon does not close its root"))?;
    let xlink = if root_tag.contains("xmlns:xlink") {
        r#" xmlns:xlink="http://www.w3.org/1999/xlink""#
    } else {
        ""
    };
    let size = tree.size();
    Ok(format!(
        r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 {} {}"{xlink}>{body}</svg>"#,
        value("x")?,
        value("y")?,
        value("width")?,
        value("height")?,
        number(size.width()),
        number(size.height()),
    ))
}

/// The attributes of `element` up to its first `>`, after checking that the element ends
/// with `close` on its line.
fn open_tag<'a>(element: &'a str, close: &str) -> Result<&'a str, RenderError> {
    if !element.ends_with(close) {
        return Err(portable_error(format!(
            "an element does not close on its line: {element:.80}"
        )));
    }
    let (open_tag, _) = element
        .split_once('>')
        .ok_or_else(|| portable_error(format!("an element has no end: {element:.80}")))?;
    Ok(open_tag.trim_end_matches('/'))
}

/// The `name="value"` pairs of an open tag as the SVG writer writes them: double quotes,
/// one space before each name, no quote inside a value.
fn parse_attributes(open_tag: &str) -> Result<Vec<(&str, &str)>, RenderError> {
    let (_, mut rest) = open_tag
        .split_once(' ')
        .ok_or_else(|| portable_error(format!("an element has no attributes: {open_tag:.80}")))?;
    let mut attributes = Vec::new();
    for _ in 0..open_tag.len() {
        rest = rest.trim_start();
        if rest.is_empty() {
            return Ok(attributes);
        }
        let (name, after_name) = rest
            .split_once("=\"")
            .ok_or_else(|| portable_error(format!("an attribute has no value: {rest:.80}")))?;
        let (value, after_value) = after_name
            .split_once('"')
            .ok_or_else(|| portable_error(format!("an attribute is not closed: {rest:.80}")))?;
        attributes.push((name, value));
        rest = after_value;
    }
    Err(portable_error(format!(
        "an open tag did not parse: {open_tag:.80}"
    )))
}

fn portable_error(message: impl Into<String>) -> RenderError {
    RenderError::Portable {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_parse_in_order_and_an_unclosed_value_is_refused() {
        let attributes = parse_attributes(r##"<text x="1" data-row="/a b" fill="#FFF""##).unwrap();
        assert_eq!(
            attributes,
            vec![("x", "1"), ("data-row", "/a b"), ("fill", "#FFF")]
        );
        assert!(parse_attributes(r#"<text x="1"#).is_err());
    }

    #[test]
    fn a_text_with_an_unmapped_attribute_is_refused() {
        let options = usvg::Options::default();
        let error = outline_text(
            r#"<text x="1" y="2" text-anchor="middle">A</text>"#,
            &options,
            &mut GlyphDefs::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("text-anchor"), "{error}");
    }

    #[test]
    fn a_text_that_spans_two_lines_is_refused() {
        let error = outline_svg("<svg>\n<text x=\"1\" y=\"2\">A\n</text>\n</svg>\n").unwrap_err();
        assert!(error.to_string().contains("does not close"), "{error}");
    }

    #[test]
    fn an_image_that_is_not_an_svg_data_uri_is_refused() {
        let options = usvg::Options::default();
        let error = inline_image(
            r#"<image x="0" y="0" width="28" height="28" href="data:image/png;base64,AAAA"/>"#,
            0,
            &options,
        )
        .unwrap_err();
        assert!(error.to_string().contains("not an SVG data URI"), "{error}");
    }

    #[test]
    fn the_hidden_text_keeps_its_attributes_and_content() {
        assert_eq!(
            hidden_text(r##"<text x="1" fill="#000">a &amp; b</text>"##).unwrap(),
            r##"<text fill-opacity="0" x="1" fill="#000">a &amp; b</text>"##
        );
    }
}
