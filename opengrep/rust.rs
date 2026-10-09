fn todo_comments() {
    // ruleid: rust-todo-without-ticket
    // TODO: split the slab pass
    // ruleid: rust-todo-without-ticket
    /// TODO measure the tag before placing it
    // ok: rust-todo-without-ticket
    // TODO(#41): split the slab pass
    // ok: rust-todo-without-ticket
    // TODO: https://github.com/example/stencil/issues/41
    // ok: rust-todo-without-ticket
    // TODO ABC-12 split the slab pass
}

fn history_comments() {
    // ruleid: rust-history-trace-comment
    // The tag previously sat on the layout leg.
    // ruleid: rust-history-trace-comment
    /// This replaces the riser model.
    // ruleid: rust-history-trace-comment
    // The router used to push obstacles out 8 px.
    // ok: rust-history-trace-comment
    // The clearance is used to keep tubes off the slab edge.
    // ok: rust-history-trace-comment
    // A tag lies nearest its own connector.
}
