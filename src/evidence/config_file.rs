use crate::error::{DoctorError, DoctorResult};
use crate::evidence::{Evidence, EvidenceType, Reliability};
use std::path::Path;
use walkdir::WalkDir;

/// Collect evidence from Spring Boot configuration files.
pub fn collect(project_path: &Path) -> DoctorResult<Vec<Evidence>> {
    let mut evidence = Vec::new();

    for entry in WalkDir::new(project_path).into_iter().filter_map(|e| e.ok()).filter(|e| {
        let name = e.file_name().to_string_lossy();
        name == "application.yml"
            || name == "application.yaml"
            || name.starts_with("application-")
            || name == "application.properties"
    }) {
        let content = std::fs::read_to_string(entry.path()).map_err(|e| DoctorError::IoError {
            path: entry.path().display().to_string(),
            source: e,
        })?;

        let source = entry.path().display().to_string();

        // Parse YAML files
        if entry.path().extension().map_or(false, |e| e == "yml" || e == "yaml") {
            if let Ok(docs) = serde_yaml::from_str::<Vec<serde_yaml::Value>>(&content) {
                for doc in docs {
                    evidence.push(Evidence::new(
                        EvidenceType::ConfigFile,
                        source.clone(),
                        format!("YAML configuration with {} top-level keys", count_keys(&doc, 0)),
                        Reliability::Confirmed,
                    ));
                }
            } else {
                // Single document YAML
                evidence.push(Evidence::new(
                    EvidenceType::ConfigFile,
                    source.clone(),
                    format!("YAML configuration file ({} bytes)", content.len()),
                    Reliability::Confirmed,
                ));
            }
        } else {
            // Properties file
            let count =
                content.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).count();
            evidence.push(Evidence::new(
                EvidenceType::ConfigFile,
                source,
                format!("Properties file with {count} entries"),
                Reliability::Confirmed,
            ));
        }
    }

    Ok(evidence)
}

fn count_keys(value: &serde_yaml::Value, depth: usize) -> usize {
    if depth > 5 {
        return 0;
    }
    match value {
        serde_yaml::Value::Mapping(map) => {
            map.len() + map.values().map(|v| count_keys(v, depth + 1)).sum::<usize>()
        }
        _ => 0,
    }
}
