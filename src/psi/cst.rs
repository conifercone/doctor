//! CST (Concrete Syntax Tree) parser using tree-sitter-java.

use std::path::Path;
use tree_sitter::{Node, Parser, Tree};

/// Parse a Java source file and return its CST tree.
pub fn parse_java_file(path: &Path) -> Result<Tree, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    parse_java_source(&source)
}

/// Parse Java source code from a string.
pub fn parse_java_source(source: &str) -> Result<Tree, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|e| format!("Failed to set tree-sitter language: {e}"))?;
    parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter parse returned None".to_string())
}

/// Node kind constants.
pub const PACKAGE_DECLARATION: &str = "package_declaration";
pub const IMPORT_DECLARATION: &str = "import_declaration";
pub const CLASS_DECLARATION: &str = "class_declaration";
pub const INTERFACE_DECLARATION: &str = "interface_declaration";
pub const FIELD_DECLARATION: &str = "field_declaration";
pub const METHOD_DECLARATION: &str = "method_declaration";
pub const CONSTRUCTOR_DECLARATION: &str = "constructor_declaration";
pub const ANNOTATION: &str = "annotation";
pub const MARKER_ANNOTATION: &str = "marker_annotation";
pub const MODIFIERS: &str = "modifiers";
pub const FORMAL_PARAMETERS: &str = "formal_parameters";
pub const FORMAL_PARAMETER: &str = "formal_parameter";
pub const BLOCK_COMMENT: &str = "block_comment";
pub const LINE_COMMENT: &str = "line_comment";

/// Collect all child nodes matching `kind`.
pub fn children_of_kind<'a>(node: &'a Node<'_>, kind: &str) -> Vec<Node<'a>> {
    let mut result = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            result.push(child);
        }
    }
    result
}

/// Walk tree depth-first, collecting nodes matching `kind`.
pub fn collect_nodes<'a>(node: &Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut result = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            result.push(child);
        }
        result.extend(collect_nodes(&child, kind));
    }
    result
}

/// Get source text for a node.
pub fn node_text<'a>(node: &Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_class() {
        let source = r#"
package com.example;
import org.springframework.stereotype.Service;

@Service
public class UserService {
    private String name;
}
"#;
        let tree = parse_java_source(source).expect("parse failed");
        let root = tree.root_node();
        let mut classes = Vec::new();
        classes = collect_nodes(&root, CLASS_DECLARATION);
        assert_eq!(classes.len(), 1);
    }

    #[test]
    fn test_comment_not_annotation() {
        let source = r#"
// @Component is a comment
public class PlainClass {}
"#;
        let tree = parse_java_source(source).expect("parse failed");
        let root = tree.root_node();
        let classes = collect_nodes(&root, CLASS_DECLARATION);
        let cls = &classes[0];
        let annotations = collect_nodes(cls, ANNOTATION);
        let markers = collect_nodes(cls, MARKER_ANNOTATION);
        assert!(annotations.is_empty(), "Comment should not produce annotation");
        assert!(markers.is_empty(), "Comment should not produce marker annotation");
    }

    #[test]
    fn test_multi_class_detection() {
        let source = r#"
@Service class A {}
@Repository class B {}
@Component class C {}
"#;
        let tree = parse_java_source(source).expect("parse failed");
        let root = tree.root_node();
        let classes = collect_nodes(&root, CLASS_DECLARATION);
        assert_eq!(classes.len(), 3);
    }
}
