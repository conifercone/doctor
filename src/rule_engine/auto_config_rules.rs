use crate::error::DoctorResult;
use crate::evidence::Evidence;
use crate::model::{Category, Confidence, Issue, Severity, SystemModel};

/// Detect auto-configuration classes that failed to activate.
pub fn detect_auto_config_failures(
    model: &SystemModel,
    _evidence: &[Evidence],
) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();

    for (i, disabled) in model.auto_config_model.disabled.iter().enumerate() {
        let id = format!("AUTO-{:03}", i + 1);
        let issue = Issue::new(
            id,
            format!("Auto-configuration '{}' not applied", disabled.class_name),
            Severity::Warning,
            Category::AutoConfig,
            format!(
                "Auto-config class '{}' was not applied because condition '{}' failed: {}",
                disabled.class_name, disabled.failed_condition, disabled.reason
            ),
            vec![Evidence::new(
                crate::evidence::EvidenceType::SourceCode,
                disabled.class_name.clone(),
                format!(
                    "Condition '{}' not satisfied: {}",
                    disabled.failed_condition, disabled.reason
                ),
                crate::evidence::Reliability::Confirmed,
            )],
            format!(
                "Ensure the required condition '{}' is met, or exclude this auto-configuration if not needed",
                disabled.failed_condition
            ),
            Confidence::High,
        );
        if let Some(issue) = issue {
            issues.push(issue);
        }
    }
    Ok(issues)
}

/// Detect @ConditionalOnXxx conditions that prevented expected beans from being created.
pub fn detect_conditional_failures(
    model: &SystemModel,
    _evidence: &[Evidence],
) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();

    for disabled in &model.auto_config_model.disabled {
        // Only flag if the auto-config class looks meaningful (not an internal one)
        if disabled.class_name.contains("springframework.boot.autoconfigure")
            && !disabled.class_name.contains("$")
        {
            let short_name = disabled.class_name.rsplit('.').next().unwrap_or(&disabled.class_name);
            let issue = Issue::new(
                format!("COND-{}", short_name),
                format!("Conditional assembly failed for '{}'", short_name),
                Severity::Info,
                Category::AutoConfig,
                format!(
                    "'{}' was not created because condition '{}' was not met: {}",
                    short_name, disabled.failed_condition, disabled.reason
                ),
                vec![Evidence::new(
                    crate::evidence::EvidenceType::ConfigFile,
                    disabled.class_name.clone(),
                    format!("Auto-config condition check for {}", disabled.failed_condition),
                    crate::evidence::Reliability::Inferred,
                )],
                "Review your configuration to ensure the condition is intentionally not met"
                    .to_string(),
                Confidence::Medium,
            );
            if let Some(issue) = issue {
                issues.push(issue);
            }
        }
    }
    Ok(issues)
}
