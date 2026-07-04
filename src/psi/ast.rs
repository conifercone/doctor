//! AST (Abstract Syntax Tree) — extracts semantic structures from CST.
//!
//! Converts tree-sitter CST nodes into typed Rust structs:
//! PsiClass, PsiField, PsiMethod, PsiAnnotation.

use crate::psi::cst::{self, *};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Node;

/// A parsed Java class/interface/enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiClass {
    pub name: String,
    pub package: String,
    pub fqcn: String,
    pub interfaces: Vec<String>,
    pub annotations: Vec<PsiAnnotation>,
    pub fields: Vec<PsiField>,
    pub methods: Vec<PsiMethod>,
    pub file_path: String,
}

/// A parsed Java annotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiAnnotation {
    pub fqcn: String,
    pub attributes: HashMap<String, String>,
}

/// A parsed Java field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiField {
    pub name: String,
    pub type_name: String,
    pub annotations: Vec<PsiAnnotation>,
}

/// A parsed Java method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiMethod {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<PsiParameter>,
    pub annotations: Vec<PsiAnnotation>,
}

/// A method parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiParameter {
    pub name: String,
    pub type_name: String,
}

/// Parse a Java source file and extract all PsiClass definitions.
pub fn parse_file(path: &Path) -> Result<Vec<PsiClass>, String> {
    let tree = cst::parse_java_file(path)?;
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let root = tree.root_node();
    let source_bytes = source.as_bytes();

    let package = extract_package(&root, source_bytes);
    let imports = extract_imports(&root, source_bytes);

    let mut classes = Vec::new();
    for cls_node in collect_nodes(&root, CLASS_DECLARATION) {
        if let Some(cls) = extract_class(
            &cls_node,
            &package,
            &imports,
            source_bytes,
            &path.display().to_string(),
        ) {
            classes.push(cls);
        }
    }
    Ok(classes)
}

/// Extract package name from CST.
fn extract_package(root: &Node<'_>, source: &[u8]) -> String {
    let pkgs = collect_nodes(root, PACKAGE_DECLARATION);
    pkgs.first()
        .map(|n| {
            node_text(n, source)
                .trim()
                .strip_prefix("package ")
                .unwrap_or("")
                .trim_end_matches(';')
                .trim()
                .to_string()
        })
        .unwrap_or_default()
}

/// Extract imports → HashMap<simple_name, FQCN>.
fn extract_imports(root: &Node<'_>, source: &[u8]) -> HashMap<String, String> {
    let mut imports = HashMap::new();
    for imp in collect_nodes(root, IMPORT_DECLARATION) {
        let text = node_text(&imp, source).trim().to_string();
        let fqcn = text
            .strip_prefix("import ")
            .unwrap_or("")
            .trim_end_matches(';')
            .trim()
            .to_string();
        if fqcn.ends_with(".*") {
            // Wildcard import — store with trailing *
            let pkg = fqcn.trim_end_matches(".*").to_string();
            imports.insert(format!("{pkg}.*"), pkg);
        } else if let Some(simple) = fqcn.rsplit('.').next() {
            imports.insert(simple.to_string(), fqcn);
        }
    }
    imports
}

/// Extract PsiClass from a class_declaration CST node.
fn extract_class(
    node: &Node<'_>,
    package: &str,
    imports: &HashMap<String, String>,
    source: &[u8],
    file_path: &str,
) -> Option<PsiClass> {
    let name = extract_class_name(node, source)?;
    let fqcn = if package.is_empty() {
        name.clone()
    } else {
        format!("{package}.{name}")
    };

    let interfaces = extract_interfaces(node, source, imports);
    let annotations = extract_class_annotations(node, source, imports);
    let fields = extract_fields(node, source, imports);
    let methods = extract_methods(node, source, imports);

    Some(PsiClass {
        name,
        package: package.to_string(),
        fqcn,
        interfaces,
        annotations,
        fields,
        methods,
        file_path: file_path.to_string(),
    })
}

fn extract_class_name(node: &Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(node_text(&child, source).to_string());
        }
    }
    None
}

