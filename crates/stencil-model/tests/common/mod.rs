#![allow(dead_code, clippy::expect_used)]

use stencil_model::{
    Arrow, BoxNode, Callout, CalloutKind, Canvas, Chrome, Col, FRAME_HEIGHT_DEFAULT, Fact,
    FactSource, Frame, Grammar, Item, LegendEntry, Line, Link, ListKind, Node, Page, Pipe, PipeDir,
    Row, Tee, TeeArm, Text, builtin_grammar,
};

pub const G7_JSON: &str = include_str!("../../../../examples/g7.json");

pub fn g7_value() -> serde_json::Value {
    serde_json::from_str(G7_JSON).expect("g7.json parses as JSON")
}

pub fn g7_page() -> Page {
    serde_json::from_str(G7_JSON).expect("g7.json parses as a Page")
}

pub fn gcp() -> Grammar {
    builtin_grammar("gcp")
        .expect("gcp is a built-in grammar")
        .expect("the gcp grammar is valid")
}

pub fn plain() -> Grammar {
    builtin_grammar("plain")
        .expect("plain is a built-in grammar")
        .expect("the plain grammar is valid")
}

/// A gcp `product` Item with only a title.
pub fn item(title: &str) -> Node {
    Node::Item(Item {
        id: None,
        kind: "product".to_string(),
        icon: None,
        title: title.to_string(),
        subtitle: None,
        facts: Vec::new(),
        shape: None,
    })
}

pub fn fact(text: &str) -> Node {
    Node::Fact(Fact {
        id: None,
        text: text.to_string(),
        source: FactSource::Doc,
    })
}

pub fn pipe_value(dir: PipeDir, line: Line, tint: Option<u8>, label: &str) -> Pipe {
    Pipe {
        id: None,
        arrow: stencil_model::Arrow::None,
        axis: None,
        form: stencil_model::PipeForm::Tube,
        dir,
        line,
        tint,
        label: label.to_string(),
        sub: None,
        from: None,
        to: None,
    }
}

pub fn pipe(line: Line, tint: Option<u8>, label: &str) -> Node {
    Node::Pipe(pipe_value(PipeDir::Horizontal, line, tint, label))
}

pub fn tee(line: Line, tint: Option<u8>, first_arm: Pipe, second_arm: Pipe) -> Node {
    Node::Tee(Tee {
        id: None,
        line,
        tint,
        hub: "hub".to_string(),
        arms: [TeeArm::Pipe(first_arm), TeeArm::Pipe(second_arm)],
    })
}

pub fn row(children: Vec<Node>) -> Node {
    Node::Row(Row {
        id: None,
        gap: None,
        grow: None,
        justify: None,
        children,
    })
}

pub fn col(children: Vec<Node>) -> Node {
    Node::Col(Col {
        id: None,
        gap: None,
        grow: None,
        justify: None,
        children,
    })
}

pub fn box_node(kind: &str, tint: Option<u8>, label: &str, children: Vec<Node>) -> Node {
    Node::Box(BoxNode {
        id: None,
        kind: kind.to_string(),
        tint,
        label: label.to_string(),
        children,
    })
}

pub fn legend_entry(line: Line, tint: Option<u8>, text: &str) -> LegendEntry {
    LegendEntry {
        line,
        tint,
        form: stencil_model::PipeForm::Tube,
        text: text.to_string(),
    }
}

/// A valid page holding `body` and a legend entry for solid tint 1.
pub fn page_with_body(body: Vec<Node>) -> Page {
    Page {
        title: "Title".to_string(),
        kicker: "Kicker".to_string(),
        lede: "Lede".to_string(),
        foot: None,
        width: 1280,
        canvas: Canvas::Customer,
        grammar: None,
        theme: None,
        theme_overrides: None,
        projection: stencil_model::Projection::Flat,
        chrome: Chrome::Full,
        body,
        legend: vec![legend_entry(Line::Solid, Some(1), "request path")],
        links: Vec::new(),
    }
}

/// `levels` nested Cols with an Item at the bottom: the Item has depth `levels + 1`.
pub fn nested_cols(levels: usize) -> Node {
    let mut node = item("leaf");
    for _ in 0..levels {
        node = col(vec![node]);
    }
    node
}

/// A gcp `product` Item carrying `id`.
pub fn item_with_id(id: &str, title: &str) -> Node {
    Node::Item(Item {
        id: Some(id.to_string()),
        kind: "product".to_string(),
        icon: None,
        title: title.to_string(),
        subtitle: None,
        facts: Vec::new(),
        shape: None,
    })
}

pub fn text_block(heading: Option<&str>, lines: &[&str], list: ListKind) -> Node {
    Node::Text(Text {
        id: None,
        heading: heading.map(str::to_string),
        body: lines.iter().map(|line| line.to_string()).collect(),
        list,
    })
}

pub fn callout(kind: CalloutKind, title: Option<&str>, text: &str) -> Node {
    Node::Callout(Callout {
        id: None,
        kind,
        title: title.map(str::to_string),
        text: text.to_string(),
    })
}

pub fn frame(label: &str) -> Node {
    Node::Frame(Frame {
        id: None,
        label: label.to_string(),
        height: FRAME_HEIGHT_DEFAULT,
    })
}

/// A solid tint 1 link with no label, sides or via points.
pub fn link(from: &str, to: &str) -> Link {
    Link {
        from: from.to_string(),
        to: to.to_string(),
        line: Line::Solid,
        tint: Some(1),
        label: None,
        sub: None,
        arrow: Arrow::End,
        axis: None,
        from_side: None,
        to_side: None,
        via: Vec::new(),
        order: None,
    }
}

/// A valid page of two items with ids `api` and `worker`, one solid tint 1 link between
/// them and a legend entry for it, so every section 11.2 rule starts from zero violations.
pub fn page_with_link() -> Page {
    let mut page = page_with_body(vec![
        item_with_id("api", "API"),
        item_with_id("worker", "Worker"),
    ]);
    page.links = vec![link("api", "worker")];
    page
}
