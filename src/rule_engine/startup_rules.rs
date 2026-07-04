use crate::error::DoctorResult;
use crate::evidence::Evidence;
use crate::model::{Category, Confidence, Issue, Severity, SystemModel};

/// Analyze Spring Boot startup configuration for potential performance issues.
pub fn analyze_startup(model: &SystemModel, _evidence: &[Evidence]) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();

    let enabled_count = model.auto_config_model.enabled.len();
    let disabled_count = model.auto_config_model.disabled.len();
    let bean_count = model.bean_graph.beans.len();

    // Flag large number of beans as potential startup concern
    if bean_count > 200 {
        let issue = Issue::new(
            "STARTUP-001",
            "Large number of beans may impact startup time",
            Severity::Info,
            Category::Startup,
            format!("Project has {bean_count} beans. {} auto-config classes enabled, {} disabled.", enabled_count, disabled_count),
            vec![Evidence::new(
                crate::evidence::EvidenceType::SourceCode,
                "Bean dependency graph".to_string(),
                format!("{bean_count} beans detected in project"),
                crate::evidence::Reliability::Confirmed,
            )],
            "Consider using @Lazy initialization for non-critical beans or enabling spring.main.lazy-initialization=true".to_string(),
            Confidence::Medium,
        );
        if let Some(issue) = issue {
            issues.push(issue);
        }
    }

    // Report overall startup health
    if enabled_count > 50 {
        let issue = Issue::new(
            "STARTUP-002",
            "Many auto-configuration classes active",
            Severity::Info,
            Category::Startup,
            format!(
                "{enabled_count} auto-configuration classes are active. Consider excluding unused ones."
            ),
            vec![Evidence::new(
                crate::evidence::EvidenceType::ConfigFile,
                "Auto-configuration model".to_string(),
                format!("{enabled_count} enabled, {disabled_count} disabled auto-config classes"),
                crate::evidence::Reliability::Confirmed,
            )],
            "Use spring.autoconfigure.exclude to disable unnecessary auto-configuration classes"
                .to_string(),
            Confidence::Medium,
        );
        if let Some(issue) = issue {
            issues.push(issue);
        }
    }

    Ok(issues)
}
