use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::error::{DoctorError, DoctorResult};

// ---------------------------------------------------------------------------
// Static regex patterns — compiled once and reused.
// We allow `unwrap()` here because the patterns are compile-time constants;
// a failure would indicate a programmer error that should surface immediately.
// ---------------------------------------------------------------------------

/// Matches Spring stereotype annotations: @Component, @Service, etc.
static STEREOTYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@(Component|Service|Repository|Controller|RestController|Configuration)\b")
        .unwrap()
});

/// Matches a (possibly annotated) class declaration line.
/// Captures the class name.
static CLASS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public\s+)?(?:abstract\s+)?class\s+(\w+)").unwrap()
});

/// Matches `implements InterfaceA, InterfaceB<Type> {` — captures the interface names.
static IMPLEMENTS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"implements\s+([\w\s,.<>]+?)\s*\{").unwrap()
});

/// Matches an @Autowired annotation.
static AUTOWIRED_ANNOTATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@Autowired\b").unwrap()
});

/// Matches a field declaration: `[private|public|protected] [static] [final] Type name[ = ...];`
/// Captures the type (group 1) and field name (group 2).
static FIELD_DECL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:private|public|protected)\s+(?:static\s+)?(?:final\s+)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*[=;]",
    )
    .unwrap()
});

/// Matches a constructor declaration for a specific class name.
/// Captures the parameter list.
fn constructor_re(class_name: &str) -> Regex {
    Regex::new(&format!(
        r"^\s*(?:public|protected)?\s*{}\s*\(([^)]*)\)",
        regex::escape(class_name)
    ))
    .unwrap()
}

/// Matches a method declaration: `[public|protected] [static] [<T>] ReturnType methodName(`
/// Captures the return type (group 1) and method name (group 2).
static METHOD_DECL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:public|protected)\s+(?:static\s+)?(?:<[^>]+>\s+)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(",
    )
    .unwrap()
});

/// Matches a @Bean annotation line.
static BEAN_ANNOTATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@Bean\b").unwrap()
});

/// Matches @Qualifier("name") — captures the qualifier value.
static QUALIFIER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"@Qualifier\s*\(\s*"(\w+)"\s*\)"#).unwrap()
});

/// Matches Lombok @RequiredArgsConstructor or @AllArgsConstructor.
static LOMBOK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@(RequiredArgsConstructor|AllArgsConstructor)\b").unwrap()
});

/// Matches `private final Type name;` — captures type (group 1) and name (group 2).
static PRIVATE_FINAL_FIELD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*private\s+final\s+(\w+(?:<[^>]+>)?)\s+(\w+)\s*;").unwrap()
});

/// Matches an import statement: `import com.example.Type;`, `import com.example.*;`,
/// or `import static com.example.Util.method;`.
/// Captures the full import path (group 1).
static IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"import\s+(?:static\s+)?([\w.]+(?:\*)?)\s*;").unwrap()
});

/// Matches `include("module")` in Gradle Kotlin DSL settings.
static GRADLE_KTS_INCLUDE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"include\s*\(\s*"([^"]+)"\s*\)"#).unwrap()
});

/// Matches `include 'module'` or `include ":module"` in Gradle Groovy DSL settings.
static GRADLE_GROOVY_INCLUDE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"include\s*[':]([^'":]+)[':]"#).unwrap()
});

// ---------------------------------------------------------------------------
// Data structures (unchanged public API)
// ---------------------------------------------------------------------------

/// Where a bean definition comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BeanSource {
    /// Bean discovered in user project source code.
    #[default]
    UserDefined,
    /// Bean provided by auto-configuration from dependency jars.
    AutoConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeanScope {
    Singleton,
    Prototype,
    Request,
    Session,
}

impl std::fmt::Display for BeanScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeanScope::Singleton => write!(f, "Singleton"),
            BeanScope::Prototype => write!(f, "Prototype"),
            BeanScope::Request => write!(f, "Request"),
            BeanScope::Session => write!(f, "Session"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectionType {
    Field,
    Constructor,
    Setter,
}

/// A single bean definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeanDef {
    pub name: String,
    pub class_name: String,
    pub scope: BeanScope,
    /// Where this bean is declared (config class or XML file)
    pub declared_in: String,
    /// Names of beans this bean depends on
    pub dependencies: Vec<String>,
    /// Where this bean definition originates
    #[serde(default)]
    pub source: BeanSource,
}