fn extract_interfaces(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Vec<String> {
    let super_interfaces = collect_nodes(node, "super_interfaces");
    let mut ifaces = Vec::new();
    for si in &super_interfaces {
        for text in node_text(si, source).split(',') {
            let trimmed = text.trim().trim_start_matches("implements ").trim();
            if !trimmed.is_empty() {
                ifaces.push(resolve_type(trimmed, imports));
            }
        }
    }
    ifaces
}

fn extract_class_annotations(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Vec<PsiAnnotation> {
    let modifiers = collect_nodes(node, MODIFIERS);
    let mut annotations = Vec::new();
    for m in &modifiers {
        for ann in collect_nodes(m, ANNOTATION) {
            if let Some(a) = parse_annotation(&ann, source, imports) {
                annotations.push(a);
            }
        }
        for ann in collect_nodes(m, MARKER_ANNOTATION) {
            if let Some(a) = parse_annotation(&ann, source, imports) {
                annotations.push(a);
            }
        }
    }
    annotations
}

fn parse_annotation(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Option<PsiAnnotation> {
    let text = node_text(node, source).trim().to_string();
    let name = text.strip_prefix('@').unwrap_or(&text).to_string();

    // Extract just the annotation name (before any parens)
    let fqcn = if let Some(paren_pos) = name.find('(') {
        let simple = &name[..paren_pos];
        resolve_type(simple, imports)
    } else {
        resolve_type(&name, imports)
    };

    // Parse attributes
    let mut attrs = HashMap::new();
    if let Some(paren_pos) = name.find('(') {
        let args_str = &name[paren_pos + 1..].trim_end_matches(')');
        for part in args_str.split(',') {
            let trimmed = part.trim();
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let val = trimmed[eq_pos + 1..].trim().trim_matches('"').to_string();
                attrs.insert(key, val);
            } else if !trimmed.is_empty() {
                // Positional value → use "value" as key (Spring convention)
                attrs.insert(
                    "value".to_string(),
                    trimmed.trim_matches('"').to_string(),
                );
            }
        }
    }

    Some(PsiAnnotation { fqcn, attributes: attrs })
}

fn extract_fields(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Vec<PsiField> {
    let body = collect_nodes(node, "class_body");
    let mut fields = Vec::new();
    for b in &body {
        for fd in collect_nodes(b, FIELD_DECLARATION) {
            if let Some(f) = extract_single_field(&fd, source, imports) {
                fields.push(f);
            }
        }
    }
    fields
}

fn extract_single_field(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Option<PsiField> {
    let annotations = extract_field_annotations(node, source, imports);

    // Walk CST children: type_identifier or generic_type, then variable_declarator
    let mut cursor = node.walk();
    let mut type_name = String::new();
    let mut field_name = String::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {} // skip — annotations already extracted above
            "type_identifier" | "generic_type" | "array_type" | "scoped_type_identifier" => {
                type_name = node_text(&child, source).to_string();
            }
            "variable_declarator" => {
                // Extract the identifier from inside the declarator
                let mut dc = child.walk();
                for decl_child in child.children(&mut dc) {
                    if decl_child.kind() == "identifier" {
                        field_name = node_text(&decl_child, source).to_string();
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    if !type_name.is_empty() && !field_name.is_empty() {
        // Resolve type through imports
        let resolved = resolve_type(&type_name, imports);
        // Filter out annotation-looking types
        let simple = resolved.rsplit('.').next().unwrap_or(&resolved);
        if simple.starts_with("NonNull") || simple.starts_with("Nullable")
            || resolved.contains(".annotations.")
        {
            return None;
        }
        Some(PsiField {
            name: field_name,
            type_name: resolved,
            annotations,
        })
    } else {
        None
    }
}

fn extract_field_annotations(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Vec<PsiAnnotation> {
    let modifiers = collect_nodes(node, MODIFIERS);
    let mut annotations = Vec::new();
    for m in &modifiers {
        for ann in collect_nodes(m, ANNOTATION) {
            if let Some(a) = parse_annotation(&ann, source, imports) {
                annotations.push(a);
            }
        }
        for ann in collect_nodes(m, MARKER_ANNOTATION) {
            if let Some(a) = parse_annotation(&ann, source, imports) {
                annotations.push(a);
            }
        }
    }
    annotations
}

fn extract_methods(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Vec<PsiMethod> {
    let body = collect_nodes(node, "class_body");
    let mut methods = Vec::new();
    for b in &body {
        for md in collect_nodes(b, METHOD_DECLARATION) {
            if let Some(m) = extract_single_method(&md, source, imports) {
                methods.push(m);
            }
        }
    }
    methods
}

fn extract_single_method(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Option<PsiMethod> {
    let annotations = extract_field_annotations(node, source, imports);

    // Get method name (identifier child)
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut return_type = String::new();
    let mut seen_type = false;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" if !seen_type => {
                return_type = node_text(&child, source).to_string();
                seen_type = true;
            }
            "identifier" if seen_type => {
                name = node_text(&child, source).to_string();
                break;
            }
            "void_type" => {
                return_type = "void".to_string();
                seen_type = true;
            }
            "generic_type" | "type_identifier" if !seen_type => {
                return_type = node_text(&child, source).to_string();
                seen_type = true;
            }
            _ => {}
        }
    }

    if name.is_empty() {
        return None;
    }

    let return_type = resolve_type(&return_type, imports);
    let params = extract_parameters(node, source, imports);

    Some(PsiMethod {
        name,
        return_type,
        parameters: params,
        annotations,
    })
}

fn extract_parameters(
    node: &Node<'_>,
    source: &[u8],
    imports: &HashMap<String, String>,
) -> Vec<PsiParameter> {
    let params_nodes = collect_nodes(node, FORMAL_PARAMETERS);
    let mut params = Vec::new();
    for pn in &params_nodes {
        for fp in collect_nodes(pn, FORMAL_PARAMETER) {
            let text = node_text(&fp, source);
            let parts: Vec<&str> = text.split_whitespace()
                .filter(|w| !w.starts_with('@')) // Skip annotation markers
                .collect();
            if parts.len() >= 2 {
                let resolved = resolve_type(parts[0], imports);
                // Skip annotation types
                if !resolved.contains(".annotations.")
                    && !resolved.starts_with("NonNull")
                    && !resolved.starts_with("Nullable")
                {
                    params.push(PsiParameter {
                        type_name: resolved,
                        name: parts[1].trim_end_matches(',').to_string(),
                    });
                }
            }
        }
    }
    params
}

/// Resolve a type name through imports.
fn resolve_type(name: &str, imports: &HashMap<String, String>) -> String {
    let name = name.trim_start_matches('@');
    if name.contains('.') {
        return name.to_string();
    }
    let key = name
        .split('<')
        .next()
        .unwrap_or(name)
        .trim_end_matches('>');
    imports.get(key).cloned().unwrap_or_else(|| name.to_string())
}
