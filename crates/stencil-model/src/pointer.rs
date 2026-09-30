use std::fmt;

/// An RFC 6901 JSON Pointer naming a node or field of the input document.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodePointer(String);

impl NodePointer {
    /// The empty pointer, which names the page itself.
    pub fn root() -> Self {
        NodePointer(String::new())
    }

    /// Appends one reference token, escaping `~` as `~0` and `/` as `~1`.
    pub fn child(&self, token: &str) -> Self {
        let escaped = token.replace('~', "~0").replace('/', "~1");
        NodePointer(format!("{}/{}", self.0, escaped))
    }

    /// Appends an array index token.
    pub fn index(&self, index: usize) -> Self {
        NodePointer(format!("{}/{}", self.0, index))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodePointer {
    /// The raw pointer. The root prints as `""` so that it stays visible in check and
    /// violation lines (section 7).
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            formatter.write_str("\"\"")
        } else {
            formatter.write_str(&self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NodePointer;

    #[test]
    fn root_is_empty_and_displays_as_quotes() {
        let root = NodePointer::root();
        assert_eq!(root.as_str(), "");
        assert_eq!(root.to_string(), "\"\"");
    }

    #[test]
    fn child_and_index_build_body_pointers() {
        let pointer = NodePointer::root()
            .child("body")
            .index(0)
            .child("children")
            .index(2);
        assert_eq!(pointer.as_str(), "/body/0/children/2");
        assert_eq!(pointer.to_string(), "/body/0/children/2");
    }

    #[test]
    fn child_escapes_tilde_before_slash() {
        let pointer = NodePointer::root().child("a/b~c");
        assert_eq!(pointer.as_str(), "/a~1b~0c");
        let document = serde_json::json!({ "a/b~c": 7 });
        assert_eq!(
            document.pointer(pointer.as_str()),
            Some(&serde_json::json!(7))
        );
    }
}