/// A dependency edge between two beans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeanDep {
    pub from: String,
    pub to: String,
    pub injection_type: InjectionType,
}

/// Complete bean dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeanGraph {
    pub beans: Vec<BeanDef>,
    pub edges: Vec<BeanDep>,
    /// Interface name → implementing class names.
    /// E.g., "OAuth2AuthorizationService" → ["DefaultOAuth2AuthorizationService"]
    #[serde(default)]
    pub interface_impls: HashMap<String, Vec<String>>,
}

impl BeanGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a class implements an interface.
    pub fn add_interface_impl(&mut self, interface_name: String, impl_class: String) {
        self.interface_impls.entry(interface_name).or_default().push(impl_class);
    }

    pub fn add_bean(&mut self, bean: BeanDef) {
        self.beans.push(bean);
    }

    pub fn add_edge(&mut self, edge: BeanDep) {
        self.edges.push(edge);
    }

    /// Find a bean by name.
    pub fn find_bean(&self, name: &str) -> Option<&BeanDef> {
        self.beans.iter().find(|b| b.name == name)
    }

    /// Get all dependencies for a given bean.
    pub fn dependencies_of(&self, bean_name: &str) -> Vec<&BeanDep> {
        self.edges.iter().filter(|e| e.from == bean_name).collect()
    }

    /// Get all beans that depend on a given bean.
    pub fn dependents_of(&self, bean_name: &str) -> Vec<&BeanDep> {
        self.edges.iter().filter(|e| e.to == bean_name).collect()
    }
}

// ---------------------------------------------------------------------------
// Import resolution
// ---------------------------------------------------------------------------

/// Resolves simple class names to fully-qualified class names using Java
/// `import` statements.
#[derive(Debug, Clone)]
struct ImportResolver {
    /// Map from simple class name to FQCN (from exact imports).
    exact: HashMap<String, String>,
    /// Wildcard import packages (e.g. `com.example.service` from `import com.example.service.*;`).
    wildcards: Vec<String>,
}

impl ImportResolver {
    fn new(imports: Vec<String>) -> Self {
        let mut exact = HashMap::new();
        let mut wildcards = Vec::new();

        for imp in imports {
            if let Some(pkg) = imp.strip_suffix(".*") {
                wildcards.push(pkg.to_string());
            } else if let Some((_pkg, class)) = imp.rsplit_once('.') {
                exact.insert(class.to_string(), imp.clone());
            }
        }

        Self { exact, wildcards }
    }

