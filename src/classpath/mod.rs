//! Classpath scanning for auto-configuration Bean discovery.
//!
//! Walks Gradle/Maven dependency caches to find jars containing
//! AutoConfiguration.imports, parses .class file constant pools to
//! extract @Bean/@Component/@Import/@ConfigurationProperties definitions,
//! and caches results for subsequent fast lookups.

pub mod cache;
pub mod class_parser;
pub mod jar_scanner;

use crate::error::DoctorResult;
use std::path::Path;

/// An auto-configuration Bean discovered from dependency jars.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutoConfigBean {
    pub bean_name: String,
    pub class_name: String,
    /// Fully-qualified auto-configuration class that provides this bean
    pub source_class: String,
    /// How this bean was discovered
    pub discovery_method: DiscoveryMethod,
    /// Interfaces implemented by this bean's class
    pub interfaces: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiscoveryMethod {
    BeanAnnotation,
    ComponentStereotype,
    ImportRecursive,
    ConfigurationProperties,
}

/// Scan all dependency jars and discover auto-configuration beans.
/// Uses cache to avoid repeated full scans (FR-A05).
pub fn discover_auto_config_beans(project_path: &Path) -> DoctorResult<Vec<AutoConfigBean>> {
    let cache_key = cache::compute_cache_key(project_path);

    // Try cache first
    if let Some(cached) = cache::load_cache(&cache_key) {
        eprintln!("  Auto-config cache: hit");
        return Ok(cached);
    }
    eprintln!("  Auto-config cache: miss, scanning jars...");

    // Full scan
    let global_index = jar_scanner::build_class_index();
    let class_names = jar_scanner::discover_auto_config_classes_with_index(&global_index)?;
    let mut beans = Vec::new();
    let mut parsed_count = 0;
    let mut bean_total = 0;

    for (class_name, jar_path) in &class_names {
        match jar_scanner::read_class_from_jar(jar_path, class_name) {
            Ok(class_bytes) => {
                match class_parser::parse_class(&class_bytes) {
                    Ok(parsed) => {
                        parsed_count += 1;
                        let found = parsed.beans.len();
                        bean_total += found;
                        if found > 0 {
                            // (debug disabled) eprintln!("  DEBUG: {} @Bean → {found} beans", class_name);
                        }
                        for bean in parsed.beans {
                            // Look up the @Bean return type's .class to extract its interfaces
                            let return_interfaces: Vec<String> = if !bean.bean_type_fqcn.is_empty() {
                                resolve_interfaces_for_type(&class_names, &global_index, &bean.bean_type_fqcn)
                                    .unwrap_or_default()
                            } else {
                                vec![]
                            };
                            beans.push(AutoConfigBean {
                                bean_name: bean.bean_name,
                                class_name: bean.bean_type,
                                source_class: parsed.class_name.clone(),
                                discovery_method: bean.discovery_method,
                                interfaces: return_interfaces,
                            });
                        }
                        // @Import recursive expansion (depth-1)
                        for imported_class in parsed.imported_classes {
                            // Look up the imported class in the same jar or any jar
                            if let Some(imported_beans) =
                                resolve_imported_class(&class_names, &imported_class)
                            {
                                beans.extend(imported_beans);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  Warning: failed to parse class {}: {e}", class_name);
                    }
                }
            }
            Err(e) => {
                eprintln!("  Warning: failed to read class {}: {e}", class_name);
            }
        }
    }

    // Phase 2: Scan ALL @Configuration classes in global index
    // Catches framework beans not in AutoConfiguration.imports
    let mut phase2 = 0usize;
    for (fqcn, jar_path) in &global_index {
        let simple = fqcn.rsplit('.').next().unwrap_or(fqcn);
        if !simple.ends_with("Configuration") { continue; }
        if class_names.iter().any(|(cn, _)| cn == fqcn) { continue; }
        if let Ok(class_bytes) = jar_scanner::read_class_from_jar(jar_path, fqcn) {
            if let Ok(parsed) = class_parser::parse_class(&class_bytes) {
                phase2 += 1;
                for bean in &parsed.beans {
                    let ifaces = if bean.bean_type_fqcn.is_empty() { vec![] }
                        else { resolve_interfaces_for_type(&class_names, &global_index, &bean.bean_type_fqcn).unwrap_or_default() };
                    beans.push(AutoConfigBean {
                        bean_name: bean.bean_name.clone(),
                        class_name: bean.bean_type.clone(),
                        source_class: parsed.class_name.clone(),
                        discovery_method: bean.discovery_method,
                        interfaces: ifaces,
                    });
                }
            }
        }
    }

    // Dedup by (bean_name, class_name)
    beans.sort_by(|a, b| a.bean_name.cmp(&b.bean_name).then(a.class_name.cmp(&b.class_name)));
    beans.dedup_by(|a, b| a.bean_name == b.bean_name && a.class_name == b.class_name);

    eprintln!("  Parsed {parsed_count} auto-config + {phase2} framework classes, found {bean_total} beans total");
    // Save cache
    if let Err(e) = cache::save_cache(&cache_key, &beans) {
        eprintln!("  Warning: failed to save cache: {e}");
    }

    Ok(beans)
}

/// Look up the .class file for a Bean return type and extract its interfaces.
fn resolve_interfaces_for_type(
    class_names: &[(String, String)],
    global_index: &std::collections::HashMap<String, String>,
    fqcn: &str,
) -> Option<Vec<String>> {
    // Direct match: class listed in auto-config imports
    for (jar_path, cls) in class_names {
        if cls == fqcn {
            if let Ok(class_bytes) = jar_scanner::read_class_from_jar(jar_path, cls) {
                if let Ok(parsed) = class_parser::parse_class(&class_bytes) {
                    return Some(parsed.interfaces);
                }
            }
        }
    }
    // Fallback: try global jar index (178K classes)
    if let Some(jar_path) = global_index.get(fqcn) {
        if let Ok(class_bytes) = jar_scanner::read_class_from_jar(jar_path, fqcn) {
            if let Ok(parsed) = class_parser::parse_class(&class_bytes) {
                return Some(parsed.interfaces);
            }
        }
    }
    None
}

/// Try to resolve an @Import-ed class to its beans.
fn resolve_imported_class(
    jar_classes: &[(String, String)],
    target_class: &str,
) -> Option<Vec<AutoConfigBean>> {
    for (jar_path, class_name) in jar_classes {
        if class_name == target_class {
            if let Ok(class_bytes) = jar_scanner::read_class_from_jar(jar_path, class_name) {
                if let Ok(parsed) = class_parser::parse_class(&class_bytes) {
                    return Some(
                        parsed
                            .beans
                            .into_iter()
                            .map(|b| AutoConfigBean {
                                bean_name: b.bean_name,
                                class_name: b.bean_type,
                                source_class: format!("{target_class} (via @Import from {})", parsed.class_name),
                                discovery_method: DiscoveryMethod::ImportRecursive,
                                interfaces: parsed.interfaces.clone(),
                            })
                            .collect(),
                    );
                }
            }
        }
    }
    None
}
