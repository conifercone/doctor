use crate::error::DoctorResult;
use crate::evidence::Evidence;
use crate::model::{Category, Confidence, Issue, Severity, SystemModel};
use std::collections::{HashMap, HashSet};

/// Detect beans whose dependencies are not satisfied.
///
/// A dependency is satisfied if a bean with that name exists as either
/// a UserDefined bean (from project source) or an AutoConfig bean
/// (discovered from dependency jars).
pub fn detect_missing_beans(
    model: &SystemModel,
    _evidence: &[Evidence],
) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();
    let bean_names: HashSet<&str> =
        model.bean_graph.beans.iter().map(|b| b.name.as_str()).collect();
    let bean_classes: HashSet<&str> =
        model.bean_graph.beans.iter().map(|b| b.class_name.as_str()).collect();
    // Simple name lookups for FQCN→simple matching
    let bean_simple_names: HashSet<String> =
        model.bean_graph.beans.iter()
            .map(|b| b.class_name.rsplit('.').next().unwrap_or(&b.class_name).to_string())
            .collect();

    // Interface → implementation matching
    let interface_impls = &model.bean_graph.interface_impls;

    /// Framework types always available via Spring container (never "missing")
    const ALWAYS_AVAILABLE: &[&str] = &[
        "String", "List", "Map", "Set", "Object", "ObjectProvider",
        "ApplicationContext", "ResourceLoader", "HttpServletRequest",
        "HttpServletResponse", "HttpSession", "HttpSecurity", "GrpcSecurity",
    ];

    for (i, bean) in model.bean_graph.beans.iter().enumerate() {
        for dep in &bean.dependencies {
            let dep_simple = dep.rsplit('.').next().unwrap_or(dep);

            // 0. Skip framework types always available via Spring
            if ALWAYS_AVAILABLE.contains(&dep_simple)
                || ALWAYS_AVAILABLE.contains(&dep.as_str())
            {
                continue;
            }

            let mut found = false;

            // 1. Direct name/class match (FQCN or simple name)
            if bean_names.contains(dep.as_str())
                || bean_classes.contains(dep.as_str())
                || bean_simple_names.contains(dep_simple)
            {
                found = true;
            }

            // 2. Interface match: dep is an interface → check if any impl class is a bean
            if !found {
                // Try original name and capitalized form (implements uses capitalized, deps use decapitalized)
                let dep_capitalized = capitalize_first(dep);
                for key in [dep.as_str(), &dep_capitalized] {
                    if let Some(impls) = interface_impls.get(key) {
                        for impl_class in impls {
                            if bean_names.contains(impl_class.as_str())
                                || bean_classes.contains(impl_class.as_str())
                            {
                                found = true;
                                break;
                            }
                        }
                    }
                    if found { break; }
                }
            }

            if !found {
                let id = format!("BEAN-{:03}", i + 1);
                let issue = Issue::new(
                    id,
                    format!("Bean '{}' not found", dep),
                    Severity::Error,
                    Category::Bean,
                    format!(
                        "Bean '{}' depends on '{}', but no bean of that type is declared. Declared in: {}",
                        bean.name, dep, bean.declared_in
                    ),
                    vec![Evidence::new(
                        crate::evidence::EvidenceType::SourceCode,
                        bean.declared_in.clone(),
                        format!("Bean '{}' declares dependency on '{}'", bean.name, dep),
                        crate::evidence::Reliability::Confirmed,
                    )],
                    format!(
                        "Define a @Bean method for '{}' or annotate its class with @Component/@Service/@Repository",
                        dep
                    ),
                    Confidence::High,
                );
                if let Some(issue) = issue {
                    issues.push(issue);
                }
            }
        }
    }
    Ok(issues)
}

