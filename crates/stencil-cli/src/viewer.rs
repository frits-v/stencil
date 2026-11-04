//! `render --html`: one self-contained page around the written SVG, with the measured JSON
//! and the description as data and a script for pan and zoom, pinned selections, legend
//! filters, text search, grid references and the theme menu of a bundle (section 7).

use serde_json::{Value, json};

const TEMPLATE: &str = include_str!("../viewer/page.html");

/// The page for one render. `svg` is inlined byte for byte, so the page draws what the SVG
/// file draws; `measured` and `description` are the other two outputs. The document's
/// title and kicker name the page.
pub fn page(measured: &Value, description: &str, svg: &str) -> Result<String, serde_json::Error> {
    let document = measured.get("document");
    let field = |name: &str| {
        document
            .and_then(|value| value.get(name))
            .and_then(Value::as_str)
            .unwrap_or("")
    };
    let data = serde_json::to_string(&json!({
        "measured": measured,
        "description": description,
    }))?;
    // A `</script` inside the data would end the block early.
    let data = data.replace('<', "\\u003c");
    Ok(fill(
        TEMPLATE,
        &[
            ("title", &escape(field("title"))),
            ("kicker", &escape(field("kicker"))),
            ("data", &data),
            ("svg", svg),
        ],
    ))
}

/// Replaces each `{{name}}` of the template with its value in one pass, so a value is never
/// scanned for placeholders.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(
        template.len() + values.iter().map(|(_, value)| value.len()).sum::<usize>(),
    );
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = &after[..end];
        match values.iter().find(|(known, _)| *known == name) {
            Some((_, value)) => out.push_str(value),
            None => out.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured() -> Value {
        json!({
            "canvas": {"width": 10, "height": 20},
            "document": {"title": "A <b> & \"c\"", "kicker": "k"},
            "nodes": [],
            "links": []
        })
    }

    #[test]
    fn the_page_inlines_the_svg_byte_for_byte_and_names_itself_after_the_title() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 20\"><text>{{data}}</text></svg>";

        let page = page(&measured(), "# Head\n", svg).unwrap();

        assert!(page.contains(svg), "the SVG is inlined unchanged");
        assert!(page.contains("<title>A &lt;b&gt; &amp; &quot;c&quot;</title>"));
        assert_eq!(
            page.matches("{{").count(),
            1,
            "only the SVG's own braces remain"
        );
        assert!(page.starts_with("<!doctype html>"));
    }

    #[test]
    fn the_data_block_escapes_angle_brackets_and_parses_back() {
        let page = page(&measured(), "a </script> b", "<svg/>").unwrap();

        let start = page
            .find("<script type=\"application/json\" id=\"stencil-data\">")
            .unwrap();
        let block = &page[start..];
        let open = block.find('>').unwrap() + 1;
        let close = block.find("</script>").unwrap();
        let data = &block[open..close];
        assert!(!data.contains('<'));
        let value: Value = serde_json::from_str(data).unwrap();
        assert_eq!(value["description"], json!("a </script> b"));
        assert_eq!(value["measured"], measured());
    }

    #[test]
    fn fill_keeps_an_unknown_placeholder_and_an_unclosed_one() {
        assert_eq!(fill("a {{x}} b {{y}} c", &[("x", "1")]), "a 1 b {{y}} c");
        assert_eq!(fill("a {{x", &[("x", "1")]), "a {{x");
        assert_eq!(fill("{{x}}{{x}}", &[("x", "{{x}}")]), "{{x}}{{x}}");
    }
}
