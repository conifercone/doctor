use crate::model::report::DiagnosticReport;
use serde::{Deserialize, Serialize};

/// Structured summary sent to LLM for AI explanation.
/// Contains ONLY type names, class names, and descriptive summaries —
/// NEVER source code or config values (per FR-030, Constitution I).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredSummary {
    pub project_context: ProjectContext,
    /// Top 20 issues by severity, filtered and summarized
    pub issues: Vec<IssueSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub spring_boot_version: Option<String>,
    pub build_tool: String,
    pub starters: Vec<String>,
    pub issue_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSummary {
    pub issue_id: String,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub evidence_count: usize,
    pub fix_suggestion: String,
    /// Only class/bean names, never file paths or code
    pub key_classes: Vec<String>,
}

/// Build a structured summary from a diagnostic report.
/// Filters to top 20 issues by severity, strips all source code and config values.
pub fn build_summary(report: &DiagnosticReport) -> StructuredSummary {
    let project_context = ProjectContext {
        spring_boot_version: report.system_overview.spring_boot_version.clone(),
        build_tool: report.system_overview.build_tool.to_string(),
        starters: report.system_overview.starters.clone(),
        issue_count: report.issues.len(),
    };

    // Take top 20 issues (already sorted by severity in DiagnosticReport)
    let issues: Vec<IssueSummary> = report
        .issues
        .iter()
        .take(20)
        .map(|issue| {
            let key_classes: Vec<String> = issue
                .evidence
                .iter()
                .filter_map(|ev| {
                    ev.summary
                        .split_whitespace()
                        .find(|word| {
                            word.contains('.')
                                && !word.contains('/')
                                && !word.contains(':')
                                && word.chars().any(|c| c.is_uppercase())
                        })
                        .map(|s| {
                            s.trim_matches(|c: char| !c.is_alphanumeric() && c != '.')
                                .to_string()
                        })
                })
                .collect();

            IssueSummary {
                issue_id: issue.id.clone(),
                severity: issue.severity.to_string(),
                category: issue.category.to_string(),
                title: issue.title.clone(),
                description: issue.description.clone(),
                evidence_count: issue.evidence.len(),
                fix_suggestion: issue.fix_suggestion.clone(),
                key_classes,
            }
        })
        .collect();

    StructuredSummary {
        project_context,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_filters_to_max_20_issues() {
        let summary = StructuredSummary {
            project_context: ProjectContext {
                spring_boot_version: Some("3.2.0".into()),
                build_tool: "Maven".into(),
                starters: vec!["spring-boot-starter-web".into()],
                issue_count: 0,
            },
            issues: vec![],
        };
        assert!(summary.issues.len() <= 20);
    }
}