/// Detect multiple beans of the same type without @Qualifier disambiguation.
///
/// Skips conflicts where all beans of the same type originate from
/// *different* Gradle submodules (US2 multi-module dedup).  Beans from
/// jar dependencies (AutoConfig) are never skipped — only project-source
/// beans in distinct `src/main/java` roots are exempt.
pub fn detect_bean_conflicts(
    model: &SystemModel,
    _evidence: &[Evidence],
) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();
    let mut type_map: HashMap<&str, Vec<&str>> = HashMap::new();

    for bean in &model.bean_graph.beans {
        type_map.entry(&bean.class_name).or_default().push(&bean.name);
    }

    for (class_name, names) in &type_map {
        if names.len() > 1 {
            // Multi-module dedup (US2): extract the src/main/java root for
            // each bean.  If every bean lives in a distinct submodule, the
            // conflict is intentional (e.g. each module defines its own
            // DataSource for independent testing).
            let module_roots: HashSet<String> = names
                .iter()
                .filter_map(|name| {
                    model
                        .bean_graph
                        .beans
                        .iter()
                        .find(|b| b.name == *name)
                        .and_then(|b| extract_module_root(&b.declared_in))
                })
                .collect();

            // Only skip when *all* beans have a resolvable module root AND
            // each lives in a different submodule.  AutoConfig beans return
            // None (no src/main/java path), so conflicts involving them are
            // always reported.
            if module_roots.len() == names.len() {
                continue;
            }

            let id = format!("BEAN-CONFLICT-{}", class_name.replace('.', "-"));
            let issue = Issue::new(
                id,
                format!("Multiple beans of type '{}'", class_name),
                Severity::Warning,
                Category::Bean,
                format!(
                    "Type '{}' has {} beans: {}. Injection points may need @Qualifier.",
                    class_name,
                    names.len(),
                    names.iter().copied().collect::<Vec<_>>().join(", ")
                ),
                vec![Evidence::new(
                    crate::evidence::EvidenceType::SourceCode,
                    class_name.to_string(),
                    format!("{} beans registered for type '{}'", names.len(), class_name),
                    crate::evidence::Reliability::Confirmed,
                )],
                "Add @Qualifier annotation at injection points to disambiguate".to_string(),
                Confidence::High,
            );
            if let Some(issue) = issue {
                issues.push(issue);
            }
        }
    }
    Ok(issues)
}

/// Extract the `src/main/java` root path from a bean's `declared_in` field.
///
/// Returns `Some(path)` with the prefix up to (and including) `src/main/java`,
/// or `None` for auto-config beans (whose `declared_in` starts with
/// `"auto-config:"` and contains no source-root path).
fn capitalize_first(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    if let Some(first) = chars.first_mut() {
        *first = first.to_uppercase().next().unwrap_or(*first);
    }
    chars.into_iter().collect()
}

fn extract_module_root(declared_in: &str) -> Option<String> {
    if let Some(pos) = declared_in.find("/src/main/java/") {
        Some(declared_in[..pos + "/src/main/java".len()].to_string())
    } else {
        None
    }
}

/// Detect circular dependencies in the Bean graph using DFS.
pub fn detect_circular_dependencies(
    model: &SystemModel,
    _evidence: &[Evidence],
) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();
    let adj: HashMap<&str, Vec<&str>> = model
        .bean_graph
        .beans
        .iter()
        .map(|b| (b.name.as_str(), b.dependencies.iter().map(|d| d.as_str()).collect()))
        .collect();

    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    let mut cycle_count = 0;

    for bean_name in adj.keys() {
        if !visited.contains(bean_name) {
            let mut path = Vec::new();
            if detect_cycle(bean_name, &adj, &mut visited, &mut path, &mut stack) {
                cycle_count += 1;
                let id = format!("BEAN-CYCLE-{:03}", cycle_count);
                let chain = stack.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" → ");
                let issue = Issue::new(
                    id,
                    "Circular dependency detected",
                    Severity::Error,
                    Category::Bean,
                    format!("Circular dependency chain: {}", chain),
                    vec![Evidence::new(
                        crate::evidence::EvidenceType::SourceCode,
                        "Bean dependency graph".to_string(),
                        format!("Cycle detected: {}", chain),
                        crate::evidence::Reliability::Confirmed,
                    )],
                    "Break the cycle by introducing an interface, using @Lazy, or refactoring to setter injection".to_string(),
                    Confidence::High,
                );
                if let Some(issue) = issue {
                    issues.push(issue);
                }
                stack.clear();
            }
        }
    }
    Ok(issues)
}

fn detect_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
    stack: &mut Vec<&'a str>,
) -> bool {
    visited.insert(node);
    path.push(node);
    stack.push(node);

    if let Some(neighbors) = adj.get(node) {
        for &neighbor in neighbors {
            if !visited.contains(neighbor) {
                if detect_cycle(neighbor, adj, visited, path, stack) {
                    return true;
                }
            } else if path.contains(&neighbor) {
                stack.push(neighbor);
                return true;
            }
        }
    }

    path.pop();
    stack.pop();
    false
}
