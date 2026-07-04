use serde::{Deserialize, Serialize};

use crate::error::{DoctorError, DoctorResult};
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionResult {
    pub condition_class: String,
    pub matched: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfigClass {
    pub class_name: String,
    pub condition_results: Vec<ConditionResult>,
    pub registered_beans: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisabledAutoConfig {
    pub class_name: String,
    pub failed_condition: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoConfigModel {
    pub enabled: Vec<AutoConfigClass>,
    pub disabled: Vec<DisabledAutoConfig>,
    pub excluded: Vec<String>,
}

// ---------------------------------------------------------------------------
// build_auto_config_model — discover auto-configuration classes, evaluate
// their @ConditionalOnXxx annotations against the project's actual source
// files and configuration, and classify each as enabled or disabled.
// ---------------------------------------------------------------------------

/// Build an `AutoConfigModel` by scanning the project for auto-configuration classes.
///
/// Searches for:
/// - Java source files annotated with `@AutoConfiguration` or `@Configuration`
/// - `META-INF/spring/*.imports` files (Spring Boot 3.x auto-config imports)
/// - `META-INF/spring.factories` files (Spring Boot 2.x format)
///
/// For each discovered auto-config class, the function inspects its source code
/// for `@ConditionalOnClass`, `@ConditionalOnMissingBean`, and `@ConditionalOnProperty`
/// annotations and attempts to verify them:
/// - `@ConditionalOnClass`: checks whether the referenced class exists in the
///   project's Java source tree
/// - `@ConditionalOnMissingBean`: unconditionally matched (cannot verify statically)
/// - `@ConditionalOnProperty`: checks project configuration files for the named
///   property and expected value
///
/// A class with all conditions matched is classified as **enabled**; otherwise
/// it is **disabled** with the first failing condition recorded as the reason.
pub fn build_auto_config_model(project_path: &Path) -> DoctorResult<AutoConfigModel> {
    // Collect all auto-config class names from source files and imports files
    let mut class_names: HashSet<String> = HashSet::new();

    // T014: Scan project Java sources for @AutoConfiguration
    let source_classes = find_auto_config_in_sources(project_path)?;
    for c in source_classes {
        class_names.insert(c);
    }

    // T014: Scan for AutoConfiguration.imports and spring.factories files
    let import_classes = find_auto_config_in_imports(project_path)?;
    for c in import_classes {
        class_names.insert(c);
    }

    if class_names.is_empty() {
        return Ok(AutoConfigModel::default());
    }

    // Pre-compute: build a set of all class FQNs present in the project
    // (used to verify @ConditionalOnClass conditions)
    let project_classes = collect_project_classes(project_path)?;

    // Pre-compute: read all config properties from application*.yml/properties
    // (used to verify @ConditionalOnProperty conditions)
    let config_properties = collect_config_properties(project_path);

    // Evaluate each auto-config class
    let mut enabled: Vec<AutoConfigClass> = Vec::new();
    let mut disabled: Vec<DisabledAutoConfig> = Vec::new();

    for class_name in &class_names {
        let (conditions, source_path) =
            evaluate_conditions(class_name, project_path, &project_classes, &config_properties)?;

        let all_matched = conditions.iter().all(|c| c.matched);

        if all_matched {
            enabled.push(AutoConfigClass {
                class_name: class_name.clone(),
                condition_results: conditions,
                registered_beans: Vec::new(), // cannot determine statically
            });
        } else {
            let first_failed = conditions
                .iter()
                .find(|c| !c.matched)
                .expect("at least one condition failed");

            // Record the source location in the reason for debugging
            let reason_detail = source_path
                .as_ref()
                .map(|p| format!(" (source: {p})"))
                .unwrap_or_default();

            disabled.push(DisabledAutoConfig {
                class_name: class_name.clone(),
                failed_condition: first_failed.condition_class.clone(),
                reason: format!("{}{reason_detail}", first_failed.message),
            });
        }
    }

    // Sort for deterministic output
    enabled.sort_by(|a, b| a.class_name.cmp(&b.class_name));
    disabled.sort_by(|a, b| a.class_name.cmp(&b.class_name));

    Ok(AutoConfigModel {
        enabled,
        disabled,
        excluded: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// T014: Discover auto-config classes from project sources
// ---------------------------------------------------------------------------

/// Walk the project source tree for `.java` files containing `@AutoConfiguration`.
/// Returns fully-qualified class names.
fn find_auto_config_in_sources(project_path: &Path) -> DoctorResult<HashSet<String>> {
    let mut classes = HashSet::new();
    let auto_config_re = Regex::new(r"@AutoConfiguration\b").unwrap();

    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path).map_err(|e| DoctorError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        if auto_config_re.is_match(&content) {
            if let Some(fqn) = extract_fully_qualified_name(&content) {
                classes.insert(fqn);
            }
        }
    }

    Ok(classes)
}

/// Scan for `META-INF/spring/*.imports` (Spring Boot 3.x) and
/// `META-INF/spring.factories` (Spring Boot 2.x) files.
fn find_auto_config_in_imports(project_path: &Path) -> DoctorResult<HashSet<String>> {
    let mut classes = HashSet::new();

    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let path_str = path.display().to_string();

        // Check for AutoConfiguration.imports (Spring Boot 3.x)
        if path_str.contains("META-INF/spring/")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.ends_with(".imports"))
        {
            let content =
                std::fs::read_to_string(path).map_err(|e| DoctorError::IoError {
                    path: path_str.clone(),
                    source: e,
                })?;
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    classes.insert(trimmed.to_string());
                }
            }
        }

        // Check for spring.factories (Spring Boot 2.x)
        if path_str.contains("META-INF/")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n == "spring.factories")
        {
            let content =
                std::fs::read_to_string(path).map_err(|e| DoctorError::IoError {
                    path: path_str.clone(),
                    source: e,
                })?;
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    // Lines look like: key=value1,value2,...
                    // Extract class names after '='
                    if let Some(pos) = trimmed.find('=') {
                        for cls in trimmed[pos + 1..].split(',') {
                            let cls = cls.trim();
                            if !cls.is_empty() {
                                classes.insert(cls.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(classes)
}

// ---------------------------------------------------------------------------
// Extract fully-qualified class name from Java source content
// ---------------------------------------------------------------------------

fn extract_fully_qualified_name(content: &str) -> Option<String> {
    let package_re = Regex::new(r"package\s+([a-z][\w.]*)\s*;").unwrap();
    let class_re =
        Regex::new(r"(?:public\s+)?(?:abstract\s+)?(?:final\s+)?class\s+(\w+)").unwrap();

    let package = package_re.captures(content)?.get(1)?.as_str();
    let class_name = class_re.captures(content)?.get(1)?.as_str();

    // Also check for @AutoConfiguration(before = ...) which may have the annotation
    // before the class keyword
    Some(format!("{package}.{class_name}"))
}

// ---------------------------------------------------------------------------
// Collect all class FQNs present in the project Java source tree
// ---------------------------------------------------------------------------

/// Walk all `.java` files in the project and collect every fully-qualified
/// class name. Used to verify `@ConditionalOnClass` references.
fn collect_project_classes(project_path: &Path) -> DoctorResult<HashSet<String>> {
    let mut classes = HashSet::new();

    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path).map_err(|e| DoctorError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        if let Some(fqn) = extract_fully_qualified_name(&content) {
            classes.insert(fqn);
        }
    }

    Ok(classes)
}

// ---------------------------------------------------------------------------
// Collect all config properties from project configuration files
// ---------------------------------------------------------------------------

/// Walk the project for `application*.yml` / `application*.properties` and
/// collect all key-value pairs into a `HashMap`. Used to verify
/// `@ConditionalOnProperty` references.
fn collect_config_properties(project_path: &Path) -> HashMap<String, String> {
    let mut props: HashMap<String, String> = HashMap::new();

    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            name.starts_with("application")
                && (name.ends_with(".yml")
                    || name.ends_with(".yaml")
                    || name.ends_with(".properties"))
        })
    {
        let path = entry.path();

        if let Ok(content) = std::fs::read_to_string(path) {
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            match ext {
                "yml" | "yaml" => {
                    parse_yaml_properties(&content, &mut props);
                }
                "properties" => {
                    parse_properties_content(&content, &mut props);
                }
                _ => {}
            }
        }
    }

    props
}

fn parse_yaml_properties(content: &str, props: &mut HashMap<String, String>) {
    // Try multi-doc first
    let docs: Vec<serde_yaml::Value> = serde_yaml::from_str(content).unwrap_or_default();
    if !docs.is_empty() {
        for doc in docs {
            flatten_yaml(&doc, String::new(), props);
        }
        return;
    }
    // Single doc
    if let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        flatten_yaml(&doc, String::new(), props);
    }
}

fn flatten_yaml(value: &serde_yaml::Value, prefix: String, out: &mut HashMap<String, String>) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                let key_str = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    other => format!("{other:?}"),
                };
                let full_key = if prefix.is_empty() {
                    key_str
                } else {
                    format!("{prefix}.{key_str}")
                };
                flatten_yaml(v, full_key, out);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            let joined: Vec<String> = seq
                .iter()
                .filter_map(|v| match v {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    other => Some(format!("{other:?}")),
                })
                .collect();
            out.insert(prefix, joined.join(","));
        }
        _ => {
            out.insert(prefix, format!("{value:?}"));
        }
    }
}

