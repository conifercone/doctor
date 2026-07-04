use crate::error::DoctorResult;
use crate::evidence::Evidence;
use crate::model::{Category, Confidence, Issue, Severity, SystemModel};

/// Detect configuration properties with conflicting values from multiple sources.
pub fn detect_config_conflicts(
    model: &SystemModel,
    _evidence: &[Evidence],
) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();

    for (i, prop) in model.config_model.properties.iter().enumerate() {
        if !prop.conflicts.is_empty() {
            let id = format!("CONFIG-{:03}", i + 1);
            let conflict_desc: Vec<String> = prop
                .conflicts
                .iter()
                .map(|c| {
                    format!(
                        "{}: '{}' vs {}: '{}'",
                        c.source_a.location, c.value_a, c.source_b.location, c.value_b
                    )
                })
                .collect();

            let issue = Issue::new(
                id,
                format!("Conflicting values for '{}'", prop.key),
                Severity::Warning,
                Category::Config,
                format!("Property '{}' has conflicting values: {}", prop.key, conflict_desc.join("; ")),
                vec![Evidence::new(
                    crate::evidence::EvidenceType::ConfigFile,
                    prop.sources.first().map(|s| s.location.as_str()).unwrap_or("unknown"),
                    format!("{} conflicting sources for key '{}'", prop.conflicts.len(), prop.key),
                    crate::evidence::Reliability::Confirmed,
                )],
                "Review the conflicting sources and consolidate to a single value. Spring resolves by priority — ensure the winning source is intentional.".to_string(),
                Confidence::High,
            );
            if let Some(issue) = issue {
                issues.push(issue);
            }
        }
    }
    Ok(issues)
}
