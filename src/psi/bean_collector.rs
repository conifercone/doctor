//! Bean collector — traverses PsiClass AST nodes and builds a BeanGraph.

use crate::model::bean_graph::{BeanDef, BeanDep, BeanGraph, BeanScope, BeanSource, InjectionType};
use crate::psi::ast::PsiClass;
use std::collections::{HashMap, HashSet};

/// Stereotype annotations that mark a class as a Spring Bean.
const BEAN_STEREOTYPES: &[&str] = &[
    "Component",
    "org.springframework.stereotype.Component",
    "Service",
    "org.springframework.stereotype.Service",
    "Repository",
    "org.springframework.stereotype.Repository",
    "Controller",
    "org.springframework.stereotype.Controller",
    "RestController",
    "org.springframework.web.bind.annotation.RestController",
    "Configuration",
    "org.springframework.context.annotation.Configuration",
    "ConfigurationProperties",
    "org.springframework.boot.context.properties.ConfigurationProperties",
];

/// Annotation types that should NOT be treated as bean dependencies.
const ANNOTATION_TYPES: &[&str] = &[
    "NonNull", "Nullable", "NotNull", "Null",
    "Override", "SuppressWarnings", "Deprecated",
    "Generated", "PostConstruct", "PreDestroy",
    "Resource", "Inject", "Lookup",
];

/// Primitive / common Java types that are never beans.
const NON_BEAN_TYPES: &[&str] = &[
    "String", "int", "long", "boolean", "void",
    "Integer", "Long", "Boolean", "Double", "Float",
    "byte", "char", "short",
    "List", "Map", "Set", "Optional", "Stream",
    "Collection", "Iterable", "Iterator",
    "HttpServletRequest", "HttpServletResponse",
    "HttpSession", "Principal", "Authentication",
    "HttpSecurity", "GrpcSecurity",
    "ObjectProvider", "ResourceLoader", "ApplicationContext",
    "HttpServletRequest", "HttpServletResponse",
];

/// Collect all beans from a list of parsed PsiClass nodes and build a BeanGraph.
pub fn collect_beans(classes: &[PsiClass]) -> BeanGraph {
    let mut graph = BeanGraph::new();
    let mut class_index: HashMap<String, &PsiClass> = HashMap::new();

    // Build class index (FQCN → PsiClass)
    for cls in classes {
        class_index.insert(cls.fqcn.clone(), cls);
        class_index.insert(cls.name.clone(), cls);
    }

    // Phase 1: Register beans from stereotype annotations
    let mut bean_class_names: HashSet<String> = HashSet::new();
    for cls in classes {
        if has_bean_stereotype(cls) {
            let bean_name = get_bean_name(cls);
            let fqcn = &cls.fqcn;
            bean_class_names.insert(fqcn.clone());

            let mut deps = Vec::new();
            // Collect dependencies from @Autowired fields
            for field in &cls.fields {
                if has_annotation(&field.annotations, "Autowired")
                    || has_annotation(&field.annotations, "Resource")
                {
                    let dep_type = normalize_type(&field.type_name);
                    if is_valid_dependency(&dep_type) {
                        deps.push(dep_type);
                    }
                }
            }

            graph.add_bean(BeanDef {
                name: bean_name.clone(),
                class_name: fqcn.clone(),
                scope: BeanScope::Singleton,
                declared_in: cls.file_path.clone(),
                dependencies: deps,
                source: BeanSource::UserDefined,
            });
        }
    }

    // Phase 2: Register beans from @Bean methods in @Configuration classes
    for cls in classes {
        if is_configuration(cls) {
            for method in &cls.methods {
                if has_annotation(&method.annotations, "Bean") {
                    let bean_name = method.name.clone();
                    let return_type = method.return_type.clone();
                    bean_class_names.insert(return_type.clone());

                    let deps: Vec<String> = method
                        .parameters
                        .iter()
                        .map(|p| p.type_name.clone())
                        .collect();

                    graph.add_bean(BeanDef {
                        name: bean_name,
                        class_name: return_type,
                        scope: BeanScope::Singleton,
                        declared_in: cls.file_path.clone(),
                        dependencies: deps,
                        source: BeanSource::UserDefined,
                    });
                }
            }
        }
    }

    // Phase 3: Build edges (collect first, then add to avoid borrow conflict)
    let bean_names: HashSet<&str> =
        graph.beans.iter().map(|b| b.name.as_str()).collect();
    let bean_classes: HashSet<&str> =
        graph.beans.iter().map(|b| b.class_name.as_str()).collect();

    let mut edges: Vec<BeanDep> = Vec::new();
    for cls in classes {
        for field in &cls.fields {
            if has_annotation(&field.annotations, "Autowired") {
                let from_name = get_bean_name(cls);
                let target = resolve_dep_target(
                    &field.type_name,
                    &bean_names,
                    &bean_classes,
                    &class_index,
                );
                edges.push(BeanDep {
                    from: from_name.clone(),
                    to: target,
                    injection_type: InjectionType::Field,
                });
            }
        }
    }
    drop(bean_names);
    drop(bean_classes);
    for edge in edges {
        graph.add_edge(edge);
    }

    // Phase 4: Add interface→impl mappings
    for cls in classes {
        let bean_fqcn = &cls.fqcn;
        if bean_class_names.contains(bean_fqcn) {
            for iface in &cls.interfaces {
                graph.add_interface_impl(iface.clone(), bean_fqcn.clone());
            }
        }
    }

    graph
}