fn parse_properties_content(content: &str, props: &mut HashMap<String, String>) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            continue;
        }
        if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            if !key.is_empty() {
                props.insert(key, value);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// T015 / T016: Evaluate @ConditionalOnXxx annotations
// ---------------------------------------------------------------------------

/// Evaluate all `@ConditionalOnXxx` annotations for a given auto-config class.
///
/// Returns the list of `ConditionResult` entries and an optional source file
/// path (for use in error messages when a condition fails).
fn evaluate_conditions(
    class_name: &str,
    project_path: &Path,
    project_classes: &HashSet<String>,
    config_properties: &HashMap<String, String>,
) -> DoctorResult<(Vec<ConditionResult>, Option<String>)> {
    // Find the source file for this class
    let source_path = find_java_file_for_class(project_path, class_name);
    let content = source_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok());

    let mut results = Vec::new();

    // Collect imports from the source file for resolving simple class names
    let imports = content
        .as_deref()
        .map(collect_imports)
        .unwrap_or_default();

    // Check @ConditionalOnClass
    if let Some(ref src) = content {
        results.extend(evaluate_conditional_on_class(
            class_name, src, project_classes, &imports,
        ));
    }

    // Check @ConditionalOnMissingBean — always matched in static analysis
    if let Some(ref src) = content {
        if let Some(r) = evaluate_conditional_on_missing_bean(class_name, src) {
            results.push(r);
        }
    }

    // Check @ConditionalOnProperty
    if let Some(ref src) = content {
        results.extend(evaluate_conditional_on_property(
            class_name,
            src,
            config_properties,
        ));
    }

    // If no conditions were found at all, add a synthetic "always-matched" result
    if results.is_empty() {
        results.push(ConditionResult {
            condition_class: "Unconditional".to_string(),
            matched: true,
            message: "No @Conditional annotations found — always enabled".to_string(),
        });
    }

    Ok((results, source_path))
}

