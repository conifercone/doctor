pub mod auto_config_rules;
pub mod bean_rules;
pub mod config_rules;
pub mod startup_rules;
pub mod transaction_rules;

use crate::error::DoctorResult;
use crate::evidence::Evidence;
use crate::model::{Issue, SystemModel};
use std::collections::HashMap;

/// Execute all diagnostic rules and compute the health score.
pub fn run_all_rules(model: &SystemModel, evidence: &[Evidence]) -> DoctorResult<DiagnoseOutput> {
    let mut all_issues = Vec::new();
    let mut rules_executed = 0;

    // Bean rules
    if let Ok(issues) = bean_rules::detect_missing_beans(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }
    if let Ok(issues) = bean_rules::detect_bean_conflicts(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }
    if let Ok(issues) = bean_rules::detect_circular_dependencies(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }

    // Auto-config rules
    if let Ok(issues) = auto_config_rules::detect_auto_config_failures(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }
    if let Ok(issues) = auto_config_rules::detect_conditional_failures(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }

    // Config rules
    if let Ok(issues) = config_rules::detect_config_conflicts(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }

    // Transaction rules
    if let Ok(issues) = transaction_rules::detect_transaction_issues(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }

    // Startup rules
    if let Ok(issues) = startup_rules::analyze_startup(model, evidence) {
        rules_executed += 1;
        all_issues.extend(issues);
    }

    // Sort by severity
    all_issues.sort_by(|a, b| b.severity.cmp(&a.severity));

    // Compute stats
    let mut severity_counts = HashMap::new();
    for issue in &all_issues {
        *severity_counts.entry(issue.severity.to_string()).or_insert(0) += 1;
    }

    let health_score = crate::model::report::DiagnosticReport::compute_health_score(&all_issues);

    Ok(DiagnoseOutput { issues: all_issues, health_score, rules_executed, severity_counts })
}

/// Output of the rule engine execution.
#[derive(Debug, Clone)]
pub struct DiagnoseOutput {
    pub issues: Vec<Issue>,
    pub health_score: u8,
    pub rules_executed: usize,
    pub severity_counts: HashMap<String, usize>,
}
