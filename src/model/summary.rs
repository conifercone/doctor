use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Execution summary for a diagnostic run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisSummary {
    pub total_rules_executed: usize,
    pub total_evidence_collected: usize,
    pub issues_by_severity: HashMap<String, usize>,
    pub runtime_sources_available: bool,
}

impl DiagnosisSummary {
    pub fn new(
        total_rules_executed: usize,
        total_evidence_collected: usize,
        issues_by_severity: HashMap<String, usize>,
        runtime_sources_available: bool,
    ) -> Self {
        Self {
            total_rules_executed,
            total_evidence_collected,
            issues_by_severity,
            runtime_sources_available,
        }
    }
}