// ---------------------------------------------------------------------------
// Find source file for a class name
// ---------------------------------------------------------------------------

/// Locate the `.java` source file for a fully-qualified class name within the
/// project tree. Converts `com.example.Foo` to `com/example/Foo.java` and
/// looks for it under `project_path`.
fn find_java_file_for_class(project_path: &Path, class_name: &str) -> Option<String> {
    let relative_path = class_name.replace('.', "/") + ".java";

    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| ext == "java")
        })
    {
        let path_str = entry.path().display().to_string();
        if path_str.ends_with(&relative_path) {
            return Some(path_str);
        }
    }

    // Also try without the full path — just match the file name
    let simple_name = class_name.rsplit('.').next().unwrap_or(class_name);
    let file_name = format!("{simple_name}.java");

    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy() == file_name)
    {
        return Some(entry.path().display().to_string());
    }

    None
}

// ---------------------------------------------------------------------------
// Collect imports from Java source
// ---------------------------------------------------------------------------

fn collect_imports(content: &str) -> HashMap<String, String> {
    let import_re = Regex::new(r"^import\s+([a-z][\w.]*(?:\.\*)?)\s*;").unwrap();
    let mut imports = HashMap::new();

    for line in content.lines() {
        if let Some(caps) = import_re.captures(line.trim()) {
            let full = caps.get(1).unwrap().as_str();
            if full.ends_with(".*") {
                // Wildcard import — skip (can't resolve simple names)
                continue;
            }
            if let Some(simple) = full.rsplit('.').next() {
                imports.insert(simple.to_string(), full.to_string());
            }
        }
    }

    imports
}