/// Strip generic parameters: `ObjectProvider<Foo>` → `ObjectProvider`, `List<String>` → `String`
fn normalize_type(type_name: &str) -> String {
    // Remove generic parameters: Foo<Bar, Baz> → Foo
    if let Some(generic_start) = type_name.find('<') {
        let base = &type_name[..generic_start];
        // For collections, extract the inner type
        if base == "List" || base == "Set" || base == "Collection" || base == "Optional" {
            let inner = &type_name[generic_start + 1..type_name.len() - 1];
            return normalize_type(inner.trim());
        }
        return base.to_string();
    }
    type_name.to_string()
}

/// Check if a type name refers to a potential bean (not an annotation or primitive).
fn is_valid_dependency(type_name: &str) -> bool {
    // Skip annotation types (check both simple name and FQCN suffix)
    let simple = type_name.rsplit('.').next().unwrap_or(type_name);
    if ANNOTATION_TYPES.contains(&simple) || ANNOTATION_TYPES.contains(&type_name) {
        return false;
    }
    // Skip Java primitives and common non-bean types
    if NON_BEAN_TYPES.contains(&simple) || NON_BEAN_TYPES.contains(&type_name) {
        return false;
    }
    // Skip types from javax.annotation / jakarta.annotation / org.jspecify.annotations packages
    if type_name.contains(".annotations.") {
        return false;
    }
    !type_name.is_empty()
}

fn has_bean_stereotype(cls: &PsiClass) -> bool {
    cls.annotations
        .iter()
        .any(|a| BEAN_STEREOTYPES.iter().any(|s| a.fqcn.ends_with(s)))
}

fn is_configuration(cls: &PsiClass) -> bool {
    cls.annotations
        .iter()
        .any(|a| a.fqcn.ends_with("Configuration"))
}

fn has_annotation(annotations: &[crate::psi::ast::PsiAnnotation], simple_name: &str) -> bool {
    annotations
        .iter()
        .any(|a| a.fqcn.ends_with(simple_name) || a.fqcn == simple_name)
}

/// Get Spring bean name: explicit @Service("name") → "name", else class name decapitalized.
fn get_bean_name(cls: &PsiClass) -> String {
    for ann in &cls.annotations {
        if let Some(name) = ann.attributes.get("value") {
            if !name.is_empty() {
                return name.clone();
            }
        }
    }
    decapitalize(&cls.name)
}

fn decapitalize(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    if let Some(first) = chars.first_mut() {
        *first = first.to_lowercase().next().unwrap_or(*first);
    }
    chars.into_iter().collect()
}

/// Resolve a dependency target to a bean name in the graph.
fn resolve_dep_target(
    type_name: &str,
    bean_names: &HashSet<&str>,
    bean_classes: &HashSet<&str>,
    _class_index: &HashMap<String, &PsiClass>,
) -> String {
    // If the type name is already a bean name, use it
    if bean_names.contains(type_name) {
        return type_name.to_string();
    }
    // If there's a bean whose class_name matches, use its name
    if bean_classes.contains(type_name) {
        return type_name.to_string();
    }
    // Fallback: use type name as-is
    type_name.to_string()
}
