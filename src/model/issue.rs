use crate::evidence::Evidence;
use crate::model::{Category, Confidence, Severity};
use serde::{Deserialize, Serialize};

/// A single diagnostic finding.
///
/// Each issue represents one problem detected by a diagnostic rule.
/// Every issue MUST have at least one piece of evidence (Constitution SC-003).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Unique identifier, format: {CATEGORY}-{NNN} (e.g., "BEAN-001")
    pub id: String,
    /// Short title (≤120 characters recommended)
    pub title: String,
    pub severity: Severity,
    pub category: Category,
    /// Detailed description with file/class references
    pub description: String,
    /// Supporting evidence — MUST NOT be empty
    pub evidence: Vec<Evidence>,
    /// Fix suggestion (empty string if no specific suggestion)
    pub fix_suggestion: String,
    pub confidence: Confidence,
}

impl Issue {
    /// Create a new Issue. Returns None if evidence is empty.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        severity: Severity,
        category: Category,
        description: impl Into<String>,
        evidence: Vec<Evidence>,
        fix_suggestion: impl Into<String>,
        confidence: Confidence,
    ) -> Option<Self> {
        if evidence.is_empty() {
            return None;
        }
        Some(Self {
            id: id.into(),
            title: title.into(),
            severity,
            category,
            description: description.into(),
            evidence,
            fix_suggestion: fix_suggestion.into(),
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Evidence, EvidenceType, Reliability};

    #[test]
    fn issue_requires_evidence() {
        let issue = Issue::new(
            "BEAN-001",
            "Test issue",
            Severity::Error,
            Category::Bean,
            "A test issue",
            vec![],
            "Fix it",
            Confidence::High,
        );
        assert!(issue.is_none(), "Issue without evidence must return None");
    }

    #[test]
    fn issue_with_evidence_is_valid() {
        let evidence = Evidence::new(
            EvidenceType::SourceCode,
            "Test.java:10",
            "Test evidence",
            Reliability::Confirmed,
        );
        let issue = Issue::new(
            "BEAN-001",
            "Test issue",
            Severity::Warning,
            Category::Bean,
            "Description",
            vec![evidence],
            "Fix it",
            Confidence::High,
        );
        assert!(issue.is_some());
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }

    #[test]
    fn health_deduction_values() {
        assert_eq!(Severity::Error.deduction(), 10);
        assert_eq!(Severity::Warning.deduction(), 3);
        assert_eq!(Severity::Info.deduction(), 1);
    }
}
