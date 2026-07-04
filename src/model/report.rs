use crate::model::Severity;
use crate::model::SystemOverview;
use crate::model::issue::Issue;
use crate::model::summary::DiagnosisSummary;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete result of a diagnostic execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub project_name: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    /// Health score 0-100. 100 = perfect, computed as:
    /// 100 - (10 * ERRORs) - (3 * WARNINGs) - (1 * INFOs), minimum 0.
    pub health_score: u8,
    pub system_overview: SystemOverview,
    /// Issues sorted by severity (ERROR > WARNING > INFO), then by category.
    pub issues: Vec<Issue>,
    pub summary: DiagnosisSummary,
}

impl DiagnosticReport {
    /// Create a new diagnostic report with computed health score.
    ///
    /// Issues are sorted by severity descending. Health score is computed
    /// as: 100 - 10*ERROR - 3*WARNING - 1*INFO, clamped to [0, 100].
    pub fn new(
        project_name: impl Into<String>,
        duration_ms: u64,
        system_overview: SystemOverview,
        mut issues: Vec<Issue>,
        summary: DiagnosisSummary,
    ) -> Self {
        // Sort by severity (ERROR first), then by category for determinism
        issues.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.category.to_string().cmp(&b.category.to_string()))
        });

        let health_score = Self::compute_health_score(&issues);

        Self {
            project_name: project_name.into(),
            timestamp: Utc::now(),
            duration_ms,
            health_score,
            system_overview,
            issues,
            summary,
        }
    }

    /// Compute health score: 100 - 10*ERROR - 3*WARNING - 1*INFO, min 0, max 100.
    pub fn compute_health_score(issues: &[Issue]) -> u8 {
        let deduction: u32 = issues.iter().map(|i| i.severity.deduction() as u32).sum();
        100u8.saturating_sub(deduction.min(100) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Evidence, EvidenceType, Reliability};
    use crate::model::summary::DiagnosisSummary;
    use crate::model::system_overview::{BuildTool, SystemOverview};
    use crate::model::{Category, Confidence, Issue, Severity};
    use std::collections::HashMap;

    fn make_issue(id: &str, severity: Severity) -> Issue {
        Issue::new(
            id,
            "Test",
            severity,
            Category::Bean,
            "desc",
            vec![Evidence::new(
                EvidenceType::SourceCode,
                "file:1",
                "summary",
                Reliability::Confirmed,
            )],
            "fix",
            Confidence::High,
        )
        .unwrap()
    }

    #[test]
    fn perfect_score_with_no_issues() {
        assert_eq!(DiagnosticReport::compute_health_score(&[]), 100);
    }

    #[test]
    fn score_deduction() {
        let issues = vec![
            make_issue("B-1", Severity::Error),   // -10
            make_issue("B-2", Severity::Warning), // -3
            make_issue("B-3", Severity::Info),    // -1
        ];
        assert_eq!(DiagnosticReport::compute_health_score(&issues), 86);
    }

    #[test]
    fn score_floor_at_zero() {
        let issues: Vec<_> =
            (0..15).map(|i| make_issue(&format!("B-{i}"), Severity::Error)).collect();
        assert_eq!(DiagnosticReport::compute_health_score(&issues), 0);
    }

    #[test]
    fn issues_sorted_by_severity() {
        let overview = SystemOverview::new(BuildTool::Maven, None, None, vec![], 1);
        let summary = DiagnosisSummary::new(0, 0, HashMap::new(), false);
        let issues = vec![
            make_issue("B-1", Severity::Info),
            make_issue("B-2", Severity::Error),
            make_issue("B-3", Severity::Warning),
        ];
        let report = DiagnosticReport::new("test", 0, overview, issues, summary);
        assert_eq!(report.issues[0].severity, Severity::Error);
        assert_eq!(report.issues[1].severity, Severity::Warning);
        assert_eq!(report.issues[2].severity, Severity::Info);
    }
}
