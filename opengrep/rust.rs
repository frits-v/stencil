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

// `opengrep test` ignores a rule's `paths`, so every `Command::new` here is also a finding of
// rust-review-child-environment, which the scan applies to the review modules only.
fn cargo_commands() {
    // ruleid: rust-cargo-build-bypasses-mbx, rust-review-child-environment
    run(Command::new("cargo").args(["clippy", "--workspace", "--all-targets"]));
    // ruleid: rust-cargo-build-bypasses-mbx, rust-review-child-environment
    run(Command::new("cargo").arg("test"));
    // ok: rust-cargo-build-bypasses-mbx
    // ruleid: rust-review-child-environment
    run(Command::new("cargo").args(["fmt", "--all", "--check"]));
    // ok: rust-cargo-build-bypasses-mbx
    // ruleid: rust-review-child-environment
    run(Command::new("mbx").args(["build", "--release"]));
}

fn review_children() {
    // ruleid: rust-review-child-environment
    let git = Command::new("git");
    // ruleid: rust-review-child-environment
    let mise = std::process::Command::new("mise");
    // ok: rust-review-child-environment
    let git = crate::child::command("git");
    // ok: rust-review-child-environment
    let claude = crate::child::empty(program);
}
