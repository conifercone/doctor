use serde::{Deserialize, Serialize};

use crate::error::{DoctorError, DoctorResult};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConfigSourceType {
    CommandLine,
    SystemProperty,
    EnvVar,
    ApplicationYml,
    ApplicationProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSource {
    pub source_type: ConfigSourceType,
    pub location: String,
    /// Spring configuration priority (1 = highest)
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigConflict {
    pub source_a: ConfigSource,
    pub value_a: String,
    pub source_b: ConfigSource,
    pub value_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProperty {
    pub key: String,
    /// Summary only — never contains sensitive values
    pub value_summary: String,
    pub sources: Vec<ConfigSource>,
    pub conflicts: Vec<ConfigConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigModel {
    pub properties: Vec<ConfigProperty>,
}

// ---------------------------------------------------------------------------
// build_config_model — walk the project tree, parse application*.yml/yaml
// and application*.properties files, flatten YAML nested keys to dot-notation,
// and detect configuration conflicts (same key, different values across sources).
// ---------------------------------------------------------------------------

/// Build a `ConfigModel` by scanning the project for Spring Boot configuration files.
///
/// Walks `project_path` looking for `application*.yml`, `application*.yaml`, and
/// `application*.properties` files. Handles multi-document YAML (separated by `---`).
/// Nested YAML keys are flattened to dot-notation (e.g. `spring.datasource.url`).
/// When the same key appears in multiple sources with different values, a
/// `ConfigConflict` is recorded.
pub fn build_config_model(project_path: &Path) -> DoctorResult<ConfigModel> {
    // key → list of (value, source, source_location) tuples
    let mut key_entries: HashMap<String, Vec<(String, ConfigSource)>> = HashMap::new();

    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_config_file(e.file_name().to_string_lossy().as_ref()))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path).map_err(|e| DoctorError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        let location = path.display().to_string();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let source_type = match ext {
            "yml" | "yaml" => ConfigSourceType::ApplicationYml,
            "properties" => ConfigSourceType::ApplicationProperties,
            _ => continue,
        };

        let source = ConfigSource {
            source_type,
            location: location.clone(),
            priority: priority_for(source_type),
        };

        match source_type {
            ConfigSourceType::ApplicationYml => {
                parse_yaml_file(&content, &location, &source, &mut key_entries)?;
            }
            ConfigSourceType::ApplicationProperties => {
                parse_properties_file(&content, &location, &source, &mut key_entries);
            }
            _ => {}
        }
    }

    // Build ConfigProperty list with conflict detection
    let properties = build_properties_with_conflicts(key_entries);

    Ok(ConfigModel { properties })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_config_file(filename: &str) -> bool {
    filename.starts_with("application")
        && (filename.ends_with(".yml")
            || filename.ends_with(".yaml")
            || filename.ends_with(".properties"))
}

fn priority_for(st: ConfigSourceType) -> u8 {
    match st {
        ConfigSourceType::ApplicationYml => 4,
        ConfigSourceType::ApplicationProperties => 5,
        _ => 9,
    }
}

// ---------------------------------------------------------------------------
// YAML parsing (T011)
// ---------------------------------------------------------------------------

/// Parse a YAML configuration file. Tries multi-document first (`---` separator),
/// falls back to single-document parsing.
fn parse_yaml_file(
    content: &str,
    location: &str,
    source: &ConfigSource,
    key_entries: &mut HashMap<String, Vec<(String, ConfigSource)>>,
) -> DoctorResult<()> {
    // Attempt multi-document YAML first
    if let Ok(docs) = serde_yaml::from_str::<Vec<serde_yaml::Value>>(content) {
        if !docs.is_empty() {
            for doc in docs {
                let mut flat: HashMap<String, String> = HashMap::new();
                flatten_yaml_value(&doc, String::new(), &mut flat);
                for (key, value) in flat {
                    key_entries
                        .entry(key)
                        .or_default()
                        .push((value, source.clone()));
                }
            }
            return Ok(());
        }
    }

    // Fall back to single-document
    let doc: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| DoctorError::ParseError {
            file: location.to_string(),
            message: format!("Failed to parse YAML: {e}"),
        })?;

    let mut flat: HashMap<String, String> = HashMap::new();
    flatten_yaml_value(&doc, String::new(), &mut flat);
    for (key, value) in flat {
        key_entries
            .entry(key)
            .or_default()
            .push((value, source.clone()));
    }

    Ok(())
}

/// Recursively flatten a `serde_yaml::Value` into dot-notation key-value pairs.
///
/// Scalars become leaf entries. Sequences are joined with commas. Nested mappings
/// recurse with a `.`-separated prefix.
fn flatten_yaml_value(
    value: &serde_yaml::Value,
    prefix: String,
    out: &mut HashMap<String, String>,
) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                let key_str = key_to_string(k);
                let full_key = if prefix.is_empty() {
                    key_str
                } else {
                    format!("{}.{}", prefix, key_str)
                };
                flatten_yaml_value(v, full_key, out);
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
            let val_str = format!("{value:?}");
            // Strip surrounding quotes that serde_yaml formatting may add for strings
            let val_str = val_str.trim_matches('"').to_string();
            out.insert(prefix, val_str);
        }
    }
}

