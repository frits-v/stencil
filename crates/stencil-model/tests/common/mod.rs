#![allow(dead_code, clippy::expect_used)]

use stencil_model::{
    Canvas, Col, Fact, LegendEntry, Node, Page, Pcard, Pipe, PipeDir, PipeKind, Row, Tee, TeeArm,
    Zone, ZoneKind,
};

pub const G7_JSON: &str = include_str!("../../../../examples/g7.json");

pub fn g7_value() -> serde_json::Value {
    serde_json::from_str(G7_JSON).expect("g7.json parses as JSON")
}

pub fn g7_page() -> Page {
    serde_json::from_str(G7_JSON).expect("g7.json parses as a Page")
}

pub fn pcard(function_name: &str) -> Node {
    Node::Pcard(Pcard {
        icon: None,
        function_name: function_name.to_string(),
        product_name: None,
        fact: None,
        ask: None,
    })
}

pub fn fact(text: &str) -> Node {
    Node::Fact(Fact {
        text: text.to_string(),
    })
}

pub fn pipe_value(dir: PipeDir, kind: PipeKind, label: &str) -> Pipe {
    Pipe {
        dir,
        kind,
        label: label.to_string(),
        sub: None,
    }
}

pub fn pipe(kind: PipeKind, label: &str) -> Node {
    Node::Pipe(pipe_value(PipeDir::Horizontal, kind, label))
}

pub fn tee(kind: PipeKind, first_arm: Pipe, second_arm: Pipe) -> Node {
    Node::Tee(Tee {
        kind,
        hub: "hub".to_string(),
        arms: [TeeArm::Pipe(first_arm), TeeArm::Pipe(second_arm)],
    })
}

pub fn row(children: Vec<Node>) -> Node {
    Node::Row(Row {
        gap: None,
        grow: None,
        justify: None,
        children,
    })
}

pub fn col(children: Vec<Node>) -> Node {
    Node::Col(Col {
        gap: None,
        grow: None,
        justify: None,
        children,
    })
}

pub fn zone(kind: ZoneKind, label: &str, children: Vec<Node>) -> Node {
    Node::Zone(Zone {
        kind,
        label: label.to_string(),
        children,
    })
}

pub fn legend_entry(kind: PipeKind, text: &str) -> LegendEntry {
    LegendEntry {
        kind,
        text: text.to_string(),
    }
}

/// A valid page holding `body` and a legend entry for the blue kind.
pub fn page_with_body(body: Vec<Node>) -> Page {
    Page {
        title: "Title".to_string(),
        kicker: "Kicker".to_string(),
        lede: "Lede".to_string(),
        foot: None,
        width: 1280,
        canvas: Canvas::Customer,
        body,
        legend: vec![legend_entry(PipeKind::Blue, "request path")],
    }
}

/// `levels` nested Cols with a Pcard at the bottom: the Pcard has depth `levels + 1`.
pub fn nested_cols(levels: usize) -> Node {
    let mut node = pcard("leaf");
    for _ in 0..levels {
        node = col(vec![node]);
    }
    node
}