// ---------------------------------------------------------------------------
// T015: @ConditionalOnClass evaluation
// ---------------------------------------------------------------------------

/// Evaluate `@ConditionalOnClass` annotations found in the source content.
///
/// Handles two common forms:
/// - `@ConditionalOnClass(name = "com.example.SomeClass")` — string literal
/// - `@ConditionalOnClass({Foo.class, Bar.class})` — class literals
fn evaluate_conditional_on_class(
    class_name: &str,
    content: &str,
    project_classes: &HashSet<String>,
    imports: &HashMap<String, String>,
) -> Vec<ConditionResult> {
    let mut results = Vec::new();

    // Pattern 1: name = "fully.qualified.ClassName"
    let name_pattern =
        Regex::new(r#"@ConditionalOnClass\s*\([^)]*name\s*=\s*"([^"]+)""#).unwrap();
    for caps in name_pattern.captures_iter(content) {
        let required_class = caps.get(1).unwrap().as_str();
        let found = project_classes.contains(required_class);
        results.push(ConditionResult {
            condition_class: "ConditionalOnClass".to_string(),
            matched: found,
            message: if found {
                format!("Required class `{required_class}` found in project")
            } else {
                format!("Required class `{required_class}` not found in project sources")
            },
        });
    }

    // Pattern 2: class references like SomeClass.class or OtherClass.class
    let class_ref_pattern =
        Regex::new(r"@ConditionalOnClass\s*\(\s*(?:value\s*=\s*)?\{?([^)}]+)\}?").unwrap();
    if name_pattern.find(content).is_none() {
        // Only try this if no name= pattern matched
        if let Some(caps) = class_ref_pattern.captures(content) {
            let args_str = caps.get(1).unwrap().as_str();
            let simple_names: Vec<&str> = args_str
                .split(',')
                .filter_map(|s| {
                    let s = s.trim();
                    s.strip_suffix(".class")
                })
                .collect();

            for simple in simple_names {
                // Try to resolve via imports or as FQN directly
                let resolved = imports
                    .get(simple)
                    .cloned()
                    .unwrap_or_else(|| simple.to_string());

                let found = project_classes.contains(&resolved)
                    || project_classes.iter().any(|c| c.ends_with(&format!(".{simple}")));

                results.push(ConditionResult {
                    condition_class: "ConditionalOnClass".to_string(),
                    matched: found,
                    message: if found {
                        format!("Required class `{simple}` (resolved: {resolved}) found in project")
                    } else {
                        format!(
                            "Required class `{simple}` (resolved: {resolved}) not found in project"
                        )
                    },
                });
            }
        }
    }

    // If @ConditionalOnClass exists but no patterns matched, add a note
    if results.is_empty() && content.contains("@ConditionalOnClass") {
        results.push(ConditionResult {
            condition_class: "ConditionalOnClass".to_string(),
            matched: false,
            message: format!(
                "Could not parse @ConditionalOnClass arguments for `{class_name}`"
            ),
        });
    }

    results
}

// ---------------------------------------------------------------------------
// T015: @ConditionalOnMissingBean evaluation
// ---------------------------------------------------------------------------

/// Evaluate `@ConditionalOnMissingBean`. In static analysis we cannot reliably
/// determine bean existence, so this condition is always considered matched.
fn evaluate_conditional_on_missing_bean(
    _class_name: &str,
    content: &str,
) -> Option<ConditionResult> {
    let re = Regex::new(r"@ConditionalOnMissingBean\b").unwrap();
    if re.is_match(content) {
        Some(ConditionResult {
            condition_class: "ConditionalOnMissingBean".to_string(),
            matched: true,
            message: "ConditionalOnMissingBean — assumed satisfied (cannot verify statically)"
                .to_string(),
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// T015: @ConditionalOnProperty evaluation
// ---------------------------------------------------------------------------

/// Evaluate `@ConditionalOnProperty` by checking the named property against
/// the project's configuration files.
///
/// Handles the form:
/// `@ConditionalOnProperty(name = "some.property", havingValue = "expected")`
///
/// If `havingValue` is omitted, the condition matches when the property is
/// present and its value is not `"false"`.
/// If `matchIfMissing = true` is set, absence of the property is treated as a match.
fn evaluate_conditional_on_property(
    _class_name: &str,
    content: &str,
    config_properties: &HashMap<String, String>,
) -> Vec<ConditionResult> {
    let mut results = Vec::new();

    // Extract property name and optional havingValue
    let prop_re = Regex::new(
        r"@ConditionalOnProperty\s*\(([^)]*(?:\([^)]*\)[^)]*)*)\)"
    ).unwrap();

    for caps in prop_re.captures_iter(content) {
        let args = caps.get(1).unwrap().as_str();

        let name = extract_string_arg(args, "name");
        let having_value = extract_string_arg(args, "havingValue");
        let match_if_missing = extract_bool_arg(args, "matchIfMissing").unwrap_or(false);

        if let Some(ref prop_name) = name {
            match config_properties.get(prop_name) {
                Some(actual_value) => {
                    if let Some(ref expected) = having_value {
                        let matched = actual_value == expected;
                        results.push(ConditionResult {
                            condition_class: "ConditionalOnProperty".to_string(),
                            matched,
                            message: if matched {
                                format!(
                                    "Property `{prop_name}` = `{actual_value}` matches expected `{expected}`"
                                )
                            } else {
                                format!(
                                    "Property `{prop_name}` = `{actual_value}`, expected `{expected}`"
                                )
                            },
                        });
                    } else {
                        // No havingValue: match if value is not "false"
                        let matched = actual_value != "false";
                        results.push(ConditionResult {
                            condition_class: "ConditionalOnProperty".to_string(),
                            matched,
                            message: if matched {
                                format!("Property `{prop_name}` = `{actual_value}` (non-false)")
                            } else {
                                format!("Property `{prop_name}` = `false` (treated as disabled)")
                            },
                        });
                    }
                }
                None => {
                    results.push(ConditionResult {
                        condition_class: "ConditionalOnProperty".to_string(),
                        matched: match_if_missing,
                        message: if match_if_missing {
                            format!("Property `{prop_name}` not found, but matchIfMissing=true")
                        } else {
                            format!(
                                "Property `{prop_name}` not found in any configuration file"
                            )
                        },
                    });
                }
            }
        }
    }

    // If @ConditionalOnProperty exists but no patterns matched
    if results.is_empty() && content.contains("@ConditionalOnProperty") {
        results.push(ConditionResult {
            condition_class: "ConditionalOnProperty".to_string(),
            matched: false,
            message: "Could not parse @ConditionalOnProperty arguments".to_string(),
        });
    }

    results
}

/// Extract a named string argument from annotation arguments text.
/// e.g. from `name = "foo", havingValue = "bar"` extracts `"foo"` for arg `name`.
fn extract_string_arg(args: &str, arg_name: &str) -> Option<String> {
    // Build a pattern that finds arg_name = "value"
    let pattern_str = format!(r#"{}\s*=\s*"([^"]*)""#, regex::escape(arg_name));
    let re = Regex::new(&pattern_str).ok()?;
    re.captures(args)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract a named boolean argument from annotation arguments text.
/// e.g. from `matchIfMissing = true` extracts `true`.
fn extract_bool_arg(args: &str, arg_name: &str) -> Option<bool> {
    let pattern_str = format!(r"{}\s*=\s*(true|false)", regex::escape(arg_name));
    let re = Regex::new(&pattern_str).ok()?;
    re.captures(args)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str() == "true")
}