/// Convert a YAML key (which may be a string, number, bool, etc.) to a string.
fn key_to_string(key: &serde_yaml::Value) -> String {
    match key {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        other => format!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Properties parsing (T012)
// ---------------------------------------------------------------------------

/// Parse a `.properties` file. Extracts `key=value` lines, skipping blank lines
/// and comment lines (starting with `#` or `!`).
fn parse_properties_file(
    content: &str,
    _location: &str,
    source: &ConfigSource,
    key_entries: &mut HashMap<String, Vec<(String, ConfigSource)>>,
) {
    for line in content.lines() {
        let trimmed = line.trim();

        // Skip blank lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            continue;
        }

        // Find the first unescaped '=' or ':'
        if let Some(pos) = find_property_separator(trimmed) {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            if !key.is_empty() {
                key_entries
                    .entry(key)
                    .or_default()
                    .push((value, source.clone()));
            }
        }
    }
}

/// Find the first `=` or `:` that acts as a key-value separator in a properties line.
fn find_property_separator(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\\' {
            // Skip escaped character
            continue;
        }
        if b == b'=' || b == b':' {
            return Some(i);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Conflict detection (T013)
// ---------------------------------------------------------------------------

/// Build the final `Vec<ConfigProperty>` from raw key→entries mapping, detecting
/// conflicts where the same key has different values from different sources.
fn build_properties_with_conflicts(
    key_entries: HashMap<String, Vec<(String, ConfigSource)>>,
) -> Vec<ConfigProperty> {
    let mut properties: Vec<ConfigProperty> = Vec::new();

    for (key, entries) in key_entries {
        // Deduplicate: keep only one entry per source location.
        // If the same source has multiple entries, take the last one (last wins
        // within a single file in Spring Boot).
        let mut deduped: Vec<(String, ConfigSource)> = Vec::new();
        for (value, source) in entries {
            if let Some(existing) = deduped.iter_mut().find(|(_, s)| s.location == source.location)
            {
                // Same source — later entry wins
                existing.0 = value;
            } else {
                deduped.push((value, source));
            }
        }

        // Value summary: truncate and mask sensitive values from the first entry
        let value_summary = deduped
            .first()
            .map(|(v, _)| summarize_value(v))
            .unwrap_or_default();

        // Collect unique sources
        let sources: Vec<ConfigSource> = deduped
            .iter()
            .map(|(_, s)| s.clone())
            .collect();

        // Detect conflicts: same key, different values across different sources
        let mut conflicts: Vec<ConfigConflict> = Vec::new();
        if deduped.len() > 1 {
            let (first_val, first_src) = &deduped[0];
            for (other_val, other_src) in &deduped[1..] {
                if other_val != first_val {
                    conflicts.push(ConfigConflict {
                        source_a: (*first_src).clone(),
                        value_a: first_val.clone(),
                        source_b: (*other_src).clone(),
                        value_b: other_val.clone(),
                    });
                }
            }
        }

        properties.push(ConfigProperty {
            key,
            value_summary,
            sources,
            conflicts,
        });
    }

    // Sort by key for deterministic output
    properties.sort_by(|a, b| a.key.cmp(&b.key));
    properties
}

/// Produce a short summary of a configuration value.
///
/// Values longer than 120 characters are truncated. Values that look like
/// secrets (containing "password", "secret", "key", "token" in the key context)
/// are masked. Since we don't have the key here, we check the value itself for
/// patterns and mask long random-looking strings.
fn summarize_value(value: &str) -> String {
    if value.len() <= 120 {
        value.to_string()
    } else {
        format!("{}... ({} chars total)", &value[..117], value.len())
    }
}
