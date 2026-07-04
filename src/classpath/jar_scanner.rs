use crate::error::DoctorResult;
use std::collections::HashMap;
use std::path::Path;

/// Run `unzip -p` to extract a single file from a jar (zip) archive.
fn unzip_read(jar_path: &str, internal_path: &str) -> Option<Vec<u8>> {
    let output = std::process::Command::new("unzip")
        .arg("-p")
        .arg(jar_path)
        .arg(internal_path)
        .output()
        .ok()?;
    if output.status.success() || !output.stdout.is_empty() {
        Some(output.stdout)
    } else {
        None
    }
}

/// Run `unzip -l` to list all entries in a jar, building a class_name→jar_path index.
fn index_jar_classes(jar_path: &str, index: &mut HashMap<String, String>) {
    let output = std::process::Command::new("unzip")
        .arg("-l")
        .arg(jar_path)
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return,
    };
    let listing = String::from_utf8_lossy(&output.stdout);
    for line in listing.lines() {
        // unzip -l format: "   length  date  time   name"
        // We look for lines ending with .class
        let trimmed = line.trim();
        if trimmed.ends_with(".class") {
            // Extract just the filename part (last component after whitespace)
            if let Some(path) = trimmed.split_whitespace().last() {
                // Convert class path to FQCN: org/foo/Bar.class → org.foo.Bar
                let fqcn = path
                    .strip_suffix(".class")
                    .unwrap_or(path)
                    .replace('/', ".");
                // Index ALL classes (need full coverage for return-type interface lookup)
                index.entry(fqcn).or_insert_with(|| jar_path.to_string());
            }
        }
    }
}

/// Scan a single jar for auto-configuration class names.
fn scan_jar_classes(jar_path: &str) -> Vec<String> {
    let imports_path =
        "META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports";
    if let Some(data) = unzip_read(jar_path, imports_path) {
        if let Ok(content) = String::from_utf8(data) {
            return content
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
        }
    }
    // Fallback: spring.factories (SB 2.x)
    if let Some(data) = unzip_read(jar_path, "META-INF/spring.factories") {
        if let Ok(content) = String::from_utf8(data) {
            let mut classes = Vec::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed
                    .starts_with("org.springframework.boot.autoconfigure.EnableAutoConfiguration")
                {
                    if let Some(after) = trimmed.split('=').nth(1) {
                        for c in after.split(',').map(|c| c.trim().replace('\\', "")) {
                            if !c.is_empty() {
                                classes.push(c);
                            }
                        }
                    }
                }
            }
            if !classes.is_empty() {
                return classes;
            }
        }
    }
    vec![]
}

/// Build a global class_name → jar_path index across all jars.
fn build_class_index() -> HashMap<String, String> {
    let mut index = HashMap::new();
    let Ok(home) = std::env::var("HOME") else { return index; };

    for dir in [
        Path::new(&home).join(".gradle/caches/modules-2/files-2.1"),
        Path::new(&home).join(".m2/repository"),
    ] {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(8)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "jar"))
        {
            index_jar_classes(&entry.path().display().to_string(), &mut index);
        }
    }
    index
}

/// Discover auto-config classes and their locations.
/// Returns: (class_name, jar_path) pairs where the class IS resolvable.
pub fn discover_auto_config_classes() -> DoctorResult<Vec<(String, String)>> {
    // Phase 1: Build global class→jar index
    eprintln!("  Building jar class index...");
    let class_index = build_class_index();
    eprintln!("  Indexed {} classes across jars", class_index.len());

    // Phase 2: Scan for AutoConfiguration.imports
    let mut results = Vec::new();
    let Ok(home) = std::env::var("HOME") else { return Ok(results) };
    let mut seen = std::collections::HashSet::new();

    for dir in [
        Path::new(&home).join(".gradle/caches/modules-2/files-2.1"),
        Path::new(&home).join(".m2/repository"),
    ] {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(8)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "jar"))
        {
            let jar_path = entry.path().display().to_string();
            for class_name in scan_jar_classes(&jar_path) {
                if !seen.insert(class_name.clone()) {
                    continue; // dedup
                }
                // Look up the class in the global index
                if let Some(resolved_jar) = class_index.get(&class_name) {
                    results.push((class_name, resolved_jar.clone()));
                }
                // If not found in index, skip gracefully (SB4 modular jars)
            }
        }
    }

    Ok(results)
}

/// Read a `.class` file from inside a jar by fully-qualified class name.
pub fn read_class_from_jar(jar_path: &str, class_name: &str) -> Result<Vec<u8>, String> {
    let class_file = format!("{}.class", class_name.replace('.', "/"));
    unzip_read(jar_path, &class_file)
        .ok_or_else(|| format!("class not found in jar: {class_file}"))
}
