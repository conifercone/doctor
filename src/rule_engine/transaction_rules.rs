use crate::error::DoctorResult;
use crate::evidence::Evidence;
use crate::model::{Category, Confidence, Issue, Severity, SystemModel};

/// Detect @Transactional usage patterns that may cause transaction invalidation.
pub fn detect_transaction_issues(
    _model: &SystemModel,
    evidence: &[Evidence],
) -> DoctorResult<Vec<Issue>> {
    let mut issues = Vec::new();
    let mut count = 0;

    for ev in evidence {
        if ev.summary.contains("@Transactional") {
            count += 1;
            // Check for common pitfalls based on evidence context
            if ev.summary.contains("private") || ev.summary.contains("protected") {
                let id = format!("TX-{:03}", count);
                let issue = Issue::new(
                    id,
                    "@Transactional on non-public method",
                    Severity::Warning,
                    Category::Transaction,
                    format!(
                        "@Transactional annotation found on non-public method at {}. Spring AOP proxies cannot intercept non-public methods.",
                        ev.source
                    ),
                    vec![ev.clone()],
                    "Move @Transactional to a public method, or use AspectJ weaving".to_string(),
                    Confidence::High,
                );
                if let Some(issue) = issue {
                    issues.push(issue);
                }
            }
        }
    }

    if issues.is_empty() {
        // If no specific issues found but @Transactional is present, note it was checked
        if count > 0 {
            let issue = Issue::new(
                "TX-000",
                "@Transactional usage verified",
                Severity::Info,
                Category::Transaction,
                format!(
                    "{count} @Transactional annotations found. No common pitfalls detected in static analysis."
                ),
                vec![Evidence::new(
                    crate::evidence::EvidenceType::SourceCode,
                    "Source scan".to_string(),
                    format!("{count} @Transactional annotations verified"),
                    crate::evidence::Reliability::Confirmed,
                )],
                "Ensure @Transactional methods are public and called from external classes"
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