    /// Resolve a simple class name to its FQCN.
    /// Returns `None` if the name cannot be resolved (e.g. same-package or JDK class).
    fn resolve(&self, simple_name: &str) -> Option<String> {
        // Strip generic parameters for lookup: List<Foo> → List
        let bare = strip_generics(simple_name);

        if let Some(fqcn) = self.exact.get(bare) {
            return Some(fqcn.clone());
        }

        for pkg in &self.wildcards {
            let candidate = format!("{pkg}.{bare}");
            // We cannot verify that the class actually exists in that package,
            // but wildcard imports are the best signal we have.
            return Some(candidate);
        }

        None
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Build a BeanGraph using the tree-sitter PSI pipeline.
fn build_bean_graph_psi(project_path: &Path) -> DoctorResult<BeanGraph> {
    let java_source_dirs = collect_java_source_dirs(project_path)?;
    let mut all_classes = Vec::new();

    // Try incremental scan with sled index
    let db = crate::psi::index::open_db().ok();
    let mut reparse_count = 0usize;
    let mut cache_hit_count = 0usize;

    for dir in java_source_dirs.iter() {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
        {
            let file_path = entry.path();
            let content = std::fs::read_to_string(file_path).unwrap_or_default();

            // Check if file is unchanged (incremental scan)
            if let Some(ref db) = db {
                if !crate::psi::index::file_changed(db, &file_path.display().to_string(), &content)
                {
                    let cached = crate::psi::index::read_classes_from_file(
                        db,
                        &file_path.display().to_string(),
                    );
                    if !cached.is_empty() {
                        all_classes.extend(cached);
                        cache_hit_count += 1;
                        continue;
                    }
                }
            }

            // Cache miss or no db: parse with tree-sitter
            match crate::psi::ast::parse_file(file_path) {
                Ok(classes) => {
                    if let Some(ref db) = db {
                        for cls in &classes {
                            if let Err(e) = crate::psi::index::write_class(db, cls) {
                                eprintln!("  Warning: sled write: {e}");
                            }
                        }
                        // Store file→FQCNs mapping for incremental cache
                        let fqcns: Vec<&str> = classes.iter().map(|c| c.fqcn.as_str()).collect();
                        let _ = db.insert(
                            format!("F:{}", file_path.display()),
                            fqcns.join(",").as_bytes(),
                        );
                        let hash = crate::psi::index::compute_hash(&content);
                        let _ = crate::psi::index::store_file_hash(
                            db,
                            &file_path.display().to_string(),
                            &hash,
                        );
                    }
                    all_classes.extend(classes);
                    reparse_count += 1;
                }
                Err(e) => {
                    eprintln!(
                        "  Warning: PSI parse failed for {}: {e}",
                        file_path.display()
                    );
                }
            }
        }
    }

    if cache_hit_count > 0 || reparse_count > 0 {
        eprintln!(
            "  PSI: {reparse_count} files parsed, {cache_hit_count} cached (sled)"
        );
    }

    if all_classes.is_empty() {
        return Err(DoctorError::ParseError {
            file: project_path.display().to_string(),
            message: "PSI parser found 0 classes".to_string(),
        });
    }

    let graph = crate::psi::bean_collector::collect_beans(&all_classes);
    Ok(graph)
}

/// Build a `BeanGraph` by scanning all Java source files in the given
/// project directory (including multi-module Gradle projects).
///
/// # Errors
///
/// Returns `DoctorError::IoError` if a required directory or file cannot be
/// read.  Individual file parse failures are logged and skipped rather than
/// causing the entire build to fail.
pub fn build_bean_graph(project_path: &Path) -> DoctorResult<BeanGraph> {
    // Use tree-sitter PSI pipeline for accurate parsing
    let mut graph = match build_bean_graph_psi(project_path) {
        Ok(g) if !g.beans.is_empty() => {
            eprintln!("  PSI: {} beans parsed via tree-sitter", g.beans.len());
            g
        }
        Ok(_) => {
            eprintln!("  Warning: PSI parser found 0 beans, falling back to regex");
            let mut g = BeanGraph::new();
            let java_source_dirs = collect_java_source_dirs(project_path)?;
            build_graph_regex(&mut g, &java_source_dirs)?;
            g
        }
        Err(e) => {
            eprintln!("  Warning: PSI parser failed: {e}, falling back to regex");
            let mut g = BeanGraph::new();
            let java_source_dirs = collect_java_source_dirs(project_path)?;
            build_graph_regex(&mut g, &java_source_dirs)?;
            g
        }
    };

    // Register auto-config beans (runs for both PSI and regex paths)
    match crate::classpath::discover_auto_config_beans(project_path) {
        Ok(auto_beans) => {
            let count = auto_beans.len();
            for ab in &auto_beans {
                for iface in &ab.interfaces {
                    graph.add_interface_impl(iface.clone(), ab.class_name.clone());
                }
                graph.add_bean(BeanDef {
                    name: ab.class_name.clone(),
                    class_name: ab.class_name.clone(),
                    scope: BeanScope::Singleton,
                    declared_in: format!("auto-config: {}", ab.source_class),
                    dependencies: vec![],
                    source: BeanSource::AutoConfig,
                });
            }
            if count > 0 {
                eprintln!("  Auto-config beans registered: {count}");
            }
        }
        Err(e) => {
            eprintln!("  Warning: auto-config scan failed: {e}");
        }
    }

    Ok(graph)
}

/// Legacy regex-based graph builder (kept for fallback).
fn build_graph_regex(graph: &mut BeanGraph, java_source_dirs: &[PathBuf]) -> DoctorResult<()> {

    for dir in java_source_dirs.iter() {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
        {
            let file_path = entry.path();
            if let Err(e) = process_java_file(file_path, dir, graph) {
                eprintln!(
                    "warning: skipping `{}` — {}",
                    file_path.display(),
                    e
                );
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Source directory collection (with multi-module support)
// ---------------------------------------------------------------------------

/// Collect all `src/main/java` directories across the root module and any
/// Gradle submodules declared in `settings.gradle.kts` or `settings.gradle`.
pub fn collect_java_source_dirs(project_path: &Path) -> DoctorResult<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    // Root module
    let root_java = project_path.join("src").join("main").join("java");
    if root_java.exists() {
        dirs.push(root_java);
    }

    // Kotlin DSL: settings.gradle.kts
    let kts = project_path.join("settings.gradle.kts");
    if kts.exists() {
        let content = read_file(&kts)?;
        for m in GRADLE_KTS_INCLUDE_RE.captures_iter(&content) {
            let module_path = m[1].replace(':', "/");
            let module_java = project_path.join(&module_path).join("src").join("main").join("java");
            if module_java.exists() {
                dirs.push(module_java);
            }
        }
    }

    // Groovy DSL: settings.gradle
    let groovy = project_path.join("settings.gradle");
    if groovy.exists() {
        let content = read_file(&groovy)?;
        for m in GRADLE_GROOVY_INCLUDE_RE.captures_iter(&content) {
            let module_path = m[1].replace(':', "/");
            let module_java = project_path.join(&module_path).join("src").join("main").join("java");
            if module_java.exists() {
                dirs.push(module_java);
            }
        }
    }

    Ok(dirs)
}

// ---------------------------------------------------------------------------
// Single-file processing
// ---------------------------------------------------------------------------

/// Parse one `.java` file and register any beans and dependencies found.
fn process_java_file(
    file_path: &Path,
    _source_root: &Path,
    graph: &mut BeanGraph,
) -> DoctorResult<()> {
    let content = read_file(file_path)?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Ok(());
    }

    let imports = parse_imports(&content);
    let resolver = ImportResolver::new(imports);
    let class_positions = find_class_declarations(&lines);

    // We collect beans per-class and add them at the end so that dependencies
    // (edges) are fully populated before the BeanDef is inserted.
    for (idx, (class_name, line_num)) in class_positions.iter().enumerate() {
        let territory_end = if idx + 1 < class_positions.len() {
            class_positions[idx + 1].1
        } else {
            lines.len()
        };

        let stereotype = detect_stereotype(&lines, *line_num);

        let Some(ref annotation_kind) = stereotype else {
            // Not a stereotype class — still check for Lombok-driven edges on
            // classes whose superclass IS a stereotype?  No; skip.
            continue;
        };

        // --- Build the bean definition for this class ---
        let bean_name = decapitalize(class_name);
        let fqcn = resolver
            .resolve(class_name)
            .unwrap_or_else(|| class_name.clone());

        let mut dependencies: Vec<String> = Vec::new();

        // ---- @Bean methods (only inside @Configuration classes) ----
        if annotation_kind == "Configuration" {
            let bean_methods = find_bean_methods(&lines, *line_num, territory_end);
            for (return_type, method_name) in bean_methods {
                let resolved_rt = resolver
                    .resolve(&return_type)
                    .unwrap_or_else(|| return_type.clone());
                graph.add_bean(BeanDef {
                    name: method_name.clone(),
                    class_name: resolved_rt,
                    scope: BeanScope::Singleton,
                    declared_in: file_path.display().to_string(),
                    dependencies: Vec::new(),
                    source: BeanSource::default(),
                });
                // Configuration class depends on its own @Bean definitions?
                // Not typically — @Bean methods produce beans, they don't
                // represent a dependency FROM the config class.  Skip edge.
            }
        }

        // ---- @Autowired field / constructor injection ----
        let autowired_deps = find_autowired_deps(
            &lines,
            *line_num,
            territory_end,
            class_name,
        );
        for (dep_type, qualifier) in autowired_deps {
            let target = qualifier.unwrap_or_else(|| decapitalize(strip_generics(&dep_type)));
            dependencies.push(target.clone());
            graph.add_edge(BeanDep {
                from: bean_name.clone(),
                to: target,
                injection_type: InjectionType::Constructor, // we distinguish below
            });
        }
        // (re)process to also capture Field-type injections
        let field_deps = find_autowired_field_deps(&lines, *line_num, territory_end);
        for (dep_type, qualifier) in field_deps {
            let target = qualifier.unwrap_or_else(|| decapitalize(strip_generics(&dep_type)));
            dependencies.push(target.clone());
            graph.add_edge(BeanDep {
                from: bean_name.clone(),
                to: target,
                injection_type: InjectionType::Field,
            });
        }

        // ---- Lombok constructor injection ----
        let has_lombok = detect_lombok(&lines, *line_num);
        if has_lombok {
            let final_fields =
                find_private_final_fields(&lines, *line_num, territory_end);
            for (field_type, _field_name) in final_fields {
                let target = decapitalize(strip_generics(&field_type));
                dependencies.push(target.clone());
                graph.add_edge(BeanDep {
                    from: bean_name.clone(),
                    to: target,
                    injection_type: InjectionType::Constructor,
                });
            }
        }

        // --- Parse `implements` for interface→impl mapping (FR-I01/I02) ---
        if let Some(caps) = IMPLEMENTS_RE.captures(&content) {
            let iface_list = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            for raw in iface_list.split(',') {
                // Strip generics: Gateway<User> → Gateway
                let iface = raw.trim().split('<').next().unwrap_or("").trim().to_string();
                if !iface.is_empty() {
                    // Resolve via imports if possible, otherwise use simple name
                    let resolved = resolver.resolve(&iface).unwrap_or_else(|| iface.clone());
                    // Extract simple name from resolved
                    let simple = resolved.rsplit('.').next().unwrap_or(&resolved).to_string();
                    graph.add_interface_impl(simple.clone(), bean_name.clone());
                }
            }
        }

        graph.add_bean(BeanDef {
            name: bean_name,
            class_name: fqcn,
            scope: BeanScope::Singleton,
            declared_in: file_path.display().to_string(),
            dependencies,
            source: BeanSource::default(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers: file reading, import parsing
// ---------------------------------------------------------------------------

fn read_file(path: &Path) -> DoctorResult<String> {
    fs::read_to_string(path).map_err(|source| DoctorError::IoError {
        path: path.display().to_string(),
        source,
    })
}

/// Extract all import statements from file content.
fn parse_imports(content: &str) -> Vec<String> {
    IMPORT_RE
        .captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers: class detection
// ---------------------------------------------------------------------------

/// Return `(class_name, 0-based_line_index)` for every top-level class
/// declaration in the file.
fn find_class_declarations(lines: &[&str]) -> Vec<(String, usize)> {
    let mut positions = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = CLASS_RE.captures(line) {
            positions.push((caps[1].to_string(), i));
        }
    }
    positions
}

/// Check the 5 lines *before* `class_line` for a Spring stereotype annotation.
/// Returns the annotation kind (e.g. "Service") or `None`.
fn detect_stereotype(lines: &[&str], class_line: usize) -> Option<String> {
    let start = class_line.saturating_sub(5);
    let window: String = lines[start..=class_line].join("\n");
    STEREOTYPE_RE
        .captures(&window)
        .map(|c| c[1].to_string())
}

/// Check the 5 lines *before* `class_line` for a Lombok constructor annotation.
fn detect_lombok(lines: &[&str], class_line: usize) -> bool {
    let start = class_line.saturating_sub(5);
    let window: String = lines[start..=class_line].join("\n");
    LOMBOK_RE.is_match(&window)
}

// ---------------------------------------------------------------------------
// Helpers: @Bean method detection
// ---------------------------------------------------------------------------

/// Find `@Bean`-annotated methods inside a class territory.
/// Returns `(return_type, method_name)` tuples.
fn find_bean_methods(
    lines: &[&str],
    territory_start: usize,
    territory_end: usize,
) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut i = territory_start;

    while i < territory_end {
        if BEAN_ANNOTATION_RE.is_match(lines[i]) {
            // Scan forward for a method declaration (skip annotation lines)
            for j in i..usize::min(i + 10, territory_end) {
                let line = lines[j].trim();
                // Skip annotation lines (starting with @) and comment lines
                if line.starts_with('@') || line.starts_with("//") || line.starts_with('*') || line.starts_with("/*") {
                    continue;
                }
                if let Some(caps) = METHOD_DECL_RE.captures(lines[j]) {
                    results.push((caps[1].to_string(), caps[2].to_string()));
                    break;
                }
                // If we hit a non-annotation, non-method line, stop scanning
                if !line.is_empty() {
                    break;
                }
            }
        }
        i += 1;
    }

    results
}

// ---------------------------------------------------------------------------
// Helpers: @Autowired injection detection
// ---------------------------------------------------------------------------

/// Find @Autowired constructor dependencies.
/// Returns `(type, qualifier)` tuples.
fn find_autowired_deps(
    lines: &[&str],
    territory_start: usize,
    territory_end: usize,
    class_name: &str,
) -> Vec<(String, Option<String>)> {
    let mut results = Vec::new();
    let constructor_re = constructor_re(class_name);
    let mut i = territory_start;

    while i < territory_end {
        if AUTOWIRED_ANNOTATION_RE.is_match(lines[i]) {
            // Scan forward for a constructor declaration
            for j in i..usize::min(i + 10, territory_end) {
                if let Some(caps) = constructor_re.captures(lines[j]) {
                    let params = caps[1].to_string();
                    for param in params.split(',') {
                        let param = param.trim();
                        if param.is_empty() {
                            continue;
                        }
                        // Extract type — first word before the parameter name
                        let type_name = param
                            .split_whitespace()
                            .next()
                            .unwrap_or(param)
                            .to_string();
                        if !type_name.is_empty() {
                            results.push((type_name, None));
                        }
                    }
                    break;
                }
                // If we hit a field declaration or non-empty non-annotation line
                // that isn't a constructor, stop this scan (it's field injection)
                let trimmed = lines[j].trim();
                if FIELD_DECL_RE.is_match(lines[j])
                    || (!trimmed.starts_with('@')
                        && !trimmed.is_empty()
                        && !trimmed.starts_with("//"))
                {
                    break;
                }
            }
        }
        i += 1;
    }

    results
}

/// Find @Autowired field dependencies.
/// Returns `(type, qualifier)` tuples.
fn find_autowired_field_deps(
    lines: &[&str],
    territory_start: usize,
    territory_end: usize,
) -> Vec<(String, Option<String>)> {
    let mut results = Vec::new();
    let mut i = territory_start;

    while i < territory_end {
        if AUTOWIRED_ANNOTATION_RE.is_match(lines[i]) {
            let mut qualifier: Option<String> = None;

            // Scan forward for a field declaration (or @Qualifier then field)
            for j in i..usize::min(i + 10, territory_end) {
                // Check for @Qualifier on intermediate lines
                if let Some(caps) = QUALIFIER_RE.captures(lines[j]) {
                    qualifier = Some(caps[1].to_string());
                    continue;
                }
                // Match field declaration
                if let Some(caps) = FIELD_DECL_RE.captures(lines[j]) {
                    let type_name = caps[1].to_string();
                    // Skip fields that also have @Autowired on the same line
                    // (they're field injection, which is what we're detecting)
                    results.push((type_name, qualifier));
                    break;
                }
                // If we hit something that is NOT an annotation or empty line,
                // stop scanning (might be constructor or something else)
                let trimmed = lines[j].trim();
                if !trimmed.starts_with('@')
                    && !trimmed.is_empty()
                    && !trimmed.starts_with("//")
                    && !trimmed.starts_with('*')
                    && !trimmed.starts_with("/*")
                {
                    break;
                }
            }
        }
        i += 1;
    }

    results
}

// ---------------------------------------------------------------------------
// Helpers: Lombok / private final fields
// ---------------------------------------------------------------------------

/// Find `private final Type name;` fields in a class territory.
/// Returns `(type, field_name)` tuples.
fn find_private_final_fields(
    lines: &[&str],
    territory_start: usize,
    territory_end: usize,
) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for i in territory_start..territory_end {
        if let Some(caps) = PRIVATE_FINAL_FIELD_RE.captures(lines[i]) {
            results.push((caps[1].to_string(), caps[2].to_string()));
        }
    }
    results
}

// ---------------------------------------------------------------------------
// String utilities
// ---------------------------------------------------------------------------

/// Spring-style bean-name decapitalization.
///
/// If the first two characters are both uppercase (e.g. `URLParser`), the
/// name is returned unchanged.  Otherwise the first character is lowercased.
fn decapitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            if first.is_uppercase()
                && chars.clone().next().map_or(false, |c| c.is_uppercase())
            {
                // First two chars are uppercase → keep as-is (e.g. URLParser)
                s.to_string()
            } else {
                first.to_lowercase().to_string() + chars.as_str()
            }
        }
    }
}

/// Strip generic type parameters from a type name.
/// `List<FooService>` → `List`, `Map<String,Object>` → `Map`
fn strip_generics(s: &str) -> &str {
    s.split('<').next().unwrap_or(s)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decapitalize() {
        assert_eq!(decapitalize("FooService"), "fooService");
        assert_eq!(decapitalize("URLParser"), "URLParser");
        assert_eq!(decapitalize("ABean"), "ABean");
        assert_eq!(decapitalize("a"), "a");
        assert_eq!(decapitalize(""), "");
    }

    #[test]
    fn test_strip_generics() {
        assert_eq!(strip_generics("List"), "List");
        assert_eq!(strip_generics("List<FooService>"), "List");
        assert_eq!(strip_generics("Map<String,Object>"), "Map");
    }

    #[test]
    fn test_import_resolver_exact() {
        let imports = vec![
            "com.example.service.FooService".to_string(),
            "com.example.repo.BarRepository".to_string(),
        ];
        let resolver = ImportResolver::new(imports);
        assert_eq!(
            resolver.resolve("FooService"),
            Some("com.example.service.FooService".to_string())
        );
        assert_eq!(
            resolver.resolve("BarRepository"),
            Some("com.example.repo.BarRepository".to_string())
        );
        assert_eq!(resolver.resolve("Unknown"), None);
    }

    #[test]
    fn test_import_resolver_wildcard() {
        let imports = vec!["com.example.service.*".to_string()];
        let resolver = ImportResolver::new(imports);
        assert_eq!(
            resolver.resolve("FooService"),
            Some("com.example.service.FooService".to_string())
        );
    }

    #[test]
    fn test_stereotype_detection() {
        let lines: Vec<&str> = vec![
            "package com.example;",
            "",
            "import org.springframework.stereotype.Service;",
            "",
            "@Service",
            "public class FooService {",
        ];
        assert_eq!(detect_stereotype(&lines, 5), Some("Service".to_string()));
    }

    #[test]
    fn test_stereotype_detection_none() {
        let lines: Vec<&str> = vec![
            "package com.example;",
            "",
            "public class FooUtil {",
        ];
        assert_eq!(detect_stereotype(&lines, 2), None);
    }

    #[test]
    fn test_lombok_detection() {
        let lines: Vec<&str> = vec![
            "import lombok.RequiredArgsConstructor;",
            "",
            "@Service",
            "@RequiredArgsConstructor",
            "public class FooService {",
        ];
        assert!(detect_lombok(&lines, 4));
    }

    #[test]
    fn test_find_bean_methods() {
        let lines: Vec<&str> = vec![
            "@Configuration",
            "public class AppConfig {",
            "",
            "    @Bean",
            "    public FooService fooService() {",
            "        return new FooService();",
            "    }",
            "}",
        ];
        let methods = find_bean_methods(&lines, 0, lines.len());
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].0, "FooService");
        assert_eq!(methods[0].1, "fooService");
    }

    #[test]
    fn test_find_autowired_field_deps() {
        let lines: Vec<&str> = vec![
            "@Service",
            "public class FooService {",
            "",
            "    @Autowired",
            "    private BarService barService;",
            "}",
        ];
        let deps = find_autowired_field_deps(&lines, 0, lines.len());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "BarService");
    }

    #[test]
    fn test_find_autowired_field_with_qualifier() {
        let lines: Vec<&str> = vec![
            "@Service",
            "public class FooService {",
            "",
            "    @Autowired",
            "    @Qualifier(\"specialBar\")",
            "    private BarService barService;",
            "}",
        ];
        let deps = find_autowired_field_deps(&lines, 0, lines.len());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "BarService");
        assert_eq!(deps[0].1.as_deref(), Some("specialBar"));
    }

    #[test]
    fn test_find_private_final_fields() {
        let lines: Vec<&str> = vec![
            "@Service",
            "@RequiredArgsConstructor",
            "public class FooService {",
            "    private final BarService barService;",
            "    private final BazService bazService;",
            "}",
        ];
        let fields = find_private_final_fields(&lines, 0, lines.len());
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "BarService");
        assert_eq!(fields[1].0, "BazService");
    }

    #[test]
    fn test_parse_imports() {
        let content = "import com.example.Foo;\nimport com.example.bar.*;\nimport static com.example.Util.method;";
        let imports = parse_imports(content);
        assert_eq!(imports.len(), 3);
        assert!(imports.contains(&"com.example.Foo".to_string()));
        assert!(imports.contains(&"com.example.bar.*".to_string()));
    }

    #[test]
    fn test_parse_imports_static() {
        let content = "import static org.junit.Assert.assertEquals;";
        let imports = parse_imports(content);
        assert_eq!(imports.len(), 1);
    }
}
